@echo off
chcp 65001 >nul
title AI Chess - Dev
cd /d "%~dp0"

echo ========================================
echo  AI Chess - Dev Launcher
echo ========================================
echo.

echo [1/2] Checking port 1420...
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":1420 " ^| findstr "LISTENING"') do (
    echo   Killing process PID %%a on port 1420...
    taskkill /F /PID %%a >nul 2>&1
)

echo [2/2] Starting Tauri dev...
echo.
npm run tauri dev

echo.
echo Dev server exited. Press any key to close...
pause >nul
