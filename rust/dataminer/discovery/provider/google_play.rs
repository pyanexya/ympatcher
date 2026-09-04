use super::{ProviderQuery, ReleaseProvider};
use crate::channel::ReleaseChannel;
use crate::discovery::http::SharedHttpClient;
use crate::discovery::model::{
    Distribution, ProviderId, ProviderResult, ProviderStatus, Release, SourceMetadata,
};
use crate::discovery::version::{inferred_distribution, inferred_platform, parse_version};
use chrono::Utc;
use prost::Message;
use reqwest::Url;
use reqwest::header::{
    ACCEPT_LANGUAGE, AUTHORIZATION, HeaderMap, HeaderName, HeaderValue, USER_AGENT,
};
use std::env;
use std::sync::Arc;

const DETAILS_URL: &str = "https://android.clients.google.com/fdfe/details";
const FINSKY_VERSION: &str = "39.7.34-21 [0] [PR] 608437623";

#[derive(Debug, Clone, Default)]
pub struct GooglePlayAuth {
    pub auth_token: Option<String>,
    pub gsf_id: Option<String>,
    pub device_config_token: Option<String>,
}

impl GooglePlayAuth {
    pub fn from_env() -> Self {
        Self {
            auth_token: env::var("YM_GOOGLE_PLAY_AUTH_TOKEN")
                .ok()
                .filter(|v| !v.is_empty()),
            gsf_id: env::var("YM_GOOGLE_PLAY_GSF_ID")
                .ok()
                .filter(|v| !v.is_empty()),
            device_config_token: env::var("YM_GOOGLE_PLAY_DEVICE_CONFIG_TOKEN")
                .ok()
                .filter(|v| !v.is_empty()),
        }
    }

    fn ready(&self) -> bool {
        self.auth_token.is_some() && self.gsf_id.is_some()
    }
}

pub struct GooglePlayProvider {
    http: Arc<SharedHttpClient>,
    auth: GooglePlayAuth,
}

impl GooglePlayProvider {
    pub fn new(http: Arc<SharedHttpClient>, auth: GooglePlayAuth) -> Self {
        Self { http, auth }
    }

    pub fn from_environment(http: Arc<SharedHttpClient>) -> Self {
        Self::new(http, GooglePlayAuth::from_env())
    }
}

impl ReleaseProvider for GooglePlayProvider {
    fn id(&self) -> ProviderId {
        ProviderId::GooglePlay
    }

    fn releases(&self, query: &ProviderQuery) -> ProviderResult<Vec<Release>> {
        tracing::info!(provider = %self.id(), package = %query.package_name, "fetching release metadata");
        if !self.auth.ready() {
            return ProviderResult::failure(
                self.id(),
                ProviderStatus::AuthRequired,
                "Google Play metadata requires YM_GOOGLE_PLAY_AUTH_TOKEN and YM_GOOGLE_PLAY_GSF_ID",
            );
        }
        let mut url = Url::parse(DETAILS_URL).expect("constant Google Play URL is valid");
        let language = query.device.locale.split(['-', '_']).next().unwrap_or("en");
        url.query_pairs_mut()
            .append_pair("doc", &query.package_name)
            .append_pair("device_country", &query.device.country.to_ascii_lowercase())
            .append_pair("lang", &language.to_ascii_lowercase());
        let headers = match self.headers(query) {
            Ok(value) => value,
            Err(message) => {
                return ProviderResult::failure(self.id(), ProviderStatus::AuthRequired, message);
            }
        };
        let response = match self.http.get(url.as_str(), headers, &query.cancellation) {
            Ok(value) => value,
            Err(error) => {
                return ProviderResult::failure(
                    self.id(),
                    ProviderStatus::NetworkError,
                    error.to_string(),
                );
            }
        };
        if matches!(response.status.as_u16(), 401 | 403) {
            return ProviderResult::failure(
                self.id(),
                ProviderStatus::AuthRequired,
                format!(
                    "Google Play rejected the configured session (HTTP {})",
                    response.status
                ),
            );
        }
        if !response.status.is_success() {
            return ProviderResult::failure(
                self.id(),
                if response.status.as_u16() == 429 {
                    ProviderStatus::RateLimited
                } else {
                    ProviderStatus::Unavailable
                },
                format!("Google Play returned HTTP {}", response.status),
            );
        }
        match parse_details(&response.bytes, query) {
            Ok(release) => {
                tracing::info!(provider = %self.id(), version = %release.version_name, "found release");
                ProviderResult::success(self.id(), vec![release])
            }
            Err(message) => ProviderResult::failure(self.id(), ProviderStatus::ParseError, message),
        }
    }
}

