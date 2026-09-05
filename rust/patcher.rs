use crate::channel::ReleaseChannel;
use crate::compatibility;
use crate::installer::guarded_install;
use crate::patches::{PatchSelection, PatchSummary, apply_all};
use crate::process::run;
use crate::signing::{ensure_signing_identity, identity_certificate, sign_apk, verify_apk};
use crate::source::{Abi, SourceTrust};
use crate::tools::{prepare_toolchain, sha256_file};
use crate::versioning::{self, VersionSpoof};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PatchOptions {
    pub source: PathBuf,
    pub output: PathBuf,
    pub state_dir: PathBuf,
    pub work_dir: Option<PathBuf>,
    pub keep_work: bool,
    pub source_trust: SourceTrust,
    pub install: bool,
    pub adb: Option<PathBuf>,
    pub selection: PatchSelection,
    pub spoof: VersionSpoof,
    pub allow_untested_version: bool,
    pub release_channel: ReleaseChannel,
    pub release_metadata: ReleaseMetadata,
    pub discord_embedded_aar: Option<PathBuf>,
    pub discord_sdk_aar: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ReleaseMetadata {
    pub source_page: Option<String>,
    pub container_sha256: Option<String>,
    pub architectures: Vec<String>,
    pub min_sdk: Option<u32>,
    pub file_type: Option<String>,
    pub abi: Option<Abi>,
    pub upstream_source: Option<String>,
    pub releases_api: Option<String>,
    pub source_verified: Option<bool>,
    pub source_certificate_sha256: Option<String>,
}

#[derive(Debug)]
pub struct PatchResult {
    pub output: PathBuf,
    pub version_name: String,
    pub version_code: String,
    pub output_version_name: String,
    pub output_version_code: String,
    pub source_sha256: String,
    pub output_sha256: String,
    pub output_cert: String,
    pub signing_key_created: bool,
    pub patch: PatchSummary,
    pub install_output: Option<String>,
    pub compatibility_status: String,
    pub compatibility_patch_version: Option<String>,
    pub compatibility_channel: ReleaseChannel,
    pub compatibility_package_format: crate::source::PackageFormat,
    pub compatibility_architectures: Vec<String>,
}

fn apk_info(decoded: &Path) -> Result<(String, String, String)> {
    let manifest = fs::read_to_string(decoded.join("AndroidManifest.xml"))?;
    let package_re = Regex::new(r#"<manifest[^>]+package="([^"]+)""#)?;
    let package = package_re
        .captures(&manifest)
        .and_then(|capture| capture.get(1))
        .context("в AndroidManifest.xml отсутствует package")?
        .as_str()
        .to_owned();
    let yml = fs::read_to_string(decoded.join("apktool.yml"))?;
    let value = |name: &str| -> Result<String> {
        let regex = Regex::new(&format!(r"(?m)^\s*{}:\s*(.+?)\s*$", regex::escape(name)))?;
        Ok(regex
            .captures(&yml)
            .and_then(|capture| capture.get(1))
            .context(format!("в apktool.yml отсутствует {name}"))?
            .as_str()
            .trim_matches([' ', '\'', '"'])
            .to_owned())
    };
    Ok((package, value("versionName")?, value("versionCode")?))
}

fn verify_zip_integrity(apk: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(apk)?)
        .context("final verification: APK не является корректным ZIP")?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("final verification: повреждена ZIP entry #{index}"))?;
        io::copy(&mut entry, &mut io::sink()).with_context(|| {
            format!("final verification: CRC/данные {} повреждены", entry.name())
        })?;
    }
    Ok(())
}

struct MetadataContext<'a> {
    original_name: &'a str,
    original_code: &'a str,
    display_name: &'a str,
    technical_code: &'a str,
    source_sha256: &'a str,
    channel: ReleaseChannel,
    applied_patches: &'a [&'a str],
    release: &'a ReleaseMetadata,
    signing_certificate: &'a str,
}

