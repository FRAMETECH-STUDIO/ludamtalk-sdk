// 루담톡 봇 SDK (JavaScript · Node.js 18 이상)
// 설치: npm i ludamtalk   (또는 https://ludamtalk.com/sdk/ludam.js 를 받아서 npm i ws)
// 설명서: https://ludamtalk.com/developers
//
// const { Client } = require("ludamtalk");   // TypeScript: import { Client } from "ludamtalk";
// const bot = new Client({ token: process.env.LUDAM_TOKEN });
// bot.on("ready", (me) => console.log(`${me.name} 온라인!`));
// bot.command("주사위", "주사위를 굴려요", async (ctx) => {
//   await ctx.reply(`🎲 **${1 + Math.floor(Math.random() * 6)}**`);
// });
// bot.login();
//
// AI 봇:  await ctx.typing(true); const answer = await askAI(ctx.args); await ctx.reply({ text: answer, ai: true });
// 보안 봇: bot.on("memberJoin", (ctx) => { if (ctx.user.newAccount) ctx.timeout(10, "새 계정"); });
"use strict";
const EventEmitter = require("events");
let WS;
try { WS = require("ws"); } catch { WS = null; }

const API = "https://api.ludamtalk.com";
const GATEWAY = "wss://api.ludamtalk.com/bot/gateway";
const VERSION = "1.0.0";

class LudamError extends Error {
  constructor(status, code, message) {
    super(message || `루담 API 오류 (${status} ${code})`);
    this.status = status;
    this.code = code;
  }
}

// 임베드 만들기 도우미 (new Embed().title("…").color(0x3b82f6) …)
class Embed {
  constructor(data = {}) { this.data = { ...data }; }
  title(t) { this.data.title = t; return this; }
  description(t) { this.data.description = t; return this; }
  url(u) { this.data.url = u; return this; }
  color(c) { this.data.color = c; return this; }
  author(name) { this.data.author = { name }; return this; }
  footer(text) { this.data.footer = { text }; return this; }
  image(url) { this.data.image = { url }; return this; }
  thumbnail(url) { this.data.thumbnail = { url }; return this; }
  field(name, value, inline = false) { (this.data.fields ||= []).push({ name, value, inline }); return this; }
  toJSON() { return this.data; }
}

// 보낼 내용 맞추기: 글자 · { text, embeds } · Embed
function payload(content) {
  if (content instanceof Embed) return { embeds: [content.toJSON()] };
  if (typeof content === "string") return { text: content };
  const out = { ...content };
  if (out.embeds) out.embeds = out.embeds.map((e) => (e instanceof Embed ? e.toJSON() : e));
  return out;
}

// 명령어·메시지 하나 (ctx)
class Context {
  constructor(client, d) {
    this.client = client;
    this.raw = d;
    this.type = d.type;
    this.room = d.room || {};
    this.user = d.user || {};
    this.message = d.message || {};
    this.text = d.text || (d.message && d.message.text) || "";
    this.command = d.command || "";
    this.args = d.args || "";
    this.argv = this.args ? this.args.trim().split(/\s+/) : [];
    this.replyToken = d.replyToken || "";
  }
  // 이 메시지에 대답 (15분 안, 5개까지)
  reply(content) { return this.client.api("reply", { replyToken: this.replyToken, ...payload(content) }); }
  // 이 방에 보내기 (1분 20개까지)
  send(content) { return this.client.send(this.room.id, content); }
  // ⌨️ "입력 중" 표시 (10초). ai: true 면 "AI가 답을 쓰는 중"으로 보여요
  typing(ai = false) { return this.client.typing(this.room.id, ai); }
  // (관리 권한) 이 메시지 지우기 · 보낸 사람 내보내기 · 잠깐 말 못 하게
  delete() { return this.client.api("delete", { room: this.room.id, message: this.message.id }); }
  kick() { return this.client.api("kick", { room: this.room.id, user: this.user.id }); }
  timeout(minutes = 10, reason = "") { return this.client.timeout(this.room.id, this.user.id, minutes, reason); }
}

class Client extends EventEmitter {
  constructor({ token, syncCommands = true, reconnect = true } = {}) {
    super();
    if (!token || !/^ldb_/.test(token)) throw new Error("봇 열쇠(token)가 필요해요. 개발자 사이트에서 받은 ldb_… 값을 넣어 주세요.");
    this.token = token;
    this.syncCommands = syncCommands;
    this.reconnect = reconnect;
    this.commands = new Map(); // 이름 → { description, run }
    this.user = null;
    this.ws = null;
    this._beat = null;
    this._tries = 0;
    this._closed = false;
  }

