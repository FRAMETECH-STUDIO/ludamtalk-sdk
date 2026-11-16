# ludamtalk

루담톡(LudamTalk) 온라인채팅 봇을 만드는 공식 파이썬 도구예요.

- 설명서: https://ludamtalk.com/developers
- 봇 만들기(열쇠 받기): https://web.ludamtalk.com/dev

## 설치

```bash
pip install ludamtalk
```

Python 3.9 이상이 필요해요.

## 예제

```python
import os, random
import ludamtalk as ludam

bot = ludam.Bot(token=os.environ["LUDAM_TOKEN"])

@bot.event
async def on_ready(me):
    print(me["name"], "온라인!")

@bot.command("주사위", "주사위를 굴려요")
async def dice(ctx):
    await ctx.reply(f"🎲 **{random.randint(1, 6)}** 이(가) 나왔어요!")

@bot.command("날씨", "오늘 날씨")
async def weather(ctx):
    e = ludam.Embed(title="오늘의 날씨", color=0x3b82f6, description="**맑음** · 최고 18°C")
    await ctx.reply(embed=e)

bot.run()
```

켜기: `LUDAM_TOKEN=ldb_… python bot.py` (윈도우 PowerShell: `$env:LUDAM_TOKEN="ldb_…"; python bot.py`)

봇 열쇠(ldb_…)는 비밀번호처럼 다뤄 주세요. 코드에 바로 적거나 다른 사람에게 보여 주지 마세요.

## 라이선스

MIT © FRAMETECH STUDIO
