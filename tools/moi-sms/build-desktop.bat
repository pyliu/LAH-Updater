@echo off
setlocal
chcp 65001 >nul
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
    echo 找不到 cargo，請先安裝 Rust 與 Windows C++ 建置工具。
    exit /b 1
)
echo [1/3] 驗證 Rust 核心與桌面模組...
cargo test --workspace --locked
if errorlevel 1 exit /b 1
echo [2/3] 建置 Tauri 桌面 Release...
cargo build -p moi-sms-desktop --release --locked
if errorlevel 1 exit /b 1
echo [3/3] 輸出執行檔...
if not exist dist mkdir dist
copy /y "target\release\moi-sms-desktop.exe" "dist\智慧控管與監控系統 - 地籍異動即時通簡訊控管.exe" >nul
if errorlevel 1 exit /b 1
echo 完成：%~dp0dist\智慧控管與監控系統 - 地籍異動即時通簡訊控管.exe
endlocal
