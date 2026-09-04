use anyhow::{Context, Result, bail};
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

fn redact(mut text: String, secrets: &[&str]) -> String {
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        text = text.replace(secret, "***");
    }
    text
}

pub fn run<I, S>(program: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_redacted(program, args, &[])
}

pub fn run_redacted<I, S>(program: &Path, args: I, secrets: &[&str]) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("не удалось запустить {}", program.display()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text = redact(text, secrets);
    if !output.status.success() {
        let tail = if text.len() > 12_000 {
            let mut start = text.len() - 12_000;
            while !text.is_char_boundary(start) {
                start += 1;
            }
            &text[start..]
        } else {
            &text
        };
        bail!(
            "команда {} завершилась с кодом {:?}\n{}",
            program.display(),
            output.status.code(),
            tail.trim()
        );
    }
    Ok(text)
}

pub fn find_command(name: &str) -> Option<std::path::PathBuf> {
    let candidate = Path::new(name);
    if candidate.is_file() {
        return Some(candidate.to_owned());
    }
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        #[cfg(windows)]
        let names = [
            format!("{name}.exe"),
            format!("{name}.cmd"),
            name.to_owned(),
        ];
        #[cfg(not(windows))]
        let names = [name.to_owned(), name.to_owned(), name.to_owned()];
        for file in names {
            let resolved = directory.join(file);
            if resolved.is_file() {
                return Some(resolved);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn redaction_logic_does_not_leave_secret_in_logs() {
        let secret = "webhook-or-password";
        let output = super::redact(format!("tool failed with {secret}"), &[secret]);
        assert_eq!(output, "tool failed with ***");
        assert!(!output.contains(secret));
    }
}
