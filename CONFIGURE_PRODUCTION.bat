@echo off
setlocal
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -Command "$p='src-tauri/keyauth.json'; $c=[ordered]@{enabled=$true;dev_bypass=$false;name='Flaw Loud';owner_id='hRgZPsngUh';version='1.0.1';api_url='https://keyauth.win/api/1.3/'}; $c|ConvertTo-Json|Set-Content -LiteralPath $p -Encoding utf8"
if errorlevel 1 (
  echo ERROR: Could not write src-tauri\keyauth.json
  exit /b 1
)
echo KeyAuth production configuration applied for Flaw Loud v1.0.1.
