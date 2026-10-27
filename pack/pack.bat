@echo off
title LAH Pack Tool
chcp 65001 >nul
python "%~dp0pack.py" %*
if errorlevel 1 pause
