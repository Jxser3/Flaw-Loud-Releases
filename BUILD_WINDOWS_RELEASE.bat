@echo off
setlocal EnableExtensions
cd /d "%~dp0"
title Build signed Flaw Loud Windows release

where npm >nul 2>nul || (echo ERROR: Node.js/npm not found.& pause & exit /b 1)
where cargo >nul 2>nul || (echo ERROR: Rust/Cargo not found.& pause & exit /b 1)

set "SIGNKEY=%USERPROFILE%\.flaw-loud\flaw-loud.key"
if not exist "%SIGNKEY%" (
  echo ERROR: Signing private key not found at %SIGNKEY%
  pause
  exit /b 1
)

set "TAURI_SIGNING_PRIVATE_KEY=%SIGNKEY%"
set /p TAURI_SIGNING_PRIVATE_KEY_PASSWORD=Signing key password: 

echo Installing locked dependencies...
call npm ci
if errorlevel 1 (echo npm ci failed.& pause & exit /b 1)

echo Building signed x64 NSIS and MSI installers...
call npm run tauri -- build --target x86_64-pc-windows-msvc
if errorlevel 1 (echo BUILD FAILED.& pause & exit /b 1)

set "TAURI_SIGNING_PRIVATE_KEY="
set "TAURI_SIGNING_PRIVATE_KEY_PASSWORD="
echo Build complete under src-tauri\target\x86_64-pc-windows-msvc\release\bundle
pause
