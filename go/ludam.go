// Package ludamtalk 은 루담톡(LudamTalk) 온라인채팅 봇 SDK 예요.
//
// 설명서: https://ludamtalk.com/developers
// 필요한 것: Go 1.21 이상, github.com/coder/websocket
//
//	bot := ludamtalk.New(os.Getenv("LUDAM_TOKEN"))
//	bot.OnReady(func(me ludamtalk.BotUser) { log.Println(me.Name, "온라인!") })
//	bot.Command("주사위", "주사위를 굴려요", func(ctx *ludamtalk.Context) error {
//		_, err := ctx.Reply(ludamtalk.Text(fmt.Sprintf("🎲 **%d**", rand.Intn(6)+1)))
//		return err
//	})
//	log.Fatal(bot.Run(context.Background()))
package ludamtalk

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math/rand"
	"net/http"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/coder/websocket"
)

const (
	API     = "https://api.ludamtalk.com"
	Gateway = "wss://api.ludamtalk.com/bot/gateway"
	Version = "1.0.0"
)

var cmdRe = regexp.MustCompile(`^[a-z0-9가-힣_]{1,20}$`)

// Error 는 루담 API 오류예요 (Status: HTTP 번호, Code: 오류 이름).
type Error struct {
	Status int
	Code   string
}

func (e *Error) Error() string { return fmt.Sprintf("루담 API 오류 (%d %s)", e.Status, e.Code) }

// ErrBadToken: 봇 열쇠가 틀렸어요 (다시 연결하지 않아요).
var ErrBadToken = errors.New("봇 열쇠가 틀렸어요. 개발자 사이트에서 확인해 주세요")

type EmbedField struct {
	Name   string `json:"name"`
	Value  string `json:"value"`
	Inline bool   `json:"inline,omitempty"`
}
type named struct {
	Name string `json:"name,omitempty"`
	Text string `json:"text,omitempty"`
	URL  string `json:"url,omitempty"`
}

// Embed 는 꾸민 카드예요 (5개까지, 칸은 10개까지).
type Embed struct {
	Title       string       `json:"title,omitempty"`
	Description string       `json:"description,omitempty"`
	URL         string       `json:"url,omitempty"`
	Color       *int         `json:"color,omitempty"`
	Author      *named       `json:"author,omitempty"`
	Footer      *named       `json:"footer,omitempty"`
	Image       *named       `json:"image,omitempty"`
	Thumbnail   *named       `json:"thumbnail,omitempty"`
	Fields      []EmbedField `json:"fields,omitempty"`
}

func (e *Embed) SetColor(c int) *Embed        { e.Color = &c; return e }
func (e *Embed) SetAuthor(n string) *Embed    { e.Author = &named{Name: n}; return e }
func (e *Embed) SetFooter(t string) *Embed    { e.Footer = &named{Text: t}; return e }
func (e *Embed) SetImage(u string) *Embed     { e.Image = &named{URL: u}; return e }
func (e *Embed) SetThumbnail(u string) *Embed { e.Thumbnail = &named{URL: u}; return e }
func (e *Embed) AddField(n, v string, inline bool) *Embed {
	e.Fields = append(e.Fields, EmbedField{Name: n, Value: v, Inline: inline})
	return e
}

// Content 는 보낼 내용이에요. Text는 마크다운을 써도 돼요. AI로 만든 답이면 AI: true ("✨ AI" 표시).
type Content struct {
	Text   string  `json:"text,omitempty"`
	Embeds []Embed `json:"embeds,omitempty"`
	AI     bool    `json:"ai,omitempty"`
}

// Text 는 글자만 보낼 때 쓰는 줄임이에요.
func Text(s string) Content { return Content{Text: s} }

type BotUser struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}
type Room struct {
	ID    string `json:"id"`
	Title string `json:"title"`
}

// User: ID는 방마다 다른 번호(u+16자리), Name은 그 방 닉네임.
type User struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	NewAccount bool   `json:"newAccount"`
}
type Message struct {
	ID       string `json:"id"`
	Text     string `json:"text"`
	Mentions int    `json:"mentions"`
	Reply    bool   `json:"reply"`
}

