// 루담톡 봇 SDK 타입 정의 (TypeScript) — https://ludamtalk.com/developers
import { EventEmitter } from "events";

export declare const VERSION: string;

export declare class LudamError extends Error {
  status: number;
  code: string;
  constructor(status: number, code: string, message?: string);
}

export interface EmbedField { name: string; value: string; inline?: boolean }
export interface EmbedData {
  title?: string;
  description?: string;
  url?: string;
  /** 왼쪽 띠 색 (0xRRGGBB) */
  color?: number;
  author?: { name: string };
  footer?: { text: string };
  image?: { url: string };
  thumbnail?: { url: string };
  /** 10개까지 */
  fields?: EmbedField[];
}

export declare class Embed {
  data: EmbedData;
  constructor(data?: EmbedData);
  title(t: string): this;
  description(t: string): this;
  url(u: string): this;
  color(c: number): this;
  author(name: string): this;
  footer(text: string): this;
  image(url: string): this;
  thumbnail(url: string): this;
  field(name: string, value: string, inline?: boolean): this;
  toJSON(): EmbedData;
}

/** 보낼 내용: 글자(마크다운) · 임베드 · { text, embeds, ai } */
export type Content = string | Embed | { text?: string; embeds?: Array<Embed | EmbedData>; /** AI로 만든 답이면 true ("✨ AI" 표시) */ ai?: boolean };

export interface Room { id: string; title?: string }
export interface User {
  /** 방마다 다른 번호 (u + 16자리) */
  id: string;
  /** 그 방 닉네임 */
  name: string;
  /** member_join 때만: 만든 지 7일 안 된 계정 */
  newAccount?: boolean;
}
export interface Message { id: string; text?: string; mentions?: number; reply?: boolean }
export type EventType = "command" | "message" | "install" | "uninstall" | "ping" | "member_join" | "member_leave";
export interface RawEvent {
  type: EventType;
  bot: { id: string; name: string };
  room?: Room;
  user?: User;
  message?: Message;
  text?: string;
  command?: string;
  args?: string;
  replyToken?: string;
  at: string;
}
export interface SendResult { ok: true; id: string }

export declare class Context {
  client: Client;
  raw: RawEvent;
  type: EventType;
  room: Room;
  user: User;
  message: Message;
  text: string;
  command: string;
  args: string;
  argv: string[];
  replyToken: string;
  /** 이 메시지에 대답 (15분 안, 5개까지) */
  reply(content: Content): Promise<SendResult>;
  /** 이 방에 보내기 (1분 20개까지) */
  send(content: Content): Promise<SendResult>;
  /** "입력 중" 10초 (ai면 "AI가 답을 쓰는 중") */
  typing(ai?: boolean): Promise<unknown>;
  /** 관리 권한 */
  delete(): Promise<unknown>;
  kick(): Promise<unknown>;
  timeout(minutes?: number, reason?: string): Promise<unknown>;
}

export interface BotInfo {
  id: string;
  name: string;
  desc: string;
  cmds: Array<{ c: string; d: string }>;
  status: "pending" | "approved" | "suspended";
  intents: { messages: boolean; manage: boolean };
  contact?: string;
  connected?: number;
}
export interface RoomMember { id: string; name: string; role: "owner" | "sub" | "member"; timeoutUntil?: string }

export interface ClientOptions {
  /** 개발자 사이트에서 받은 봇 열쇠 (ldb_…) */
  token: string;
  /** 켤 때 command()로 적은 명령어를 서버에 등록 (기본 true) */
  syncCommands?: boolean;
  /** 끊기면 다시 연결 (기본 true) */
  reconnect?: boolean;
}

export interface ClientEvents {
  ready: [bot: { id: string; name: string }];
  command: [ctx: Context];
  message: [ctx: Context];
  memberJoin: [ctx: Context];
  memberLeave: [ctx: Context];
  install: [ctx: Context];
  uninstall: [ctx: Context];
  ping: [ctx: Context];
  raw: [event: RawEvent];
  disconnect: [code: number];
  warn: [message: string];
  error: [error: Error];
}

export declare class Client extends EventEmitter {
  constructor(options: ClientOptions);
  token: string;
  user: { id: string; name: string } | null;
  on<K extends keyof ClientEvents>(event: K, listener: (...args: ClientEvents[K]) => void): this;
  once<K extends keyof ClientEvents>(event: K, listener: (...args: ClientEvents[K]) => void): this;
  /** 명령어: bot.command("이름", "설명", async (ctx) => {...}) */
  command(name: string, description: string, run: (ctx: Context) => unknown): this;
  command(name: string, run: (ctx: Context) => unknown): this;
  login(): Promise<this>;
  destroy(): void;
  api<T = any>(path: string, body?: object): Promise<T>;
  me(): Promise<BotInfo>;
  send(roomId: string, content: Content): Promise<SendResult>;
  rooms(): Promise<Array<{ id: string; title: string; members: number }>>;
  typing(roomId: string, ai?: boolean): Promise<unknown>;
  members(roomId: string): Promise<{ members: RoomMember[]; locked: boolean }>;
  timeout(roomId: string, userId: string, minutes?: number, reason?: string): Promise<{ ok: true; until: string }>;
  untimeout(roomId: string, userId: string): Promise<unknown>;
  lock(roomId: string, reason?: string): Promise<unknown>;
  unlock(roomId: string): Promise<unknown>;
  purge(roomId: string, messageIds: string[]): Promise<{ ok: true; deleted: number }>;
  unban(roomId: string, userId: string): Promise<unknown>;
}
