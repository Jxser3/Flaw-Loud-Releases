@echo off
setlocal
cd /d "%~dp0"
echo.
echo =============================================
echo   FLAW LOUD v1.0.0 - PREFLIGHT CHECK
echo =============================================
echo.
where node >nul 2>nul && echo [OK] Node.js || echo [MISSING] Node.js
where npm >nul 2>nul && echo [OK] npm || echo [MISSING] npm
where cargo >nul 2>nul && echo [OK] Rust Cargo || echo [MISSING] Rust Cargo
if exist "src-tauri\keyauth.json" (echo [OK] License config) else (echo [MISSING] License config)
powershell -NoProfile -Command "$c=Get-Content 'src-tauri/tauri.conf.json' -Raw|ConvertFrom-Json; if($c.plugins.updater.pubkey -and $c.plugins.updater.endpoints.Count -gt 0){exit 0}else{exit 1}"
if errorlevel 1 (echo [MISSING] Updater config) else (echo [OK] Signed updater config)
if exist "%USERPROFILE%\.flaw-loud\flaw-loud.key" (echo [OK] Private key is outside project) else (echo [MISSING] Local signing key)
if exist "src-tauri\icons\icon.ico" (echo [OK] Windows icon) else (echo [MISSING] Windows icon)
echo.
echo Run RUN_FLAW_LOUD.bat after all core checks show OK.
pause
