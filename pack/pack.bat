@echo off
title LAH Pack Tool
chcp 65001 >nul

rem 優先採用相容之 Node 16 執行期 (NVM 或 AppServ 可攜版本)
if exist "%LOCALAPPDATA%\nvm\v16.20.2\node.exe" (
    set "PATH=%LOCALAPPDATA%\nvm\v16.20.2;%PATH%"
) else if exist "C:\AppServ\node\node.exe" (
    set "PATH=C:\AppServ\node;%PATH%"
) else if exist "D:\AppServ\node\node.exe" (
    set "PATH=D:\AppServ\node;%PATH%"
)

python "%~dp0pack.py" %*
if errorlevel 1 pause