  // 명령어 등록: bot.command("이름", "설명", async (ctx) => {...})
  command(name, description, run) {
    if (typeof description === "function") { run = description; description = ""; }
    const n = String(name).replace(/^\//, "").toLowerCase();
    if (!/^[a-z0-9가-힣_]{1,20}$/.test(n)) throw new Error(`명령어 이름이 규칙에 맞지 않아요: ${name} (한글·영어 소문자·숫자·_ 20자까지)`);
    this.commands.set(n, { description: String(description || "").slice(0, 60), run });
    return this;
  }

  async api(path, body = {}) {
    const r = await fetch(`${API}/bot/api/${path}`, {
      method: "POST",
      headers: { Authorization: `Bot ${this.token}`, "Content-Type": "application/json", "User-Agent": `LudamBotJS/${VERSION}` },
      body: JSON.stringify(body),
    });
    let data = {};
    try { data = await r.json(); } catch {}
    if (!r.ok) throw new LudamError(r.status, data.error || "error", data.error_description);
    return data;
  }
  me() { return this.api("me"); }
  // ⌨️ 입력 중 표시 (AI면 ai: true)
  typing(roomId, ai = false) { return this.api("typing", { room: roomId, ai: !!ai }); }
  // 🛡️ 보안 봇 (관리 권한)
  members(roomId) { return this.api("members", { room: roomId }); }
  timeout(roomId, userId, minutes = 10, reason = "") { return this.api("timeout", { room: roomId, user: userId, minutes, reason }); }
  untimeout(roomId, userId) { return this.api("untimeout", { room: roomId, user: userId }); }
  lock(roomId, reason = "") { return this.api("lock", { room: roomId, locked: true, reason }); }
  unlock(roomId) { return this.api("lock", { room: roomId, locked: false }); }
  purge(roomId, messageIds) { return this.api("purge", { room: roomId, messages: messageIds }); }
  unban(roomId, userId) { return this.api("unban", { room: roomId, user: userId }); }
  send(roomId, content) { return this.api("send", { room: roomId, ...payload(content) }); }
  async rooms() {
    const all = [];
    let cursor = "";
    do {
      const r = await this.api("rooms", cursor ? { cursor } : {});
      all.push(...(r.rooms || []));
      cursor = r.cursor || "";
    } while (cursor);
    return all;
  }

  async login() {
    if (!WS) throw new Error("ws 패키지가 필요해요: npm i ws");
    this._closed = false;
    if (this.syncCommands && this.commands.size) {
      try {
        await this.api("commands", { commands: [...this.commands].map(([name, c]) => ({ name, description: c.description })) });
      } catch (e) { this.emit("warn", `명령어 맞추기 실패: ${e.message}`); }
    }
    this._connect();
    return this;
  }

  _connect() {
    const ws = new WS(GATEWAY, { headers: { Authorization: `Bot ${this.token}`, "User-Agent": `LudamBotJS/${VERSION}` } });
    this.ws = ws;
    ws.on("message", (raw) => {
      let m;
      try { m = JSON.parse(raw.toString()); } catch { return; }
      if (m.op === "ready") {
        this._tries = 0;
        this.user = m.bot;
        clearInterval(this._beat);
        this._beat = setInterval(() => { try { ws.send('{"op":"ping"}'); } catch {} }, m.heartbeat || 30000);
        this.emit("ready", m.bot);
      } else if (m.op === "event" && m.d) {
        this._dispatch(m.d).catch((e) => this._err(e));
      }
    });
    ws.on("unexpected-response", (_req, res) => {
      const msg = { 401: "봇 열쇠가 틀렸어요", 403: "아직 승인되지 않았거나 멈춘 봇이에요", 429: "연결 시도가 너무 많아요" }[res.statusCode] || `연결 거절 (${res.statusCode})`;
      this._err(new LudamError(res.statusCode, "gateway", msg));
      if (res.statusCode === 401) this._closed = true; // 열쇠가 틀리면 다시 시도하지 않아요
      try { _req.destroy(); } catch {}
      this._retry(res.statusCode);
    });
    ws.on("error", (e) => this._err(e));
    ws.on("close", (code) => {
      if (code === 4004) this._closed = true; // 열쇠를 새로 받아서 끊긴 경우
      this._retry(code);
    });
  }

  // 끊기면 기다렸다가 다시 연결 (2초 → 최대 1분). 열쇠가 틀렸으면 멈춰요
  _retry(code) {
    clearInterval(this._beat);
    if (this._waiting) return;
    this.emit("disconnect", code);
    if (this._closed || !this.reconnect) return;
    this._waiting = true;
    const wait = Math.min(60000, 2000 * 2 ** Math.min(this._tries++, 5)) + Math.floor(Math.random() * 1000);
    setTimeout(() => { this._waiting = false; this._connect(); }, wait);
  }

  async _dispatch(d) {
    const ctx = new Context(this, d);
    this.emit("raw", d);
    if (d.type === "command") {
      const c = this.commands.get(String(d.command || "").toLowerCase());
      this.emit("command", ctx);
      if (c) await c.run(ctx);
    } else if (d.type === "message") this.emit("message", ctx);
    else if (d.type === "member_join") this.emit("memberJoin", ctx);
    else if (d.type === "member_leave") this.emit("memberLeave", ctx);
    else if (d.type === "install") this.emit("install", ctx);
    else if (d.type === "uninstall") this.emit("uninstall", ctx);
    else if (d.type === "ping") this.emit("ping", ctx);
  }

  // "error" 듣는 곳이 없으면 프로그램이 꺼지지 않게 화면에만 보여 줘요
  _err(e) {
    if (this.listenerCount("error")) this.emit("error", e);
    else console.error("[루담 봇]", e && e.message ? e.message : e);
  }

  destroy() {
    this._closed = true;
    clearInterval(this._beat);
    try { this.ws && this.ws.close(1000); } catch {}
  }
}

module.exports = { Client, Embed, LudamError, VERSION };
