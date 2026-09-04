use super::model::{
    Confidence, DiscoveryResult, Distribution, ProviderId, ProviderResult, ProviderStatus, Release,
    ResolvedRelease,
};
use crate::channel::ReleaseChannel;
use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDto {
    pub schema_version: u32,
    pub package: String,
    pub checked_at: String,
    pub latest: Option<ReleaseDto>,
    pub distributions: DistributionsDto,
    pub providers: ProvidersDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionsDto {
    pub google_play: Option<ReleaseDto>,
    pub ru_store: Option<ReleaseDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidersDto {
    pub google_play: ProviderDto,
    pub apk_mirror: ProviderDto,
    pub ru_store: ProviderDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDto {
    pub status: ProviderStatus,
    pub message: Option<String>,
    pub cached: bool,
    pub release_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseDto {
    pub package_name: String,
    pub version_name: String,
    pub version_code: Option<u64>,
    pub release_base: Option<String>,
    pub build_number: Option<String>,
    pub build_suffix: Option<String>,
    pub channel: ReleaseChannel,
    pub distribution: Distribution,
    pub platform: String,
    pub published_at: Option<String>,
    pub min_android: Option<String>,
    pub changelog: Option<String>,
    pub unstable: bool,
    pub confirmed_by: Vec<ProviderId>,
    pub confidence: Confidence,
    pub testing_program_available: bool,
    pub testing_program_subscribed: bool,
    pub early_access_available: bool,
}

impl DiscoveryDto {
    pub fn from_result(result: &DiscoveryResult) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            package: result.package_name.clone(),
            checked_at: result.checked_at.to_rfc3339(),
            latest: result.latest.as_ref().map(ReleaseDto::from),
            distributions: DistributionsDto {
                google_play: result.latest_google_play.as_ref().map(ReleaseDto::from),
                ru_store: result.latest_rustore.as_ref().map(ReleaseDto::from),
            },
            providers: ProvidersDto {
                google_play: provider_dto(&result.providers, ProviderId::GooglePlay),
                apk_mirror: provider_dto(&result.providers, ProviderId::ApkMirror),
                ru_store: provider_dto(&result.providers, ProviderId::RuStore),
            },
        }
    }
}

impl From<&ResolvedRelease> for ReleaseDto {
    fn from(resolved: &ResolvedRelease) -> Self {
        let release = &resolved.release;
        Self {
            package_name: release.package_name.clone(),
            version_name: release.version_name.clone(),
            version_code: release.version_code,
            release_base: release
                .parsed
                .as_ref()
                .map(|value| format!("{}.{:02}.{}", value.year, value.month, value.patch)),
            build_number: release.parsed.as_ref().map(|value| {
                value
                    .build
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(".")
            }),
            build_suffix: release
                .parsed
                .as_ref()
                .and_then(|value| value.suffix.clone()),
            channel: release.channel,
            distribution: release.distribution,
            platform: release.platform.to_string(),
            published_at: release.published_at.clone(),
            min_android: release.min_android.clone(),
            changelog: release.changelog.clone(),
            unstable: release.unstable,
            confirmed_by: resolved.confirmed_by.clone(),
            confidence: resolved.confidence,
            testing_program_available: release.source.testing_program_available,
            testing_program_subscribed: release.source.testing_program_subscribed,
            early_access_available: release.source.early_access_available,
        }
    }
}

fn provider_dto(providers: &[ProviderResult<Vec<Release>>], provider: ProviderId) -> ProviderDto {
    providers
        .iter()
        .find(|result| result.provider == provider)
        .map(|result| ProviderDto {
            status: result.status,
            message: result.message.clone(),
            cached: result.cached,
            release_count: result.value.as_ref().map(Vec::len).unwrap_or_default(),
        })
        .unwrap_or(ProviderDto {
            status: ProviderStatus::Unavailable,
            message: Some("provider was not configured".to_owned()),
            cached: false,
            release_count: 0,
        })
}

pub fn render_human(result: &DiscoveryResult) -> String {
    let mut output = format!(
        "Yandex Music Android\nChecked: {}\n\n",
        result.checked_at.to_rfc3339()
    );
    for (id, title) in [
        (ProviderId::GooglePlay, "Google Play"),
        (ProviderId::ApkMirror, "APKMirror"),
        (ProviderId::RuStore, "RuStore"),
    ] {
        output.push_str(title);
        output.push('\n');
        if let Some(provider) = result
            .providers
            .iter()
            .find(|provider| provider.provider == id)
        {
            output.push_str(&format!(
                "  Status:       {:?}{}\n",
                provider.status,
                if provider.cached { " (cached)" } else { "" }
            ));
            if let Some(release) = provider.value.as_ref().and_then(|items| items.first()) {
                output.push_str(&format!("  Version:      {}\n", release.version_name));
                output.push_str(&format!(
                    "  Version code: {}\n",
                    release
                        .version_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "not reported".to_owned())
                ));
            }
            if let Some(message) = &provider.message {
                output.push_str(&format!("  Diagnostic:   {message}\n"));
            }
        }
        output.push('\n');
    }
    output.push_str("Resolved latest\n");
    if let Some(latest) = &result.latest {
        output.push_str(&format!(
            "  Version:      {}\n",
            latest.release.version_name
        ));
        output.push_str(&format!(
            "  Distribution: {}\n",
            latest.release.distribution
        ));
        output.push_str(&format!("  Confidence:   {:?}\n", latest.confidence));
        output.push_str(&format!(
            "  Confirmed by: {}\n",
            latest
                .confirmed_by
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        output.push_str("  No matching release was found.\n");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn schema_is_explicit_and_stable() {
        let result = DiscoveryResult {
            package_name: "ru.yandex.music".to_owned(),
            checked_at: Utc::now(),
            latest: None,
            latest_google_play: None,
            latest_rustore: None,
            providers: Vec::new(),
        };
        let json = serde_json::to_value(DiscoveryDto::from_result(&result)).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert!(json.get("providers").is_some());
        assert!(json.get("distributions").is_some());
    }
}
