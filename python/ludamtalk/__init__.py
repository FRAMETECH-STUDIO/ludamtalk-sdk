# 루담톡 봇 SDK (Python 3.9 이상)
# 설치: pip install ludamtalk   (또는 https://ludamtalk.com/sdk/ludam.py 를 받아서 pip install websockets)
# 설명서: https://ludamtalk.com/developers
#
# import os, random, ludamtalk as ludam
# bot = ludam.Bot(token=os.environ["LUDAM_TOKEN"])
#
# @bot.event
# async def on_ready(me):
#     print(f"{me['name']} 온라인!")
#
# @bot.command("주사위", "주사위를 굴려요")
# async def dice(ctx):
#     await ctx.reply(f"🎲 **{random.randint(1, 6)}**")
#
# bot.run()
#
# AI 봇:  await ctx.typing(ai=True); answer = await ask_ai(ctx.args); await ctx.reply(answer, ai=True)
# 보안 봇: @bot.event
#          async def on_member_join(ctx):
#              if ctx.user.get("newAccount"): await ctx.timeout(10, "새 계정")
import asyncio
import json
import random
import re
import urllib.error
import urllib.request

try:
    import websockets
except ImportError:  # pragma: no cover
    websockets = None

API = "https://api.ludamtalk.com"
GATEWAY = "wss://api.ludamtalk.com/bot/gateway"
VERSION = "1.0.0"
_CMD_RE = re.compile(r"^[a-z0-9가-힣_]{1,20}$")


class LudamError(Exception):
    def __init__(self, status, code, message=None):
        super().__init__(message or f"루담 API 오류 ({status} {code})")
        self.status = status
        self.code = code


class Embed:
    """임베드 만들기 도우미: Embed(title="…", color=0x3b82f6).add_field("이름", "값", inline=True)"""

    def __init__(self, title=None, description=None, url=None, color=None):
        self.data = {}
        if title: self.data["title"] = title
        if description: self.data["description"] = description
        if url: self.data["url"] = url
        if color is not None: self.data["color"] = color

    def set_author(self, name): self.data["author"] = {"name": name}; return self
    def set_footer(self, text): self.data["footer"] = {"text": text}; return self
    def set_image(self, url): self.data["image"] = {"url": url}; return self
    def set_thumbnail(self, url): self.data["thumbnail"] = {"url": url}; return self

    def add_field(self, name, value, inline=False):
        self.data.setdefault("fields", []).append({"name": name, "value": value, "inline": inline})
        return self

    def to_dict(self): return dict(self.data)


def _payload(content=None, embed=None, embeds=None, ai=False):
    out = {"ai": True} if ai else {}
    if isinstance(content, Embed):
        embed, content = content, None
    if content:
        out["text"] = str(content)
    es = list(embeds or []) + ([embed] if embed else [])
    if es:
        out["embeds"] = [e.to_dict() if isinstance(e, Embed) else e for e in es]
    return out


class Context:
    """명령어·메시지 하나. ctx.reply("…"), ctx.send("…"), ctx.delete(), ctx.kick()"""

    def __init__(self, bot, d):
        self.bot = bot
        self.raw = d
        self.type = d.get("type")
        self.room = d.get("room") or {}
        self.user = d.get("user") or {}
        self.message = d.get("message") or {}
        self.text = d.get("text") or self.message.get("text") or ""
        self.command = d.get("command") or ""
        self.args = d.get("args") or ""
        self.argv = self.args.split() if self.args else []
        self.reply_token = d.get("replyToken") or ""

    async def reply(self, content=None, *, embed=None, embeds=None, ai=False):
        return await self.bot.api("reply", {"replyToken": self.reply_token, **_payload(content, embed, embeds, ai)})

    async def send(self, content=None, *, embed=None, embeds=None, ai=False):
        return await self.bot.send(self.room.get("id"), content, embed=embed, embeds=embeds, ai=ai)

    async def typing(self, ai=False):  # ⌨️ "입력 중" 10초 (ai=True면 "AI가 답을 쓰는 중")
        return await self.bot.typing(self.room.get("id"), ai)

    async def timeout(self, minutes=10, reason=""):  # 관리 권한: 잠깐 말 못 하게
        return await self.bot.timeout(self.room.get("id"), self.user.get("id"), minutes, reason)

    async def delete(self):  # 관리 권한
        return await self.bot.api("delete", {"room": self.room.get("id"), "message": self.message.get("id")})

    async def kick(self):  # 관리 권한
        return await self.bot.api("kick", {"room": self.room.get("id"), "user": self.user.get("id")})


