//! 루담톡(LudamTalk) 온라인채팅 봇 SDK
//!
//! 설명서: <https://ludamtalk.com/developers>
//!
//! ```no_run
//! use ludamtalk::{Bot, Content};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ludamtalk::Error> {
//!     let mut bot = Bot::new(std::env::var("LUDAM_TOKEN").unwrap_or_default());
//!     bot.on_ready(|me| println!("{} 온라인!", me.name));
//!     bot.command("주사위", "주사위를 굴려요", |ctx| async move {
//!         ctx.reply(Content::text("🎲 **4**")).await?;
//!         Ok(())
//!     });
//!     bot.run().await
//! }
//! ```
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub const API: &str = "https://api.ludamtalk.com";
pub const GATEWAY: &str = "wss://api.ludamtalk.com/bot/gateway";
pub const VERSION: &str = "1.0.0";

/// 루담 봇 오류
#[derive(Debug)]
pub enum Error {
    /// 봇 열쇠가 틀렸어요 (다시 연결하지 않아요)
    BadToken,
    /// 루담 API가 거절했어요 (HTTP 번호, 오류 이름)
    Api(u16, String),
    /// 인터넷·연결 오류
    Network(String),
    /// 그 밖의 오류
    Other(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadToken => write!(f, "봇 열쇠가 틀렸어요. 개발자 사이트에서 확인해 주세요"),
            Error::Api(s, c) => write!(f, "루담 API 오류 ({s} {c})"),
            Error::Network(e) => write!(f, "연결 오류: {e}"),
            Error::Other(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self { Error::Network(e.to_string()) }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

/// 꾸민 카드 (5개까지, 칸 10개까지)
#[derive(Debug, Clone, Serialize, Default)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 왼쪽 띠 색 (0xRRGGBB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
}
impl Embed {
    pub fn new() -> Self { Self::default() }
    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }
    pub fn description(mut self, t: impl Into<String>) -> Self { self.description = Some(t.into()); self }
    pub fn url(mut self, u: impl Into<String>) -> Self { self.url = Some(u.into()); self }
    pub fn color(mut self, c: u32) -> Self { self.color = Some(c); self }
    pub fn author(mut self, n: impl Into<String>) -> Self { self.author = Some(json!({ "name": n.into() })); self }
    pub fn footer(mut self, t: impl Into<String>) -> Self { self.footer = Some(json!({ "text": t.into() })); self }
    pub fn image(mut self, u: impl Into<String>) -> Self { self.image = Some(json!({ "url": u.into() })); self }
    pub fn thumbnail(mut self, u: impl Into<String>) -> Self { self.thumbnail = Some(json!({ "url": u.into() })); self }
    pub fn field(mut self, n: impl Into<String>, v: impl Into<String>, inline: bool) -> Self {
        self.fields.push(EmbedField { name: n.into(), value: v.into(), inline });
        self
    }
}

/// 보낼 내용 (글자는 마크다운 가능). AI로 만든 답이면 `.ai()` → "✨ AI" 표시
#[derive(Debug, Clone, Serialize, Default)]
pub struct Content {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ai: bool,
}
impl Content {
    pub fn text(t: impl Into<String>) -> Self { Content { text: t.into(), ..Default::default() } }
    pub fn embed(e: Embed) -> Self { Content { embeds: vec![e], ..Default::default() } }
    pub fn with_embed(mut self, e: Embed) -> Self { self.embeds.push(e); self }
    pub fn ai(mut self) -> Self { self.ai = true; self }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BotUser {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Room {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
}
/// id: 방마다 다른 번호 (u+16자리) · name: 그 방 닉네임
#[derive(Debug, Clone, Deserialize, Default)]
pub struct User {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "newAccount")]
    pub new_account: bool,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Message {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub mentions: u32,
    #[serde(default)]
    pub reply: bool,
}
/// 게이트웨이 이벤트 (type: command · message · install · uninstall · ping · member_join · member_leave)
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Event {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub bot: BotUser,
    #[serde(default)]
    pub room: Room,
    #[serde(default)]
    pub user: User,
    #[serde(default)]
    pub message: Message,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: String,
    #[serde(default, rename = "replyToken")]
    pub reply_token: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    pub id: String,
    pub name: String,
    /// owner · sub · member
    pub role: String,
    #[serde(default, rename = "timeoutUntil")]
    pub timeout_until: Option<String>,
}

/// 봇 API (REST)를 부르는 부분 — Context와 Bot이 같이 써요
#[derive(Clone)]
pub struct Api {
    token: String,
    http: reqwest::Client,
}
impl Api {
    /// POST /bot/api/<path>
    pub async fn call(&self, path: &str, body: Value) -> Result<Value, Error> {
        let r = self
            .http
            .post(format!("{API}/bot/api/{path}"))
            .header("Authorization", format!("Bot {}", self.token))
            .header("User-Agent", format!("LudamBotRust/{VERSION}"))
            .json(&body)
            .send()
            .await?;
        let status = r.status().as_u16();
        let v: Value = r.json().await.unwrap_or(Value::Null);
        if !(200..300).contains(&status) {
            return Err(Error::Api(status, v.get("error").and_then(|x| x.as_str()).unwrap_or("error").to_string()));
        }
        Ok(v)
    }
    fn content(mut base: Value, c: &Content) -> Value {
        let extra = serde_json::to_value(c).unwrap_or(Value::Null);
        if let (Some(b), Some(e)) = (base.as_object_mut(), extra.as_object()) {
            for (k, v) in e { b.insert(k.clone(), v.clone()); }
        }
        base
    }
    pub async fn send(&self, room: &str, c: Content) -> Result<String, Error> {
        let v = self.call("send", Self::content(json!({ "room": room }), &c)).await?;
        Ok(v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string())
    }
    pub async fn typing(&self, room: &str, ai: bool) -> Result<(), Error> { self.call("typing", json!({ "room": room, "ai": ai })).await.map(|_| ()) }
    pub async fn members(&self, room: &str) -> Result<(Vec<Member>, bool), Error> {
        let v = self.call("members", json!({ "room": room })).await?;
        let m: Vec<Member> = serde_json::from_value(v.get("members").cloned().unwrap_or(json!([]))).unwrap_or_default();
        Ok((m, v.get("locked").and_then(|x| x.as_bool()).unwrap_or(false)))
    }
    pub async fn timeout(&self, room: &str, user: &str, minutes: u32, reason: &str) -> Result<(), Error> {
        self.call("timeout", json!({ "room": room, "user": user, "minutes": minutes, "reason": reason })).await.map(|_| ())
    }
    pub async fn untimeout(&self, room: &str, user: &str) -> Result<(), Error> { self.call("untimeout", json!({ "room": room, "user": user })).await.map(|_| ()) }
    pub async fn lock(&self, room: &str, locked: bool, reason: &str) -> Result<(), Error> {
        self.call("lock", json!({ "room": room, "locked": locked, "reason": reason })).await.map(|_| ())
    }
    pub async fn purge(&self, room: &str, ids: &[String]) -> Result<u64, Error> {
        let ids: Vec<&String> = ids.iter().take(100).collect();
        let v = self.call("purge", json!({ "room": room, "messages": ids })).await?;
        Ok(v.get("deleted").and_then(|x| x.as_u64()).unwrap_or(0))
    }
    pub async fn unban(&self, room: &str, user: &str) -> Result<(), Error> { self.call("unban", json!({ "room": room, "user": user })).await.map(|_| ()) }
}

/// 이벤트 하나 (ctx.reply / send / typing / delete / kick / timeout)
#[derive(Clone)]
pub struct Context {
    pub api: Api,
    pub event: Event,
    pub argv: Vec<String>,
}
impl Context {
    pub fn room(&self) -> &Room { &self.event.room }
    pub fn user(&self) -> &User { &self.event.user }
    pub fn text(&self) -> &str { if self.event.text.is_empty() { &self.event.message.text } else { &self.event.text } }
    /// 이 메시지에 대답 (15분 안, 5개까지)
    pub async fn reply(&self, c: Content) -> Result<String, Error> {
        let v = self.api.call("reply", Api::content(json!({ "replyToken": self.event.reply_token }), &c)).await?;
        Ok(v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string())
    }
    pub async fn send(&self, c: Content) -> Result<String, Error> { self.api.send(&self.event.room.id, c).await }
    /// "입력 중" 10초 (ai면 "AI가 답을 쓰는 중")
    pub async fn typing(&self, ai: bool) -> Result<(), Error> { self.api.typing(&self.event.room.id, ai).await }
    /// 관리 권한
    pub async fn delete(&self) -> Result<(), Error> {
        self.api.call("delete", json!({ "room": self.event.room.id, "message": self.event.message.id })).await.map(|_| ())
    }
    pub async fn kick(&self) -> Result<(), Error> {
        self.api.call("kick", json!({ "room": self.event.room.id, "user": self.event.user.id })).await.map(|_| ())
    }
    pub async fn timeout(&self, minutes: u32, reason: &str) -> Result<(), Error> {
        self.api.timeout(&self.event.room.id, &self.event.user.id, minutes, reason).await
    }
}

type BoxFut = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;
type Handler = Arc<dyn Fn(Context) -> BoxFut + Send + Sync>;

fn boxed<F, Fut>(f: F) -> Handler
where
    F: Fn(Context) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), Error>> + Send + 'static,
{
    Arc::new(move |c| Box::pin(f(c)))
}

/// 봇 하나
pub struct Bot {
    pub api: Api,
    /// 켤 때 명령어를 서버에 등록 (기본 true)
    pub sync_commands: bool,
    /// 끊기면 다시 연결 (기본 true)
    pub reconnect: bool,
    commands: HashMap<String, (String, Handler)>,
    handlers: HashMap<String, Vec<Handler>>,
    ready: Vec<Arc<dyn Fn(BotUser) + Send + Sync>>,
}

impl Bot {
    pub fn new(token: impl Into<String>) -> Self {
        Bot {
            api: Api { token: token.into(), http: reqwest::Client::builder().timeout(Duration::from_secs(20)).build().unwrap_or_default() },
            sync_commands: true,
            reconnect: true,
            commands: HashMap::new(),
            handlers: HashMap::new(),
            ready: Vec::new(),
        }
    }

