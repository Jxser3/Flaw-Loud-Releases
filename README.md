# Flaw Loud v1.1.2 — Phase-Safe APO + Connected Platform

This RC is an audio-focused update. The primary change is **APO Compatibility Engine 2.0**, a rebuilt input and loudness architecture for Equalizer APO / external EQ sets. See `AUDIO_APO_ENGINE_2.md` for the exact signal flow and test procedure.

Created by **Bnet**  
**Bnet domina siempre**

## Main additions
- **External EQ / Equalizer APO Mode** in the native DSP engine. It reserves configurable input headroom and softens the dynamic processors that can make aggressive external EQ sets sound covered/muffled, while keeping CleanGuard protection.
- New **Owner / Moderator / User** account layer, independent from KeyAuth.
- First-run Owner setup and username/password login.
- Owner controls for creating users/moderators, revoking moderator powers, suspending/banning/blocking logins and revoking sessions.
- Moderator permissions for announcements and report triage without Owner-only account/release powers.
- Dashboard **notification bell** with unread badge and inline announcement tray.
- User **Report a problem** flow with Bug/Audio/APO/Crash/Updater/Account/Other categories and optional image/video attachments.
- Admin report inbox with New / Reviewing / Fixed / Closed states.
- Admin audit history.
- Release policy with **24-hour update grace** and forced-update screen after the grace window for normal users.
- **Owner version bypass** for retired builds.
- Existing signed Tauri updater remains available.

## Important RC architecture note
The Connected Platform in this ZIP is **fully testable locally on one Windows installation**. Its data is persisted under:

`%LOCALAPPDATA%\Flaw Loud\ConnectedPlatform`

That means Owner/User/Moderator flows, announcements, reports, access control and update-policy logic can be exercised now, but they are **not yet synchronized between different PCs**. A real public launch needs the same API/data model moved to a hosted HTTPS backend/database. This RC deliberately avoids pretending a local JSON store is a global service.

Use `RESET_LOCAL_PLATFORM.bat` if you want to reset the local RC account/platform database and repeat first-run Owner setup.

## KeyAuth status
**KeyAuth is deliberately outside the v1.1 Connected Platform.** It is dormant and not required for the new account/admin/report/announcement flows. `CONFIGURE_KEYAUTH.bat` performs no configuration. It can be reintroduced later without mixing it into this RC.

## Signed updater / Windows build
1. If needed, run `GENERATE_UPDATER_KEYS.bat` once.
2. Run `CONFIGURE_PRODUCTION.bat` and enter only:
   - signed updater HTTPS endpoint
   - Tauri updater **public** signing key
3. Run `BUILD_WINDOWS_RELEASE.bat` on Windows with Node.js/npm and Rust/Cargo installed.
4. Installers/update artifacts are copied into `RELEASE`.

The updater private key stays only on the release machine/GitHub Actions secret. Never commit it.

## Development run
`RUN_FLAW_LOUD.bat`

See `CONNECTED_PLATFORM_NOTES.md`, `QA_V1.1.md` and `PRODUCTION_RELEASE_GUIDE.md`.


## v1.1.0-rc.2 Updater startup hotfix

- Fixed Tauri updater plugin initialization crash caused by a missing/null `plugins.updater` config.
- The dev build now boots with a structurally valid updater config while the real endpoint/public key continue to come from `src-tauri/updater.json` after `CONFIGURE_PRODUCTION.bat`.
- No DSP, Connected Platform, account, announcement, report, APO mode, or UI behavior was changed.
