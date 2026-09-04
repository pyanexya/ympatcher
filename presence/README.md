# Discord Presence modules

- app is a standalone companion prototype using Android's user-approved Notification Listener.
- embedded is the optional library injected by --patch discord-rpc. It accepts the client's MediaSession.Token and provides localized settings. It is disabled by default.

No Discord user token or gateway self-bot is used. Supply the official Social SDK AAR separately under its license; it is excluded from Git. The local tested SDK artifact was 1.10.19337, SHA-256 b1b2491f1e1848c79fd6f1986d5aa1e0c8019e88a6e62c7be06b89e8e4870933.

## Local build

Requires JDK 17+, Gradle 8.13, Android SDK platform 35, NDK 28.2.13676358 and CMake 3.22.1. Set ANDROID_HOME and DISCORD_SOCIAL_SDK_AAR (or place the SDK at presence/app/libs/discord_partner_sdk.aar).

```sh
gradle -p presence :embedded:assembleRelease --no-daemon
ympatcher official.apkm -o music.apk --patch about --patch experiments --patch session-compat --patch discord-rpc --discord-embedded-aar presence/embedded/build/outputs/aar/embedded-release.aar --discord-sdk-aar /path/to/discord_partner_sdk.aar
```

The packager stores and aligns native libraries after injection. Java linkage errors are isolated in startup, update and clear; this cannot catch native process crashes inside the SDK. Build verification does not replace on-device testing of startup, playback, account linking and Presence.

When enabled, selected track/artist/artwork/timeline/link fields may be sent to Discord. The module and SDK binaries must not be attached to this repository's public releases.
