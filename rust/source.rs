use crate::channel::ReleaseChannel;
use crate::signing::{ensure_signing_identity, sign_apk, verify_apk};
use crate::tools::Toolchain;
use crate::tools::sha256_file;
use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use ympatcher::discovery::http::CancellationToken;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub const PACKAGE_NAME: &str = "ru.yandex.music";
pub const RELEASES_API_URL: &str = "https://releases.pyanexy.cc";
pub const OFFICIAL_CERTIFICATE_SHA256: &str =
    "aca405ded8b25cb2e8c6da69425d2b4307d087c1276fc06ad5942731ccc51dba";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Abi {
    #[serde(rename = "arm64-v8a")]
    Arm64V8a,
    #[serde(rename = "armeabi-v7a")]
    ArmeabiV7a,
}

impl Abi {
    pub fn from_device_abi(value: &str) -> Result<Self> {
        let values = value.split(',').map(str::trim).collect::<Vec<_>>();
        if values.contains(&"arm64-v8a") {
            return Ok(Self::Arm64V8a);
        }
        if values.contains(&"armeabi-v7a") {
            return Ok(Self::ArmeabiV7a);
        }
        bail!("устройство сообщает неподдерживаемый ABI: {value}")
    }
}

impl fmt::Display for Abi {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Arm64V8a => "arm64-v8a",
            Self::ArmeabiV7a => "armeabi-v7a",
        })
    }
}

