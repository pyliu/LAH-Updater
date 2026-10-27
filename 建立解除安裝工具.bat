@echo off
chcp 65001 >nul
python "%~dp0uninstaller\build_uninstaller.py"
pause
