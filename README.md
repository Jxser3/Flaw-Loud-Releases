# Flaw Loud v1.0 RC — Commercial Release Candidate

Created by **Bnet**  
**Bnet domina siempre**

## What this build contains
- Native Tauri/Rust audio engine and current DSP chain.
- KeyAuth **Client API 1.3** license activation from the Rust backend.
- Remembered license support tied to the current PC.
- Production mode disables RC Developer Preview.
- Official **Tauri signed updater** wiring: check, download, signature verification and install from inside Flaw Loud.
- `createUpdaterArtifacts: true` for signed Windows updater artifacts.
- Stream Mode, global show/hide hotkey, Unload hotkey, capture protection, taskbar hiding and frameless UI.
- Safe Boot, diagnostics, support report and QA tools.
- Purple Fall remains available; unverified experimental Theme FX are hidden from the production UI.

## Important: two external values are required before a real commercial build
A downloadable source ZIP cannot invent credentials for your accounts. Run `CONFIGURE_PRODUCTION.bat` and provide:
1. **KeyAuth Owner ID** for your Flaw Loud application.
2. **HTTPS updater endpoint** and your **Tauri public signing key**.

Those values are not secret admin credentials. **Never put a KeyAuth Seller Key or the Tauri private signing key inside the app.**

## Make the Windows EXE / installer
On the Windows PC where your existing Flaw Loud RCs already compile:
1. Run `GENERATE_UPDATER_KEYS.bat` once if you do not have Tauri updater signing keys.
2. Run `CONFIGURE_PRODUCTION.bat`.
3. Run `BUILD_WINDOWS_RELEASE.bat`.
4. The script builds the app and copies `.exe`, `.msi`, `.sig` and updater artifacts into `RELEASE`.

The build script requires Node.js, npm and Rust/Cargo. Your existing Windows test PC already used these for earlier Tauri RCs.

## In-app updates
Diagnostics → **Secure Update Center** now supports:
- CHECK UPDATE
- DOWNLOAD & INSTALL when a newer signed release exists

Updates are accepted only through the configured HTTPS endpoint and signature public key.

See `PRODUCTION_RELEASE_GUIDE.md` for publishing each future release.
