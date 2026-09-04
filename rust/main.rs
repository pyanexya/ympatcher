mod channel {
    pub use ympatcher::channel::*;
}
mod apk;
mod compatibility;
mod convert;
mod dataminer;
mod installer;
mod patcher;
mod patches;
mod presence;
mod process;
mod reporters;
mod signing;
mod source;
mod tools;
mod versioning;

use anyhow::{Context, Result};
use channel::ReleaseChannel;
use clap::{Args, Parser, Subcommand};
use patcher::{PatchOptions, patch_apk};
use patches::{PATCH_CATALOG, PatchSelection};
use source::{PackageFormat, ReleaseProvider};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use ympatcher::discovery::api::{DiscoveryDto, render_human};
use ympatcher::discovery::model::{DeviceProfile, Distribution, Platform};
use ympatcher::discovery::{DataminerConfig, YandexMusicDataminer};

#[derive(Parser, Debug)]
#[command(
    name = "ympatcher",
    version,
    about = "Rust Yandex Music APK patcher by Pyanexya"
)]
struct Cli {
    /// Official Yandex Music APK, APKS or APKM
    apk: Option<PathBuf>,
    /// Resolve the newest release through the selected provider
    #[arg(long, conflicts_with = "apk")]
    latest: bool,
    /// Yandex Music release channel, independent from the Git branch
    #[arg(long, default_value = "stable")]
    channel: ReleaseChannel,
    /// Release provider used by --latest (only ympatcher-api is supported)
    #[arg(
        long,
        default_value = "ympatcher-api",
        requires = "latest",
        hide = true
    )]
    provider: String,
    /// Output .apk (merged by default) or .apks (preserve splits)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Persistent tools and signing-key directory
    #[arg(long)]
    state_dir: Option<PathBuf>,
    /// Parent directory for temporary work
    #[arg(long)]
    work_dir: Option<PathBuf>,
    /// Keep decoded APK after completion
    #[arg(long)]
    keep_work: bool,
    /// Allow an input APK with a non-official certificate
    #[arg(long)]
    allow_unknown_source: bool,
    /// Install safely through adb after patching
    #[arg(long)]
    install: bool,
    /// Explicit adb executable
    #[arg(long)]
    adb: Option<PathBuf>,
    /// Apply only these patch IDs; repeat the option. Defaults to the standard profile
    #[arg(long = "patch", value_name = "ID")]
    patches: Vec<String>,
    /// Print the patch catalog and exit
    #[arg(long)]
    list_patches: bool,
    /// Override Android versionName in the rebuilt APK
    #[arg(long)]
    spoof_version_name: Option<String>,
    /// Deprecated alias for --technical-version-code
    #[arg(long, conflicts_with = "technical_version_code", hide = true)]
    spoof_version_code: Option<u32>,
    /// Monotonic Android versionCode used for safe patched-client updates
    #[arg(long)]
    technical_version_code: Option<u32>,
    /// Attempt strict fingerprints for a build not marked supported
    #[arg(long)]
    allow_untested_version: bool,
    /// Built embedded Presence AAR (required by --patch discord-rpc)
    #[arg(long, value_name = "FILE")]
    discord_embedded_aar: Option<PathBuf>,
    /// Discord Social SDK AAR (required by --patch discord-rpc)
    #[arg(long, value_name = "FILE")]
    discord_sdk_aar: Option<PathBuf>,
    /// Write a machine-readable APK dataminer report and exit
    #[arg(long, value_name = "REPORT.json")]
    datamine: Option<PathBuf>,
    /// Compare dataminer output with an earlier report
    #[arg(long, value_name = "REPORT.json", requires = "datamine")]
    compare_report: Option<PathBuf>,
    /// Write the human-readable dataminer report (defaults beside JSON)
    #[arg(long, value_name = "REPORT.md", requires = "datamine")]
    datamine_markdown: Option<PathBuf>,
    /// Dry-run: write Discord webhook payloads without sending them
    #[arg(long, value_name = "PAYLOAD.json", requires = "datamine")]
    discord_payload: Option<PathBuf>,
}

#[derive(Parser, Debug)]
#[command(
    name = "ympatcher dataminer",
    version,
    about = "Discover upstream Android releases"
)]
struct DiscoveryCli {
    #[command(subcommand)]
    command: DiscoveryCommand,
}

#[derive(Subcommand, Debug)]
enum DiscoveryCommand {
    /// Resolve the newest upstream release
    Latest(DiscoveryLatestArgs),
}

