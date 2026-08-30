# Flaw Loud v1.1 RC — Connected Platform

## What is included

- External EQ / Equalizer APO compatibility mode in the native DSP.
- Local Connected Platform account system with Owner / Moderator / User roles.
- First-run Owner bootstrap. There is no public self-registration.
- In-app announcements with unread badge and compact notification tray.
- User bug / audio / compatibility / crash / updater reports with image/video attachments.
- Admin report inbox and report status workflow.
- Owner account creation, Moderator promotion/revocation, suspension, login blocking, banning and session revocation.
- Audit log for sensitive admin actions.
- Version policy with a 24-hour grace period and forced-update gate for public accounts.
- Owner version bypass for retired versions.
- Existing signed Tauri updater remains available for download/install.

## KeyAuth status

KeyAuth is deliberately **not part of the v1.1 Connected Platform login or role system**. The legacy module remains dormant in source so it can be reintroduced later without rebuilding the licensing work from zero.

## Important: local RC backend

This RC intentionally uses a local native backend so every workflow can be tested immediately without deploying infrastructure. Data is stored under the current Windows user's Local AppData in `Flaw Loud/ConnectedPlatform`.

That means announcements/users/reports in this RC are shared only by sessions using the same Windows installation. Before public multi-PC rollout, replace the local command backend with the same contract against a hosted HTTPS service/database. The UI, role model, update policy and workflows are already wired for that transition.

## First run

1. Run `RUN_FLAW_LOUD.bat`.
2. Create the first Owner username/password when prompted.
3. Open **Admin** to create User and Moderator accounts.
4. Send announcements and test the notification bell.
5. Use the bug button in the header to submit a report with optional image/video.
6. Enable **External EQ / APO Mode** when using an Equalizer APO set. Start the headroom slider near the largest positive boost in the EQ set (for example, +8 dB boost → about -8 dB headroom).

## Production backend next step

For a public release, deploy an HTTPS API with the same entities: users, sessions, announcements, announcement reads, reports, attachments, audit events and release policy. Use a production password KDF (Argon2id/bcrypt), database transactions, object storage for media, rate limits, session expiry and moderation permission checks server-side.
