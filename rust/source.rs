use crate::channel::ReleaseChannel;
use crate::signing::{ensure_signing_identity, sign_apk, verify_apk};
use crate::tools::Toolchain;
use crate::tools::sha256_file;
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use ympatcher::discovery::http::CancellationToken;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub const PACKAGE_NAME: &str = "ru.yandex.music";
pub const LATEST_API_URL: &str = "https://ympatcher.pyanexy.cc/v1/apks/latest";

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
    pub container_path: Option<PathBuf>,
    pub container_sha256: Option<String>,
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
#[serde(rename_all = "camelCase")]
struct LatestApiEnvelope {
    message: String,
    data: LatestApiData,
    #[serde(default)]
    tech: LatestApiTech,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestApiData {
    result_apks: Vec<LatestApiApk>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestApiTech {
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestApiApk {
    id: String,
    channel: ReleaseChannel,
    #[serde(default)]
    labels: Vec<String>,
    version_name: String,
    version_code: u64,
    release_date: Option<String>,
    file_name: String,
    file_type: String,
    size_bytes: u64,
    sha256: String,
    #[serde(deserialize_with = "string_or_strings")]
    architecture: Vec<String>,
    #[serde(deserialize_with = "string_or_strings")]
    dpi: Vec<String>,
    min_android: Option<String>,
    min_sdk: Option<u32>,
    split_count: Option<u32>,
    is_bundle: bool,
    source_page: Option<String>,
    download_url: String,
    available: bool,
}

fn string_or_strings<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Value::deserialize(deserializer)? {
        Value::One(value) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
        Value::Many(values) => values,
    })
}

#[derive(Clone)]
pub struct YmpatcherApiProvider {
    endpoint: String,
    client: Client,
    cancellation: CancellationToken,
    max_attempts: usize,
}

impl fmt::Debug for YmpatcherApiProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YmpatcherApiProvider")
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}

impl YmpatcherApiProvider {
    pub fn new(cancellation: CancellationToken) -> Result<Self> {
        Self::with_endpoint(LATEST_API_URL, cancellation)
    }

    fn with_endpoint(endpoint: impl Into<String>, cancellation: CancellationToken) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.into(),
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

