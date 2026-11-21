# 루담톡 봇 SDK

[English](README.md) · **한국어**

루담톡(LudamTalk) 온라인채팅 봇을 만드는 공식 도구 모음이에요.

- 설명서: https://ludamtalk.com/developers
- 봇 만들기(열쇠 받기): https://web.ludamtalk.com/dev

| 언어 | 설치 | 폴더 |
| --- | --- | --- |
| 자바스크립트 · TypeScript | `npm i ludamtalk` | [js](js) |
| 파이썬 | `pip install ludamtalk` | [python](python) |
| Go | `go get ludamtalk.com/sdk/go` | [go](go) |
| Rust | `cargo add ludamtalk` | [rust](rust) |
| Kotlin · Java | JitPack (아래) | [kotlin](kotlin) |

모든 언어에서 똑같이 쓸 수 있어요: 끊기면 다시 붙는 실시간 연결(게이트웨이), 코드에 적은 `/명령어` 자동 등록, 마크다운·임베드 대답, "입력 중"(AI면 "AI가 답을 쓰는 중") 표시, AI 답 표시, 보안 봇 도구(지우기·내보내기·잠깐 말 막기·방 잠그기·한꺼번에 지우기).

```js
const { Client } = require("ludamtalk");
const bot = new Client({ token: process.env.LUDAM_TOKEN });
bot.command("안녕", "인사해요", (ctx) => ctx.reply(`안녕하세요, **${ctx.user.name}**님!`));
bot.login();
```

더 많은 예제: [ludamtalk-bot-examples](https://github.com/FRAMETECH-STUDIO/ludamtalk-bot-examples)

### Kotlin · Java (Gradle)

```kotlin
repositories { maven("https://jitpack.io") }
dependencies { implementation("com.github.FRAMETECH-STUDIO:ludamtalk-sdk:v1.0.0") }
```

### 보안 문제 알리기

보안 문제를 찾으면 공개 이슈 대신 help@ludamtalk.com 으로 알려 주세요.

### 라이선스

MIT © FRAMETECH STUDIO
