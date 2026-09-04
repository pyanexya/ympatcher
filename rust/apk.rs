//! APK ZIP layout is finalized before signing, then checked again after signing.
use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::fs::File;
use std::io;
use std::path::Path;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Validate before extracting, including Windows path aliases and ZIP bombs.
pub fn validate_archive(archive: &mut ZipArchive<File>) -> Result<()> {
    if archive.len() > 100_000 {
        bail!("слишком много ZIP entries");
    }
    let mut names = BTreeSet::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().trim_end_matches('/');
        validate_zip_path(name)?;
        if !names.insert(name.to_ascii_lowercase()) {
            bail!("повторяющийся ZIP path: {name}");
        }
        if entry.is_symlink() {
            bail!("символические ссылки в ZIP не поддерживаются: {name}");
        }
        total = total
            .checked_add(entry.size())
            .context("ZIP size overflow")?;
        if entry.size() > MAX_ENTRY_BYTES || total > MAX_ARCHIVE_BYTES {
            bail!("ZIP превышает лимит распаковки (2 GiB/file, 4 GiB/archive)");
        }
    }
    Ok(())
}

pub fn validate_zip_path(name: &str) -> Result<()> {
    if name.is_empty() || name.contains(['\\', ':', '\0']) {
        bail!("небезопасный ZIP path: {name:?}");
    }
    for part in name.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            bail!("небезопасный ZIP path: {name:?}");
        }
    }
    Ok(())
}

fn signature_entry(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.strip_prefix("META-INF/").is_some_and(|rest| {
        !rest.contains('/')
            && (rest == "MANIFEST.MF"
                || rest.starts_with("SIG-")
                || [".SF", ".RSA", ".DSA", ".EC"]
                    .iter()
                    .any(|ext| rest.ends_with(ext)))
    })
}

pub fn normalize(apk: &Path) -> Result<()> {
    let mut input = ZipArchive::new(File::open(apk)?)?;
    validate_archive(&mut input)?;
    let parent = apk
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut output = ZipWriter::new(temporary.reopen()?);
    for index in 0..input.len() {
        let mut entry = input.by_index(index)?;
        if entry.is_dir() || signature_entry(entry.name()) {
            continue;
        }
        let native = entry.name().ends_with(".so");
        let stored = native
            || entry.name() == "resources.arsc"
            || entry.compression() == CompressionMethod::Stored;
        if !stored {
            output.raw_copy_file(entry)?;
            continue;
        }
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .with_alignment(if native { 16384 } else { 4 })
            .unix_permissions(0o644);
        output.start_file(entry.name(), options)?;
        io::copy(&mut entry, &mut output)?;
    }
    output.finish()?.sync_all()?;
    drop(input);
    verify_layout(temporary.path())?;
    temporary
        .persist(apk)
        .context("не удалось заменить APK после нормализации")?;
    Ok(())
}

pub fn verify_layout(apk: &Path) -> Result<usize> {
    let mut archive = ZipArchive::new(File::open(apk)?)?;
    validate_archive(&mut archive)?;
    archive
        .by_name("AndroidManifest.xml")
        .context("APK не содержит manifest")?;
    let mut libraries = 0;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let native = entry.name().ends_with(".so");
        if native {
            libraries += 1;
        }
        if (native || entry.name() == "resources.arsc")
            && entry.compression() != CompressionMethod::Stored
        {
            bail!("{} должен храниться без сжатия", entry.name());
        }
        let alignment = if native { 16384 } else { 4 };
        if entry.compression() == CompressionMethod::Stored && entry.data_start() % alignment != 0 {
            bail!("{}: нарушено выравнивание {alignment} bytes", entry.name());
        }
    }
    Ok(libraries)
}

/// Publish only a completed result; a failed build must preserve the old output.
pub fn publish(source: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    io::copy(&mut File::open(source)?, &mut temporary)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(destination)
        .context("не удалось сохранить результат")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn normalizes_native_libs_and_resources_without_changing_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.apk");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        for name in [
            "AndroidManifest.xml",
            "resources.arsc",
            "lib/arm64-v8a/test.so",
            "classes.dex",
            "META-INF/CERT.RSA",
            "META-INF/services/example",
        ] {
            zip.start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
            zip.write_all(name.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        assert!(verify_layout(&path).is_err());
        normalize(&path).unwrap();
        assert_eq!(verify_layout(&path).unwrap(), 1);
        let mut zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        assert!(zip.by_name("META-INF/CERT.RSA").is_err());
        for name in [
            "AndroidManifest.xml",
            "resources.arsc",
            "lib/arm64-v8a/test.so",
            "classes.dex",
            "META-INF/services/example",
        ] {
            let mut contents = String::new();
            zip.by_name(name)
                .unwrap()
                .read_to_string(&mut contents)
                .unwrap();
            assert_eq!(contents, name);
        }
        drop(zip);
        let first = std::fs::read(&path).unwrap();
        normalize(&path).unwrap();
        assert_eq!(first, std::fs::read(&path).unwrap());
    }

    #[test]
    fn rejects_windows_aliases_and_traversal() {
        for name in [
            "../a",
            "/a",
            "a/../b",
            "a\\b",
            "C:/a",
            "a:stream",
            "NUL.apk",
            "a/COM1.txt",
            "a./b",
            "a//b",
        ] {
            assert!(validate_zip_path(name).is_err(), "{name}");
        }
        assert!(validate_zip_path("splits/base-master.apk").is_ok());
    }

    #[test]
    fn invalid_apk_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.apk");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        zip.start_file("classes.dex", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"dex").unwrap();
        zip.finish().unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(normalize(&path).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
    }

    #[test]
    fn replaces_output_only_after_successful_copy() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output.apk");
        std::fs::write(&output, "old").unwrap();
        assert!(publish(&dir.path().join("missing"), &output).is_err());
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "old");
        let source = dir.path().join("source.apk");
        std::fs::write(&source, "new").unwrap();
        publish(&source, &output).unwrap();
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "new");
    }
}