impl FromStr for Abi {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "arm64-v8a" => Ok(Self::Arm64V8a),
            "armeabi-v7a" => Ok(Self::ArmeabiV7a),
            _ => bail!("неподдерживаемый ABI {value}; выберите arm64-v8a или armeabi-v7a"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageFormat {
    MonolithicApk,
    SplitApks,
    ApkmBundle,
}

impl fmt::Display for PackageFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MonolithicApk => "monolithic-apk",
            Self::SplitApks => "split-apks",
            Self::ApkmBundle => "apkm-bundle",
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageFileRole {
    Base,
    Split,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub provider: String,
    pub package_name: String,
    pub version_name: Option<String>,
    pub version_code: Option<u64>,
    pub channel: ReleaseChannel,
    pub format: PackageFormat,
    pub expected_size: Option<u64>,
    pub expected_sha256: Option<String>,
    pub file_name: Option<String>,
    pub download_url: Option<String>,
    pub release_date: Option<String>,
    pub architecture: Vec<String>,
    pub dpi: Vec<String>,
    pub min_android: Option<String>,
    pub min_sdk: Option<u32>,
    pub split_count: Option<u32>,
    pub source_page: Option<String>,
    pub available: bool,
    pub opaque_id: String,
    pub abi: Option<Abi>,
    pub source: Option<String>,
    pub artifact_signed: Option<bool>,
    pub source_verified: Option<bool>,
    pub source_cert_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SourceTrust {
    OfficialSignedApk,
    VerifiedNormalizedArtifact {
        package_name: String,
        channel: ReleaseChannel,
        version_name: String,
        version_code: u64,
        abi: Abi,
        artifact_size: u64,
        artifact_sha256: String,
        source_cert_sha256: String,
    },
    UserSuppliedUnknown,
}

#[derive(Debug, Clone)]
pub struct DownloadedFile {
    pub path: PathBuf,
    pub role: PackageFileRole,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct DownloadedPackage {
    pub release: Release,
    pub files: Vec<DownloadedFile>,
    pub container_sha256: Option<String>,
    pub trust: SourceTrust,
}

impl DownloadedPackage {
    pub fn base_apk(&self) -> Result<&Path> {
        let mut base = self
            .files
            .iter()
            .filter(|file| file.role == PackageFileRole::Base);
        let first = base.next().context("в пакете отсутствует base APK")?;
        if base.next().is_some() {
            bail!("в пакете найдено несколько base APK");
        }
        Ok(&first.path)
    }
}

pub trait ReleaseProvider {
    fn id(&self) -> &'static str;
    fn list_versions(&self, channel: ReleaseChannel) -> Result<Vec<Release>>;

    fn latest(&self, channel: ReleaseChannel) -> Result<Release> {
        self.list_versions(channel)?
            .into_iter()
            .max_by_key(|release| release.version_code.unwrap_or_default())
            .with_context(|| format!("provider {} не вернул версии для {channel}", self.id()))
    }

    fn download(&self, release: &Release, destination: &Path) -> Result<DownloadedPackage>;
}

#[derive(Debug, Deserialize)]
struct ReleasesApiResponse {
    package: String,
    channel: ReleaseChannel,
    version_name: String,
    version_code: u64,
    source: String,
    artifacts: Vec<ReleasesApiArtifact>,
}

#[derive(Debug, Deserialize)]
struct ReleasesApiArtifact {
    abi: Abi,
    format: String,
    kind: String,
    size: u64,
    sha256: String,
    signed: bool,
    source_verified: bool,
    source_cert_sha256: String,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheMetadata {
    api: String,
    package_name: String,
    channel: ReleaseChannel,
    version_name: String,
    version_code: u64,
    abi: Abi,
    size: u64,
    sha256: String,
    source: String,
    source_verified: bool,
    source_cert_sha256: String,
    artifact_url: String,
}

#[derive(Clone)]
pub struct ReleasesApiProvider {
    base_url: String,
    client: Client,
    cancellation: CancellationToken,
    max_attempts: usize,
}

impl fmt::Debug for ReleasesApiProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReleasesApiProvider")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl ReleasesApiProvider {
    pub fn new(cancellation: CancellationToken) -> Result<Self> {
        Self::with_endpoint(RELEASES_API_URL, cancellation)
    }

    fn with_endpoint(base_url: impl Into<String>, cancellation: CancellationToken) -> Result<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_owned();
        let parsed = reqwest::Url::parse(&base_url).context("невалидный releases API URL")?;
        if parsed.scheme() != "https" && parsed.host_str() != Some("127.0.0.1") {
            bail!("releases API должен использовать HTTPS");
        }
        Ok(Self {
            base_url,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(45))
                .user_agent(concat!("ympatcher/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("не удалось создать HTTP-клиент")?,
            cancellation,
            max_attempts: 3,
        })
    }

    pub fn latest_for_abi(&self, channel: ReleaseChannel, abi: Abi) -> Result<Release> {
        if !matches!(channel, ReleaseChannel::Stable | ReleaseChannel::Beta) {
            bail!("releases API поддерживает только stable и beta");
        }
        let endpoint = format!("{}/v1/releases/{channel}/latest", self.base_url);
        let response = self.request_json(&endpoint)?;
        self.validate_release(response, channel, abi)
    }

    fn request_json(&self, endpoint: &str) -> Result<ReleasesApiResponse> {
        let mut last_error = None;
        for attempt in 1..=self.max_attempts {
            self.check_cancelled()?;
            match self.client.get(endpoint).send() {
                Ok(response) if response.status().is_success() => {
                    return response.json().context("API metadata: невалидный JSON");
                }
                Ok(response) => {
                    let status = response.status();
                    let retryable =
                        status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
                    let message = format!("API metadata: HTTP {}", status.as_u16());
                    if !retryable || attempt == self.max_attempts {
                        bail!(message);
                    }
                    last_error = Some(anyhow::anyhow!(message));
                }
                Err(error) => {
                    let retryable = error.is_connect() || error.is_timeout() || error.is_request();
                    if !retryable || attempt == self.max_attempts {
                        return Err(error).context("API metadata: запрос не выполнен");
                    }
                    last_error = Some(error.into());
                }
            }
            self.backoff(attempt)?;
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("API metadata недоступен")))
    }

    fn validate_release(
        &self,
        response: ReleasesApiResponse,
        requested_channel: ReleaseChannel,
        abi: Abi,
    ) -> Result<Release> {
        if response.package != PACKAGE_NAME {
            bail!(
                "source provenance: ожидался package {PACKAGE_NAME}, получен {}",
                response.package
            );
        }
        if response.channel != requested_channel {
            bail!(
                "source provenance: API вернул канал {}, ожидался {requested_channel}",
                response.channel
            );
        }
        if response.source != "google-play" {
            bail!(
                "source provenance: ожидался google-play, получен {}",
                response.source
            );
        }
        let artifact = response
            .artifacts
            .into_iter()
            .find(|artifact| artifact.abi == abi)
            .with_context(|| {
                format!("artifact ABI {abi} отсутствует для канала {requested_channel}")
            })?;
        if artifact.format != "apk" || artifact.kind != "standalone" {
            bail!("source provenance: artifact должен быть standalone APK");
        }
        if !artifact.source_verified {
            bail!("source provenance: API не подтвердил официальный источник");
        }
        if !artifact
            .source_cert_sha256
            .eq_ignore_ascii_case(OFFICIAL_CERTIFICATE_SHA256)
        {
            bail!("source provenance: неверный сертификат исходного Yandex APK");
        }
        if artifact.sha256.len() != 64
            || !artifact
                .sha256
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            bail!("source provenance: невалидный SHA-256 artifact");
        }
        let url = self.validate_artifact_url(&artifact.url)?;
        Ok(Release {
            provider: "releases-api".to_owned(),
            package_name: response.package,
            version_name: Some(response.version_name),
            version_code: Some(response.version_code),
            channel: response.channel,
            format: PackageFormat::MonolithicApk,
            expected_size: Some(artifact.size),
            expected_sha256: Some(artifact.sha256.to_ascii_lowercase()),
            file_name: Some("source.apk".to_owned()),
            download_url: Some(url),
            release_date: None,
            architecture: vec![abi.to_string()],
            dpi: Vec::new(),
            min_android: None,
            min_sdk: None,
            split_count: None,
            source_page: Some(self.base_url.clone()),
            available: true,
            opaque_id: response.version_code.to_string(),
            abi: Some(abi),
            source: Some(response.source),
            artifact_signed: Some(artifact.signed),
            source_verified: Some(artifact.source_verified),
            source_cert_sha256: Some(artifact.source_cert_sha256.to_ascii_lowercase()),
        })
    }

    fn validate_artifact_url(&self, value: &str) -> Result<String> {
        let base = reqwest::Url::parse(&format!("{}/", self.base_url))?;
        let url = base
            .join(value)
            .context("source provenance: невалидный artifact URL")?;
        if url.scheme() != "https" && url.host_str() != Some("127.0.0.1") {
            bail!("artifact URL должен использовать HTTPS");
        }
        if url.host_str() != base.host_str()
            || url.port_or_known_default() != base.port_or_known_default()
        {
            bail!("artifact URL указывает за пределы releases API");
        }
        Ok(url.into())
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.cancellation.is_cancelled() {
            bail!("операция отменена пользователем");
        }
        Ok(())
    }

    fn backoff(&self, attempt: usize) -> Result<()> {
        for _ in 0..attempt * 5 {
            self.check_cancelled()?;
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    fn download_once(&self, url: &str, output: &Path, size: u64, sha256: &str) -> Result<()> {
        let response = self
            .client
            .get(url)
            .send()
            .context("download: запрос не выполнен")?;
        if !response.status().is_success() {
            bail!("download: HTTP {}", response.status().as_u16());
        }
        stream_download(response, output, size, sha256, &self.cancellation)
    }
}

impl ReleaseProvider for ReleasesApiProvider {
    fn id(&self) -> &'static str {
        "releases-api"
    }

    fn list_versions(&self, _channel: ReleaseChannel) -> Result<Vec<Release>> {
        bail!("для releases API необходимо явно выбрать ABI")
    }

    fn download(&self, release: &Release, cache_root: &Path) -> Result<DownloadedPackage> {
        if release.provider != self.id() {
            bail!("release принадлежит другому provider");
        }
        let version_code = release
            .version_code
            .context("API release не содержит versionCode")?;
        let version_name = release
            .version_name
            .clone()
            .context("API release не содержит versionName")?;
        let abi = release.abi.context("API release не содержит ABI")?;
        let expected_size = release
            .expected_size
            .context("API release не содержит size")?;
        let expected_sha = release
            .expected_sha256
            .as_deref()
            .context("API release не содержит sha256")?;
        let url = release
            .download_url
            .as_deref()
            .context("API release не содержит artifact URL")?;
        let directory = cache_root
            .join("releases")
            .join(release.channel.to_string())
            .join(version_code.to_string())
            .join(abi.to_string());
        fs::create_dir_all(&directory)?;
        let output = directory.join("source.apk");
        let metadata_path = directory.join("metadata.json");
        if output.is_file() && verify_download(&output, expected_size, expected_sha).is_ok() {
            eprintln!("  кэш: размер и SHA-256 подтверждены");
            return downloaded_api_package(release, output);
        }
        let temporary = directory.join("source.apk.part");
        let mut last_error = None;
        for attempt in 1..=self.max_attempts {
            self.check_cancelled()?;
            match self.download_once(url, &temporary, expected_size, expected_sha) {
                Ok(()) => {
                    if output.exists() {
                        fs::remove_file(&output)?;
                    }
                    fs::rename(&temporary, &output)?;
                    let metadata = CacheMetadata {
                        api: self.base_url.clone(),
                        package_name: release.package_name.clone(),
                        channel: release.channel,
                        version_name: version_name.clone(),
                        version_code,
                        abi,
                        size: expected_size,
                        sha256: expected_sha.to_owned(),
                        source: release.source.clone().unwrap_or_default(),
                        source_verified: release.source_verified.unwrap_or(false),
                        source_cert_sha256: release.source_cert_sha256.clone().unwrap_or_default(),
                        artifact_url: url.to_owned(),
                    };
                    let metadata_part = directory.join("metadata.json.part");
                    fs::write(&metadata_part, serde_json::to_vec_pretty(&metadata)?)?;
                    if metadata_path.exists() {
                        fs::remove_file(&metadata_path)?;
                    }
                    fs::rename(metadata_part, metadata_path)?;
                    return downloaded_api_package(release, output);
                }
                Err(error) => {
                    let message = error.to_string();
                    let retryable = message.contains("HTTP 429")
                        || message.contains("HTTP 5")
                        || message.contains("timeout")
                        || message.contains("connect")
                        || message.contains("запрос не выполнен");
                    let _ = fs::remove_file(&temporary);
                    if !retryable || attempt == self.max_attempts {
                        return Err(error).context("download: artifact не получен");
                    }
                    last_error = Some(error);
                    self.backoff(attempt)?;
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("download: artifact недоступен")))
    }
}

fn downloaded_api_package(release: &Release, output: PathBuf) -> Result<DownloadedPackage> {
    let trust = SourceTrust::VerifiedNormalizedArtifact {
        package_name: release.package_name.clone(),
        channel: release.channel,
        version_name: release.version_name.clone().context("нет versionName")?,
        version_code: release.version_code.context("нет versionCode")?,
        abi: release.abi.context("нет ABI")?,
        artifact_size: release.expected_size.context("нет size")?,
        artifact_sha256: release.expected_sha256.clone().context("нет sha256")?,
        source_cert_sha256: release
            .source_cert_sha256
            .clone()
            .context("нет source cert")?,
    };
    Ok(DownloadedPackage {
        release: release.clone(),
        files: vec![describe_file(output.clone(), PackageFileRole::Base)?],
        container_sha256: release.expected_sha256.clone(),
        trust,
    })
}

fn stream_download(
    mut response: Response,
    output: &Path,
    expected_size: u64,
    expected_sha: &str,
    cancellation: &CancellationToken,
) -> Result<()> {
    let mut writer = File::create(output)?;
    let mut buffer = [0_u8; 128 * 1024];
    let mut received = 0_u64;
    loop {
        if cancellation.is_cancelled() {
            bail!("загрузка отменена пользователем");
        }
        let count = response.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        writer.write_all(&buffer[..count])?;
        received += count as u64;
        if received > expected_size {
            bail!("размер загрузки превышает ожидаемые {expected_size} bytes");
        }
    }
    writer.flush()?;
    drop(writer);
    verify_download(output, expected_size, expected_sha)
}

fn verify_download(path: &Path, expected_size: u64, expected_sha: &str) -> Result<()> {
    let actual_size = path.metadata()?.len();
    if actual_size != expected_size {
        bail!("размер загрузки не совпал: ожидалось {expected_size}, получено {actual_size}");
    }
    let actual_sha = sha256_file(path)?;
    if !actual_sha.eq_ignore_ascii_case(expected_sha) {
        bail!("SHA-256 загрузки не совпал: ожидался {expected_sha}, получен {actual_sha}");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct UserImportProvider {
    input: PathBuf,
    channel: ReleaseChannel,
}

impl UserImportProvider {
    pub fn new(input: PathBuf, channel: ReleaseChannel) -> Result<Self> {
        if !input.is_file() {
            bail!("входной APK/APKS не найден: {}", input.display());
        }
        Ok(Self { input, channel })
    }

    fn format(&self) -> Result<PackageFormat> {
        match self
            .input
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("apk") => Ok(PackageFormat::MonolithicApk),
            Some("apks") => Ok(PackageFormat::SplitApks),
            Some("apkm") => Ok(PackageFormat::ApkmBundle),
            _ => bail!("поддерживаются только .apk, .apks и .apkm"),
        }
    }
}

impl ReleaseProvider for UserImportProvider {
    fn id(&self) -> &'static str {
        "user-import"
    }

    fn list_versions(&self, channel: ReleaseChannel) -> Result<Vec<Release>> {
        if channel != self.channel {
            return Ok(Vec::new());
        }
        Ok(vec![Release {
            provider: self.id().to_owned(),
            package_name: PACKAGE_NAME.to_owned(),
            version_name: None,
            version_code: None,
            channel,
            format: self.format()?,
            expected_size: Some(self.input.metadata()?.len()),
            expected_sha256: None,
            file_name: self
                .input
                .file_name()
                .map(|value| value.to_string_lossy().into_owned()),
            download_url: None,
            release_date: None,
            architecture: Vec::new(),
            dpi: Vec::new(),
            min_android: None,
            min_sdk: None,
            split_count: None,
            source_page: None,
            available: true,
            opaque_id: self.input.display().to_string(),
            abi: None,
            source: None,
            artifact_signed: None,
            source_verified: None,
            source_cert_sha256: None,
        }])
    }

    fn download(&self, release: &Release, destination: &Path) -> Result<DownloadedPackage> {
        if release.provider != self.id() || release.channel != self.channel {
            bail!("release не принадлежит user-import provider");
        }
        fs::create_dir_all(destination)?;
        match release.format {
            PackageFormat::MonolithicApk => {
                let file_name = self
                    .input
                    .file_name()
                    .context("у входного APK отсутствует имя")?;
                let output = destination.join(file_name);
                atomic_copy(&self.input, &output)?;
                let package = DownloadedPackage {
                    release: release.clone(),
                    files: vec![describe_file(output, PackageFileRole::Base)?],
                    container_sha256: None,
                    trust: SourceTrust::OfficialSignedApk,
                };
                if package.files[0].size != release.expected_size.unwrap_or(package.files[0].size) {
                    bail!("размер импортированного APK изменился во время копирования");
                }
                Ok(package)
            }
            PackageFormat::SplitApks | PackageFormat::ApkmBundle => {
                import_split_archive(&self.input, destination, release.clone())
            }
        }
    }
}

fn atomic_copy(source: &Path, destination: &Path) -> Result<()> {
    let temporary = destination.with_extension("part");
    let mut input = File::open(source)?;
    let mut output = File::create(&temporary)?;
    io::copy(&mut input, &mut output)?;
    output.flush()?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary, destination)?;
    Ok(())
}

fn describe_file(path: PathBuf, role: PackageFileRole) -> Result<DownloadedFile> {
    Ok(DownloadedFile {
        size: path.metadata()?.len(),
        sha256: sha256_file(&path)?,
        path,
        role,
    })
}

pub fn import_split_archive(
    input: &Path,
    destination: &Path,
    release: Release,
) -> Result<DownloadedPackage> {
    fs::create_dir_all(destination)?;
    let mut archive = ZipArchive::new(File::open(input)?).context("APKM/APKS повреждён")?;
    crate::apk::validate_archive(&mut archive)?;
    if archive.len() > 512 {
        bail!("слишком много файлов в APKS/APKM (лимит 512)");
    }
    let mut files = Vec::new();
    let mut file_names = BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() || !entry.name().to_ascii_lowercase().ends_with(".apk") {
            continue;
        }
        let enclosed = entry
            .enclosed_name()
            .context("APKM/APKS содержит небезопасный путь")?;
        let file_name = enclosed
            .file_name()
            .context("APK entry не содержит имени")?;
        if !file_names.insert(file_name.to_string_lossy().to_ascii_lowercase()) {
            bail!(
                "APKM/APKS содержит повторяющееся имя APK: {}",
                file_name.to_string_lossy()
            );
        }
        let output = destination.join(file_name);
        let temporary = output.with_extension("apk.part");
        let mut writer = File::create(&temporary)?;
        io::copy(&mut entry, &mut writer)?;
        writer.flush()?;
        if output.exists() {
            fs::remove_file(&output)?;
        }
        fs::rename(temporary, &output)?;
        let lower = file_name.to_string_lossy().to_ascii_lowercase();
        let role = if matches!(
            lower.as_str(),
            "base.apk" | "base-master.apk" | "universal.apk"
        ) {
            PackageFileRole::Base
        } else {
            PackageFileRole::Split
        };
        files.push(describe_file(output, role)?);
    }
    if files.is_empty() {
        bail!("APKM/APKS не содержит APK files");
    }
    let package = DownloadedPackage {
        release,
        files,
        container_sha256: Some(sha256_file(input)?),
        trust: SourceTrust::OfficialSignedApk,
    };
    package.base_apk()?;
    Ok(package)
}

pub fn assemble_signed_bundle(
    package: &DownloadedPackage,
    patched_base: &Path,
    output: &Path,
    state_dir: &Path,
    tools: &Toolchain,
) -> Result<(String, usize)> {
    if package.release.format == PackageFormat::MonolithicApk {
        bail!("assemble_signed_bundle вызван для monolithic APK");
    }
    let temporary = tempfile::Builder::new()
        .prefix("ympatch-bundle-")
        .tempdir()?;
    let staging = temporary.path();
    let base = staging.join("base.apk");
    fs::copy(patched_base, &base)?;
    let (identity, _) = ensure_signing_identity(state_dir, tools)?;
    let expected_certificate = verify_apk(&base, tools)?;
    let mut signed = vec![base];
    for file in package
        .files
        .iter()
        .filter(|file| file.role == PackageFileRole::Split)
    {
        let name = file
            .path
            .file_name()
            .context("split APK не содержит имени")?;
        let destination = staging.join(name);
        fs::copy(&file.path, &destination)?;
        sign_apk(&destination, &identity, tools)
            .with_context(|| format!("не удалось подписать split {}", file.path.display()))?;
        let certificate = verify_apk(&destination, tools)?;
        if certificate != expected_certificate {
            bail!(
                "split {} подписан другим сертификатом",
                name.to_string_lossy()
            );
        }
        signed.push(destination);
    }
    if signed.len() < 2 {
        bail!("bundle не содержит split APK");
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = staging.join("output.apks");
    let writer = File::create(&part)?;
    let mut archive = ZipWriter::new(writer);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o644);
    for apk in &signed {
        let name = apk
            .file_name()
            .context("подписанный APK не содержит имени")?
            .to_string_lossy();
        archive.start_file(name.as_ref(), options)?;
        let mut reader = File::open(apk)?;
        io::copy(&mut reader, &mut archive)?;
    }
    archive.start_file("ympatcher-manifest.json", options)?;
    archive.write_all(&serde_json::to_vec_pretty(&serde_json::json!({
        "schemaVersion": 1,
        "packageName": package.release.package_name,
        "versionName": package.release.version_name,
        "versionCode": package.release.version_code,
        "channel": package.release.channel,
        "patcherVersion": env!("CARGO_PKG_VERSION"),
        "certificateSha256": expected_certificate,
        "apkCount": signed.len(),
        "install": "adb install-multiple -r base.apk split_*.apk"
    }))?)?;
    archive.finish()?.sync_all()?;
    crate::apk::publish(&part, output)?;
    Ok((expected_certificate, signed.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    const SHA_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn release_json(channel: &str, abi: &str) -> String {
        format!(
            r#"{{"package":"{PACKAGE_NAME}","channel":"{channel}","version_name":"2026.09.1 #163gpr","version_code":24026461,"source":"google-play","artifacts":[{{"abi":"{abi}","format":"apk","kind":"standalone","size":3,"sha256":"{SHA_ABC}","signed":false,"source_verified":true,"source_cert_sha256":"{OFFICIAL_CERTIFICATE_SHA256}","url":"/artifact"}}]}}"#
        )
    }

    fn serve(responses: Vec<(u16, String)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for (stream, (status, body)) in listener.incoming().zip(responses) {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        format!("http://{address}")
    }

    fn serve_delayed(body: String, delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                thread::sleep(delay);
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        format!("http://{address}")
    }

    #[test]
    fn selects_stable_beta_and_both_abis() {
        for (channel, abi) in [
            (ReleaseChannel::Stable, Abi::Arm64V8a),
            (ReleaseChannel::Beta, Abi::ArmeabiV7a),
        ] {
            let endpoint = serve(vec![(
                200,
                release_json(&channel.to_string(), &abi.to_string()),
            )]);
            let provider =
                ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
            let release = provider.latest_for_abi(channel, abi).unwrap();
            assert_eq!(release.abi, Some(abi));
            assert_eq!(release.format, PackageFormat::MonolithicApk);
        }
    }

    #[test]
    fn rejects_missing_artifact_and_bad_provenance() {
        let valid = release_json("stable", "arm64-v8a");
        let cases = [
            release_json("stable", "armeabi-v7a"),
            valid.replace(PACKAGE_NAME, "evil.package"),
            release_json("beta", "arm64-v8a"),
            valid.replace("\"source_verified\":true", "\"source_verified\":false"),
            valid.replace(OFFICIAL_CERTIFICATE_SHA256, &"0".repeat(64)),
        ];
        for body in cases {
            let endpoint = serve(vec![(200, body)]);
            let provider =
                ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
            assert!(
                provider
                    .latest_for_abi(ReleaseChannel::Stable, Abi::Arm64V8a)
                    .is_err()
            );
        }
    }

    #[test]
    fn handles_http_json_and_retry() {
        for (status, body) in [(404, "{}"), (200, "not-json")] {
            let endpoint = serve(vec![(status, body.to_owned())]);
            let provider =
                ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
            assert!(
                provider
                    .latest_for_abi(ReleaseChannel::Stable, Abi::Arm64V8a)
                    .is_err()
            );
        }
        for status in [429, 500] {
            let endpoint = serve(vec![
                (status, "{}".into()),
                (status, "{}".into()),
                (200, release_json("stable", "arm64-v8a")),
            ]);
            let provider =
                ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
            assert!(
                provider
                    .latest_for_abi(ReleaseChannel::Stable, Abi::Arm64V8a)
                    .is_ok()
            );
        }
    }

    #[test]
    fn verifies_size_and_sha256() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("source.apk");
        fs::write(&path, b"abc").unwrap();
        verify_download(&path, 3, SHA_ABC).unwrap();
        assert!(verify_download(&path, 4, SHA_ABC).is_err());
        assert!(verify_download(&path, 3, &"0".repeat(64)).is_err());
    }

    #[test]
    fn download_rejects_wrong_size_and_sha() {
        for metadata in [
            release_json("stable", "arm64-v8a").replace("\"size\":3", "\"size\":4"),
            release_json("stable", "arm64-v8a").replace(SHA_ABC, &"0".repeat(64)),
        ] {
            let endpoint = serve(vec![(200, metadata), (200, "abc".to_owned())]);
            let provider =
                ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
            let release = provider
                .latest_for_abi(ReleaseChannel::Stable, Abi::Arm64V8a)
                .unwrap();
            let cache = tempfile::tempdir().unwrap();
            assert!(provider.download(&release, cache.path()).is_err());
        }
    }

    #[test]
    fn valid_cache_hit_does_not_redownload() {
        let endpoint = serve(vec![(200, release_json("stable", "arm64-v8a"))]);
        let provider =
            ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        let release = provider
            .latest_for_abi(ReleaseChannel::Stable, Abi::Arm64V8a)
            .unwrap();
        let cache = tempfile::tempdir().unwrap();
        let directory = cache.path().join("releases/stable/24026461/arm64-v8a");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("source.apk"), b"abc").unwrap();
        let package = provider.download(&release, cache.path()).unwrap();
        assert_eq!(package.files[0].sha256, SHA_ABC);
    }

    #[test]
    fn metadata_timeout_is_reported() {
        let endpoint = serve_delayed(
            release_json("stable", "arm64-v8a"),
            Duration::from_millis(250),
        );
        let mut provider =
            ReleasesApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        provider.client = Client::builder()
            .timeout(Duration::from_millis(50))
            .build()
            .unwrap();
        provider.max_attempts = 1;
        assert!(
            provider
                .latest_for_abi(ReleaseChannel::Stable, Abi::Arm64V8a)
                .unwrap_err()
                .to_string()
                .contains("API metadata")
        );
    }

    #[test]
    fn live_releases_api_is_opt_in() {
        if std::env::var_os("YMPATCHER_LIVE_TESTS").is_none() {
            return;
        }
        let provider = ReleasesApiProvider::new(CancellationToken::default()).unwrap();
        for channel in [ReleaseChannel::Stable, ReleaseChannel::Beta] {
            for abi in [Abi::Arm64V8a, Abi::ArmeabiV7a] {
                let release = provider.latest_for_abi(channel, abi).unwrap();
                assert_eq!(release.package_name, PACKAGE_NAME);
                assert_eq!(release.channel, channel);
                assert_eq!(release.abi, Some(abi));
                assert_eq!(release.source.as_deref(), Some("google-play"));
                assert_eq!(release.source_verified, Some(true));
            }
        }
    }
    fn test_release() -> Release {
        Release {
            provider: "test".to_owned(),
            package_name: PACKAGE_NAME.to_owned(),
            version_name: Some("1".to_owned()),
            version_code: Some(1),
            channel: ReleaseChannel::Stable,
            format: PackageFormat::ApkmBundle,
            expected_size: None,
            expected_sha256: None,
            file_name: None,
            download_url: None,
            release_date: None,
            architecture: Vec::new(),
            dpi: Vec::new(),
            min_android: None,
            min_sdk: None,
            split_count: None,
            source_page: None,
            available: true,
            opaque_id: "test".to_owned(),
            abi: None,
            source: None,
            artifact_signed: None,
            source_verified: None,
            source_cert_sha256: None,
        }
    }

    #[test]
    fn safely_extracts_apkm_and_requires_base() {
        let directory = tempfile::tempdir().unwrap();
        let bundle = directory.path().join("ok.apkm");
        let mut zip = ZipWriter::new(File::create(&bundle).unwrap());
        zip.start_file("base.apk", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"base").unwrap();
        zip.start_file("split_config.en.apk", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"split").unwrap();
        zip.finish().unwrap();
        let package =
            import_split_archive(&bundle, &directory.path().join("out"), test_release()).unwrap();
        assert_eq!(package.files.len(), 2);

        let missing = directory.path().join("missing.apkm");
        let mut zip = ZipWriter::new(File::create(&missing).unwrap());
        zip.start_file("split_config.en.apk", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"split").unwrap();
        zip.finish().unwrap();
        assert!(
            import_split_archive(
                &missing,
                &directory.path().join("missing-out"),
                test_release()
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_zip_slip_entries() {
        let directory = tempfile::tempdir().unwrap();
        let bundle = directory.path().join("unsafe.apkm");
        let mut zip = ZipWriter::new(File::create(&bundle).unwrap());
        zip.start_file("../base.apk", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"base").unwrap();
        zip.finish().unwrap();
        let error = import_split_archive(&bundle, &directory.path().join("out"), test_release())
            .unwrap_err()
            .to_string();
        assert!(error.contains("небезопасный"));
    }

    #[test]
    fn imports_nested_bundletool_layout_and_rejects_colliding_basenames() {
        let directory = tempfile::tempdir().unwrap();
        for (name, entries, valid) in [
            (
                "nested",
                vec!["splits/base-master.apk", "splits/base-arm64_v8a.apk"],
                true,
            ),
            ("collision", vec!["a/base.apk", "b/BASE.apk"], false),
            ("multiple-base", vec!["base.apk", "universal.apk"], false),
            ("windows", vec!["base.apk", "split:stream.apk"], false),
        ] {
            let bundle = directory.path().join(format!("{name}.apks"));
            let mut zip = ZipWriter::new(File::create(&bundle).unwrap());
            for entry in entries {
                zip.start_file(entry, SimpleFileOptions::default()).unwrap();
                zip.write_all(b"apk").unwrap();
            }
            zip.finish().unwrap();
            let result =
                import_split_archive(&bundle, &directory.path().join(name), test_release());
            assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        }
    }
}
