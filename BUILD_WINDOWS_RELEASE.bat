@echo off
setlocal EnableExtensions
cd /d "%~dp0"
title Build Flaw Loud v1.1 RC Connected Platform

echo [1/6] Checking Node...
where npm >nul 2>nul || (echo ERROR: Node.js/npm not found.& pause & exit /b 1)
echo [2/6] Checking Rust...
where cargo >nul 2>nul || (echo ERROR: Rust/Cargo not found. Install Rust from https://rustup.rs/ then reopen this window.& pause & exit /b 1)

echo [3/6] Checking signed updater configuration...
powershell -NoProfile -Command "$u=Get-Content 'src-tauri/updater.json'|ConvertFrom-Json; if(-not $u.enabled -or $u.endpoint -like 'PASTE*' -or $u.public_key -like 'PASTE*'){exit 3}"
if errorlevel 3 (echo ERROR: Updater is not configured. Run CONFIGURE_PRODUCTION.bat first.& pause & exit /b 1)

echo       Connected Platform: LOCAL RC backend
echo       KeyAuth: deliberately disabled for v1.1

if not exist node_modules (
  echo [4/6] Installing dependencies...
  call npm install
  if errorlevel 1 (echo npm install failed.& pause & exit /b 1)
) else (
  echo [4/6] Dependencies already installed.
)

echo.
set /p SIGNKEY=Private updater key path [%%USERPROFILE%%\.flaw-loud\flaw-loud.key]: 
if "%SIGNKEY%"=="" set "SIGNKEY=%USERPROFILE%\.flaw-loud\flaw-loud.key"
if not exist "%SIGNKEY%" (
  echo ERROR: Signing private key not found: %SIGNKEY%
  echo Run GENERATE_UPDATER_KEYS.bat first.
  pause
  exit /b 1
)
set "TAURI_SIGNING_PRIVATE_KEY=%SIGNKEY%"
set /p TAURI_SIGNING_PRIVATE_KEY_PASSWORD=Signing key password (leave blank if none): 

echo [5/6] Building Windows EXE/MSI and signed updater artifacts...
call npm run tauri build
if errorlevel 1 (echo BUILD FAILED.& pause & exit /b 1)

echo [6/6] Collecting release files...
if not exist RELEASE mkdir RELEASE
for /r "src-tauri\target\release\bundle" %%F in (*.exe *.msi *.sig *.zip) do copy /Y "%%F" "RELEASE\" >nul

echo.
echo ========================================
echo Flaw Loud v1.1 RC build completed.
echo Open: %CD%\RELEASE
echo ========================================
start "" "%CD%\RELEASE"
pause
