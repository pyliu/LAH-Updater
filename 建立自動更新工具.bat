@echo off
chcp 65001 >nul
python "%~dp0tools\updater\build.py"
pause
