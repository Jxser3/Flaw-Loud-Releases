@echo off
cd /d "%~dp0"
echo Applying the public KeyAuth Client API production configuration...
call "%~dp0CONFIGURE_PRODUCTION.bat"