#[derive(Args, Debug)]
struct DiscoveryLatestArgs {
    /// Android package to inspect
    #[arg(long, default_value = "ru.yandex.music")]
    package: String,
    /// Release channel
    #[arg(long, default_value = "stable")]
    channel: ympatcher::channel::ReleaseChannel,
    /// Select a distribution stream in the top-level latest field
    #[arg(long)]
    distribution: Option<Distribution>,
    /// Target platform; phone releases are isolated from Wear OS
    #[arg(long, default_value = "android-phone")]
    platform: Platform,
    /// Emit stable JSON schema v1
    #[arg(long)]
    json: bool,
    /// Bypass the in-memory metadata cache
    #[arg(long)]
    force_refresh: bool,
    #[arg(long, default_value_t = 35)]
    sdk: u32,
    #[arg(long, default_value = "Pixel 8")]
    model: String,
    #[arg(long, default_value = "Google")]
    manufacturer: String,
    #[arg(long, default_value = "arm64-v8a")]
    abi: String,
    #[arg(long, default_value = "ru-RU")]
    locale: String,
    #[arg(long, default_value = "RU")]
    country: String,
}

fn default_output(source: &std::path::Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("yandex-music");
    source.with_file_name(format!("{stem}-patched-v{}.apk", patches::PATCH_VERSION))
}

fn latest_output(release: &source::Release) -> PathBuf {
    let version = release
        .version_name
        .as_deref()
        .unwrap_or("latest")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let extension = "apk";
    PathBuf::from("dist").join(format!(
        "YandexMusic-{}-{version}-ympatcher-v{}.{}",
        release.channel,
        patches::PATCH_VERSION,
        extension
    ))
}

fn main() {
    if let Err(error) = real_main() {
        eprintln!("\nОшибка: {error:#}");
        std::process::exit(2);
    }
}

