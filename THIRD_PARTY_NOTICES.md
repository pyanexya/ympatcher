# Third-party notices

The `info` and `science` vector icons are derived from Google Material Symbols Rounded.

Copyright Google LLC. Licensed under the Apache License, Version 2.0.

Source: https://github.com/google/material-design-icons

The optional Presence prototype integrates the separately supplied Discord Social SDK. The SDK binary is not committed or redistributed by this repository and remains subject to Discord's applicable developer terms and license. `DISCORD_SOCIAL_SDK_AAR` must point to a lawfully obtained local SDK artifact.

Yandex Music APKs, application code, media, and trademarks are not part of the MIT-licensed project and are not distributed in this repository.

APK merging invokes the separately downloaded [REAndroid APKEditor](https://github.com/REAndroid/APKEditor), version 1.4.9 (Apache-2.0). Its release SHA-256 is pinned in `rust/tools.rs`; the JAR is not vendored. APKEditor uses ARSCLib; consult the upstream distribution for its dependency notices.

The patcher also downloads pinned releases of Apktool (Apache-2.0), uber-apk-signer (Apache-2.0) and Google's R8 (BSD-3-Clause). They are external tools and are not covered by this project's MIT license. Rust dependency versions are recorded in Cargo.lock; each dependency retains its own license.
