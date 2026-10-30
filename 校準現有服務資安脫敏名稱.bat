@echo off
chcp 65001 >nul
:: 檢查是否具備管理員權限，若無則自動提權
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] 正在請求系統管理員權限以更新 Windows 服務註冊表...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process cmd -ArgumentList '/c \"\"%~f0\"\"' -Verb RunAs"
    exit /b
)

echo ============================================================
echo   LAH 智慧管控 - 現有服務名稱與描述資安脫敏熱校準工具
echo ============================================================
echo.

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; " ^
  "$ws = 'HKLM:\SYSTEM\CurrentControlSet\Services\LAH_Web_Service'; " ^
  "$ps = 'HKLM:\SYSTEM\CurrentControlSet\Services\LAH_PM2_Service'; " ^
  "if (Test-Path $ws) { " ^
  "    Set-ItemProperty -Path $ws -Name 'DisplayName' -Value 'LAH 智慧管控：應用系統入口與 API 閘道服務' -Force; " ^
  "    Set-ItemProperty -Path $ws -Name 'Description' -Value '提供地政智慧管控系統前端介面存取、業務查詢 API 轉發與安全反向代理，負責核心通訊連線與身分驗證傳輸。' -Force; " ^
  "    Write-Host '  [OK] LAH_Web_Service 顯示名稱與描述已成功脫敏更新' -ForegroundColor Green; " ^
  "} else { Write-Host '  [WARN] 找不到服務 LAH_Web_Service' -ForegroundColor Yellow; }; " ^
  "if (Test-Path $ps) { " ^
  "    Set-ItemProperty -Path $ps -Name 'DisplayName' -Value 'LAH 智慧管控：背景服務總線與健康管理系統' -Force; " ^
  "    Set-ItemProperty -Path $ps -Name 'Description' -Value '協調地政管控平台背景多工任務執行、即時事件分發與系統運作指標監測，確保關鍵作業永續運作。' -Force; " ^
  "    Write-Host '  [OK] LAH_PM2_Service 顯示名稱與描述已成功脫敏更新' -ForegroundColor Green; " ^
  "} else { Write-Host '  [WARN] 找不到服務 LAH_PM2_Service' -ForegroundColor Yellow; }; " ^
  "$wXml = 'C:\AppServ\Apache24\service\lah-web-service.xml'; " ^
  "$srcWXml = '%~dp0updater\service\lah-web-service.xml'; " ^
  "if (Test-Path $wXml) { Copy-Item -Path $srcWXml -Destination $wXml -Force; Write-Host '  [OK] 已同步更新 C:\AppServ\Apache24\service\lah-web-service.xml' -ForegroundColor Green; }; " ^
  "$pXml = 'C:\AppServ\node\service\lah-service.xml'; " ^
  "$srcPXml = '%~dp0updater\service\lah-service.xml'; " ^
  "if (Test-Path $pXml) { Copy-Item -Path $srcPXml -Destination $pXml -Force; Write-Host '  [OK] 已同步更新 C:\AppServ\node\service\lah-service.xml' -ForegroundColor Green; }; "

echo.
echo ============================================================
echo   校準完成！請回到 services.msc 視窗按下 F5 重新整理查看效果。
echo ============================================================
echo.
pause
