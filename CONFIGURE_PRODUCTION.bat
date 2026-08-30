@echo off
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0CONFIGURE_PRODUCTION.ps1"
if errorlevel 1 (
  echo.
  echo Production configuration failed.
  pause
  exit /b 1
)
pause
