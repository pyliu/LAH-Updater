@echo off
chcp 65001 >nul
cd /d "%~dp0"
where python >nul 2>nul
if %errorlevel%==0 (
    python "%~dp0tools\moi-sms\build.py"
) else (
    py -3 "%~dp0tools\moi-sms\build.py"
)
pause
