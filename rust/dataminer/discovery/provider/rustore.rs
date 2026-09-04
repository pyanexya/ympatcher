use super::{ProviderQuery, ReleaseProvider};
use crate::channel::ReleaseChannel;
use crate::discovery::http::SharedHttpClient;
use crate::discovery::model::{
    ArtifactVariant, Distribution, ProviderId, ProviderResult, ProviderStatus, Release,
    SourceMetadata,
};
use crate::discovery::version::{inferred_platform, parse_version};
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use std::sync::Arc;

const BASE_URL: &str = "https://backapi.rustore.ru/applicationData/overallInfo/";

pub struct RuStoreProvider {
    http: Arc<SharedHttpClient>,
}

impl RuStoreProvider {
    pub fn new(http: Arc<SharedHttpClient>) -> Self {
        Self { http }
    }
}

#[derive(Debug, Deserialize)]
struct Envelope {
    code: String,
    body: Option<Body>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    package_name: String,
    version_name: String,
    version_code: Option<u64>,
    app_ver_updated_at: Option<String>,
    whats_new: Option<String>,
    min_android_version: Option<String>,
}

impl ReleaseProvider for RuStoreProvider {
    fn id(&self) -> ProviderId {
        ProviderId::RuStore
    }

    fn releases(&self, query: &ProviderQuery) -> ProviderResult<Vec<Release>> {
        tracing::info!(provider = %self.id(), package = %query.package_name, "fetching release metadata");
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("rustorevercode"),
            HeaderValue::from_static("1000"),
        );
        let url = format!("{BASE_URL}{}", query.package_name);
        let response = match self.http.get(&url, headers, &query.cancellation) {
            Ok(value) => value,
            Err(error) => {
                return ProviderResult::failure(
                    self.id(),
                    ProviderStatus::NetworkError,
                    error.to_string(),
                );
            }
        };
        if !response.status.is_success() {
            return ProviderResult::failure(
                self.id(),
                if response.status.as_u16() == 429 {
                    ProviderStatus::RateLimited
                } else {
                    ProviderStatus::Unavailable
                },
                format!("RuStore returned HTTP {}", response.status),
            );
        }
        let envelope: Envelope = match serde_json::from_slice(&response.bytes) {
            Ok(value) => value,
            Err(error) => {
                return ProviderResult::failure(
                    self.id(),
                    ProviderStatus::ParseError,
                    format!("invalid RuStore JSON: {error}"),
                );
            }
        };
        if envelope.code != "OK" {
            return ProviderResult::failure(
                self.id(),
                ProviderStatus::Unavailable,
                format!("RuStore response code: {}", envelope.code),
            );
        }
        let Some(body) = envelope.body else {
            return ProviderResult::failure(
                self.id(),
                ProviderStatus::ParseError,
                "RuStore response has no body",
            );
        };
        if body.package_name != query.package_name {
            return ProviderResult::failure(
                self.id(),
                ProviderStatus::ParseError,
                "RuStore package name mismatch",
            );
        }
        let parsed = parse_version(&body.version_name);
        let platform = inferred_platform(parsed.as_ref());
        if platform != query.platform {
            return ProviderResult::success(self.id(), Vec::new());
        }
        let release = Release {
            package_name: body.package_name,
            version_name: body.version_name,
            version_code: body.version_code,
            parsed,
            channel: ReleaseChannel::Stable,
            distribution: Distribution::RuStore,
            platform,
            published_at: body.app_ver_updated_at,
            discovered_at: Utc::now(),
            min_android: body.min_android_version,
            changelog: body.whats_new,
            unstable: false,
            source: SourceMetadata {
                provider: self.id(),
                source_url: Some(url),
                offer_type: None,
                testing_program_available: false,
                testing_program_subscribed: false,
                early_access_available: false,
            },
            variants: vec![ArtifactVariant {
                architecture: None,
                min_android: None,
                dpi: None,
            }],
        };
        tracing::info!(provider = %self.id(), version = %release.version_name, "found release");
        ProviderResult::success(self.id(), vec![release])
    }
}