fn enrich_patch_metadata(decoded: &Path, context: MetadataContext<'_>) -> Result<()> {
    let path = decoded.join("assets/danger-patch.json");
    if !path.is_file() {
        return Ok(());
    }
    let mut metadata: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    let object = metadata
        .as_object_mut()
        .context("danger-patch.json должен быть JSON object")?;
    object.insert("originalVersionName".into(), context.original_name.into());
    object.insert("originalVersionCode".into(), context.original_code.into());
    object.insert("displayVersionName".into(), context.display_name.into());
    object.insert("technicalVersionCode".into(), context.technical_code.into());
    object.insert("channel".into(), context.channel.to_string().into());
    object.insert("sourceArtifactSha256".into(), context.source_sha256.into());
    object.insert(
        "appliedPatches".into(),
        serde_json::json!(context.applied_patches),
    );
    object.insert("buildDate".into(), chrono::Utc::now().to_rfc3339().into());
    object.insert("patcherVersion".into(), env!("CARGO_PKG_VERSION").into());
    object.insert("abi".into(), serde_json::json!(context.release.abi));
    object.insert(
        "upstreamSource".into(),
        serde_json::json!(context.release.upstream_source),
    );
    object.insert(
        "releasesApi".into(),
        serde_json::json!(context.release.releases_api),
    );
    object.insert(
        "sourceVerified".into(),
        serde_json::json!(context.release.source_verified),
    );
    object.insert(
        "sourceCertificateSha256".into(),
        serde_json::json!(context.release.source_certificate_sha256),
    );
    object.insert(
        "signingCertificateSha256".into(),
        context.signing_certificate.into(),
    );
    object.insert(
        "sourcePage".into(),
        serde_json::json!(context.release.source_page),
    );
    object.insert(
        "containerSha256".into(),
        serde_json::json!(context.release.container_sha256),
    );
    object.insert(
        "architectures".into(),
        serde_json::json!(context.release.architectures),
    );
    object.insert("minSdk".into(), serde_json::json!(context.release.min_sdk));
    object.insert(
        "fileType".into(),
        serde_json::json!(context.release.file_type),
    );
    object.insert("signatureStatus".into(), "signed and verified".into());
    fs::write(path, serde_json::to_vec_pretty(&metadata)?)?;

    let details = format!(
        "Yandex Music {} ({})\\nChannel: {} · ABI: {} · minSdk {}\\nPatches: {}\\nSource SHA-256: {}\\nSignature: verified by ympatcher",
        context.original_name,
        context.original_code,
        context.channel,
        if context.release.architectures.is_empty() { "unknown".to_owned() } else { context.release.architectures.join(", ") },
        context.release.min_sdk.map(|value| value.to_string()).unwrap_or_else(|| "unknown".to_owned()),
        context.applied_patches.join(", "),
        context.source_sha256,
    )
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;");
    let regex = Regex::new(r#"<string name="danger_about_build_details"[^>]*>.*?</string>"#)?;
    for resources in [
        decoded.join("res/values/danger_strings.xml"),
        decoded.join("res/values-ru/danger_strings.xml"),
    ] {
        if resources.is_file() {
            let text = fs::read_to_string(&resources)?;
            let replacement = format!(
                r#"<string name="danger_about_build_details" translatable="false">{details}</string>"#,
            );
            fs::write(resources, regex.replace(&text, replacement).as_bytes())?;
        }
    }
    Ok(())
}

pub fn patch_apk(options: PatchOptions) -> Result<PatchResult> {
    if !options.source.is_file() {
        bail!("входной APK не найден: {}", options.source.display());
    }
    if options.output.exists() {
        bail!("выходной файл уже существует: {}", options.output.display());
    }
    fs::create_dir_all(&options.state_dir)?;
    let tools = prepare_toolchain(&options.state_dir)?;
    match &options.source_trust {
        SourceTrust::OfficialSignedApk => {
            let source_cert = verify_apk(&options.source, &tools)?;
            let official_certificate = compatibility::official_certificate();
            if source_cert != official_certificate {
                bail!(
                    "сертификат входного APK не официальный\nПолучен:  {source_cert}\nОжидался: {official_certificate}"
                );
            }
        }
        SourceTrust::UserSuppliedUnknown => {
            verify_apk(&options.source, &tools)
                .context("пользовательский APK должен иметь валидную подпись")?;
        }
        SourceTrust::VerifiedNormalizedArtifact {
            artifact_size,
            artifact_sha256,
            source_cert_sha256,
            ..
        } => {
            let size = options.source.metadata()?.len();
            if size != *artifact_size {
                bail!("source provenance: размер cache artifact изменился");
            }
            let sha256 = sha256_file(&options.source)?;
            if !sha256.eq_ignore_ascii_case(artifact_sha256) {
                bail!("source provenance: SHA-256 cache artifact изменился");
            }
            if !source_cert_sha256.eq_ignore_ascii_case(compatibility::official_certificate()) {
                bail!("source provenance: сертификат origin не официальный");
            }
        }
    }

    let temporary = if let Some(root) = &options.work_dir {
        fs::create_dir_all(root)?;
        tempfile::Builder::new()
            .prefix("ympatch-")
            .tempdir_in(root)?
    } else {
        tempfile::Builder::new().prefix("ympatch-").tempdir()?
    };
    let root = temporary.path().to_owned();
    let decoded = root.join("decoded");
    let unsigned = root.join("unsigned.apk");

    let decode_args: Vec<OsString> = vec![
        "-Xmx4g".into(),
        "-jar".into(),
        tools.apktool.as_os_str().to_owned(),
        "d".into(),
        "--force".into(),
        "--output".into(),
        decoded.as_os_str().to_owned(),
        options.source.as_os_str().to_owned(),
    ];
    run(&tools.java, decode_args).context("apktool decode завершился ошибкой")?;
    let (package, version_name, version_code) = apk_info(&decoded)?;
    let source_sha256 = sha256_file(&options.source)?;
    if let SourceTrust::VerifiedNormalizedArtifact {
        package_name,
        channel,
        version_name: expected_name,
        version_code: expected_code,
        abi: expected_abi,
        ..
    } = &options.source_trust
        && (package != *package_name
            || version_name != *expected_name
            || version_code != expected_code.to_string()
            || options.release_channel != *channel
            || options.release_metadata.abi != Some(*expected_abi))
    {
        bail!(
            "source provenance: decoded APK metadata не совпадает с API (package={package}, versionName={version_name}, versionCode={version_code})"
        );
    }
    let compatibility = compatibility::check(
        options.release_channel,
        &package,
        &version_name,
        &version_code,
        options.release_metadata.abi,
        options.allow_untested_version,
    )?;
    let original_version_code = version_code
        .parse::<u64>()
        .context("оригинальный versionCode не является целым числом")?;
    options
        .spoof
        .validate_against_original(original_version_code)?;
    let patch = apply_all(&decoded, &version_name, &version_code, options.selection)?;
    if options.selection.discord_rpc {
        let embedded = options
            .discord_embedded_aar
            .as_deref()
            .context("для discord-rpc передайте --discord-embedded-aar FILE")?;
        let sdk = options
            .discord_sdk_aar
            .as_deref()
            .context("для discord-rpc передайте --discord-sdk-aar FILE")?;
        crate::presence::inject(
            &decoded,
            embedded,
            sdk,
            &tools,
            options.release_metadata.abi,
        )?;
    }
    versioning::apply(&decoded, &options.spoof)?;
    let output_version_name = options
        .spoof
        .version_name
        .clone()
        .unwrap_or_else(|| version_name.clone());
    let output_version_code = options
        .spoof
        .technical_version_code
        .map(|value| value.to_string())
        .unwrap_or_else(|| version_code.clone());
    let (identity, signing_key_created) = ensure_signing_identity(&options.state_dir, &tools)?;
    let signing_certificate = identity_certificate(&identity, &tools)?;
    enrich_patch_metadata(
        &decoded,
        MetadataContext {
            original_name: &version_name,
            original_code: &version_code,
            display_name: &output_version_name,
            technical_code: &output_version_code,
            source_sha256: &source_sha256,
            channel: options.release_channel,
            applied_patches: &patch.applied,
            release: &options.release_metadata,
            signing_certificate: &signing_certificate,
        },
    )?;

    let build_args: Vec<OsString> = vec![
        "-Xmx4g".into(),
        "-jar".into(),
        tools.apktool.as_os_str().to_owned(),
        "b".into(),
        "--output".into(),
        unsigned.as_os_str().to_owned(),
        decoded.as_os_str().to_owned(),
    ];
    run(&tools.java, build_args).context("apktool build завершился ошибкой")?;
    sign_apk(&unsigned, &identity, &tools)?;
    let signed_cert = verify_apk(&unsigned, &tools)?;
    if signed_cert != signing_certificate {
        bail!("final verification: APK подписан неожиданным сертификатом");
    }
    crate::apk::publish(&unsigned, &options.output)?;
    verify_zip_integrity(&options.output)?;
    let output_cert = verify_apk(&options.output, &tools)?;
    let install_output = if options.install {
        Some(guarded_install(
            &options.output,
            &tools,
            options.adb.as_deref(),
            output_version_code.parse()?,
        )?)
    } else {
        None
    };
    let result = PatchResult {
        output: options.output.clone(),
        version_name,
        version_code,
        output_version_name,
        output_version_code,
        source_sha256,
        output_sha256: sha256_file(&options.output)?,
        output_cert,
        signing_key_created,
        patch,
        install_output,
        compatibility_status: compatibility.status,
        compatibility_patch_version: compatibility.tested_patch_version,
        compatibility_channel: compatibility.channel,
        compatibility_package_format: compatibility.package_format,
        compatibility_architectures: compatibility.architectures,
    };
    if options.keep_work {
        let kept = temporary.keep();
        eprintln!("Рабочий каталог сохранён: {}", kept.display());
    }
    Ok(result)
}
