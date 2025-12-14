# bot — minimal Telegram polling example (Rust)

Кратко: минимальный пример Telegram-бота на Rust с простым `Client` и `Dispatcher`.

Quick start

1. Сгенерируйте токен через @BotFather и НЕ публикуйте его.
2. В текущей PowerShell-сессии:

```powershell
$env:TELEGRAM_BOT_TOKEN = 'YOUR_TOKEN_HERE'
cd 'C:\Users\vdoro\RustShit\bot'
cargo run
```

3. Отправьте `/start` боту в Telegram — бот должен ответить.

Security

- Никогда не коммитьте токены. Используйте `.env` и добавьте его в `.gitignore`.
- Если токен скомпрометирован — немедленно регенерируйте через @BotFather.

Repository

Commands to initialize and push to GitHub (example):

```bash
git init
git add .
git commit -m "Initial bot skeleton"
# create remote via gh (recommended) or create repo on github.com and add remote
# gh repo create my-bot --public --source=. --push
# or manual:
# git remote add origin https://github.com/youruser/yourrepo.git
# git push -u origin main
```

CI

Included: simple GitHub Actions workflow for `cargo build` and `cargo test`.
