use crate::process::{find_command, run};
use crate::signing::verify_apk;
use crate::tools::Toolchain;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs::File;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const PACKAGE_NAME: &str = "ru.yandex.music";

pub fn guarded_install(
    apk: &Path,
    tools: &Toolchain,
    adb_override: Option<&Path>,
    output_version_code: u64,
) -> Result<String> {
    let adb = match adb_override {
        Some(path) if path.is_file() => path.to_owned(),
        Some(path) => find_command(&path.to_string_lossy()).context("переданный adb не найден")?,
        None => find_command("adb").context("adb не найден; передайте --adb PATH")?,
    };
    let state = run(&adb, ["get-state"])?;
    if state.trim() != "device" {
        bail!("ADB-устройство не готово: {:?}", state.trim());
    }
    let output_cert = verify_apk(apk, tools)?;
    let paths = run(&adb, ["shell", "pm", "path", PACKAGE_NAME])?;
    if paths.trim().is_empty() {
        return run(&adb, ["install".as_ref(), apk.as_os_str()]);
    }
    let remote = paths
        .lines()
        .find(|line| line.starts_with("package:") && line.contains("base.apk"))
        .map(|line| line.trim_start_matches("package:").trim())
        .context("не найден base.apk установленного Yandex Music")?;
    let temporary = tempfile::Builder::new()
        .prefix("ympatch-installed-")
        .tempdir()?;
    let pulled = temporary.path().join("installed-base.apk");
    run(&adb, ["pull".as_ref(), remote.as_ref(), pulled.as_os_str()])?;
    let installed_cert = verify_apk(&pulled, tools)?;
    if installed_cert != output_cert {
        bail!(
            "установка остановлена: подписи не совпадают; данные и сессия не изменены\nУстановлено: {installed_cert}\nПатч:       {output_cert}"
        );
    }
    let package_dump = run(&adb, ["shell", "dumpsys", "package", PACKAGE_NAME])?;
    if let Some(installed_version_code) = installed_version_code(&package_dump)? {
        ensure_not_downgrade(installed_version_code, output_version_code)?;
    }
    run(&adb, ["install".as_ref(), "-r".as_ref(), apk.as_os_str()])
}

pub fn guarded_install_bundle(
    bundle: &Path,
    tools: &Toolchain,
    adb_override: Option<&Path>,
    output_version_code: u64,
) -> Result<String> {
    let temporary = tempfile::Builder::new()
        .prefix("ympatch-install-")
        .tempdir()?;
    let mut archive = ZipArchive::new(File::open(bundle)?).context("APKS повреждён")?;
    let mut apks = Vec::<PathBuf>::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() || !entry.name().to_ascii_lowercase().ends_with(".apk") {
            continue;
        }
        let enclosed = entry
            .enclosed_name()
            .context("APKS содержит небезопасный путь")?;
        if enclosed.components().count() != 1 {
            bail!("APKS содержит вложенный APK path");
        }
        let output = temporary.path().join(enclosed);
        let mut writer = File::create(&output)?;
        std::io::copy(&mut entry, &mut writer)?;
        apks.push(output);
    }
    apks.sort_by_key(|path| path.file_name().map(|name| name != "base.apk"));
    let base = apks
        .iter()
        .find(|path| path.file_name().is_some_and(|name| name == "base.apk"))
        .context("APKS не содержит base.apk")?;
    let adb = resolve_adb(adb_override)?;
    guard_device_and_installed(&adb, base, tools, output_version_code)?;
    let mut args = vec!["install-multiple".into(), "-r".into()];
    args.extend(apks.iter().map(|path| path.as_os_str().to_owned()));
    run(&adb, args)
}

fn resolve_adb(adb_override: Option<&Path>) -> Result<std::path::PathBuf> {
    match adb_override {
        Some(path) if path.is_file() => Ok(path.to_owned()),
        Some(path) => find_command(&path.to_string_lossy()).context("переданный adb не найден"),
        None => find_command("adb").context("adb не найден; передайте --adb PATH"),
    }
}

fn guard_device_and_installed(
    adb: &Path,
    candidate_base: &Path,
    tools: &Toolchain,
    output_version_code: u64,
) -> Result<()> {
    let state = run(adb, ["get-state"])?;
    if state.trim() != "device" {
        bail!("ADB-устройство не готово: {:?}", state.trim());
    }
    let output_cert = verify_apk(candidate_base, tools)?;
    let paths = run(adb, ["shell", "pm", "path", PACKAGE_NAME])?;
    if paths.trim().is_empty() {
        return Ok(());
    }
    let remote = paths
        .lines()
        .find(|line| line.starts_with("package:") && line.contains("base.apk"))
        .map(|line| line.trim_start_matches("package:").trim())
        .context("не найден base.apk установленного Yandex Music")?;
    let temporary = tempfile::Builder::new()
        .prefix("ympatch-installed-")
        .tempdir()?;
    let pulled = temporary.path().join("installed-base.apk");
    run(adb, ["pull".as_ref(), remote.as_ref(), pulled.as_os_str()])?;
    let installed_cert = verify_apk(&pulled, tools)?;
    if installed_cert != output_cert {
        bail!(
            "установка остановлена: подписи не совпадают; данные и сессия не изменены\nУстановлено: {installed_cert}\nПатч:       {output_cert}"
        );
    }
    let package_dump = run(adb, ["shell", "dumpsys", "package", PACKAGE_NAME])?;
    if let Some(installed_version_code) = installed_version_code(&package_dump)? {
        ensure_not_downgrade(installed_version_code, output_version_code)?;
    }
    Ok(())
}

fn installed_version_code(package_dump: &str) -> Result<Option<u64>> {
    let regex = Regex::new(r"(?m)\bversionCode=(\d+)\b")?;
    regex
        .captures(package_dump)
        .map(|capture| capture[1].parse().map_err(Into::into))
        .transpose()
}

fn ensure_not_downgrade(installed: u64, candidate: u64) -> Result<()> {
    if candidate < installed {
        bail!(
            "установка остановлена: versionCode {candidate} ниже установленного {installed}; downgrade запрещён, данные не изменены"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_code_and_rejects_downgrade() {
        let dump = "Packages:\n  versionCode=24026431 minSdk=26";
        assert_eq!(installed_version_code(dump).unwrap(), Some(24_026_431));
        assert!(ensure_not_downgrade(24_026_431, 24_026_430).is_err());
        assert!(ensure_not_downgrade(24_026_431, 24_026_431).is_ok());
    }
}
