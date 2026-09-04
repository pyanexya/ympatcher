# Privacy

Revision date: 2 September 2026. This is a technical draft pending legal review.

`ympatcher` has no mandatory analytics, advertising identifier collection, crash upload, or automatic account-data upload.

The patcher processes input APK files, package metadata, certificate and file hashes, compatibility data, tool output, and signing material locally. Its persistent state can include the signing keystore and descriptor required to update a previously patched installation. These files remain on the user's machine unless the user explicitly moves them.

The Android dataminer reports package metadata, code/resource findings, evidence paths, confidence levels, and hashes. Reports are designed not to include credentials. Before publishing a report, users must still review it for accidentally embedded source data.

Discord Presence is opt-in and uses the official Discord Social SDK. When enabled, selected current-playback fields—such as track, artist, album artwork URL, timing, pause state, and a track link—may be sent to Discord. The module does not use or collect a Discord user token. Disabling Presence clears the activity and stops further updates.

The checked-in GitHub Actions build only the tool and do not request source APKs, signing keys, Google/device credentials or Discord webhooks. Tool downloads and optional online discovery contact their configured providers. GitHub, Discord, Yandex, and download hosts process data under their own policies.

Security reports should avoid APK files, keys, tokens, cookies, account data, and personal information. See `SECURITY.md`.