// Event 는 게이트웨이로 온 이벤트 그대로예요.
type Event struct {
	Type       string  `json:"type"` // command · message · install · uninstall · ping · member_join · member_leave
	Bot        BotUser `json:"bot"`
	Room       Room    `json:"room"`
	User       User    `json:"user"`
	Message    Message `json:"message"`
	Text       string  `json:"text"`
	Command    string  `json:"command"`
	Args       string  `json:"args"`
	ReplyToken string  `json:"replyToken"`
	At         string  `json:"at"`
}

// Context 는 이벤트 하나예요. ctx.Reply / Send / Typing / Delete / Kick / Timeout
type Context struct {
	context.Context
	Bot *Bot
	Event
	Argv []string
}

type SendResult struct {
	OK bool   `json:"ok"`
	ID string `json:"id"`
}

func (c *Context) Reply(content Content) (*SendResult, error) {
	var r SendResult
	body := map[string]any{"replyToken": c.ReplyToken, "text": content.Text, "embeds": content.Embeds, "ai": content.AI}
	return &r, c.Bot.Call(c, "reply", body, &r)
}
func (c *Context) Send(content Content) (*SendResult, error) {
	return c.Bot.Send(c, c.Room.ID, content)
}
func (c *Context) Typing(ai bool) error { return c.Bot.Typing(c, c.Room.ID, ai) }
func (c *Context) Delete() error {
	return c.Bot.Call(c, "delete", map[string]any{"room": c.Room.ID, "message": c.Message.ID}, nil)
}
func (c *Context) Kick() error {
	return c.Bot.Call(c, "kick", map[string]any{"room": c.Room.ID, "user": c.User.ID}, nil)
}
func (c *Context) Timeout(minutes int, reason string) error {
	return c.Bot.Timeout(c, c.Room.ID, c.User.ID, minutes, reason)
}

type Handler func(*Context) error

type command struct {
	desc string
	run  Handler
}

// Bot 은 봇 하나예요.
type Bot struct {
	Token     string
	HTTP      *http.Client
	SyncCmds  bool // 켤 때 명령어를 서버에 등록 (기본 true)
	Reconnect bool // 끊기면 다시 연결 (기본 true)
	OnError   func(error)

	mu       sync.RWMutex
	commands map[string]command
	handlers map[string][]Handler
	ready    []func(BotUser)
}

func New(token string) *Bot {
	return &Bot{
		Token: token, HTTP: &http.Client{Timeout: 20 * time.Second}, SyncCmds: true, Reconnect: true,
		commands: map[string]command{}, handlers: map[string][]Handler{},
	}
}

// Command 는 /명령어를 등록해요 (한글·영어 소문자·숫자·_ 20자까지).
func (b *Bot) Command(name, description string, run Handler) {
	n := strings.ToLower(strings.TrimPrefix(name, "/"))
	if !cmdRe.MatchString(n) {
		panic("명령어 이름이 규칙에 맞지 않아요: " + name)
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	if len([]rune(description)) > 60 {
		description = string([]rune(description)[:60])
	}
	b.commands[n] = command{description, run}
}

func (b *Bot) on(t string, h Handler) {
	b.mu.Lock()
	b.handlers[t] = append(b.handlers[t], h)
	b.mu.Unlock()
}
func (b *Bot) OnReady(f func(BotUser)) { b.mu.Lock(); b.ready = append(b.ready, f); b.mu.Unlock() }
func (b *Bot) OnCommand(h Handler)     { b.on("command", h) }
func (b *Bot) OnMessage(h Handler)     { b.on("message", h) } // "모든 메시지" 권한
func (b *Bot) OnMemberJoin(h Handler)  { b.on("member_join", h) }
func (b *Bot) OnMemberLeave(h Handler) { b.on("member_leave", h) }
func (b *Bot) OnInstall(h Handler)     { b.on("install", h) }
func (b *Bot) OnUninstall(h Handler)   { b.on("uninstall", h) }

func (b *Bot) fail(err error) {
	if b.OnError != nil {
		b.OnError(err)
	} else {
		fmt.Println("[루담 봇]", err)
	}
}

// Call 은 봇 API를 불러요 (POST /bot/api/<path>). out이 있으면 대답을 풀어 넣어요.
func (b *Bot) Call(ctx context.Context, path string, body any, out any) error {
	raw, _ := json.Marshal(body)
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, API+"/bot/api/"+path, bytes.NewReader(raw))
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bot "+b.Token)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("User-Agent", "LudamBotGo/"+Version)
	res, err := b.HTTP.Do(req)
	if err != nil {
		return err
	}
	defer res.Body.Close()
	data, _ := io.ReadAll(io.LimitReader(res.Body, 1<<20))
	if res.StatusCode < 200 || res.StatusCode > 299 {
		var e struct {
			Error string `json:"error"`
		}
		_ = json.Unmarshal(data, &e)
		return &Error{Status: res.StatusCode, Code: e.Error}
	}
	if out != nil {
		return json.Unmarshal(data, out)
	}
	return nil
}

