@echo off
chcp 65001 >nul
cd /d "%~dp0"

echo ========================================
echo  AI Chess - Build
echo ========================================
echo.

echo [1/4] Setting up Visual Studio environment...
call "D:\RUANJIAN\M VS\VC\Auxiliary\Build\vcvars64.bat"
if %errorlevel% neq 0 (
    echo Failed to setup VS environment.
    pause
    exit /b 1
)

echo [2/4] Installing npm dependencies...
call npm install
if %errorlevel% neq 0 (
    echo npm install failed.
    pause
    exit /b 1
)

echo [3/4] Building frontend...
call npm run build
if %errorlevel% neq 0 (
    echo Frontend build failed.
    pause
    exit /b 1
)

echo [4/4] Building Tauri app...
call npm run tauri build
if %errorlevel% neq 0 (
    echo Tauri build failed.
    pause
    exit /b 1
)

echo.
echo Build complete! Check src-tauri/target/release/bundle/ for the installer.
echo.
pause
