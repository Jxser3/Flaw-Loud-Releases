# Flaw Loud v1.1 RC — Release Guide

## What is production-ready in this RC
The native audio engine, External EQ/APO processing path, signed Tauri updater wiring and Windows bundling path remain part of the desktop app.

## Connected Platform status
The v1.1 Connected Platform ships in **local RC mode** so every workflow can be tested before committing to hosting. Local data lives in `%LOCALAPPDATA%\Flaw Loud\ConnectedPlatform`.

Before distributing the account/admin system to users on different computers, deploy the Connected Platform model behind an authenticated HTTPS API and database, then replace the local transport with that remote endpoint. Do not distribute this local store as if it were a multi-user cloud service.

For a hosted production backend use, at minimum:
- strong password hashing such as Argon2id/bcrypt/scrypt;
- server-side sessions/revocation;
- Owner/Moderator/User authorization enforced server-side;
- durable database backups;
- object storage for report image/video attachments;
- upload MIME/size validation and malware scanning policy;
- rate limits, audit retention and HTTPS only.

## KeyAuth
KeyAuth is intentionally disabled/separate in v1.1. No KeyAuth Owner ID or license secret is required for Connected Platform. Do not add a Seller Key to the client.

## Signed updater
1. Keep using the same Tauri updater signing key pair used by already-installed releases.
2. Run `CONFIGURE_PRODUCTION.bat` to set the HTTPS `latest.json` endpoint and updater public key.
3. Build with the matching private key via `BUILD_WINDOWS_RELEASE.bat` or your GitHub Actions release workflow.
4. Publish installer/update artifacts and their signatures along with a matching `latest.json`.

## Update policy vs signed updater
The Connected Platform release policy decides whether a user is current, inside the 24-hour grace period, or requires an update. The actual download/install is still performed by the signed Tauri updater. Owner accounts bypass the policy block so old builds remain testable by the Owner.

## Version release checklist
1. Update version metadata.
2. QA audio, APO mode, account roles, announcements, reports, session revocation and update grace.
3. Build with the same Tauri signing private key.
4. Publish the signed release.
5. Set `latest_version`, `minimum_supported_version`, notes and 24h grace in Admin -> Release Policy.