class Bot:
    def __init__(self, token, sync_commands=True, reconnect=True):
        if not token or not str(token).startswith("ldb_"):
            raise ValueError("봇 열쇠(token)가 필요해요. 개발자 사이트에서 받은 ldb_… 값을 넣어 주세요.")
        self.token = token
        self.sync_commands = sync_commands
        self.reconnect = reconnect
        self.commands = {}  # 이름 → (설명, 함수)
        self.events = {}  # on_ready · on_message · on_command · on_install · on_uninstall · on_error
        self.user = None
        self._closed = False

    # @bot.event 로 on_ready 같은 함수를 등록해요
    def event(self, fn):
        self.events[fn.__name__] = fn
        return fn

    # @bot.command("이름", "설명")
    def command(self, name, description=""):
        n = str(name).lstrip("/").lower()
        if not _CMD_RE.match(n):
            raise ValueError(f"명령어 이름이 규칙에 맞지 않아요: {name} (한글·영어 소문자·숫자·_ 20자까지)")

        def deco(fn):
            self.commands[n] = (str(description)[:60], fn)
            return fn

        return deco

    def _post(self, path, body):
        req = urllib.request.Request(
            f"{API}/bot/api/{path}", data=json.dumps(body).encode("utf-8"), method="POST",
            headers={"Authorization": f"Bot {self.token}", "Content-Type": "application/json", "User-Agent": f"LudamBotPy/{VERSION}"},
        )
        try:
            with urllib.request.urlopen(req, timeout=15) as r:
                return json.loads(r.read().decode("utf-8") or "{}")
        except urllib.error.HTTPError as e:
            try:
                data = json.loads(e.read().decode("utf-8") or "{}")
            except Exception:
                data = {}
            raise LudamError(e.code, data.get("error", "error"), data.get("error_description")) from None

    async def api(self, path, body=None):
        return await asyncio.to_thread(self._post, path, body or {})

    async def me(self):
        return await self.api("me")

    async def send(self, room_id, content=None, *, embed=None, embeds=None, ai=False):
        return await self.api("send", {"room": room_id, **_payload(content, embed, embeds, ai)})

    async def typing(self, room_id, ai=False):
        return await self.api("typing", {"room": room_id, "ai": bool(ai)})

    # 🛡️ 보안 봇 (관리 권한)
    async def members(self, room_id):
        return await self.api("members", {"room": room_id})

    async def timeout(self, room_id, user_id, minutes=10, reason=""):
        return await self.api("timeout", {"room": room_id, "user": user_id, "minutes": minutes, "reason": reason})

    async def untimeout(self, room_id, user_id):
        return await self.api("untimeout", {"room": room_id, "user": user_id})

    async def lock(self, room_id, reason=""):
        return await self.api("lock", {"room": room_id, "locked": True, "reason": reason})

    async def unlock(self, room_id):
        return await self.api("lock", {"room": room_id, "locked": False})

    async def purge(self, room_id, message_ids):
        return await self.api("purge", {"room": room_id, "messages": list(message_ids)[:100]})

    async def unban(self, room_id, user_id):
        return await self.api("unban", {"room": room_id, "user": user_id})

    async def rooms(self):
        out, cursor = [], ""
        while True:
            r = await self.api("rooms", {"cursor": cursor} if cursor else {})
            out += r.get("rooms", [])
            cursor = r.get("cursor") or ""
            if not cursor:
                return out

    async def _emit(self, name, *args):
        fn = self.events.get(name)
        if fn:
            try:
                await fn(*args)
            except Exception as e:  # 봇 코드 오류로 연결이 끊기지 않게
                await self._error(e)

    async def _error(self, e):
        fn = self.events.get("on_error")
        if fn:
            await fn(e)
        else:
            print("[루담 봇]", e)

    async def _dispatch(self, d):
        ctx = Context(self, d)
        t = d.get("type")
        if t == "command":
            await self._emit("on_command", ctx)
            c = self.commands.get(str(d.get("command") or "").lower())
            if c:
                try:
                    await c[1](ctx)
                except Exception as e:
                    await self._error(e)
        elif t in ("message", "install", "uninstall", "ping", "member_join", "member_leave"):
            await self._emit(f"on_{t}", ctx)

    def _connect(self):
        headers = {"Authorization": f"Bot {self.token}", "User-Agent": f"LudamBotPy/{VERSION}"}
        try:  # websockets 14 이상
            return websockets.connect(GATEWAY, additional_headers=headers, ping_interval=None)
        except TypeError:  # 예전 버전
            return websockets.connect(GATEWAY, extra_headers=headers, ping_interval=None)

    async def start(self):
        if websockets is None:
            raise RuntimeError("websockets 패키지가 필요해요: pip install websockets")
        if self.sync_commands and self.commands:
            try:
                await self.api("commands", {"commands": [{"name": n, "description": c[0]} for n, c in self.commands.items()]})
            except LudamError as e:
                print("[루담 봇] 명령어 맞추기 실패:", e)
        tries = 0
        while not self._closed:
            try:
                async with self._connect() as ws:
                    beat = None
                    async for raw in ws:
                        try:
                            m = json.loads(raw)
                        except ValueError:
                            continue
                        if m.get("op") == "ready":
                            tries = 0
                            self.user = m.get("bot")
                            gap = (m.get("heartbeat") or 30000) / 1000

                            async def heartbeat():
                                while True:
                                    await asyncio.sleep(gap)
                                    await ws.send('{"op":"ping"}')

                            beat = asyncio.create_task(heartbeat())
                            await self._emit("on_ready", self.user)
                        elif m.get("op") == "event" and m.get("d"):
                            asyncio.create_task(self._dispatch(m["d"]))
                    if beat:
                        beat.cancel()
                    if getattr(ws, "close_code", None) == 4004:  # 열쇠를 새로 받아서 끊김
                        self._closed = True
            except Exception as e:
                status = getattr(getattr(e, "response", None), "status_code", None) or getattr(e, "status_code", None)
                if status == 401:
                    print("[루담 봇] 봇 열쇠가 틀렸어요. 개발자 사이트에서 확인해 주세요.")
                    return
                await self._error(e)
            if self._closed or not self.reconnect:
                return
            await asyncio.sleep(min(60, 2 * 2 ** min(tries, 5)) + random.random())
            tries += 1

    def run(self):
        """봇 켜기 (Ctrl+C로 끄기)"""
        try:
            asyncio.run(self.start())
        except KeyboardInterrupt:
            print("[루담 봇] 껐어요")

    def close(self):
        self._closed = True
