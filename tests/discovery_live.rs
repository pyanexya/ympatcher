use ympatcher::discovery::model::{ProviderId, ProviderStatus};
use ympatcher::discovery::{DataminerConfig, YandexMusicDataminer};

fn live_enabled() -> bool {
    std::env::var("YM_DATAMINER_LIVE_TESTS").as_deref() == Ok("1")
}

#[test]
fn live_rustore_test() {
    if !live_enabled() {
        return;
    }
    let result = YandexMusicDataminer::new(DataminerConfig::default())
        .unwrap()
        .discover(true);
    let provider = result
        .providers
        .iter()
        .find(|result| result.provider == ProviderId::RuStore)
        .unwrap();
    assert_eq!(
        provider.status,
        ProviderStatus::Success,
        "{:?}",
        provider.message
    );
    assert!(!provider.value.as_ref().unwrap().is_empty());
}

#[test]
fn live_apkmirror_test() {
    if !live_enabled() {
        return;
    }
    let result = YandexMusicDataminer::new(DataminerConfig::default())
        .unwrap()
        .discover(true);
    let provider = result
        .providers
        .iter()
        .find(|result| result.provider == ProviderId::ApkMirror)
        .unwrap();
    if provider.status == ProviderStatus::Success {
        assert!(!provider.value.as_ref().unwrap().is_empty());
    } else {
        assert!(
            matches!(
                provider.status,
                ProviderStatus::RateLimited | ProviderStatus::Unavailable
            ),
            "unexpected APKMirror failure: {:?}",
            provider.message
        );
    }
}

#[test]
fn live_google_play_test() {
    if !live_enabled()
        || std::env::var_os("YM_GOOGLE_PLAY_AUTH_TOKEN").is_none()
        || std::env::var_os("YM_GOOGLE_PLAY_GSF_ID").is_none()
    {
        return;
    }
    let result = YandexMusicDataminer::new(DataminerConfig::default())
        .unwrap()
        .discover(true);
    let provider = result
        .providers
        .iter()
        .find(|result| result.provider == ProviderId::GooglePlay)
        .unwrap();
    assert_eq!(
        provider.status,
        ProviderStatus::Success,
        "{:?}",
        provider.message
    );
    let release = provider.value.as_ref().unwrap().first().unwrap();
    assert!(release.version_code.is_some());
}
