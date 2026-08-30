# Flaw Loud v1.1 RC QA

1. First run creates exactly one Owner account.
2. Owner can create User and Moderator accounts; neither can self-promote.
3. Moderator can send announcements and manage reports, but cannot edit roles/access/release policy.
4. Announcement bell increments unread count; reading announcements decrements it.
5. User can submit each report category with up to three image/video attachments.
6. Owner can suspend 24h, block login, ban, restore and revoke sessions for non-Owner accounts.
7. Admin actions appear in Audit Log.
8. Release policy produces UPDATE AVAILABLE during the 24h grace and UPDATE REQUIRED after grace for non-Owner accounts.
9. Owner bypass remains usable on an old version.
10. External EQ/APO Mode audibly preserves more clarity when heavy external EQ is used; verify limiter still protects the output.
11. Existing Engine, Stream Mode, Unload, presets, updater and routing are regression-tested.
