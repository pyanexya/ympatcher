mod android;
pub mod core;

use crate::channel::ReleaseChannel;
use crate::compatibility;
use crate::process::run as process_run;
use crate::reporters;
use crate::signing::verify_apk;
use crate::tools::{prepare_toolchain, sha256_file};
use anyhow::{Context, Result, bail};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RunOptions<'a> {
    pub source: &'a Path,
    pub state_dir: &'a Path,
    pub json_output: &'a Path,
    pub markdown_output: &'a Path,
    pub compare_report: Option<&'a Path>,
    pub discord_payload: Option<&'a Path>,
    pub channel: ReleaseChannel,
}

pub fn default_markdown_path(json: &Path) -> PathBuf {
    json.with_extension("md")
}

pub fn run(options: RunOptions<'_>) -> Result<()> {
    if !options.source.is_file() {
        bail!("входной APK не найден: {}", options.source.display());
    }
    let tools = prepare_toolchain(options.state_dir)?;
    let source_certificate = verify_apk(options.source, &tools)?;
    if source_certificate != compatibility::official_certificate() {
        bail!("датамайнер принимает только APK с официальной подписью");
    }
    let temporary = tempfile::Builder::new().prefix("ymdatamine-").tempdir()?;
    let decoded = temporary.path().join("decoded");
    let args: Vec<OsString> = vec![
        "-Xmx4g".into(),
        "-jar".into(),
        tools.apktool.as_os_str().to_owned(),
        "d".into(),
        "--force".into(),
        "--output".into(),
        decoded.as_os_str().to_owned(),
        options.source.as_os_str().to_owned(),
    ];
    process_run(&tools.java, args).context("apktool decode для датамайнера завершился ошибкой")?;

    let mut report = android::scan(
        &decoded,
        options.channel,
        sha256_file(options.source)?,
        source_certificate,
    )?;
    report.content_sha256 = report.deterministic_hash()?;
    report.generated_at_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if let Some(path) = options.compare_report {
        let previous: core::DatamineReport = serde_json::from_str(&fs::read_to_string(path)?)
            .context("не удалось разобрать предыдущий dataminer report v2")?;
        report.comparison = Some(core::compare(&previous, &report));
    }

    reporters::json::write(options.json_output, &report)?;
    reporters::markdown::write(options.markdown_output, &report)?;
    if let Some(path) = options.discord_payload {
        reporters::discord::write_dry_run(path, &report)?;
    }
    println!(
        "Dataminer: {} ({}), {} findings, {} classes → {}, {}",
        report.metadata.version_name,
        report.metadata.version_code,
        report.snapshot.len(),
        report.metadata.smali_classes,
        options.json_output.display(),
        options.markdown_output.display()
    );
    Ok(())
}
