@echo off
setlocal
cd /d "%~dp0"
echo.
echo =============================================
echo   FLAW LOUD v1.1 RC - PREFLIGHT CHECK
echo =============================================
echo.
where node >nul 2>nul && echo [OK] Node.js || echo [MISSING] Node.js
where npm >nul 2>nul && echo [OK] npm || echo [MISSING] npm
where cargo >nul 2>nul && echo [OK] Rust Cargo || echo [MISSING] Rust Cargo
if exist "src-tauri\src\platform.rs" (echo [OK] Connected Platform backend) else (echo [MISSING] Connected Platform backend)
if exist "src-tauri\updater.json" (echo [OK] Updater config file) else (echo [MISSING] Updater config)
if exist "src-tauri\icons\icon.ico" (echo [OK] Windows icon) else (echo [MISSING] Windows icon)
echo [INFO] KeyAuth is intentionally disabled/separate in v1.1.
echo.
echo Run RUN_FLAW_LOUD.bat for the RC development build.
pause