    /// /명령어 등록 (한글·영어 소문자·숫자·_ 20자까지)
    pub fn command<F, Fut>(&mut self, name: &str, description: &str, f: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        let n = name.trim_start_matches('/').to_lowercase();
        let ok = !n.is_empty() && n.chars().count() <= 20 && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || ('가'..='힣').contains(&c));
        assert!(ok, "명령어 이름이 규칙에 맞지 않아요: {name}");
        self.commands.insert(n, (description.chars().take(60).collect(), boxed(f)));
    }
    fn on<F, Fut>(&mut self, kind: &str, f: F)
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Error>> + Send + 'static,
    {
        self.handlers.entry(kind.to_string()).or_default().push(boxed(f));
    }
    pub fn on_ready(&mut self, f: impl Fn(BotUser) + Send + Sync + 'static) { self.ready.push(Arc::new(f)); }
    /// "모든 메시지" 권한
    pub fn on_message<F, Fut>(&mut self, f: F) where F: Fn(Context) -> Fut + Send + Sync + 'static, Fut: Future<Output = Result<(), Error>> + Send + 'static { self.on("message", f) }
    pub fn on_member_join<F, Fut>(&mut self, f: F) where F: Fn(Context) -> Fut + Send + Sync + 'static, Fut: Future<Output = Result<(), Error>> + Send + 'static { self.on("member_join", f) }
    pub fn on_member_leave<F, Fut>(&mut self, f: F) where F: Fn(Context) -> Fut + Send + Sync + 'static, Fut: Future<Output = Result<(), Error>> + Send + 'static { self.on("member_leave", f) }
    pub fn on_install<F, Fut>(&mut self, f: F) where F: Fn(Context) -> Fut + Send + Sync + 'static, Fut: Future<Output = Result<(), Error>> + Send + 'static { self.on("install", f) }

    /// 봇 켜기: 명령어 등록 → 게이트웨이 연결 → 끊기면 다시 연결
    pub async fn run(self) -> Result<(), Error> {
        if !self.api.token.starts_with("ldb_") {
            return Err(Error::Other("봇 열쇠(token)가 필요해요. 개발자 사이트에서 받은 ldb_… 값을 넣어 주세요".into()));
        }
        if self.sync_commands && !self.commands.is_empty() {
            let list: Vec<Value> = self.commands.iter().map(|(n, (d, _))| json!({ "name": n, "description": d })).collect();
            if let Err(e) = self.api.call("commands", json!({ "commands": list })).await {
                eprintln!("[루담 봇] 명령어 맞추기 실패: {e}");
            }
        }
        let me = Arc::new(self);
        let mut tries: u32 = 0;
        loop {
            match me.clone().connect(&mut tries).await {
                Err(Error::BadToken) => return Err(Error::BadToken),
                Err(e) => eprintln!("[루담 봇] {e}"),
                Ok(()) => {}
            }
            if !me.reconnect { return Ok(()); }
            let secs = (2u64 << tries.min(5)).min(60);
            tries += 1;
            tokio::time::sleep(Duration::from_secs(secs)).await;
        }
    }

    async fn connect(self: Arc<Self>, tries: &mut u32) -> Result<(), Error> {
        let mut req = GATEWAY.into_client_request().map_err(|e| Error::Other(e.to_string()))?;
        let h = req.headers_mut();
        h.insert("Authorization", HeaderValue::from_str(&format!("Bot {}", self.api.token)).map_err(|e| Error::Other(e.to_string()))?);
        h.insert("User-Agent", HeaderValue::from_static("LudamBotRust/1.0.0"));
        let (ws, _) = match tokio_tungstenite::connect_async(req).await {
            Ok(x) => x,
            Err(tokio_tungstenite::tungstenite::Error::Http(r)) if r.status().as_u16() == 401 => return Err(Error::BadToken),
            Err(e) => return Err(Error::Network(e.to_string())),
        };
        let (mut tx, mut rx) = ws.split();
        let (ping_tx, mut ping_rx) = tokio::sync::mpsc::channel::<()>(4);
        let mut beat: Option<tokio::task::JoinHandle<()>> = None;
        loop {
            tokio::select! {
                _ = ping_rx.recv() => {
                    if tx.send(WsMessage::Text("{\"op\":\"ping\"}".into())).await.is_err() { break; }
                }
                msg = rx.next() => {
                    let Some(msg) = msg else { break };
                    let msg = match msg { Ok(m) => m, Err(e) => { if let Some(b) = beat { b.abort(); } return Err(Error::Network(e.to_string())); } };
                    match msg {
                        WsMessage::Close(f) => {
                            if let Some(b) = beat.take() { b.abort(); }
                            if f.map(|x| u16::from(x.code) == 4004).unwrap_or(false) { return Err(Error::BadToken); }
                            break;
                        }
                        WsMessage::Text(t) => {
                            let v: Value = match serde_json::from_str(&t) { Ok(v) => v, Err(_) => continue };
                            match v.get("op").and_then(|x| x.as_str()) {
                                Some("ready") => {
                                    *tries = 0;
                                    let gap = v.get("heartbeat").and_then(|x| x.as_u64()).unwrap_or(30000);
                                    let p = ping_tx.clone();
                                    if let Some(b) = beat.take() { b.abort(); }
                                    beat = Some(tokio::spawn(async move {
                                        loop { tokio::time::sleep(Duration::from_millis(gap)).await; if p.send(()).await.is_err() { break; } }
                                    }));
                                    let who: BotUser = serde_json::from_value(v.get("bot").cloned().unwrap_or(Value::Null)).unwrap_or_default();
                                    for f in &self.ready { f(who.clone()); }
                                }
                                Some("event") => {
                                    if let Ok(ev) = serde_json::from_value::<Event>(v.get("d").cloned().unwrap_or(Value::Null)) {
                                        let me = self.clone();
                                        tokio::spawn(async move { me.dispatch(ev).await });
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if let Some(b) = beat { b.abort(); }
        Ok(())
    }

    async fn dispatch(&self, ev: Event) {
        let ctx = Context { api: self.api.clone(), argv: ev.args.split_whitespace().map(String::from).collect(), event: ev };
        let kind = ctx.event.kind.clone();
        if let Some(hs) = self.handlers.get(&kind) {
            for h in hs {
                if let Err(e) = h(ctx.clone()).await { eprintln!("[루담 봇] {e}"); }
            }
        }
        if kind == "command" {
            if let Some((_, h)) = self.commands.get(&ctx.event.command.to_lowercase()) {
                if let Err(e) = h(ctx.clone()).await { eprintln!("[루담 봇] {e}"); }
            }
        }
    }
}