fn real_main() -> Result<()> {
    let raw_args: Vec<OsString> = std::env::args_os().collect();
    if raw_args.get(1).is_some_and(|value| value == "convert") {
        init_logging(false);
        let mut forwarded = vec![raw_args[0].clone()];
        forwarded.extend(raw_args.into_iter().skip(2));
        return convert::run(convert::ConvertCli::parse_from(forwarded));
    }
    if raw_args.get(1).is_some_and(|value| value == "dataminer") {
        init_logging(true);
        let mut forwarded = vec![raw_args[0].clone()];
        forwarded.extend(raw_args.into_iter().skip(2));
        return run_discovery(DiscoveryCli::parse_from(forwarded));
    }
    init_logging(false);
    let cli = Cli::parse();
    if cli.list_patches {
        for patch in PATCH_CATALOG {
            let default = if patch.default_enabled {
                "default"
            } else {
                "optional"
            };
            let dependencies = if patch.dependencies.is_empty() {
                String::new()
            } else {
                format!("; requires {}", patch.dependencies.join(", "))
            };
            println!(
                "{:<16} {} ({default}{dependencies})",
                patch.id, patch.description
            );
        }
        return Ok(());
    }
    if cli.apk.is_none() && !cli.latest {
        anyhow::bail!("укажите путь к APK либо --latest");
    }
    let state_dir = cli.state_dir.unwrap_or_else(tools::default_state_dir);
    fs::create_dir_all(&state_dir)?;
    let _job = PatchJobLock::acquire(&state_dir)?;
    let selection = PatchSelection::from_ids(&cli.patches)?;
    let requested_input = cli.apk.clone();
    let downloaded = if cli.latest {
        if cli.provider != "ympatcher-api" {
            anyhow::bail!("неизвестный release provider: {}", cli.provider);
        }
        let cancellation = ympatcher::discovery::http::CancellationToken::default();
        let handler_token = cancellation.clone();
        ctrlc::set_handler(move || handler_token.cancel())?;
        let provider = source::YmpatcherApiProvider::new(cancellation)?;
        println!(
            "[1/10] Проверяю {} для канала {}…",
            provider.id(),
            cli.channel
        );
        let release = provider.latest(cli.channel)?;
        println!(
            "  Версия:       {}",
            release.version_name.as_deref().unwrap_or("unknown")
        );
        println!(
            "  versionCode:   {}",
            release.version_code.unwrap_or_default()
        );
        println!(
            "  Дата:          {}",
            release.release_date.as_deref().unwrap_or("unknown")
        );
        println!(
            "  Размер:        {} bytes",
            release.expected_size.unwrap_or_default()
        );
        println!(
            "  Android:       {} (minSdk {})",
            release.min_android.as_deref().unwrap_or("unknown"),
            release.min_sdk.unwrap_or_default()
        );
        println!("  ABI:           {}", release.architecture.join(", "));
        println!("  Формат:        {}", release.format);
        println!("[2/10] Скачиваю release только через downloadUrl…");
        Some(provider.download(&release, &state_dir.join("cache"))?)
    } else if let Some(input) = &cli.apk {
        let provider = source::UserImportProvider::new(input.clone(), cli.channel)?;
        let release = provider.latest(cli.channel)?;
        Some(provider.download(&release, &state_dir.join("cache/import"))?)
    } else {
        unreachable!("source was validated")
    };
    let source = downloaded
        .as_ref()
        .map(|value| value.base_apk().map(PathBuf::from))
        .transpose()?
        .expect("source was validated");
    if let Some(package) = &downloaded
        && package.release.format != PackageFormat::MonolithicApk
    {
        let toolchain = tools::prepare_toolchain(&state_dir)?;
        for file in &package.files {
            let certificate = signing::verify_apk(&file.path, &toolchain)?;
            if certificate != compatibility::official_certificate() {
                anyhow::bail!(
                    "split {} не подписан официальным сертификатом: {certificate}",
                    file.path.display()
                );
            }
        }
    }
    if let Some(package) = &downloaded {
        println!(
            "Источник: {} / {} / {} file(s)",
            package.release.provider,
            package.release.format,
            package.files.len()
        );
        for file in &package.files {
            println!(
                "  {} bytes  {}  {}",
                file.size,
                file.sha256,
                file.path.display()
            );
        }
    }
    if let Some(report) = &cli.datamine {
        let markdown = cli
            .datamine_markdown
            .clone()
            .unwrap_or_else(|| dataminer::default_markdown_path(report));
        dataminer::run(dataminer::RunOptions {
            source: &source,
            state_dir: &state_dir,
            json_output: report,
            markdown_output: &markdown,
            compare_report: cli.compare_report.as_deref(),
            discord_payload: cli.discord_payload.as_deref(),
            channel: cli.channel,
        })?;
        return Ok(());
    }
    let package = downloaded.as_ref().expect("source was validated");
    let is_bundle = package.release.format != PackageFormat::MonolithicApk;
    let output = cli.output.clone().unwrap_or_else(|| {
        if cli.latest {
            latest_output(&package.release)
        } else {
            default_output(requested_input.as_deref().unwrap_or(&source))
        }
    });
    let output_splits = output
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("apks"));
    if output_splits && !is_bundle {
        anyhow::bail!("для monolithic input укажите выход .apk");
    }
    if !output_splits {
        convert::require_apk_output(&output)?;
    }
    if output_splits && (cli.technical_version_code.is_some() || cli.spoof_version_code.is_some()) {
        anyhow::bail!(
            "изменение versionCode поддерживается только при выходе .apk; splits должны иметь одну исходную версию"
        );
    }
    let bundle_work = if is_bundle {
        Some(
            tempfile::Builder::new()
                .prefix("ympatch-output-")
                .tempdir()?,
        )
    } else {
        None
    };
    let base_output = bundle_work
        .as_ref()
        .map(|directory| directory.path().join("base.apk"))
        .unwrap_or_else(|| output.clone());
    println!("[3/10] Размер и SHA-256 источника проверены.");
    println!("[4/10] APKM безопасно распакован; base и splits определены.");
    println!("[5/10] Проверяю APK и подготавливаю pinned-инструменты…");
    println!("[6/10] Декомпилирую, проверяю fingerprints и применяю патчи…");
    let mut result = patch_apk(PatchOptions {
        source,
        output: base_output.clone(),
        state_dir: state_dir.clone(),
        work_dir: cli.work_dir,
        keep_work: cli.keep_work,
        allow_unknown_source: cli.allow_unknown_source,
        install: cli.install && !is_bundle,
        adb: cli.adb.clone(),
        selection,
        spoof: versioning::VersionSpoof {
            version_name: cli.spoof_version_name,
            technical_version_code: cli.technical_version_code.or(cli.spoof_version_code),
        },
        allow_untested_version: cli.allow_untested_version,
        release_channel: cli.channel,
        release_metadata: patcher::ReleaseMetadata {
            source_page: package.release.source_page.clone(),
            container_sha256: package.container_sha256.clone(),
            architectures: package.release.architecture.clone(),
            min_sdk: package.release.min_sdk,
            file_type: Some(package.release.format.to_string()),
        },
        discord_embedded_aar: cli.discord_embedded_aar,
        discord_sdk_aar: cli.discord_sdk_aar,
    })?;
    if is_bundle {
        let toolchain = tools::prepare_toolchain(&state_dir)?;
        let certificate = if output_splits {
            println!("[7/10] Исправляю упаковку и переподписываю все splits…");
            let (certificate, count) = source::assemble_signed_bundle(
                package,
                &base_output,
                &output,
                &state_dir,
                &toolchain,
            )?;
            println!("[8/10] Устанавливаемый APKS собран ({count} APK).");
            certificate
        } else {
            println!("[7/10] Объединяю base, ресурсы и ABI splits в единый APK…");
            let merged = bundle_work
                .as_ref()
                .expect("bundle work")
                .path()
                .join("merged.apk");
            convert::merge(package, &base_output, &merged, &state_dir, &toolchain)?;
            let (identity, _) = signing::ensure_signing_identity(&state_dir, &toolchain)?;
            signing::sign_apk(&merged, &identity, &toolchain)?;
            let certificate = signing::verify_apk(&merged, &toolchain)?;
            apk::publish(&merged, &output)?;
            println!("[8/10] Единый APK подписан; .so без сжатия с выравниванием 16 КБ.");
            certificate
        };
        result.output = output.clone();
        result.output_sha256 = tools::sha256_file(&output)?;
        result.output_cert = certificate;
        if cli.install {
            let install = if output_splits {
                installer::guarded_install_bundle
            } else {
                installer::guarded_install
            };
            result.install_output = Some(install(
                &output,
                &toolchain,
                cli.adb.as_deref(),
                result.output_version_code.parse()?,
            )?);
        }
    } else {
        println!("[7/10] Monolithic APK пересобран.");
        println!("[8/10] APK подписан одним постоянным сертификатом.");
    }
    println!(
        "[9/10] Info и локализованные Developer experiments внедрены ({} флагов).",
        result.patch.experiments
    );
    println!("[10/10] Итоговая подпись и целостность проверены.");
    println!(
        "{}",
        if result.install_output.is_some() {
            "Обновление установлено без очистки данных."
        } else {
            "Установка не запрашивалась."
        }
    );
    println!("\nГотово:       {}", result.output.display());
    println!(
        "Оригинал:     {} ({})",
        result.version_name, result.version_code
    );
    println!("Отображается: {}", result.output_version_name);
    println!("Technical:    {}", result.output_version_code);
    println!("ympatcher:    {}", patches::PATCH_VERSION);
    println!("Патчи:        {}", result.patch.applied.join(", "));
    println!("Совместимость: {}", result.compatibility_status);
    println!("Канал:        {}", result.compatibility_channel);
    println!("Поставка:     {:?}", result.compatibility_package_format);
    if !result.compatibility_architectures.is_empty() {
        println!(
            "Архитектуры:  {}",
            result.compatibility_architectures.join(", ")
        );
    }
    if let Some(tested_with) = &result.compatibility_patch_version {
        println!("Проверено с:   ympatcher {tested_with}");
    }
    println!("SHA-256 source: {}", result.source_sha256);
    println!("SHA-256 APK:    {}", result.output_sha256);
    println!("Сертификат:     {}", result.output_cert);
    if let Some(renderer) = &result.patch.renderer {
        println!("Renderer:       {}", renderer.display());
    }
    if result.signing_key_created {
        println!(
            "Важно: сохраните state-dir — без keystore обновления потеряют совместимость подписи."
        );
    }
    if let Some(output) = result.install_output {
        println!("{}", output.trim());
    }
    Ok(())
}