impl GooglePlayProvider {
    fn headers(&self, query: &ProviderQuery) -> Result<HeaderMap, String> {
        let token = self.auth.auth_token.as_ref().ok_or("missing auth token")?;
        let gsf_id = self.auth.gsf_id.as_ref().ok_or("missing GSF id")?;
        let mut headers = HeaderMap::new();
        let device_name = query
            .device
            .manufacturer
            .to_ascii_lowercase()
            .replace(' ', "_");
        let user_agent = format!(
            "Android-Finsky/{FINSKY_VERSION} (api={sdk},sdk={sdk},device={device},hardware={device},product={device},platformVersionRelease={sdk},model={model},isWideScreen=0,supportedAbis={abi})",
            sdk = query.device.sdk,
            device = device_name,
            model = query.device.model,
            abi = query.device.abi,
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent).map_err(|_| "invalid device profile")?,
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| "invalid Google Play auth token")?,
        );
        headers.insert(
            HeaderName::from_static("x-dfe-device-id"),
            HeaderValue::from_str(gsf_id).map_err(|_| "invalid Google Play GSF id")?,
        );
        headers.insert(
            HeaderName::from_static("x-dfe-client-id"),
            HeaderValue::from_static("am-android-google"),
        );
        headers.insert(
            HeaderName::from_static("x-dfe-network-type"),
            HeaderValue::from_static("4"),
        );
        headers.insert(
            HeaderName::from_static("x-dfe-request-params"),
            HeaderValue::from_static("timeoutMs=4000"),
        );
        headers.insert(
            HeaderName::from_static("x-dfe-userlanguages"),
            HeaderValue::from_str(&query.device.locale).map_err(|_| "invalid device locale")?,
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_str(&query.device.locale).map_err(|_| "invalid device locale")?,
        );
        if let Some(config_token) = &self.auth.device_config_token {
            headers.insert(
                HeaderName::from_static("x-dfe-device-config-token"),
                HeaderValue::from_str(config_token).map_err(|_| "invalid device config token")?,
            );
        }
        Ok(headers)
    }
}

fn parse_details(bytes: &[u8], query: &ProviderQuery) -> Result<Release, String> {
    let wrapper = ResponseWrapper::decode(bytes)
        .map_err(|error| format!("invalid Google Play protobuf: {error}"))?;
    let item = wrapper
        .payload
        .and_then(|value| value.details_response)
        .and_then(|value| value.item)
        .ok_or("Google Play protobuf has no details item")?;
    let offer_type = item.offer.first().and_then(|offer| offer.offer_type);
    let app = item
        .details
        .and_then(|value| value.app_details)
        .ok_or("Google Play protobuf has no app details")?;
    let package_name = app
        .package_name
        .ok_or("Google Play response has no package name")?;
    if package_name != query.package_name {
        return Err("Google Play package name mismatch".to_owned());
    }
    let version_name = app
        .version_string
        .ok_or("Google Play response has no version string")?;
    let parsed = parse_version(&version_name);
    let platform = inferred_platform(parsed.as_ref());
    let inferred = inferred_distribution(parsed.as_ref());
    if inferred != Distribution::Unknown && inferred != Distribution::GooglePlay {
        tracing::warn!(provider = "google-play", suffix_distribution = %inferred, version = %version_name, "distribution suffix disagrees with provider");
    }
    if platform != query.platform {
        return Err(format!(
            "Google Play returned {platform}, requested {}",
            query.platform
        ));
    }
    let testing = app.testing_program_info;
    let early_access_available = app.early_access_info.is_some();
    let testing_program_available = testing.is_some();
    let testing_program_subscribed = testing
        .as_ref()
        .and_then(|value| value.subscribed)
        .unwrap_or(false);
    let unstable = early_access_available || testing_program_subscribed;
    let channel = if unstable && query.channel == ReleaseChannel::Beta {
        ReleaseChannel::Beta
    } else {
        ReleaseChannel::Stable
    };
    Ok(Release {
        package_name,
        version_name,
        version_code: app.version_code.and_then(|value| u64::try_from(value).ok()),
        parsed,
        channel,
        distribution: Distribution::GooglePlay,
        platform,
        published_at: app.info_updated_on,
        discovered_at: Utc::now(),
        min_android: None,
        changelog: app.recent_changes_html,
        unstable,
        source: SourceMetadata {
            provider: ProviderId::GooglePlay,
            source_url: Some(DETAILS_URL.to_owned()),
            offer_type,
            testing_program_available,
            testing_program_subscribed,
            early_access_available,
        },
        variants: Vec::new(),
    })
}