func (b *Bot) Send(ctx context.Context, roomID string, content Content) (*SendResult, error) {
	var r SendResult
	return &r, b.Call(ctx, "send", map[string]any{"room": roomID, "text": content.Text, "embeds": content.Embeds, "ai": content.AI}, &r)
}
func (b *Bot) Typing(ctx context.Context, roomID string, ai bool) error {
	return b.Call(ctx, "typing", map[string]any{"room": roomID, "ai": ai}, nil)
}

// 🛡️ 보안 봇 (관리 권한)
type Member struct {
	ID           string `json:"id"`
	Name         string `json:"name"`
	Role         string `json:"role"` // owner · sub · member
	TimeoutUntil string `json:"timeoutUntil,omitempty"`
}

func (b *Bot) Members(ctx context.Context, roomID string) ([]Member, bool, error) {
	var r struct {
		Members []Member `json:"members"`
		Locked  bool     `json:"locked"`
	}
	err := b.Call(ctx, "members", map[string]any{"room": roomID}, &r)
	return r.Members, r.Locked, err
}
func (b *Bot) Timeout(ctx context.Context, roomID, userID string, minutes int, reason string) error {
	return b.Call(ctx, "timeout", map[string]any{"room": roomID, "user": userID, "minutes": minutes, "reason": reason}, nil)
}
func (b *Bot) Untimeout(ctx context.Context, roomID, userID string) error {
	return b.Call(ctx, "untimeout", map[string]any{"room": roomID, "user": userID}, nil)
}
func (b *Bot) Lock(ctx context.Context, roomID string, locked bool, reason string) error {
	return b.Call(ctx, "lock", map[string]any{"room": roomID, "locked": locked, "reason": reason}, nil)
}
func (b *Bot) Purge(ctx context.Context, roomID string, messageIDs []string) (int, error) {
	if len(messageIDs) > 100 {
		messageIDs = messageIDs[:100]
	}
	var r struct {
		Deleted int `json:"deleted"`
	}
	err := b.Call(ctx, "purge", map[string]any{"room": roomID, "messages": messageIDs}, &r)
	return r.Deleted, err
}
func (b *Bot) Unban(ctx context.Context, roomID, userID string) error {
	return b.Call(ctx, "unban", map[string]any{"room": roomID, "user": userID}, nil)
}

// Rooms 는 봇이 들어간 방 목록이에요 (100개씩 넘겨 가며 모두).
func (b *Bot) Rooms(ctx context.Context) ([]Room, error) {
	var all []Room
	cursor := ""
	for {
		var r struct {
			Rooms  []Room `json:"rooms"`
			Cursor string `json:"cursor"`
		}
		body := map[string]any{}
		if cursor != "" {
			body["cursor"] = cursor
		}
		if err := b.Call(ctx, "rooms", body, &r); err != nil {
			return all, err
		}
		all = append(all, r.Rooms...)
		if r.Cursor == "" {
			return all, nil
		}
		cursor = r.Cursor
	}
}

