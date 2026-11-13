@echo off
setlocal
chcp 65001 >nul
cd /d "%~dp0"
cargo run -p moi-sms-desktop --locked
endlocal
