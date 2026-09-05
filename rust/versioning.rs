use anyhow::{Context, Result, bail};
use regex::{Captures, Regex};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct VersionSpoof {
    pub version_name: Option<String>,
    pub technical_version_code: Option<u32>,
}

impl VersionSpoof {
    pub fn enabled(&self) -> bool {
        self.version_name.is_some() || self.technical_version_code.is_some()
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(name) = &self.version_name
            && (name.trim().is_empty() || name.contains(['\r', '\n']))
        {
            bail!("spoof versionName должен быть непустой строкой без переносов");
        }
        if self.technical_version_code == Some(0) {
            bail!("technical versionCode должен быть больше нуля");
        }
        Ok(())
    }

    pub fn validate_against_original(&self, original_version_code: u64) -> Result<()> {
        self.validate()?;
        if self
            .technical_version_code
            .is_some_and(|code| u64::from(code) < original_version_code)
        {
            bail!(
                "technical versionCode не может быть ниже исходного {original_version_code}: это создаёт downgrade"
            );
        }
        Ok(())
    }
}

fn replace_yaml_value(text: &str, key: &str, value: &str) -> Result<String> {
    let pattern = Regex::new(&format!(r"(?m)^(\s*{}:\s*).+?$", regex::escape(key)))?;
    if !pattern.is_match(text) {
        bail!("apktool.yml не содержит {key}");
    }
    Ok(pattern
        .replace(text, |captures: &Captures<'_>| {
            format!("{}{}", &captures[1], value)
        })
        .into_owned())
}

pub fn apply(decoded: &Path, spoof: &VersionSpoof) -> Result<()> {
    spoof.validate()?;
    if !spoof.enabled() {
        return Ok(());
    }
    let path = decoded.join("apktool.yml");
    let mut text = fs::read_to_string(&path)
        .with_context(|| format!("не удалось прочитать {}", path.display()))?;
    if let Some(name) = &spoof.version_name {
        text = replace_yaml_value(&text, "versionName", name)?;
    }
    if let Some(code) = spoof.technical_version_code {
        text = replace_yaml_value(&text, "versionCode", &code.to_string())?;
    }
    fs::write(&path, text).with_context(|| format!("не удалось записать {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_requested_yaml_value() {
        let input = "versionInfo:\n  versionCode: 42\n  versionName: release #1\n";
        let output = replace_yaml_value(input, "versionName", "custom").unwrap();
        assert!(output.contains("versionCode: 42"));
        assert!(output.contains("versionName: custom"));
    }

    #[test]
    fn rejects_multiline_version_name() {
        let spoof = VersionSpoof {
            version_name: Some("bad\nname".to_owned()),
            technical_version_code: None,
        };
        assert!(spoof.validate().is_err());
    }

    #[test]
    fn update_semantics_preserve_or_increase_version_code() {
        VersionSpoof::default()
            .validate_against_original(24_026_442)
            .unwrap();
        VersionSpoof {
            version_name: None,
            technical_version_code: Some(24_026_461),
        }
        .validate_against_original(24_026_442)
        .unwrap();
        assert!(
            VersionSpoof {
                version_name: None,
                technical_version_code: Some(24_026_441),
            }
            .validate_against_original(24_026_442)
            .is_err()
        );
    }
}
