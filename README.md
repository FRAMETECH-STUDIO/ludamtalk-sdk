# LudamTalk Bot SDK

**English** · [한국어](README.ko.md)

The official SDKs for building bots for [LudamTalk](https://ludamtalk.com/en) open chat rooms.

- Docs: https://ludamtalk.com/developers (Korean for now)
- Create a bot and get its token: https://web.ludamtalk.com/dev

| Language | Install | Folder |
| --- | --- | --- |
| JavaScript · TypeScript | `npm i ludamtalk` | [js](js) |
| Python | `pip install ludamtalk` | [python](python) |
| Go | `go get ludamtalk.com/sdk/go` | [go](go) |
| Rust | `cargo add ludamtalk` | [rust](rust) |
| Kotlin · Java | JitPack (below) | [kotlin](kotlin) |

What you get in every language: a real-time gateway connection with auto-reconnect, `/commands` registered from your code, replies with Markdown and embeds, a typing indicator (including "AI is typing"), an AI label for AI-generated answers, and moderation tools (delete, kick, timeout, room lock, purge).

```js
const { Client } = require("ludamtalk");
const bot = new Client({ token: process.env.LUDAM_TOKEN });
bot.command("hello", "Say hi", (ctx) => ctx.reply(`Hi, **${ctx.user.name}**!`));
bot.login();
```

More examples: [ludamtalk-bot-examples](https://github.com/FRAMETECH-STUDIO/ludamtalk-bot-examples)

### Kotlin · Java (Gradle)

```kotlin
repositories { maven("https://jitpack.io") }
dependencies { implementation("com.github.FRAMETECH-STUDIO:ludamtalk-sdk:v1.0.0") }
```

### Security

Found a security issue? Please email help@ludamtalk.com instead of opening a public issue.

### License

MIT © FRAMETECH STUDIO
