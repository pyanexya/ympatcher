use crate::channel::ReleaseChannel;
use crate::source::{DownloadedPackage, PackageFormat, ReleaseProvider, UserImportProvider};
use crate::{apk, process, signing, tools};
use anyhow::{Context, Result, bail};
use clap::Parser;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "ympatcher convert",
    about = "Merge a local APKS/APKM into a real APK, or repair an APK's native library layout. No application patches are applied."
)]
pub struct ConvertCli {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Reuse the same directory as previous builds to preserve the signing identity.
    #[arg(long)]
    pub state_dir: Option<PathBuf>,
}

pub fn run(cli: ConvertCli) -> Result<()> {
    require_apk_output(&cli.output)?;
    let state = cli.state_dir.unwrap_or_else(tools::default_state_dir);
    fs::create_dir_all(&state)?;
    let _lock = crate::PatchJobLock::acquire(&state)?;
    let temporary = tempfile::tempdir()?;
    let provider = UserImportProvider::new(cli.input, ReleaseChannel::Stable)?;
    let release = provider.latest(ReleaseChannel::Stable)?;
    let package = provider.download(&release, &temporary.path().join("input"))?;
    let toolchain = tools::prepare_toolchain(&state)?;
    let mut certificate = None;
    for file in &package.files {
        eprintln!("Проверяю подпись: {}", file.path.display());
        let found = signing::verify_apk(&file.path, &toolchain)?;
        if certificate
            .as_ref()
            .is_some_and(|expected| expected != &found)
        {
            bail!("APKS содержит APK с разными сертификатами");
        }
        certificate = Some(found);
    }
    let candidate = temporary.path().join("converted.apk");
    if package.release.format == PackageFormat::MonolithicApk {
        fs::copy(package.base_apk()?, &candidate)?;
    } else {
        merge(
            &package,
            package.base_apk()?,
            &candidate,
            &state,
            &toolchain,
        )?;
    }
    let (identity, created) = signing::ensure_signing_identity(&state, &toolchain)?;
    signing::sign_apk(&candidate, &identity, &toolchain)?;
    let cert = signing::verify_apk(&candidate, &toolchain)?;
    let count = apk::verify_layout(&candidate)?;
    apk::publish(&candidate, &cli.output)?;
    println!(
        "Готово: {}\nNative libraries: {count} (Stored, 16 KiB)\nSHA-256: {}\nСертификат: {cert}",
        cli.output.display(),
        tools::sha256_file(&cli.output)?
    );
    if created {
        println!("Создан ключ подписи; сохраните --state-dir для совместимых обновлений.");
    }
    Ok(())
}

pub fn require_apk_output(output: &Path) -> Result<()> {
    if !output
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("apk"))
    {
        bail!("единый APK должен иметь расширение .apk");
    }
    Ok(())
}

/// Only stage the already validated APKs, never pass an untrusted container to Java.
pub fn merge(
    package: &DownloadedPackage,
    patched_base: &Path,
    output: &Path,
    state: &Path,
    toolchain: &tools::Toolchain,
) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let staging = temporary.path().join("apks");
    fs::create_dir(&staging)?;
    let expected = manifest_identity(package.base_apk()?, temporary.path(), "base", toolchain)?;
    let target = manifest_identity(patched_base, temporary.path(), "target", toolchain)?;
    if target.package != expected.package {
        bail!("package пропатченного base изменился");
    }
    let mut split_ids = std::collections::BTreeSet::new();
    for (index, file) in package.files.iter().enumerate() {
        if file.role == crate::source::PackageFileRole::Base {
            continue;
        }
        eprintln!("Проверяю manifest: {}", file.path.display());
        let id = manifest_identity(
            &file.path,
            temporary.path(),
            &format!("split-{index}"),
            toolchain,
        )?;
        if id.package != expected.package || id.version != expected.version {
            bail!(
                "split {} принадлежит другому package/versionCode",
                file.path.display()
            );
        }
        let split = id.split.context("split APK не содержит split id")?;
        if id.feature || !split.starts_with("config.") || !split_ids.insert(split) {
            bail!(
                "поддерживаются только уникальные config splits одного base; feature modules и несколько device variants не поддерживаются"
            );
        }
        fs::copy(&file.path, staging.join(format!("split-{index}.apk")))?;
    }
    fs::copy(patched_base, staging.join("base.apk"))?;
    let merger = tools::prepare_merger(state)?;
    eprintln!("Объединяю ресурсы APK…");
    let args: Vec<OsString> = vec![
        "-Xmx2g".into(),
        "-jar".into(),
        merger.into_os_string(),
        "m".into(),
        "-i".into(),
        staging.into_os_string(),
        "-o".into(),
        output.as_os_str().to_owned(),
    ];
    process::run(&toolchain.java, args)
        .context("не удалось объединить split resources и manifest")?;
    if !output.is_file() {
        bail!("merger не создал APK");
    }
    let merged = manifest_identity(output, temporary.path(), "merged", toolchain)?;
    if merged.package != expected.package
        || merged.version != target.version
        || merged.split.is_some()
        || merged.requires_splits
    {
        bail!("после объединения manifest всё ещё требует splits или изменился package");
    }
    Ok(())
}

