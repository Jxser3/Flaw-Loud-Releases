@echo off
setlocal
set "DATA=%LOCALAPPDATA%\Flaw Loud\ConnectedPlatform"
echo.
echo Flaw Loud v1.1 RC - Reset LOCAL Connected Platform
 echo This deletes local RC users, announcements, reports, sessions and audit data.
echo It does NOT touch presets, DSP settings, KeyAuth files or updater keys.
echo.
set /p OK=Type RESET to continue: 
if /I not "%OK%"=="RESET" exit /b 0
if exist "%DATA%" rmdir /S /Q "%DATA%"
echo Local Connected Platform data reset.
pause
