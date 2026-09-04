# Patched-client settings architecture

This document separates shipped controls from planned controls so the UI never claims a non-functional feature.

| Section | Current state |
|---|---|
| General | Yandex Music retains its native theme and language controls; update channel is explicit patch metadata, but automatic update UI is planned |
| Interface | Native Yandex Music interface settings remain untouched; column reordering is planned |
| Discord Presence | Localized experimental embedded screen implements enable, activity type, artwork, progress, link, pause behavior, and artist/title order; not in stable profile |
| Developer | Separate localized bottom sheet implements experiment search, individual overrides, force-all and reset-to-server |
| About | Contains only project identity, patcher version, author, GitHub, and close action |

The About sheet must not gain experiments, column controls, version-spoof switches, or diagnostics. Developer controls stay behind the science/flask row. New text must use Android string resources with English defaults and Russian overrides. Colors follow the host/system light or dark theme; surfaces, touch targets, contrast, and bottom-sheet dismissal follow Material 3 guidance while retaining the visual rhythm of Yandex Music.
