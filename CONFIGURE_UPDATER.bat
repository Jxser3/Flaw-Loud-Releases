@echo off
cd /d "%~dp0"
echo Flaw Loud v1.1 configures only the signed updater here.
echo KeyAuth is intentionally separate/disabled until further notice.
start "" "%~dp0CONFIGURE_PRODUCTION.bat"
