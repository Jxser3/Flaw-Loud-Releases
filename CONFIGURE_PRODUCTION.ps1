$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Write-Host ""; Write-Host "=== Flaw Loud v1.1 RC - Signed Updater Setup ===" -ForegroundColor Magenta
Write-Host "Connected Platform accounts are independent from KeyAuth in this release." -ForegroundColor DarkGray
Write-Host "This script configures only the signed Tauri updater." -ForegroundColor DarkGray

$keyauthPath = Join-Path $root 'src-tauri\keyauth.json'
$updaterPath = Join-Path $root 'src-tauri\updater.json'

$endpoint = Read-Host 'Signed updater HTTPS endpoint [https://github.com/Jxser3/Flaw-Loud-Releases/releases/latest/download/latest.json]'
if ([string]::IsNullOrWhiteSpace($endpoint)) { $endpoint = 'https://github.com/Jxser3/Flaw-Loud-Releases/releases/latest/download/latest.json' }
if (-not $endpoint.StartsWith('https://')) { throw 'Updater endpoint must use HTTPS.' }
$pubkey = Read-Host 'Tauri updater PUBLIC signing key'
if ([string]::IsNullOrWhiteSpace($pubkey) -or $pubkey.Length -lt 16) { throw 'Public signing key looks invalid.' }

$updater = @{
  enabled = $true
  endpoint = $endpoint
  public_key = $pubkey
  channel = 'stable'
}
$updater | ConvertTo-Json | Set-Content -Path $updaterPath -Encoding UTF8

# KeyAuth remains deliberately dormant in v1.1 Connected Platform.
$keyauth = @{
  enabled = $false
  dev_bypass = $false
  name = 'Flaw Loud'
  owner_id = 'DISABLED_V1_1'
  version = '1.1.0-rc.1'
  api_url = 'https://keyauth.win/api/1.3/'
}
$keyauth | ConvertTo-Json | Set-Content -Path $keyauthPath -Encoding UTF8

Write-Host ""; Write-Host 'Signed updater configuration saved.' -ForegroundColor Green
Write-Host 'KeyAuth remains disabled and separate from Connected Platform.' -ForegroundColor Yellow
Write-Host 'Next: run BUILD_WINDOWS_RELEASE.bat' -ForegroundColor Cyan
