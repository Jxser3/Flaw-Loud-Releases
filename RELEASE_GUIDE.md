# Flaw Loud — Production Release Guide

This project publishes signed Windows releases to `Jxser3/Flaw-Loud-Releases`. Installed copies check:

`https://github.com/Jxser3/Flaw-Loud-Releases/releases/latest/download/latest.json`

The updater public key is embedded in `src-tauri/tauri.conf.json`. This is safe and required for signature verification. Never copy the private key into this project, an artifact, a commit, an issue, or a workflow file.

## One-time GitHub setup

Create these Actions secrets in the repository that contains this workflow:

- `TAURI_SIGNING_PRIVATE_KEY`: the complete contents of the existing private updater key. Preserve line breaks. Do not base64-encode it again.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the password used when the updater key was generated. If the key has no password, create the secret with an empty value if GitHub permits it; otherwise remove this environment line from the workflow.
- `RELEASES_TOKEN`: a fine-grained GitHub personal access token owned by an account with access to `Jxser3/Flaw-Loud-Releases`. Limit repository access to that repo and grant **Contents: Read and write**. This is necessary because the built-in `GITHUB_TOKEN` cannot publish to a different repository.

In `Jxser3/Flaw-Loud-Releases`, ensure the default branch is `main`. If it has another name, change `releaseCommitish` in `.github/workflows/release.yml`.

The existing local key files remain outside the project:

- Private: `C:\Users\User\.flaw-loud\flaw-loud.key`
- Public: `C:\Users\User\.flaw-loud\flaw-loud.key.pub`

Back up the private key and its password securely. Losing either prevents publishing updates accepted by existing installations.

## Publish a version

1. Choose a stable SemVer such as `1.0.1`.
2. Update the version to exactly the same value in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `src-tauri/keyauth.json`. Keep `package-lock.json` synchronized by running `npm install --package-lock-only`.
3. Update the visible changelog/release text in the app when appropriate.
4. Commit and push the source changes.
5. Create and push the matching tag:

   ```powershell
   git tag v1.0.1
   git push origin v1.0.1
   ```

6. Follow the **Release Flaw Loud for Windows** workflow. It refuses to publish when tag, npm, Cargo, and Tauri versions differ.
7. Verify the new GitHub Release contains at least:
   - the NSIS `.exe` installer and its `.sig`;
   - the WiX `.msi` installer and its `.sig`;
   - `latest.json`.
8. Download `latest.json` and confirm its `version`, `notes`, RFC 3339 `pub_date`, and Windows platform entries point to assets from the same tag and contain non-empty signatures.
9. From an older installed build, test **CHECK UPDATE**, review the displayed version/changelog, then use **DOWNLOAD & INSTALL**. Tauri verifies the signature before installation and relaunches the app when the platform permits it. Windows may close the running app while the installer replaces it.

## What the workflow does

On every pushed `v*` tag, GitHub Actions installs Node and Rust, runs `npm ci`, checks version consistency, builds x64 Windows NSIS and MSI bundles, signs updater artifacts using the Actions secrets, creates the release in the dedicated releases repo, uploads installers/signatures, and generates `latest.json`. NSIS is preferred for the generic `windows-x86_64` updater entry.

`latest.json` is generated from the actual filenames and `.sig` contents, so it must not be hand-edited. GitHub-generated release notes become the changelog delivered to the Update Center.

## Local signed build (optional)

Set secrets only for the current PowerShell process, then build:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw -LiteralPath 'C:\Users\User\.flaw-loud\flaw-loud.key'
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Read-Host 'Updater key password'
npm ci
npm run tauri -- build --target x86_64-pc-windows-msvc
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Installers are produced below `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`. Do not copy the key into that directory.

