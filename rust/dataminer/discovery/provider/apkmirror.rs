use super::{ProviderQuery, ReleaseProvider};
use crate::channel::ReleaseChannel;
use crate::discovery::http::SharedHttpClient;
use crate::discovery::model::{
    ArtifactVariant, Distribution, Platform, ProviderId, ProviderResult, ProviderStatus, Release,
    SourceMetadata,
};
use crate::discovery::version::{inferred_platform, parse_version};
use chrono::Utc;
use regex::Regex;
use reqwest::{Url, header::HeaderMap};
use scraper::{Html, Selector};
use std::sync::{Arc, OnceLock};

const PHONE_URL: &str =
    "https://www.apkmirror.com/apk/direct-cursus-computer-systems-trading-llc/yandex-music/";
const WEAR_URL: &str = "https://www.apkmirror.com/apk/direct-cursus-computer-systems-trading-llc/yandex-music-books-podcasts-wear-os/";

pub struct ApkMirrorProvider {
    http: Arc<SharedHttpClient>,
}

impl ApkMirrorProvider {
    pub fn new(http: Arc<SharedHttpClient>) -> Self {
        Self { http }
    }
}

impl ReleaseProvider for ApkMirrorProvider {
    fn id(&self) -> ProviderId {
        ProviderId::ApkMirror
    }

    fn releases(&self, query: &ProviderQuery) -> ProviderResult<Vec<Release>> {
        tracing::info!(provider = %self.id(), package = %query.package_name, "fetching release metadata");
        if query.package_name != "ru.yandex.music" {
            return ProviderResult::failure(
                self.id(),
                ProviderStatus::Unavailable,
                "APKMirror page mapping is not configured for this package",
            );
        }
        let url = match query.platform {
            Platform::AndroidPhone => PHONE_URL,
            Platform::WearOs => WEAR_URL,
        };
        let response = match self.http.get(url, HeaderMap::new(), &query.cancellation) {
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
                if matches!(response.status.as_u16(), 403 | 429) {
                    ProviderStatus::RateLimited
                } else {
                    ProviderStatus::Unavailable
                },
                format!("APKMirror returned HTTP {}", response.status),
            );
        }
        let html = match std::str::from_utf8(&response.bytes) {
            Ok(value) => value,
            Err(error) => {
                return ProviderResult::failure(
                    self.id(),
                    ProviderStatus::ParseError,
                    format!("APKMirror returned non-UTF-8 HTML: {error}"),
                );
            }
        };
        match parse_listing(html, &query.package_name, query.platform, url) {
            Ok(mut releases) if !releases.is_empty() => {
                self.enrich_variants(&mut releases, query);
                tracing::info!(provider = %self.id(), releases = releases.len(), "found releases");
                ProviderResult::success(self.id(), releases)
            }
            Ok(_) => ProviderResult::failure(
                self.id(),
                ProviderStatus::ParseError,
                "APKMirror page contained no recognizable release rows",
            ),
            Err(error) => ProviderResult::failure(self.id(), ProviderStatus::ParseError, error),
        }
    }
}

impl ApkMirrorProvider {
    fn enrich_variants(&self, releases: &mut [Release], query: &ProviderQuery) {
        for release in releases.iter_mut().take(3) {
            let Some(url) = release.source.source_url.as_deref() else {
                continue;
            };
            let Ok(response) = self.http.get(url, HeaderMap::new(), &query.cancellation) else {
                continue;
            };
            if !response.status.is_success() {
                continue;
            }
            let Ok(html) = std::str::from_utf8(&response.bytes) else {
                continue;
            };
            let variants = parse_variant_page(html);
            if !variants.is_empty() {
                release.min_android = variants
                    .iter()
                    .find_map(|variant| variant.min_android.clone());
                release.variants = variants;
            }
        }
    }
}

fn release_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(\d{4}\.\d{1,2}\.\d+(?:-[A-Za-z0-9_-]+)?\s+#\d+(?:\.\d+)*[A-Za-z][A-Za-z0-9_-]*)",
        )
        .expect("release pattern is valid")
    })
}

fn metadata_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?i)(arm64-v8a|armeabi-v7a|x86_64|x86|universal|Android\s+[0-9.]+\+|[0-9]+-[0-9]+dpi|nodpi)")
            .expect("metadata pattern is valid")
    })
}

