@echo off
chcp 65001 >nul
cd /d "%~dp0"
python tools\dashboard\build.py
if errorlevel 1 pause
