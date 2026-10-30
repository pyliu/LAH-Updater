@echo off
chcp 65001 >nul
python "%~dp0tools\node-portable\build.py"
pause
