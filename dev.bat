@echo off
chcp 65001 >nul
title AI Chess - Dev
cd /d "%~dp0"

echo ========================================
echo  AI Chess - Dev Launcher
echo ========================================
echo.

echo [1/3] Installing npm dependencies...
call npm install
if %errorlevel% neq 0 (
    echo npm install failed. Make sure Node.js is installed.
    pause
    exit /b 1
)

echo [2/3] Checking port 1420...
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":1420 " ^| findstr "LISTENING"') do (
    echo   Killing process PID %%a on port 1420...
    taskkill /F /PID %%a >nul 2>&1
)

echo [3/3] Starting Tauri dev...
echo.
echo  Note: Requires Rust toolchain. If not installed, see:
echo  https://rustup.rs
echo.
npm run tauri dev

echo.
echo Dev server exited. Press any key to close...
pause >nul
