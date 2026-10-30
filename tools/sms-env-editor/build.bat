@echo off
chcp 65001 >nul
python "%~dp0build.py"
if %errorlevel% neq 0 pause