    fn request_releases(&self) -> Result<Vec<LatestApiApk>> {
        let mut last_error = None;
        for attempt in 1..=self.max_attempts {
            if self.cancellation.is_cancelled() {
                bail!("операция отменена пользователем");
            }
            let response = self
                .client
                .post(&self.endpoint)
                .header(CONTENT_TYPE, "application/json")
                .json(&serde_json::json!({"channels": ["stable", "beta"]}))
                .send();
            match response {
                Ok(response) => {
                    let status = response.status();
                    let bytes = response
                        .bytes()
                        .context("API: не удалось прочитать ответ")?;
                    let parsed = serde_json::from_slice::<LatestApiEnvelope>(&bytes);
                    if !status.is_success() {
                        let details = parsed
                            .ok()
                            .map(|body| api_error(&body.message, body.tech.request_id.as_deref()))
                            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
                        if (status.is_server_error() || status.as_u16() == 429)
                            && attempt < self.max_attempts
                        {
                            last_error = Some(anyhow::anyhow!(details));
                            self.backoff(attempt)?;
                            continue;
                        }
                        bail!("API latest: {details}");
                    }
                    let body = parsed.context("API latest вернул невалидный JSON")?;
                    return Ok(body.data.result_apks);
                }
                Err(error) => {
                    let retryable = error.is_connect() || error.is_timeout() || error.is_request();
                    last_error = Some(error.into());
                    if !retryable || attempt == self.max_attempts {
                        break;
                    }
                    self.backoff(attempt)?;
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("API latest недоступен")))
    }

    fn backoff(&self, attempt: usize) -> Result<()> {
        for _ in 0..attempt * 5 {
            if self.cancellation.is_cancelled() {
                bail!("операция отменена пользователем");
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }
}

fn api_error(message: &str, request_id: Option<&str>) -> String {
    match request_id {
        Some(request_id) if !request_id.is_empty() => {
            format!("{message} (requestId: {request_id})")
        }
        _ => message.to_owned(),
    }
}

impl ReleaseProvider for YmpatcherApiProvider {
    fn id(&self) -> &'static str {
        "ympatcher-api"
    }

    fn list_versions(&self, channel: ReleaseChannel) -> Result<Vec<Release>> {
        let expected_label = match channel {
            ReleaseChannel::Stable => "latest-stable",
            ReleaseChannel::Beta => "latest-beta",
            _ => bail!("latest API поддерживает только stable и beta"),
        };
        let releases = self
            .request_releases()?
            .into_iter()
            .filter(|apk| apk.channel == channel)
            .filter(|apk| apk.labels.iter().any(|label| label == expected_label))
            .filter(|apk| apk.available)
            .map(|apk| Release {
                provider: self.id().to_owned(),
                package_name: PACKAGE_NAME.to_owned(),
                version_name: Some(apk.version_name),
                version_code: Some(apk.version_code),
                channel: apk.channel,
                format: if apk.is_bundle || apk.file_type.eq_ignore_ascii_case("apkm") {
                    PackageFormat::ApkmBundle
                } else {
                    PackageFormat::MonolithicApk
                },
                expected_size: Some(apk.size_bytes),
                expected_sha256: Some(apk.sha256.to_ascii_lowercase()),
                file_name: Some(apk.file_name),
                download_url: Some(apk.download_url),
                release_date: apk.release_date,
                architecture: apk.architecture,
                dpi: apk.dpi,
                min_android: apk.min_android,
                min_sdk: apk.min_sdk,
                split_count: apk.split_count,
                source_page: apk.source_page,
                available: apk.available,
                opaque_id: apk.id,
            })
            .collect::<Vec<_>>();
        if releases.is_empty() {
            bail!("API latest не вернул доступный {expected_label} release");
        }
        Ok(releases)
    }

    fn download(&self, release: &Release, destination: &Path) -> Result<DownloadedPackage> {
        if release.provider != self.id() || !release.available {
            bail!("release недоступен или принадлежит другому provider");
        }
        let url = release
            .download_url
            .as_deref()
            .context("API release не содержит downloadUrl")?;
        let expected_size = release
            .expected_size
            .context("API release не содержит sizeBytes")?;
        let expected_sha = release
            .expected_sha256
            .as_deref()
            .context("API release не содержит sha256")?;
        fs::create_dir_all(destination)?;
        let file_name = release
            .file_name
            .as_deref()
            .context("API release не содержит fileName")?;
        crate::apk::validate_zip_path(file_name)?;
        if file_name.contains('/') {
            bail!("API fileName должен быть простым именем файла");
        }
        let output = destination.join(file_name);
        if output.is_file() && verify_download(&output, expected_size, expected_sha).is_ok() {
            eprintln!("  кэш: размер и SHA-256 подтверждены");
            return normalize_downloaded_release(&output, destination, release, expected_sha);
        }
        let temporary = output.with_extension("download.part");
        let mut last_error = None;
        for attempt in 1..=self.max_attempts {
            let result = download_once(
                &self.client,
                url,
                &temporary,
                expected_size,
                expected_sha,
                &self.cancellation,
            );
            match result {
                Ok(()) => {
                    if output.exists() {
                        fs::remove_file(&output)?;
                    }
                    fs::rename(&temporary, &output)?;
                    return normalize_downloaded_release(
                        &output,
                        destination,
                        release,
                        expected_sha,
                    );
                }
                Err(error) => {
                    let _ = fs::remove_file(&temporary);
                    last_error = Some(error);
                    if attempt < self.max_attempts {
                        self.backoff(attempt)?;
                    }
                }
            }
        }
        Err(last_error.context("скачивание latest release не удалось")?)
    }
}

fn normalize_downloaded_release(
    output: &Path,
    destination: &Path,
    release: &Release,
    expected_sha: &str,
) -> Result<DownloadedPackage> {
    if release.opaque_id.is_empty()
        || !release
            .opaque_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '_' | '-'))
    {
        bail!("API release id содержит небезопасные символы");
    }
    let extracted = destination.join(format!("{}-splits", release.opaque_id));
    if extracted.exists() {
        fs::remove_dir_all(&extracted)?;
    }
    fs::create_dir_all(&extracted)?;
    let mut package = match release.format {
        PackageFormat::ApkmBundle | PackageFormat::SplitApks => {
            import_split_archive(output, &extracted, release.clone())?
        }
        PackageFormat::MonolithicApk => DownloadedPackage {
            release: release.clone(),
            files: vec![describe_file(output.to_owned(), PackageFileRole::Base)?],
            container_path: Some(output.to_owned()),
            container_sha256: Some(expected_sha.to_owned()),
        },
    };
    package.container_path = Some(output.to_owned());
    package.container_sha256 = Some(expected_sha.to_owned());
    Ok(package)
}

