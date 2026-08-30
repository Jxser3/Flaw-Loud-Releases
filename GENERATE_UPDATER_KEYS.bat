@echo off
setlocal
cd /d "%~dp0"
title Flaw Loud - Generate Update Signing Keys
where npm >nul 2>nul || (echo Node/npm is required.& pause & exit /b 1)
if not exist node_modules call npm install
if errorlevel 1 (pause & exit /b 1)
if not exist "%USERPROFILE%\.flaw-loud" mkdir "%USERPROFILE%\.flaw-loud"
echo.
echo IMPORTANT: Keep the PRIVATE key safe. Never upload or ship it with Flaw Loud.
echo Copy the PUBLIC key printed by Tauri into CONFIGURE_PRODUCTION.bat when asked.
echo.
call npm run tauri signer generate -- -w "%USERPROFILE%\.flaw-loud\flaw-loud.key"
pause
