# ludamtalk

루담톡(LudamTalk) 온라인채팅 봇을 만드는 공식 Rust 도구예요.

- 설명서: https://ludamtalk.com/developers
- 봇 만들기(열쇠 받기): https://web.ludamtalk.com/dev

```bash
cargo add ludamtalk tokio --features tokio/full
```

```rust
use ludamtalk::{Bot, Content};

#[tokio::main]
async fn main() {
    let mut bot = Bot::new(std::env::var("LUDAM_TOKEN").unwrap());
    bot.command("주사위", "주사위를 굴려요", |ctx| async move {
        ctx.reply(Content::text("🎲 4")).await.map(|_| ())
    });
    bot.run().await.unwrap();
}
```

윈도우에서는 rustup 설치 때 나오는 C++ 빌드 도구도 같이 설치해 주세요.

MIT © FRAMETECH STUDIO