fn download_once(
    client: &Client,
    url: &str,
    output: &Path,
    expected_size: u64,
    expected_sha: &str,
    cancellation: &CancellationToken,
) -> Result<()> {
    let mut response = client.get(url).send().context("ошибка скачивания")?;
    if !response.status().is_success() {
        bail!("downloadUrl вернул HTTP {}", response.status().as_u16());
    }
    let mut writer = File::create(output)?;
    let mut buffer = [0_u8; 128 * 1024];
    let mut received = 0_u64;
    let mut next_report = 5_u64;
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
            bail!("загрузка превысила заявленный размер {expected_size}");
        }
        let percent = received
            .saturating_mul(100)
            .checked_div(expected_size)
            .unwrap_or(100);
        if percent >= next_report {
            eprintln!("  загрузка: {percent}% ({received}/{expected_size} bytes)");
            next_report = (percent / 5 + 1) * 5;
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
                    container_path: None,
                    container_sha256: None,
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
        container_path: Some(input.to_owned()),
        container_sha256: Some(sha256_file(input)?),
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

    fn api_json(stable_available: bool) -> String {
        format!(
            r#"{{"message":"ok","data":{{"resultApks":[{{"id":"s","channel":"stable","labels":["latest","latest-stable"],"versionName":"2026.08.4 #162.1gpr","versionCode":24026442,"releaseDate":"2026-09-02","fileName":"stable.apkm","fileType":"apkm","sizeBytes":3,"sha256":"abc","architecture":["arm64-v8a"],"dpi":["480"],"minAndroid":"7.0","minSdk":24,"splitCount":9,"isBundle":true,"sourcePage":"https://example.test/stable","downloadUrl":"https://example.test/stable.apkm","available":{stable_available}}},{{"id":"b","channel":"beta","labels":["latest","latest-beta"],"versionName":"beta","versionCode":2,"releaseDate":null,"fileName":"beta.apkm","fileType":"apkm","sizeBytes":4,"sha256":"def","architecture":"arm64-v8a","dpi":"480","minAndroid":"7.0","minSdk":24,"splitCount":4,"isBundle":true,"sourcePage":null,"downloadUrl":"https://example.test/beta.apkm","available":true}}]}},"tech":{{"requestId":"req-ok"}}}}"#
        )
    }

    fn serve(status: u16, body: String, requests: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(requests) {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut content_length = 0_usize;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        content_length = value.trim().parse().unwrap();
                    }
                }
                let mut request_body = vec![0; content_length];
                reader.read_exact(&mut request_body).unwrap();
                assert!(String::from_utf8(request_body).unwrap().contains("stable"));
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        format!("http://{address}/v1/apks/latest")
    }

    struct FakeProvider;

    impl ReleaseProvider for FakeProvider {
        fn id(&self) -> &'static str {
            "fake"
        }

        fn list_versions(&self, channel: ReleaseChannel) -> Result<Vec<Release>> {
            Ok([1_u64, 3, 2]
                .into_iter()
                .map(|version| Release {
                    provider: self.id().to_owned(),
                    package_name: PACKAGE_NAME.to_owned(),
                    version_name: Some(version.to_string()),
                    version_code: Some(version),
                    channel,
                    format: PackageFormat::MonolithicApk,
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
                    opaque_id: version.to_string(),
                })
                .collect())
        }

        fn download(&self, _release: &Release, _destination: &Path) -> Result<DownloadedPackage> {
            unreachable!()
        }
    }

    #[test]
    fn provider_default_latest_uses_version_code() {
        let release = FakeProvider.latest(ReleaseChannel::Beta).unwrap();
        assert_eq!(release.version_code, Some(3));
        assert_eq!(release.channel, ReleaseChannel::Beta);
    }

    #[test]
    fn parses_api_and_selects_stable_and_beta_by_channel_and_label() {
        let endpoint = serve(200, api_json(true), 2);
        let provider =
            YmpatcherApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        let stable = provider.latest(ReleaseChannel::Stable).unwrap();
        let beta = provider.latest(ReleaseChannel::Beta).unwrap();
        assert_eq!(stable.version_code, Some(24_026_442));
        assert_eq!(stable.format, PackageFormat::ApkmBundle);
        assert_eq!(beta.version_name.as_deref(), Some("beta"));
    }

    #[test]
    fn rejects_invalid_json_and_http_errors_with_request_id() {
        let endpoint = serve(200, "not-json".to_owned(), 1);
        let provider =
            YmpatcherApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        assert!(
            provider
                .latest(ReleaseChannel::Stable)
                .unwrap_err()
                .to_string()
                .contains("JSON")
        );

        let body = r#"{"message":"release unavailable","data":{"resultApks":[]},"tech":{"requestId":"req-42"}}"#;
        let endpoint = serve(404, body.to_owned(), 1);
        let provider =
            YmpatcherApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        let error = provider
            .latest(ReleaseChannel::Stable)
            .unwrap_err()
            .to_string();
        assert!(error.contains("release unavailable"));
        assert!(error.contains("req-42"));
    }

    #[test]
    fn retries_server_errors_and_rejects_unavailable_release() {
        let body =
            r#"{"message":"temporary","data":{"resultApks":[]},"tech":{"requestId":"req-500"}}"#;
        let endpoint = serve(500, body.to_owned(), 3);
        let provider =
            YmpatcherApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        assert!(
            provider
                .latest(ReleaseChannel::Stable)
                .unwrap_err()
                .to_string()
                .contains("req-500")
        );

        let endpoint = serve(200, api_json(false), 1);
        let provider =
            YmpatcherApiProvider::with_endpoint(endpoint, CancellationToken::default()).unwrap();
        assert!(
            provider
                .latest(ReleaseChannel::Stable)
                .unwrap_err()
                .to_string()
                .contains("не вернул")
        );
    }

    #[test]
    fn verifies_size_and_sha256() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("download.apkm");
        fs::write(&path, b"abc").unwrap();
        let sha = sha256_file(&path).unwrap();
        verify_download(&path, 3, &sha).unwrap();
        assert!(
            verify_download(&path, 4, &sha)
                .unwrap_err()
                .to_string()
                .contains("размер")
        );
        assert!(
            verify_download(&path, 3, &"0".repeat(64))
                .unwrap_err()
                .to_string()
                .contains("SHA-256")
        );
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

    #[test]
    fn api_filename_cannot_escape_download_directory() {
        let directory = tempfile::tempdir().unwrap();
        let provider =
            YmpatcherApiProvider::with_endpoint("http://127.0.0.1:1", CancellationToken::default())
                .unwrap();
        let mut release = test_release();
        release.provider = provider.id().to_owned();
        release.download_url = Some("http://127.0.0.1:1/download".to_owned());
        release.expected_size = Some(3);
        release.expected_sha256 = Some("0".repeat(64));
        for name in [
            "../escape.apk",
            "C:/escape.apk",
            "sub/escape.apk",
            "sub\\escape.apk",
        ] {
            release.file_name = Some(name.to_owned());
            let error = provider
                .download(&release, directory.path())
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("небезопасный") || error.contains("простым"),
                "{error}"
            );
        }
    }
}
