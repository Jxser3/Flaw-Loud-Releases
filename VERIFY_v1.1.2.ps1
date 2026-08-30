param(
  [Parameter(Mandatory=$true)]
  [string]$Root
)

$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $Root

$expected = '1.1.2'

$packageVersion = (Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json).version
$tauriConfig = Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json
$tauriVersion = $tauriConfig.version

$cargoText = Get-Content -LiteralPath 'src-tauri/Cargo.toml' -Raw
$cargoMatch = [regex]::Match($cargoText, '(?m)^\s*version\s*=\s*"([^"]+)"')
if (-not $cargoMatch.Success) {
  throw 'Could not read package version from src-tauri/Cargo.toml.'
}
$cargoVersion = $cargoMatch.Groups[1].Value

if ($packageVersion -ne $expected -or $tauriVersion -ne $expected -or $cargoVersion -ne $expected) {
  throw "Version mismatch: package=$packageVersion tauri=$tauriVersion cargo=$cargoVersion expected=$expected"
}

if ($null -eq $tauriConfig.plugins -or $null -eq $tauriConfig.plugins.updater) {
  throw 'Tauri updater configuration is missing.'
}
if ([string]::IsNullOrWhiteSpace([string]$tauriConfig.plugins.updater.pubkey)) {
  throw 'Tauri updater public key is missing.'
}
if ($tauriConfig.plugins.updater.endpoints.Count -lt 1) {
  throw 'Tauri updater endpoint is missing.'
}

$keyAuth = Get-Content -LiteralPath 'src-tauri/keyauth.json' -Raw | ConvertFrom-Json
if ($keyAuth.enabled -ne $false) {
  throw 'KeyAuth must remain disabled for Flaw Loud v1.1.2.'
}

Write-Host "Versions OK: package=$packageVersion tauri=$tauriVersion cargo=$cargoVersion"
Write-Host 'Updater config OK.'
Write-Host 'KeyAuth separation OK.'
exit 0