// Run 은 봇을 켜요: 명령어 등록 → 게이트웨이 연결 → 끊기면 다시 연결. ctx가 끝나면 멈춰요.
func (b *Bot) Run(ctx context.Context) error {
	if !strings.HasPrefix(b.Token, "ldb_") {
		return errors.New("봇 열쇠(token)가 필요해요. 개발자 사이트에서 받은 ldb_… 값을 넣어 주세요")
	}
	if b.SyncCmds {
		b.mu.RLock()
		list := make([]map[string]string, 0, len(b.commands))
		for n, c := range b.commands {
			list = append(list, map[string]string{"name": n, "description": c.desc})
		}
		b.mu.RUnlock()
		if len(list) > 0 {
			if err := b.Call(ctx, "commands", map[string]any{"commands": list}, nil); err != nil {
				b.fail(fmt.Errorf("명령어 맞추기 실패: %w", err))
			}
		}
	}
	tries := 0
	for {
		err := b.connect(ctx, &tries)
		if errors.Is(err, ErrBadToken) || ctx.Err() != nil {
			return err
		}
		if err != nil {
			b.fail(err)
		}
		if !b.Reconnect {
			return err
		}
		wait := time.Duration(2<<min(tries, 5))*time.Second + time.Duration(rand.Intn(1000))*time.Millisecond
		if wait > time.Minute {
			wait = time.Minute
		}
		tries++
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(wait):
		}
	}
}

func (b *Bot) connect(ctx context.Context, tries *int) error {
	h := http.Header{}
	h.Set("Authorization", "Bot "+b.Token)
	h.Set("User-Agent", "LudamBotGo/"+Version)
	conn, res, err := websocket.Dial(ctx, Gateway, &websocket.DialOptions{HTTPHeader: h})
	if err != nil {
		if res != nil && res.StatusCode == http.StatusUnauthorized {
			return ErrBadToken
		}
		if res != nil {
			return fmt.Errorf("게이트웨이 연결 거절 (%d)", res.StatusCode)
		}
		return err
	}
	defer conn.CloseNow()
	conn.SetReadLimit(1 << 20)
	beatCtx, stop := context.WithCancel(ctx)
	defer stop()
	for {
		_, data, err := conn.Read(ctx)
		if err != nil {
			if websocket.CloseStatus(err) == 4004 { // 열쇠를 새로 받아서 끊겼어요
				return ErrBadToken
			}
			return err
		}
		var m struct {
			Op        string          `json:"op"`
			Bot       BotUser         `json:"bot"`
			Heartbeat int             `json:"heartbeat"`
			D         json.RawMessage `json:"d"`
		}
		if json.Unmarshal(data, &m) != nil {
			continue
		}
		switch m.Op {
		case "ready":
			*tries = 0
			gap := time.Duration(m.Heartbeat) * time.Millisecond
			if gap <= 0 {
				gap = 30 * time.Second
			}
			go func() {
				t := time.NewTicker(gap)
				defer t.Stop()
				for {
					select {
					case <-beatCtx.Done():
						return
					case <-t.C:
						_ = conn.Write(beatCtx, websocket.MessageText, []byte(`{"op":"ping"}`))
					}
				}
			}()
			b.mu.RLock()
			rs := append([]func(BotUser){}, b.ready...)
			b.mu.RUnlock()
			for _, f := range rs {
				f(m.Bot)
			}
		case "event":
			var ev Event
			if json.Unmarshal(m.D, &ev) == nil {
				go b.dispatch(ctx, ev)
			}
		}
	}
}

func (b *Bot) dispatch(parent context.Context, ev Event) {
	defer func() {
		if r := recover(); r != nil {
			b.fail(fmt.Errorf("봇 코드 오류: %v", r))
		}
	}()
	c := &Context{Context: parent, Bot: b, Event: ev, Argv: strings.Fields(ev.Args)}
	b.mu.RLock()
	hs := append([]Handler{}, b.handlers[ev.Type]...)
	cmd, hasCmd := b.commands[strings.ToLower(ev.Command)]
	b.mu.RUnlock()
	for _, h := range hs {
		if err := h(c); err != nil {
			b.fail(err)
		}
	}
	if ev.Type == "command" && hasCmd {
		if err := cmd.run(c); err != nil {
			b.fail(err)
		}
	}
}
