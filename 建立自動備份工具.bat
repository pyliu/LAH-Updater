@echo off
chcp 65001 >nul
python "%~dp0tools\backuper\build.py"
pause
