use crate::dataminer::core::DatamineReport;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!(
        "{}.part",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp")
    ));
    fs::write(&temporary, contents)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn write(path: &Path, report: &DatamineReport) -> Result<()> {
    let mut contents = serde_json::to_vec_pretty(report)?;
    contents.push(b'\n');
    atomic_write(path, &contents)
}
