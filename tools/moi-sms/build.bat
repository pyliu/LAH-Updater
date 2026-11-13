@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
    echo [錯誤] 找不到 cargo，請確認 Rust 工具鏈已加入 PATH。
    pause
    exit /b 1
)

echo ============================================
echo [1/3] 執行單元測試 (cargo test)...
echo ============================================
cargo test
if errorlevel 1 (
    echo [錯誤] 單元測試未通過，已中止建置。
    pause
    exit /b 1
)

echo.
echo ============================================
echo [2/3] 編譯 Release 執行檔 (cargo build --release)...
echo ============================================
cargo build --release
if errorlevel 1 (
    echo [錯誤] Release 編譯失敗。
    pause
    exit /b 1
)

echo.
echo ============================================
echo [3/3] 輸出執行檔至 dist\moi_sms.exe 並試跑觀察...
echo ============================================
if not exist dist mkdir dist
copy /y "target\release\moi_sms.exe" "dist\moi_sms.exe" >nul
if exist "r:\exchange\@TBD\1151113 MOI_SMS\env.properties" (
    copy /y "r:\exchange\@TBD\1151113 MOI_SMS\env.properties" "dist\env.properties" >nul
)

echo.
echo 執行檔位置: %~dp0dist\moi_sms.exe
echo --------------------------------------------
pushd dist
moi_sms.exe
popd
echo --------------------------------------------
pause
endlocal
