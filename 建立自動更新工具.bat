@echo off
chcp 65001 >nul
python "%~dp0updater\build.py"
pause
