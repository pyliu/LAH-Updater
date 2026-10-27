@echo off
chcp 65001 >nul
python "%~dp0uninstaller\build.py"
pause
