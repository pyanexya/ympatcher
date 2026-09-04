use super::core::{Confidence, DatamineReport, Evidence, Finding, Metadata};
use crate::channel::ReleaseChannel;
use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn add(
    findings: &mut BTreeSet<Finding>,
    section: &str,
    key: impl Into<String>,
    value: impl Into<String>,
    source: impl Into<String>,
    confidence: Confidence,
) {
    findings.insert(Finding {
        section: section.into(),
        key: key.into(),
        value: value.into(),
        evidence: Evidence {
            source: source.into(),
            confidence,
        },
    });
}

fn apk_metadata(decoded: &Path) -> Result<(String, String, String)> {
    let manifest = fs::read_to_string(decoded.join("AndroidManifest.xml"))?;
    let package = Regex::new(r#"<manifest[^>]+package="([^"]+)""#)?
        .captures(&manifest)
        .and_then(|capture| capture.get(1))
        .context("AndroidManifest.xml не содержит package")?
        .as_str()
        .to_owned();
    let yml = fs::read_to_string(decoded.join("apktool.yml"))?;
    let value = |key: &str| -> Result<String> {
        Ok(
            Regex::new(&format!(r"(?m)^\s*{}:\s*(.+?)\s*$", regex::escape(key)))?
                .captures(&yml)
                .and_then(|capture| capture.get(1))
                .context(format!("apktool.yml не содержит {key}"))?
                .as_str()
                .trim_matches([' ', '\'', '"'])
                .to_owned(),
        )
    };
    Ok((package, value("versionName")?, value("versionCode")?))
}

fn scan_manifest(decoded: &Path, findings: &mut BTreeSet<Finding>) -> Result<()> {
    let path = decoded.join("AndroidManifest.xml");
    let text = fs::read_to_string(&path)?;
    let permission = Regex::new(r#"<uses-permission[^>]+android:name="([^"]+)""#)?;
    for capture in permission.captures_iter(&text) {
        add(
            findings,
            "permissions",
            capture[1].to_owned(),
            "declared",
            "AndroidManifest.xml",
            Confidence::High,
        );
    }
    let component =
        Regex::new(r#"<(activity|service|receiver|provider)\b[^>]+android:name="([^"]+)"[^>]*"#)?;
    for capture in component.captures_iter(&text) {
        add(
            findings,
            "components",
            capture[2].to_owned(),
            capture[1].to_owned(),
            "AndroidManifest.xml",
            Confidence::High,
        );
    }
    let scheme = Regex::new(r#"<data[^>]+android:scheme="([^"]+)"[^>]*"#)?;
    let host = Regex::new(r#"android:host="([^"]+)""#)?;
    for capture in scheme.captures_iter(&text) {
        add(
            findings,
            "deep_links",
            capture[1].to_owned(),
            host.captures(capture.get(0).unwrap().as_str())
                .map(|item| item[1].to_owned())
                .unwrap_or_else(|| "*".into()),
            "AndroidManifest.xml",
            Confidence::High,
        );
    }
    Ok(())
}

fn scan_resources(decoded: &Path, findings: &mut BTreeSet<Finding>) -> Result<()> {
    let mut resource_counts = BTreeMap::<String, usize>::new();
    let public = decoded.join("res/values/public.xml");
    if public.is_file() {
        let text = fs::read_to_string(public)?;
        let entry = Regex::new(r#"<public\s+type="([^"]+)"\s+name="([^"]+)""#)?;
        for capture in entry.captures_iter(&text) {
            *resource_counts.entry(capture[1].to_owned()).or_default() += 1;
            if &capture[1] == "string" {
                add(
                    findings,
                    "strings",
                    capture[2].to_owned(),
                    "present",
                    "res/values/public.xml",
                    Confidence::High,
                );
            }
        }
    }
    for (kind, count) in resource_counts {
        add(
            findings,
            "resources",
            kind,
            count.to_string(),
            "res/values/public.xml",
            Confidence::High,
        );
    }
    let res = decoded.join("res");
    if res.is_dir() {
        for entry in fs::read_dir(res)?.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "values" || name.starts_with("values-") {
                add(
                    findings,
                    "localizations",
                    name.trim_start_matches("values-").to_owned(),
                    "present",
                    format!("res/{name}"),
                    Confidence::High,
                );
            }
        }
    }
    Ok(())
}

fn scan_smali(decoded: &Path, findings: &mut BTreeSet<Finding>) -> Result<usize> {
    let const_string = Regex::new(r#"const-string(?:/jumbo)?\s+v\d+,\s+"([^"]+)""#)?;
    let experiment = Regex::new(r"(?i)^(android|music|ym|feature)[a-z0-9_.-]{4,}$")?;
    let endpoint = Regex::new(r#"(?i)^https?://[^\s"<>]+"#)?;
    let method = Regex::new(r"^(GET|POST|PUT|PATCH|DELETE|HEAD)$")?;
    let route = Regex::new(r"^/[a-zA-Z0-9_./{}:-]{3,}$")?;
    let sdk = Regex::new(r"^\.class[^L]+L(com|io|org)/([^/;]+)")?;
    let model_marker = Regex::new(r"(GeneratedMessageLite|JsonClass|SerialName|ProtoAdapter)")?;
    let method_signature = Regex::new(r"(?m)^\.method\s+(?:[^\n ]+\s+)*([^\n]+)$")?;
    let paths = WalkDir::new(decoded)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("smali")
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let scanned = paths
        .par_iter()
        .map(|path| -> Result<(Option<String>, Vec<Finding>)> {
            let relative = path.strip_prefix(decoded)?.to_string_lossy().into_owned();
            let text = fs::read_to_string(path)?;
            let sdk_name = sdk.captures(&text).map(|capture| capture[2].to_owned());
            let mut local = Vec::new();
            let mut push = |section: &str, key: String, value: String, confidence: Confidence| {
                local.push(Finding {
                    section: section.into(),
                    key,
                    value,
                    evidence: Evidence {
                        source: relative.clone(),
                        confidence,
                    },
                });
            };
            if model_marker.is_match(&text) {
                push(
                    "models",
                    relative.clone(),
                    "protobuf_or_json".into(),
                    Confidence::Medium,
                );
            }
            let constructor_nearby = text.contains("Experiment")
                || text.contains("experiment")
                || text.contains("FeatureFlag")
                || text.contains("feature_flag");
            let significant = constructor_nearby
                || model_marker.is_match(&text)
                || text.contains("https://")
                || text.contains("http://");
            for capture in const_string.captures_iter(&text) {
                let value = &capture[1];
                if endpoint.is_match(value) {
                    push(
                        "endpoints",
                        value.into(),
                        "referenced".into(),
                        Confidence::High,
                    );
                } else if method.is_match(value) {
                    push(
                        "http_methods",
                        format!("{}:{value}", relative),
                        value.into(),
                        Confidence::Medium,
                    );
                } else if route.is_match(value) {
                    push(
                        "routes",
                        value.into(),
                        "referenced".into(),
                        Confidence::Medium,
                    );
                } else if experiment.is_match(value) && constructor_nearby {
                    push(
                        "experiments",
                        value.into(),
                        "declared_or_referenced".into(),
                        Confidence::Medium,
                    );
                }
            }
            if significant {
                push(
                    "significant_smali",
                    relative.clone(),
                    hex::encode(Sha256::digest(text.as_bytes())),
                    Confidence::High,
                );
                for capture in method_signature.captures_iter(&text).take(200) {
                    push(
                        "functions",
                        format!("{}::{}", relative, capture[1].trim()),
                        "present".into(),
                        Confidence::High,
                    );
                }
            }
            Ok((sdk_name, local))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut sdk_counts = BTreeMap::<String, usize>::new();
    for (sdk_name, local) in scanned {
        if let Some(name) = sdk_name {
            *sdk_counts.entry(name).or_default() += 1;
        }
        findings.extend(local);
    }
    for (name, count) in sdk_counts {
        add(
            findings,
            "sdk_packages",
            name,
            count.to_string(),
            "smali class namespace aggregate",
            Confidence::Medium,
        );
    }
    Ok(paths.len())
}

pub fn scan(
    decoded: &Path,
    channel: ReleaseChannel,
    source_sha256: String,
    certificate: String,
) -> Result<DatamineReport> {
    let (package_name, version_name, version_code) = apk_metadata(decoded)?;
    if package_name != crate::source::PACKAGE_NAME {
        anyhow::bail!(
            "датамайнер ожидал package {}, получен {package_name}",
            crate::source::PACKAGE_NAME
        );
    }
    let mut findings = BTreeSet::new();
    scan_manifest(decoded, &mut findings)?;
    scan_resources(decoded, &mut findings)?;
    let smali_classes = scan_smali(decoded, &mut findings)?;
    Ok(DatamineReport {
        schema_version: 2,
        generated_at_unix: 0,
        target: "android".into(),
        metadata: Metadata {
            package_name,
            version_name,
            version_code,
            channel,
            source_sha256,
            source_certificate_sha256: certificate,
            smali_classes,
        },
        content_sha256: String::new(),
        snapshot: findings.into_iter().collect(),
        comparison: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fixture_scanner_finds_manifest_and_non_obfuscated_experiment() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("res/values")).unwrap();
        fs::create_dir_all(root.path().join("smali/dev/example")).unwrap();
        fs::write(
            root.path().join("AndroidManifest.xml"),
            r#"<manifest package="ru.yandex.music"><uses-permission android:name="android.permission.INTERNET"/><application><activity android:name=".MainActivity"/></application></manifest>"#,
        )
        .unwrap();
        fs::write(
            root.path().join("apktool.yml"),
            "versionName: '1.2'\nversionCode: '12'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("res/values/public.xml"),
            r#"<resources><public type="string" name="hello" id="0x7f010001" /></resources>"#,
        )
        .unwrap();
        fs::write(
            root.path().join("smali/dev/example/Flags.smali"),
            ".class public Ldev/example/Flags;\n# FeatureFlag\nconst-string v0, \"android_new_wave\"",
        )
        .unwrap();
        let report = scan(
            root.path(),
            ReleaseChannel::Dev,
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap();
        assert!(
            report
                .snapshot
                .iter()
                .any(|item| item.section == "experiments")
        );
        assert!(
            report
                .snapshot
                .iter()
                .any(|item| item.section == "permissions")
        );
    }
}
