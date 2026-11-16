# ludamtalk

루담톡(LudamTalk) 온라인채팅 봇을 만드는 공식 도구예요. 자바스크립트·TypeScript에서 쓸 수 있어요.

- 설명서: https://ludamtalk.com/developers
- 봇 만들기(열쇠 받기): https://web.ludamtalk.com/dev

## 설치

```bash
npm i ludamtalk
```

Node.js 18 이상이 필요해요.

## 예제

```js
const { Client, Embed } = require("ludamtalk");
const bot = new Client({ token: process.env.LUDAM_TOKEN });

bot.on("ready", (me) => console.log(me.name + " 온라인!"));

bot.command("주사위", "주사위를 굴려요", async (ctx) => {
  const n = 1 + Math.floor(Math.random() * 6);
  await ctx.reply(`🎲 **${n}** 이(가) 나왔어요!`);
});

bot.command("날씨", "오늘 날씨", async (ctx) => {
  await ctx.reply(new Embed().title("오늘의 날씨").color(0x3b82f6).description("**맑음** · 최고 18°C"));
});

bot.login();
```

TypeScript: `import { Client, Embed } from "ludamtalk";`

켜기: `LUDAM_TOKEN=ldb_… node bot.js` (윈도우 PowerShell: `$env:LUDAM_TOKEN="ldb_…"; node bot.js`)

봇 열쇠(ldb_…)는 비밀번호처럼 다뤄 주세요. 코드에 바로 적거나 다른 사람에게 보여 주지 마세요.

## 라이선스

MIT © FRAMETECH STUDIO