struct PatchJobLock {
    path: PathBuf,
    _file: std::fs::File,
}

impl PatchJobLock {
    fn acquire(state_dir: &std::path::Path) -> Result<Self> {
        let path = state_dir.join("patch-job.lock");
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| {
                format!(
                    "другая patch job уже выполняется (lock: {})",
                    path.display()
                )
            })?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for PatchJobLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn init_logging(discovery: bool) {
    let default = if discovery { "info" } else { "warn" };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .try_init();
}

fn run_discovery(cli: DiscoveryCli) -> Result<()> {
    match cli.command {
        DiscoveryCommand::Latest(args) => {
            let config = DataminerConfig {
                package_name: args.package,
                channel: args.channel,
                platform: args.platform,
                device: DeviceProfile {
                    sdk: args.sdk,
                    model: args.model,
                    manufacturer: args.manufacturer,
                    abi: args.abi,
                    locale: args.locale,
                    country: args.country,
                },
                ..DataminerConfig::default()
            };
            let dataminer = YandexMusicDataminer::new(config)?;
            let mut result = dataminer.discover(args.force_refresh);
            if let Some(distribution) = args.distribution {
                result.latest = match distribution {
                    Distribution::GooglePlay => result.latest_google_play.clone(),
                    Distribution::RuStore => result.latest_rustore.clone(),
                    Distribution::Unknown => result.latest.clone(),
                };
            }
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&DiscoveryDto::from_result(&result))?
                );
            } else {
                print!("{}", render_human(&result));
            }
            Ok(())
        }
    }
}
