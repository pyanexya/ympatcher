use super::http::{CancellationToken, SharedHttpClient};
use super::model::{
    DeviceProfile, DiscoveryResult, Distribution, Platform, ProviderId, ProviderResult, Release,
    ResolvedRelease,
};
use super::provider::{
    ApkMirrorProvider, GooglePlayProvider, ProviderQuery, ReleaseProvider, RuStoreProvider,
};
use super::resolver::LatestReleaseResolver;
use crate::channel::ReleaseChannel;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const YANDEX_MUSIC_PACKAGE: &str = "ru.yandex.music";

#[derive(Debug, Clone)]
pub struct DataminerConfig {
    pub package_name: String,
    pub channel: ReleaseChannel,
    pub platform: Platform,
    pub device: DeviceProfile,
    pub cache_ttl: Duration,
}

impl Default for DataminerConfig {
    fn default() -> Self {
        Self {
            package_name: YANDEX_MUSIC_PACKAGE.to_owned(),
            channel: ReleaseChannel::Stable,
            platform: Platform::AndroidPhone,
            device: DeviceProfile::default(),
            cache_ttl: Duration::from_secs(15 * 60),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct CacheKey {
    package_name: String,
    provider: ProviderId,
    device: DeviceProfile,
    channel: ReleaseChannel,
    platform: Platform,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    stored_at: Instant,
    result: ProviderResult<Vec<Release>>,
}

pub struct YandexMusicDataminer {
    config: DataminerConfig,
    providers: Vec<Arc<dyn ReleaseProvider>>,
    cache: Mutex<HashMap<CacheKey, CacheEntry>>,
}

impl YandexMusicDataminer {
    pub fn new(config: DataminerConfig) -> Result<Self> {
        let http = Arc::new(SharedHttpClient::new()?);
        let providers: Vec<Arc<dyn ReleaseProvider>> = vec![
            Arc::new(GooglePlayProvider::from_environment(http.clone())),
            Arc::new(ApkMirrorProvider::new(http.clone())),
            Arc::new(RuStoreProvider::new(http)),
        ];
        Ok(Self::with_providers(config, providers))
    }

    pub fn with_providers(
        config: DataminerConfig,
        providers: Vec<Arc<dyn ReleaseProvider>>,
    ) -> Self {
        Self {
            config,
            providers,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn discover(&self, force_refresh: bool) -> DiscoveryResult {
        self.discover_with_cancellation(force_refresh, CancellationToken::default())
    }

    pub fn discover_with_cancellation(
        &self,
        force_refresh: bool,
        cancellation: CancellationToken,
    ) -> DiscoveryResult {
        let query = ProviderQuery {
            package_name: self.config.package_name.clone(),
            channel: self.config.channel,
            platform: self.config.platform,
            device: self.config.device.clone(),
            cancellation,
        };
        let results = thread::scope(|scope| {
            let handles: Vec<_> = self
                .providers
                .iter()
                .map(|provider| {
                    let query = query.clone();
                    let provider_id = provider.id();
                    (
                        provider_id,
                        scope.spawn(move || {
                            self.query_provider(provider.as_ref(), &query, force_refresh)
                        }),
                    )
                })
                .collect();
            handles
                .into_iter()
                .map(|(provider, handle)| {
                    handle.join().unwrap_or_else(|_| {
                        ProviderResult::failure(
                            provider,
                            super::model::ProviderStatus::Unavailable,
                            "provider worker panicked",
                        )
                    })
                })
                .collect()
        });
        LatestReleaseResolver.resolve(
            &self.config.package_name,
            self.config.channel,
            self.config.platform,
            results,
        )
    }

    pub fn get_latest_release(&self, force_refresh: bool) -> Option<ResolvedRelease> {
        self.discover(force_refresh).latest
    }

    pub fn get_latest_release_for(
        &self,
        distribution: Distribution,
        force_refresh: bool,
    ) -> Option<ResolvedRelease> {
        let result = self.discover(force_refresh);
        match distribution {
            Distribution::GooglePlay => result.latest_google_play,
            Distribution::RuStore => result.latest_rustore,
            Distribution::Unknown => result.latest,
        }
    }

    fn query_provider(
        &self,
        provider: &dyn ReleaseProvider,
        query: &ProviderQuery,
        force_refresh: bool,
    ) -> ProviderResult<Vec<Release>> {
        let key = CacheKey {
            package_name: query.package_name.clone(),
            provider: provider.id(),
            device: query.device.clone(),
            channel: query.channel,
            platform: query.platform,
        };
        if !force_refresh
            && let Some(entry) = self.cache.lock().expect("cache mutex poisoned").get(&key)
            && entry.stored_at.elapsed() < self.config.cache_ttl
        {
            let mut cached = entry.result.clone();
            cached.cached = true;
            return cached;
        }
        let result = provider.releases(query);
        if result.value.is_some() {
            self.cache.lock().expect("cache mutex poisoned").insert(
                key,
                CacheEntry {
                    stored_at: Instant::now(),
                    result: result.clone(),
                },
            );
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{ProviderId, ProviderStatus};
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingProvider(Arc<AtomicUsize>);

    impl ReleaseProvider for CountingProvider {
        fn id(&self) -> ProviderId {
            ProviderId::ApkMirror
        }

        fn releases(&self, _query: &ProviderQuery) -> ProviderResult<Vec<Release>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            ProviderResult::success(self.id(), Vec::new())
        }
    }

    #[test]
    fn caches_success_and_force_refresh_bypasses_it() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider: Arc<dyn ReleaseProvider> = Arc::new(CountingProvider(calls.clone()));
        let dataminer =
            YandexMusicDataminer::with_providers(DataminerConfig::default(), vec![provider]);
        let first = dataminer.discover(false);
        let second = dataminer.discover(false);
        assert_eq!(first.providers[0].status, ProviderStatus::Success);
        assert!(second.providers[0].cached);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        dataminer.discover(true);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
