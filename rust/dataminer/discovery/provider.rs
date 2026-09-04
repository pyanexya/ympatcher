#[path = "provider/apkmirror.rs"]
mod apkmirror;
#[path = "provider/google_play.rs"]
mod google_play;
#[path = "provider/rustore.rs"]
mod rustore;

use super::http::CancellationToken;
use super::model::{DeviceProfile, Platform, ProviderId, ProviderResult, Release};
use crate::channel::ReleaseChannel;

pub use apkmirror::ApkMirrorProvider;
pub use google_play::GooglePlayProvider;
pub use rustore::RuStoreProvider;

#[derive(Debug, Clone)]
pub struct ProviderQuery {
    pub package_name: String,
    pub channel: ReleaseChannel,
    pub platform: Platform,
    pub device: DeviceProfile,
    pub cancellation: CancellationToken,
}

pub trait ReleaseProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn releases(&self, query: &ProviderQuery) -> ProviderResult<Vec<Release>>;
}