struct ManifestIdentity {
    package: String,
    version: String,
    split: Option<String>,
    feature: bool,
    requires_splits: bool,
}

fn manifest_identity(
    input: &Path,
    work: &Path,
    name: &str,
    toolchain: &tools::Toolchain,
) -> Result<ManifestIdentity> {
    let decoded = work.join(name);
    let args: Vec<OsString> = vec![
        "-jar".into(),
        toolchain.apktool.as_os_str().to_owned(),
        "d".into(),
        "--only-manifest".into(),
        "--no-src".into(),
        "--no-assets".into(),
        "-o".into(),
        decoded.as_os_str().to_owned(),
        input.as_os_str().to_owned(),
    ];
    process::run(&toolchain.java, args)?;
    parse_identity(
        &fs::read_to_string(decoded.join("AndroidManifest.xml"))?,
        &fs::read_to_string(decoded.join("apktool.yml"))?,
    )
}

fn parse_identity(xml: &str, yaml: &str) -> Result<ManifestIdentity> {
    let root = regex::Regex::new(r"(?s)<manifest\b[^>]*>")?
        .find(xml)
        .context("manifest root отсутствует")?
        .as_str();
    let attribute = |name: &str| -> Option<String> {
        regex::Regex::new(&format!(r#"\s{}="([^"]*)""#, regex::escape(name)))
            .ok()?
            .captures(root)
            .map(|c| c[1].to_owned())
    };
    let version = attribute("android:versionCode")
        .or_else(|| {
            regex::Regex::new(r#"(?m)^\s*versionCode:\s*['"]?([0-9]+)"#)
                .ok()?
                .captures(yaml)
                .map(|c| c[1].to_owned())
        })
        .context("versionCode отсутствует")?;
    Ok(ManifestIdentity {
        package: attribute("package").context("package отсутствует")?,
        version,
        split: attribute("split").filter(|v| !v.is_empty()),
        feature: attribute("android:isFeatureSplit").as_deref() == Some("true"),
        requires_splits: attribute("android:isSplitRequired").as_deref() == Some("true")
            || attribute("android:requiredSplitTypes").is_some_and(|v| !v.is_empty())
            || xml.contains("<uses-split"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_split_requirements_and_version() {
        let id = parse_identity(r#"<manifest package="ru.yandex.music" split="config.arm64_v8a" android:versionCode="123" android:isSplitRequired="true"/>"#, "").unwrap();
        assert_eq!(id.version, "123");
        assert_eq!(id.split.as_deref(), Some("config.arm64_v8a"));
        assert!(id.requires_splits);
        let id = parse_identity(r#"<manifest package="ru.yandex.music"><uses-split android:name="feature"/></manifest>"#, "versionInfo:\n  versionCode: '42'\n").unwrap();
        assert_eq!(id.version, "42");
        assert!(id.requires_splits);
    }
}