#[derive(Clone, PartialEq, Message)]
struct ResponseWrapper {
    #[prost(message, optional, tag = "1")]
    payload: Option<Payload>,
}

#[derive(Clone, PartialEq, Message)]
struct Payload {
    #[prost(message, optional, tag = "2")]
    details_response: Option<DetailsResponse>,
}

#[derive(Clone, PartialEq, Message)]
struct DetailsResponse {
    #[prost(message, optional, tag = "4")]
    item: Option<Item>,
}

#[derive(Clone, PartialEq, Message)]
struct Item {
    #[prost(message, repeated, tag = "8")]
    offer: Vec<Offer>,
    #[prost(message, optional, tag = "13")]
    details: Option<DocumentDetails>,
}

#[derive(Clone, PartialEq, Message)]
struct Offer {
    #[prost(int32, optional, tag = "8")]
    offer_type: Option<i32>,
}

#[derive(Clone, PartialEq, Message)]
struct DocumentDetails {
    #[prost(message, optional, tag = "1")]
    app_details: Option<AppDetails>,
}

#[derive(Clone, PartialEq, Message)]
struct AppDetails {
    #[prost(int64, optional, tag = "3")]
    version_code: Option<i64>,
    #[prost(string, optional, tag = "4")]
    version_string: Option<String>,
    #[prost(string, optional, tag = "14")]
    package_name: Option<String>,
    #[prost(string, optional, tag = "15")]
    recent_changes_html: Option<String>,
    #[prost(string, optional, tag = "16")]
    info_updated_on: Option<String>,
    #[prost(message, optional, tag = "35")]
    testing_program_info: Option<TestingProgramInfo>,
    #[prost(message, optional, tag = "36")]
    early_access_info: Option<EarlyAccessInfo>,
}

#[derive(Clone, PartialEq, Message)]
struct TestingProgramInfo {
    #[prost(bool, optional, tag = "2")]
    subscribed: Option<bool>,
    #[prost(bool, optional, tag = "3")]
    subscribed_and_installed: Option<bool>,
}

#[derive(Clone, PartialEq, Message)]
struct EarlyAccessInfo {
    #[prost(string, optional, tag = "3")]
    email: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::http::CancellationToken;
    use crate::discovery::model::{DeviceProfile, Platform};

    #[test]
    fn parses_minimal_details_protobuf() {
        let wrapper = ResponseWrapper {
            payload: Some(Payload {
                details_response: Some(DetailsResponse {
                    item: Some(Item {
                        offer: vec![Offer {
                            offer_type: Some(1),
                        }],
                        details: Some(DocumentDetails {
                            app_details: Some(AppDetails {
                                version_code: Some(24_026_482),
                                version_string: Some("2026.08.4 #162.1gpr".to_owned()),
                                package_name: Some("ru.yandex.music".to_owned()),
                                recent_changes_html: Some("Changes".to_owned()),
                                info_updated_on: Some("2026-08-29".to_owned()),
                                testing_program_info: None,
                                early_access_info: None,
                            }),
                        }),
                    }),
                }),
            }),
        };
        let query = ProviderQuery {
            package_name: "ru.yandex.music".to_owned(),
            channel: ReleaseChannel::Stable,
            platform: Platform::AndroidPhone,
            device: DeviceProfile::default(),
            cancellation: CancellationToken::default(),
        };
        let encoded = wrapper.encode_to_vec();
        let fixture =
            hex::decode(include_str!("../../../../tests/fixtures/google-play-details.hex").trim())
                .unwrap();
        assert_eq!(encoded, fixture);
        let release = parse_details(&fixture, &query).unwrap();
        assert_eq!(release.version_code, Some(24_026_482));
        assert_eq!(release.source.offer_type, Some(1));
    }
}
