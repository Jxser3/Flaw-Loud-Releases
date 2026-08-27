@echo off
setlocal
cd /d "%~dp0"
title Flaw Loud v0.9.5.2 RC

echo.
echo ==============================================
echo        FLAW LOUD v0.9.5.2 RC - FRAMELESS PREMIUM SHELL
echo ==============================================
echo.

where node >nul 2>nul || (
  echo [ERROR] Node.js is not installed.
  echo Install Node.js LTS and run this file again.
  pause
  exit /b 1
)

where cargo >nul 2>nul || (
  echo [ERROR] Rust/Cargo is not installed.
  echo Install Rust with rustup and run this file again.
  pause
  exit /b 1
)

if not exist node_modules (
  echo [1/2] Installing Flaw Loud dependencies...
  call npm install
  if errorlevel 1 goto :fail
)

echo [2/2] Starting Flaw Loud native audio engine...
call npm run tauri dev
if errorlevel 1 goto :fail
exit /b 0

:fail
echo.
echo Flaw Loud could not start. Read README.md for prerequisites.
pause
exit /b 1