fn parse_listing(
    html: &str,
    package_name: &str,
    requested_platform: Platform,
    source_url: &str,
) -> Result<Vec<Release>, String> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse(".appRow, .listWidget .appRow, article, [data-postid]")
        .map_err(|error| error.to_string())?;
    let time_selector = Selector::parse("time").map_err(|error| error.to_string())?;
    let link_selector = Selector::parse("a[href]").map_err(|error| error.to_string())?;
    let mut releases = Vec::new();
    for row in document.select(&row_selector) {
        let text = row.text().collect::<Vec<_>>().join(" ");
        let Some(version_match) = release_pattern().find(&text) else {
            continue;
        };
        let version_name = version_match.as_str().trim().to_owned();
        let parsed = parse_version(&version_name);
        let platform = inferred_platform(parsed.as_ref());
        if platform != requested_platform {
            continue;
        }
        let variant = parse_variant_text(&text);
        let min_android = variant
            .as_ref()
            .and_then(|variant| variant.min_android.clone());
        let published_at = row
            .select(&time_selector)
            .next()
            .and_then(|node| node.value().attr("datetime"))
            .map(str::to_owned);
        let release_url = row
            .select(&link_selector)
            .filter_map(|link| link.value().attr("href"))
            .find(|href| href.contains("release") || href.contains("apk"))
            .and_then(|href| Url::parse(source_url).ok()?.join(href).ok())
            .map(|url| url.to_string())
            .or_else(|| Some(source_url.to_owned()));
        let unstable = text.to_ascii_lowercase().contains("beta");
        releases.push(Release {
            package_name: package_name.to_owned(),
            version_name,
            version_code: None,
            parsed,
            channel: if unstable {
                ReleaseChannel::Beta
            } else {
                ReleaseChannel::Stable
            },
            distribution: Distribution::GooglePlay,
            platform,
            published_at,
            discovered_at: Utc::now(),
            min_android: min_android.clone(),
            changelog: None,
            unstable,
            source: SourceMetadata {
                provider: ProviderId::ApkMirror,
                source_url: release_url,
                offer_type: None,
                testing_program_available: false,
                testing_program_subscribed: false,
                early_access_available: false,
            },
            variants: variant.into_iter().collect(),
        });
    }
    releases.dedup_by(|left, right| left.version_name == right.version_name);
    Ok(releases)
}

fn parse_variant_page(html: &str) -> Vec<ArtifactVariant> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(".table-row, .variant, [data-variant-id], article")
        .expect("constant selector is valid");
    let mut variants: Vec<_> = document
        .select(&selector)
        .filter_map(|row| parse_variant_text(&row.text().collect::<Vec<_>>().join(" ")))
        .collect();
    variants.dedup();
    variants
}

fn parse_variant_text(text: &str) -> Option<ArtifactVariant> {
    let mut architecture = None;
    let mut min_android = None;
    let mut dpi = None;
    for value in metadata_pattern()
        .find_iter(text)
        .map(|value| value.as_str())
    {
        let lower = value.to_ascii_lowercase();
        if lower.starts_with("android") {
            min_android = Some(value.trim_start_matches("Android").trim().to_owned());
        } else if lower.contains("dpi") {
            dpi = Some(value.to_owned());
        } else {
            architecture = Some(value.to_owned());
        }
    }
    if architecture.is_none() && min_android.is_none() && dpi.is_none() {
        None
    } else {
        Some(ArtifactVariant {
            architecture,
            min_android,
            dpi,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture_and_filters_wear() {
        let html = include_str!("../../../../tests/fixtures/apkmirror-yandex.html");
        let phone =
            parse_listing(html, "ru.yandex.music", Platform::AndroidPhone, PHONE_URL).unwrap();
        assert_eq!(phone.len(), 1);
        assert_eq!(phone[0].version_name, "2026.08.4 #162.1gpr");
        assert_eq!(
            phone[0].variants[0].architecture.as_deref(),
            Some("arm64-v8a")
        );
        let wear = parse_listing(html, "ru.yandex.music", Platform::WearOs, WEAR_URL).unwrap();
        assert_eq!(wear.len(), 1);
        assert!(wear[0].version_name.contains("-wear"));
    }

    #[test]
    fn parses_variant_rows_without_whole_page_regex() {
        let html = include_str!("../../../../tests/fixtures/apkmirror-variants.html");
        let variants = parse_variant_page(html);
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[1].dpi.as_deref(), Some("nodpi"));
    }
}
