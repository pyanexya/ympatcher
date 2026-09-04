use super::model::{
    Confidence, DiscoveryResult, Distribution, Platform, ProviderId, ProviderResult, Release,
    ResolvedRelease,
};
use super::version::compare_versions;
use crate::channel::ReleaseChannel;
use chrono::Utc;
use std::cmp::Ordering;

#[derive(Debug, Default)]
pub struct LatestReleaseResolver;

impl LatestReleaseResolver {
    pub fn resolve(
        &self,
        package_name: &str,
        channel: ReleaseChannel,
        platform: Platform,
        providers: Vec<ProviderResult<Vec<Release>>>,
    ) -> DiscoveryResult {
        let google_candidates = collect(&providers, Distribution::GooglePlay, channel, platform);
        let rustore_candidates = collect(&providers, Distribution::RuStore, channel, platform);
        let latest_google_play = resolve_stream(&google_candidates, Distribution::GooglePlay);
        let latest_rustore = resolve_stream(&rustore_candidates, Distribution::RuStore);
        // `latest` means the upstream Google Play stream. RuStore is an
        // independent distribution and must never silently replace it.
        let latest = latest_google_play.clone();
        if let Some(selected) = &latest {
            tracing::info!(provider = "resolver", version = %selected.release.version_name, distribution = %selected.release.distribution, "selected release");
        }
        DiscoveryResult {
            package_name: package_name.to_owned(),
            checked_at: Utc::now(),
            latest,
            latest_google_play,
            latest_rustore,
            providers,
        }
    }
}

fn collect(
    results: &[ProviderResult<Vec<Release>>],
    distribution: Distribution,
    channel: ReleaseChannel,
    platform: Platform,
) -> Vec<Release> {
    results
        .iter()
        .filter_map(|result| result.value.as_ref())
        .flatten()
        .filter(|release| {
            release.distribution == distribution
                && release.channel == channel
                && release.platform == platform
        })
        .cloned()
        .collect()
}

fn resolve_stream(candidates: &[Release], distribution: Distribution) -> Option<ResolvedRelease> {
    let selected = candidates.iter().max_by(|left, right| {
        compare_release(left, right)
            .then_with(|| provider_priority(left).cmp(&provider_priority(right)))
    })?;
    let mut confirmed_by: Vec<_> = candidates
        .iter()
        .filter(|release| release.version_name == selected.version_name)
        .map(|release| release.source.provider)
        .collect();
    confirmed_by.sort();
    confirmed_by.dedup();
    let has_google = confirmed_by.contains(&ProviderId::GooglePlay);
    let has_apkmirror = confirmed_by.contains(&ProviderId::ApkMirror);
    let confidence = match distribution {
        Distribution::RuStore => Confidence::High,
        Distribution::GooglePlay if has_google && has_apkmirror => Confidence::High,
        Distribution::GooglePlay if has_google => Confidence::MediumHigh,
        _ => Confidence::Medium,
    };
    Some(ResolvedRelease {
        release: selected.clone(),
        confirmed_by,
        confidence,
    })
}

fn compare_release(left: &Release, right: &Release) -> Ordering {
    match (&left.parsed, &right.parsed) {
        (Some(left), Some(right)) => compare_versions(left, right),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => left.version_code.cmp(&right.version_code),
    }
}

fn provider_priority(release: &Release) -> u8 {
    match release.source.provider {
        ProviderId::GooglePlay => 3,
        ProviderId::RuStore => 3,
        ProviderId::ApkMirror => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{ProviderStatus, SourceMetadata};
    use super::super::version::{inferred_platform, parse_version};
    use super::*;

    fn release(name: &str, provider: ProviderId, distribution: Distribution) -> Release {
        let parsed = parse_version(name);
        Release {
            package_name: "ru.yandex.music".to_owned(),
            version_name: name.to_owned(),
            version_code: None,
            platform: inferred_platform(parsed.as_ref()),
            parsed,
            channel: ReleaseChannel::Stable,
            distribution,
            published_at: None,
            discovered_at: Utc::now(),
            min_android: None,
            changelog: None,
            unstable: false,
            source: SourceMetadata {
                provider,
                source_url: None,
                offer_type: None,
                testing_program_available: false,
                testing_program_subscribed: false,
                early_access_available: false,
            },
            variants: Vec::new(),
        }
    }

    fn result(provider: ProviderId, releases: Vec<Release>) -> ProviderResult<Vec<Release>> {
        ProviderResult {
            provider,
            status: ProviderStatus::Success,
            value: Some(releases),
            message: None,
            cached: false,
        }
    }

    #[test]
    fn confirms_google_stream_and_keeps_rustore_separate() {
        let a = "2026.08.4 #162.1gpr";
        let b = "2026.08.3 #161rur";
        let resolved = LatestReleaseResolver.resolve(
            "ru.yandex.music",
            ReleaseChannel::Stable,
            Platform::AndroidPhone,
            vec![
                result(
                    ProviderId::GooglePlay,
                    vec![release(a, ProviderId::GooglePlay, Distribution::GooglePlay)],
                ),
                result(
                    ProviderId::ApkMirror,
                    vec![release(a, ProviderId::ApkMirror, Distribution::GooglePlay)],
                ),
                result(
                    ProviderId::RuStore,
                    vec![release(b, ProviderId::RuStore, Distribution::RuStore)],
                ),
            ],
        );
        assert_eq!(resolved.latest.as_ref().unwrap().release.version_name, a);
        assert_eq!(
            resolved.latest.as_ref().unwrap().confidence,
            Confidence::High
        );
        assert_eq!(
            resolved
                .latest_rustore
                .as_ref()
                .unwrap()
                .release
                .version_name,
            b
        );
    }

    #[test]
    fn provider_failure_does_not_hide_fallback() {
        let fallback = release(
            "2026.08.4 #162.1gpr",
            ProviderId::ApkMirror,
            Distribution::GooglePlay,
        );
        let resolved = LatestReleaseResolver.resolve(
            "ru.yandex.music",
            ReleaseChannel::Stable,
            Platform::AndroidPhone,
            vec![
                ProviderResult::failure(
                    ProviderId::GooglePlay,
                    ProviderStatus::AuthRequired,
                    "auth",
                ),
                result(ProviderId::ApkMirror, vec![fallback]),
            ],
        );
        assert_eq!(resolved.latest.unwrap().confidence, Confidence::Medium);
    }

    #[test]
    fn phone_resolver_ignores_wear() {
        let resolved = LatestReleaseResolver.resolve(
            "ru.yandex.music",
            ReleaseChannel::Stable,
            Platform::AndroidPhone,
            vec![result(
                ProviderId::ApkMirror,
                vec![
                    release(
                        "2026.08.4 #162.1gpr",
                        ProviderId::ApkMirror,
                        Distribution::GooglePlay,
                    ),
                    release(
                        "2026.09.1-wear #170gpr",
                        ProviderId::ApkMirror,
                        Distribution::GooglePlay,
                    ),
                ],
            )],
        );
        assert_eq!(
            resolved.latest.unwrap().release.version_name,
            "2026.08.4 #162.1gpr"
        );
    }
}
