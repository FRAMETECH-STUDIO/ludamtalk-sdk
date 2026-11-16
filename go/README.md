# ludamtalk (Go)

루담톡(LudamTalk) 온라인채팅 봇을 만드는 공식 Go 도구예요. 설명서: https://ludamtalk.com/developers

```bash
go get ludamtalk.com/sdk/go
```

```go
package main

import (
	"context"
	"os"

	ludamtalk "ludamtalk.com/sdk/go"
)

func main() {
	bot := ludamtalk.New(os.Getenv("LUDAM_TOKEN"))
	bot.Command("주사위", "주사위를 굴려요", func(c *ludamtalk.Context) error {
		_, err := c.Reply(ludamtalk.Text("🎲 4"))
		return err
	})
	bot.Run(context.Background())
}
```

MIT © FRAMETECH STUDIO
