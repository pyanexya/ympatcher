use crate::channel::ReleaseChannel;
use crate::source::PackageFormat;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;

const STABLE_MANIFEST: &str = include_str!("../compatibility/android/stable.json");
const DEV_MANIFEST: &str = include_str!("../compatibility/android/dev.json");
const OFFICIAL_CERTIFICATE_SHA256: &str =
    "aca405ded8b25cb2e8c6da69425d2b4307d087c1276fc06ad5942731ccc51dba";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityManifest {
    schema_version: u32,
    package_name: String,
    lane: String,
    releases: Vec<CompatibleRelease>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompatibleRelease {
    version_name: String,
    version_code: u64,
    channel: ReleaseChannel,
    source: String,
    source_sha256: String,
    official_certificate_sha256: String,
    architectures: Vec<String>,
    package_format: PackageFormat,
    compatibility_status: String,
    patcher_version: Option<String>,
    discovered_at: String,
    verified_at: Option<String>,
    incompatible_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub status: String,
    pub tested_patch_version: Option<String>,
    pub channel: ReleaseChannel,
    pub package_format: PackageFormat,
    pub architectures: Vec<String>,
}

fn parse_manifest(channel: ReleaseChannel) -> Result<CompatibilityManifest> {
    let source = if channel.manifest_lane() == "stable" {
        STABLE_MANIFEST
    } else {
        DEV_MANIFEST
    };
    let manifest: CompatibilityManifest = serde_json::from_str(source).with_context(|| {
        format!(
            "compatibility/android/{}.json повреждён",
            channel.manifest_lane()
        )
    })?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &CompatibilityManifest) -> Result<()> {
    if manifest.schema_version != 2 {
        bail!("неподдерживаемая схема compatibility manifest");
    }
    if manifest.package_name != "ru.yandex.music" {
        bail!("compatibility manifest содержит неверный package name");
    }
    if !matches!(manifest.lane.as_str(), "stable" | "dev") {
        bail!("compatibility manifest содержит неверную lane");
    }
    let mut version_codes = BTreeSet::new();
    for release in &manifest.releases {
        if release.channel.manifest_lane() != manifest.lane {
            bail!(
                "канал {} не принадлежит manifest lane {}",
                release.channel,
                manifest.lane
            );
        }
        if !version_codes.insert(release.version_code) {
            bail!("повторяется versionCode {}", release.version_code);
        }
        if release.source.trim().is_empty()
            || release.source_sha256.len() != 64
            || release.official_certificate_sha256 != OFFICIAL_CERTIFICATE_SHA256
            || release.architectures.is_empty()
            || release.discovered_at.trim().is_empty()
        {
            bail!("неполная compatibility entry для {}", release.version_name);
        }
        if release.compatibility_status == "supported"
            && (release.patcher_version.is_none() || release.verified_at.is_none())
        {
            bail!("supported entry не содержит patcherVersion/verifiedAt");
        }
        if release.compatibility_status == "unsupported" && release.incompatible_reason.is_none() {
            bail!("unsupported entry не содержит incompatibleReason");
        }
    }
    Ok(())
}

pub fn official_certificate() -> &'static str {
    OFFICIAL_CERTIFICATE_SHA256
}

pub fn check(
    channel: ReleaseChannel,
    package_name: &str,
    version_name: &str,
    version_code: &str,
    source_sha256: &str,
    allow_untested: bool,
) -> Result<CompatibilityReport> {
    let manifest = parse_manifest(channel)?;
    if package_name != manifest.package_name {
        bail!(
            "ожидался package {}, получен {package_name}",
            manifest.package_name
        );
    }
    let numeric_code = version_code
        .parse::<u64>()
        .context("versionCode не является положительным числом")?;
    let inferred = ReleaseChannel::classify(version_name, None);
    if !matches!(inferred, ReleaseChannel::Stable | ReleaseChannel::Unknown) && inferred != channel
    {
        bail!(
            "versionName выглядит как канал {inferred}, а выбран {channel}; укажите канал явно и проверьте manifest"
        );
    }
    let Some(release) = manifest
        .releases
        .iter()
        .find(|release| release.version_code == numeric_code)
    else {
        if allow_untested {
            return Ok(CompatibilityReport {
                status: "untested".to_owned(),
                tested_patch_version: None,
                channel,
                package_format: PackageFormat::MonolithicApk,
                architectures: Vec::new(),
            });
        }
        bail!(
            "версия {version_name} ({version_code}) отсутствует в {} manifest; используйте --allow-untested-version только для диагностики",
            channel.manifest_lane()
        );
    };
    if release.channel != channel {
        bail!(
            "версия {} объявлена в канале {}, а выбран {}",
            release.version_name,
            release.channel,
            channel
        );
    }
    if release.version_name != version_name {
        bail!(
            "versionCode {version_code} известен как {}, а APK сообщает {version_name}",
            release.version_name
        );
    }
    if release.source_sha256 != source_sha256 {
        bail!(
            "SHA-256 APK не совпала с {} build {}",
            release.source,
            release.version_name
        );
    }
    if release.compatibility_status != "supported" && !allow_untested {
        let reason = release
            .incompatible_reason
            .as_deref()
            .unwrap_or("fingerprints требуют адаптации");
        bail!(
            "версия {} ({}) имеет статус {}: {}",
            release.version_name,
            release.version_code,
            release.compatibility_status,
            reason
        );
    }
    Ok(CompatibilityReport {
        status: release.compatibility_status.clone(),
        tested_patch_version: release.patcher_version.clone(),
        channel: release.channel,
        package_format: release.package_format,
        architectures: release.architectures.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_are_valid_and_separated_by_lane() {
        let stable = parse_manifest(ReleaseChannel::Stable).unwrap();
        let dev = parse_manifest(ReleaseChannel::Dev).unwrap();
        assert_eq!(stable.lane, "stable");
        assert_eq!(stable.releases.len(), 3);
        assert_eq!(dev.lane, "dev");
        assert!(dev.releases.is_empty());
    }

    #[test]
    fn accepts_current_supported_release() {
        let report = check(
            ReleaseChannel::Stable,
            "ru.yandex.music",
            "2026.08.3 #161rur",
            "24026431",
            "286f6cea9643182d3c47d6a63e1e1f8229450fde6f7011148fccdc0cb68e5ca2",
            false,
        )
        .unwrap();
        assert_eq!(report.status, "supported");
        assert_eq!(report.tested_patch_version.as_deref(), Some("0.3.0"));
        assert_eq!(report.channel, ReleaseChannel::Stable);
    }

    #[test]
    fn rejects_channel_mismatch() {
        let error = check(
            ReleaseChannel::Beta,
            "ru.yandex.music",
            "2026.08.3 #161rur",
            "24026431",
            "286f6cea9643182d3c47d6a63e1e1f8229450fde6f7011148fccdc0cb68e5ca2",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("dev manifest"));
    }
}
