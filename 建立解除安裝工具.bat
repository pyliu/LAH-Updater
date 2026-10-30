@echo off
chcp 65001 >nul
python "%~dp0tools\uninstaller\build.py"
pause
