use crate::process::{run, run_redacted};
use crate::tools::Toolchain;
use anyhow::{Context, Result, bail};
use rand::{Rng, distributions::Alphanumeric};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SigningIdentity {
    pub keystore: PathBuf,
    pub alias: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
struct SigningDescriptor {
    alias: String,
    password: String,
}

pub fn ensure_signing_identity(
    state_dir: &Path,
    tools: &Toolchain,
) -> Result<(SigningIdentity, bool)> {
    let directory = state_dir.join("signing");
    fs::create_dir_all(&directory)?;
    let preferred_keystore = directory.join("ympatcher.p12");
    let legacy_keystore = directory.join("danger-patcher.p12");
    let keystore = if preferred_keystore.exists() {
        preferred_keystore
    } else if legacy_keystore.exists() {
        legacy_keystore
    } else {
        preferred_keystore
    };
    let descriptor = directory.join("signing.json");
    if keystore.exists() != descriptor.exists() {
        bail!("повреждено состояние подписи в {}", directory.display());
    }
    if keystore.exists() {
        let data: SigningDescriptor = serde_json::from_slice(&fs::read(&descriptor)?)?;
        return Ok((
            SigningIdentity {
                keystore,
                alias: data.alias,
                password: data.password,
            },
            false,
        ));
    }
    let alias = "ympatcher".to_owned();
    let password: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    let args = vec![
        "-genkeypair".into(),
        "-noprompt".into(),
        "-keystore".into(),
        keystore.as_os_str().to_owned(),
        "-storetype".into(),
        "PKCS12".into(),
        "-storepass".into(),
        password.clone().into(),
        "-keypass".into(),
        password.clone().into(),
        "-alias".into(),
        alias.clone().into(),
        "-keyalg".into(),
        "RSA".into(),
        "-keysize".into(),
        "4096".into(),
        "-validity".into(),
        "36500".into(),
        "-dname".into(),
        "CN=Pyanexy ympatcher, OU=Local Build, O=Pyanexya".into(),
    ];
    run_redacted(&tools.keytool, args, &[&password])?;
    let data = SigningDescriptor {
        alias: alias.clone(),
        password: password.clone(),
    };
    fs::write(&descriptor, serde_json::to_vec_pretty(&data)?)?;
    Ok((
        SigningIdentity {
            keystore,
            alias,
            password,
        },
        true,
    ))
}

pub fn verify_apk(apk: &Path, tools: &Toolchain) -> Result<String> {
    let args = vec![
        "-jar".into(),
        tools.signer.as_os_str().to_owned(),
        "--apks".into(),
        apk.as_os_str().to_owned(),
        "--onlyVerify".into(),
        "--verbose".into(),
    ];
    let output = run(&tools.java, args)?;
    if !output.to_ascii_lowercase().contains("signature verified") {
        bail!("подпись APK не прошла проверку");
    }
    let regex = Regex::new(r"(?mi)^\s*SHA256:\s*([0-9a-f]{64})\b")?;
    let certificate = regex
        .captures(&output)
        .and_then(|capture| capture.get(1))
        .context("signer не вернул SHA-256 сертификата")?
        .as_str()
        .to_ascii_lowercase();
    Ok(certificate)
}

pub fn sign_apk(apk: &Path, identity: &SigningIdentity, tools: &Toolchain) -> Result<()> {
    crate::apk::normalize(apk).context("не удалось исправить упаковку APK")?;
    let args = vec![
        "-jar".into(),
        tools.signer.as_os_str().to_owned(),
        "--apks".into(),
        apk.as_os_str().to_owned(),
        "--ks".into(),
        identity.keystore.as_os_str().to_owned(),
        "--ksAlias".into(),
        identity.alias.clone().into(),
        "--ksPass".into(),
        identity.password.clone().into(),
        "--ksKeyPass".into(),
        identity.password.clone().into(),
        "--allowResign".into(),
        "--skipZipAlign".into(),
        "--overwrite".into(),
    ];
    run_redacted(&tools.java, args, &[&identity.password]).context("не удалось подписать APK")?;
    crate::apk::verify_layout(apk).context("подписанный APK имеет неверную упаковку")?;
    Ok(())
}
