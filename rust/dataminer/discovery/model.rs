use crate::channel::ReleaseChannel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Distribution {
    GooglePlay,
    #[serde(rename = "rustore")]
    RuStore,
    Unknown,
}

impl fmt::Display for Distribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GooglePlay => "google-play",
            Self::RuStore => "rustore",
            Self::Unknown => "unknown",
        })
    }
}

impl FromStr for Distribution {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "google-play" | "googleplay" | "gpr" => Ok(Self::GooglePlay),
            "rustore" | "ru-store" | "rur" => Ok(Self::RuStore),
            "unknown" => Ok(Self::Unknown),
            _ => anyhow::bail!("unknown distribution: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    AndroidPhone,
    WearOs,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AndroidPhone => "android-phone",
            Self::WearOs => "wear-os",
        })
    }
}

impl FromStr for Platform {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "android-phone" | "phone" | "android" => Ok(Self::AndroidPhone),
            "wear-os" | "wear" => Ok(Self::WearOs),
            _ => anyhow::bail!("unknown platform: {value}"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProfile {
    pub sdk: u32,
    pub model: String,
    pub manufacturer: String,
    pub abi: String,
    pub locale: String,
    pub country: String,
}

impl Default for DeviceProfile {
    fn default() -> Self {
        Self {
            sdk: 35,
            model: "Pixel 8".to_owned(),
            manufacturer: "Google".to_owned(),
            abi: "arm64-v8a".to_owned(),
            locale: "ru-RU".to_owned(),
            country: "RU".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedVersion {
    pub year: u32,
    pub month: u32,
    pub patch: u32,
    pub variant: Option<String>,
    pub build: Vec<u32>,
    pub suffix: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVariant {
    pub architecture: Option<String>,
    pub min_android: Option<String>,
    pub dpi: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMetadata {
    pub provider: ProviderId,
    pub source_url: Option<String>,
    pub offer_type: Option<i32>,
    pub testing_program_available: bool,
    pub testing_program_subscribed: bool,
    pub early_access_available: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub package_name: String,
    pub version_name: String,
    pub version_code: Option<u64>,
    pub parsed: Option<ParsedVersion>,
    pub channel: ReleaseChannel,
    pub distribution: Distribution,
    pub platform: Platform,
    pub published_at: Option<String>,
    pub discovered_at: DateTime<Utc>,
    pub min_android: Option<String>,
    pub changelog: Option<String>,
    pub unstable: bool,
    pub source: SourceMetadata,
    pub variants: Vec<ArtifactVariant>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    GooglePlay,
    #[serde(rename = "apkmirror")]
    ApkMirror,
    #[serde(rename = "rustore")]
    RuStore,
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GooglePlay => "google-play",
            Self::ApkMirror => "apkmirror",
            Self::RuStore => "rustore",
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Success,
    Unavailable,
    AuthRequired,
    RateLimited,
    ParseError,
    NetworkError,
}

#[derive(Debug, Clone)]
pub struct ProviderResult<T> {
    pub provider: ProviderId,
    pub status: ProviderStatus,
    pub value: Option<T>,
    pub message: Option<String>,
    pub cached: bool,
}

impl<T> ProviderResult<T> {
    pub fn success(provider: ProviderId, value: T) -> Self {
        Self {
            provider,
            status: ProviderStatus::Success,
            value: Some(value),
            message: None,
            cached: false,
        }
    }

    pub fn failure(
        provider: ProviderId,
        status: ProviderStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            status,
            value: None,
            message: Some(message.into()),
            cached: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    High,
    MediumHigh,
    Medium,
}

#[derive(Debug, Clone)]
pub struct ResolvedRelease {
    pub release: Release,
    pub confirmed_by: Vec<ProviderId>,
    pub confidence: Confidence,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub package_name: String,
    pub checked_at: DateTime<Utc>,
    pub latest: Option<ResolvedRelease>,
    pub latest_google_play: Option<ResolvedRelease>,
    pub latest_rustore: Option<ResolvedRelease>,
    pub providers: Vec<ProviderResult<Vec<Release>>>,
}
