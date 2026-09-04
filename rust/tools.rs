use crate::process::find_command;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const APKTOOL_VERSION: &str = "3.0.3";
const APKTOOL_URL: &str =
    "https://github.com/iBotPeaches/Apktool/releases/download/v3.0.3/apktool_3.0.3.jar";
const APKTOOL_SHA256: &str = "dbf930b076c6b9be08d57c449cacefc3bdd6b71ebd59b3066fc0e1f5b14f9423";
const SIGNER_VERSION: &str = "1.3.0";
const SIGNER_URL: &str = "https://github.com/patrickfav/uber-apk-signer/releases/download/v1.3.0/uber-apk-signer-1.3.0.jar";
const SIGNER_SHA256: &str = "e1299fd6fcf4da527dd53735b56127e8ea922a321128123b9c32d619bba1d835";
const R8_VERSION: &str = "8.11.32";
const R8_URL: &str =
    "https://dl.google.com/dl/android/maven2/com/android/tools/r8/8.11.32/r8-8.11.32.jar";
const R8_SHA256: &str = "f94dae9f2c852ad80115ad6e8ba803c6cdf8f01c137c96d7afd98482cdbc5f5a";

#[derive(Debug, Clone)]
pub struct Toolchain {
    pub java: PathBuf,
    pub keytool: PathBuf,
    pub apktool: PathBuf,
    pub signer: PathBuf,
    pub r8: PathBuf,
}

pub fn default_state_dir() -> PathBuf {
    if let Some(value) = std::env::var_os("LOCALAPPDATA") {
        let root = PathBuf::from(value);
        let legacy = root.join("yandexmusic-danger-patcher");
        if legacy.is_dir() && !root.join("ympatcher").exists() {
            return legacy;
        }
        return root.join("ympatcher");
    }
    if let Some(value) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(value).join("ympatcher");
    }
    dirs::state_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ympatcher")
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("не удалось открыть {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn download_verified(url: &str, destination: &Path, expected: &str) -> Result<()> {
    if destination.is_file() && sha256_file(destination)? == expected {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension("jar.download");
    let mut response = reqwest::blocking::Client::builder()
        .user_agent("pyanexu-ympatcher/0.2.0")
        .build()?
        .get(url)
        .send()
        .with_context(|| format!("не удалось скачать {url}"))?
        .error_for_status()?;
    let mut output = File::create(&temporary)?;
    std::io::copy(&mut response, &mut output)?;
    output.flush()?;
    let actual = sha256_file(&temporary)?;
    if actual != expected {
        let _ = fs::remove_file(&temporary);
        bail!(
            "SHA-256 {} не совпала: ожидалась {expected}, получена {actual}",
            destination.display()
        );
    }
    fs::rename(&temporary, destination)?;
    Ok(())
}

pub fn prepare_toolchain(state_dir: &Path) -> Result<Toolchain> {
    let java = find_command("java").context("нужен JDK 17+ с java в PATH")?;
    let keytool = find_command("keytool").context("нужен JDK 17+ с keytool в PATH")?;
    let tools = state_dir.join("tools");
    let apktool = tools.join(format!("apktool-{APKTOOL_VERSION}.jar"));
    let signer = tools.join(format!("uber-apk-signer-{SIGNER_VERSION}.jar"));
    let r8 = tools.join(format!("r8-{R8_VERSION}.jar"));
    download_verified(APKTOOL_URL, &apktool, APKTOOL_SHA256)?;
    download_verified(SIGNER_URL, &signer, SIGNER_SHA256)?;
    download_verified(R8_URL, &r8, R8_SHA256)?;
    Ok(Toolchain {
        java,
        keytool,
        apktool,
        signer,
        r8,
    })
}

pub fn prepare_merger(state_dir: &Path) -> Result<PathBuf> {
    let path = state_dir.join("tools/APKEditor-1.4.9.jar");
    download_verified(
        "https://github.com/REAndroid/APKEditor/releases/download/V1.4.9/APKEditor-1.4.9.jar",
        &path,
        "a9cd40df818845456be6d696de6110c89edf4b0a0580cb83438ed6b25a366e67",
    )?;
    Ok(path)
}
