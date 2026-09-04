//! Upstream Android release discovery.
//!
//! This is deliberately separate from the APK content dataminer in the parent
//! module: discovery finds releases, while the content dataminer inspects an
//! APK that has already been obtained and verified.

#[path = "discovery/api.rs"]
pub mod api;
#[path = "discovery/http.rs"]
pub mod http;
#[path = "discovery/model.rs"]
pub mod model;
#[path = "discovery/provider.rs"]
pub mod provider;
#[path = "discovery/resolver.rs"]
pub mod resolver;
#[path = "discovery/service.rs"]
pub mod service;
#[path = "discovery/version.rs"]
pub mod version;

pub use service::{DataminerConfig, YandexMusicDataminer};
