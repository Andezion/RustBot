# Rust Bot

Это минимальный пример Telegram-бота на Rust с простым `Client` и `Dispatcher`.

Prerequisites

- Rust (install via rustup): https://rustup.rs
- `cargo` в PATH (будет после установки `rustup`)
- Токен бота, полученный у @BotFather

Run (Windows - PowerShell)

```powershell
$env:TELEGRAM_BOT_TOKEN = 'YOUR_TOKEN_HERE'
cd ~/path/to/bot
cargo run
```

Run (Unix / macOS)

```bash
export TELEGRAM_BOT_TOKEN="YOUR_TOKEN_HERE"
cd ~/path/to/bot
cargo run
```
