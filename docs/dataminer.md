# Upstream release dataminer

The upstream dataminer discovers Android package metadata before an APK enters the patch pipeline. It is separate from the existing APK content analyzer: discovery answers “what release exists?”, while content analysis answers “what changed inside this APK?”.

## Resolution model

The reusable Rust API is `ympatcher::discovery::YandexMusicDataminer`. Providers return normalized releases and independent status values; one failed provider never aborts the other providers.

Source priority for the Google Play distribution stream is:

1. Google Play FDFE structured protobuf metadata;
2. APKMirror HTML metadata as confirmation or fallback.

RuStore is authoritative only for its own distribution stream. A RuStore release is exposed under `distributions.ruStore` and never silently replaces the top-level Google Play `latest`. Android phone and Wear OS releases are resolved separately.

## Google Play authentication

Google Play has no public API for reading or downloading another publisher's APK. The provider therefore implements the current reverse-engineered FDFE `details` protobuf contract used by Aurora Store/GPlayApi, behind an isolated provider boundary. It accepts an externally acquired session and never stores credentials:

- `YM_GOOGLE_PLAY_AUTH_TOKEN` — bearer auth token;
- `YM_GOOGLE_PLAY_GSF_ID` — registered device GSF id;
- `YM_GOOGLE_PLAY_DEVICE_CONFIG_TOKEN` — optional device configuration token.

Without the first two values, Google Play returns `auth_required` and APKMirror/RuStore still run. ympatcher does not embed somebody else's anonymous token dispenser, Google account password, cookies, or static credentials. An expired or incompatible session is reported as `auth_required` without logging secret headers.

Protocol references are the maintained [AuroraOSS GPlayApi source](https://gitlab.com/AuroraOSS/gplayapi/-/tree/master), its [published Maven artifact](https://central.sonatype.com/artifact/com.auroraoss/gplayapi), and [Aurora Store](https://github.com/whyorean/AuroraStore). ympatcher contains an original minimal `prost` model for only the fields it consumes; it does not vendor their GPL implementation.

## CLI

```text
ympatcher dataminer latest
ympatcher dataminer latest --json
ympatcher dataminer latest --distribution google-play --force-refresh
ympatcher dataminer latest --platform wear-os
```

Device-sensitive Play metadata can be queried with `--sdk`, `--model`, `--manufacturer`, `--abi`, `--locale`, and `--country`. The default is a current Android phone profile, not Wear OS.

JSON is a public DTO with `schemaVersion: 1`; it does not serialize internal Rust models directly. `versionCode` is only populated when a provider reports a real value. Provider statuses are `success`, `unavailable`, `auth_required`, `rate_limited`, `parse_error`, or `network_error`.

## HTTP, cache, and diagnostics

All providers share one HTTP client with connect/request timeouts, bounded exponential retry for transport errors, HTTP 429 and 5xx, a cancellation token, and secret-free structured tracing. Successful provider results are cached in memory for 15 minutes. `--force-refresh` bypasses the cache. Set `RUST_LOG` to customize tracing.

## Tests and limitations

Unit tests use compact APKMirror HTML and Google Play protobuf fixtures and never need the network. Optional live checks are enabled with:

```text
YM_DATAMINER_LIVE_TESTS=1 cargo test --test discovery_live
```

The Google Play live check also requires the session variables above. APKMirror may return HTTP 403/429 to automated clients; that is surfaced as `rate_limited` rather than bypassed with an untrusted proxy. Google Play protocol or authentication changes are isolated to its provider. RuStore can publish a different version and changelog from Google Play by design.
