use crate::process::run;
use crate::tools::Toolchain;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipArchive;

const STRING_NAMES: &[&str] = &[
    "ymp_presence_activity_type",
    "ymp_presence_artist_first",
    "ymp_presence_artwork",
    "ymp_presence_display",
    "ymp_presence_enable",
    "ymp_presence_link",
    "ymp_presence_listening",
    "ymp_presence_paused",
    "ymp_presence_playing",
    "ymp_presence_progress",
    "ymp_presence_summary",
    "ymp_presence_title",
    "ymp_presence_watching",
];

pub fn inject(
    decoded: &Path,
    embedded_aar: &Path,
    discord_sdk_aar: &Path,
    tools: &Toolchain,
) -> Result<()> {
    if !embedded_aar.is_file() {
        bail!(
            "Discord RPC payload AAR не найден: {}",
            embedded_aar.display()
        );
    }
    if !discord_sdk_aar.is_file() {
        bail!(
            "Discord Social SDK AAR не найден: {}",
            discord_sdk_aar.display()
        );
    }
    let temporary = tempfile::Builder::new()
        .prefix("ympatch-presence-")
        .tempdir()?;
    let embedded = temporary.path().join("embedded");
    let sdk = temporary.path().join("sdk");
    extract_aar(embedded_aar, &embedded)?;
    extract_aar(discord_sdk_aar, &sdk)?;
    let embedded_classes = embedded.join("classes.jar");
    let sdk_classes = sdk.join("libs/discord_partner_sdk.jar");
    if !embedded_classes.is_file() || !sdk_classes.is_file() {
        bail!("Discord AAR не содержит обязательные classes.jar");
    }

    let dex_output = temporary.path().join("dex");
    fs::create_dir_all(&dex_output)?;
    let args: Vec<OsString> = vec![
        "-cp".into(),
        tools.r8.as_os_str().to_owned(),
        "com.android.tools.r8.D8".into(),
        "--min-api".into(),
        "24".into(),
        "--output".into(),
        dex_output.as_os_str().to_owned(),
        embedded_classes.as_os_str().to_owned(),
        sdk_classes.as_os_str().to_owned(),
    ];
    run(&tools.java, args).context("D8 не смог собрать Discord RPC classes")?;
    let dex = dex_output.join("classes.dex");
    let dex_name = next_dex_name(decoded)?;
    fs::copy(&dex, decoded.join(dex_name))?;

    copy_native_libraries(&embedded.join("jni"), &decoded.join("lib"))?;
    merge_resources(decoded, &embedded)?;
    merge_manifest(decoded)?;
    hook_activity(decoded)?;
    hook_media_session(decoded)?;
    Ok(())
}

fn extract_aar(input: &Path, output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    let mut archive = ZipArchive::new(File::open(input)?).context("AAR повреждён")?;
    crate::apk::validate_archive(&mut archive)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let enclosed = entry
            .enclosed_name()
            .context("AAR содержит небезопасный путь")?;
        let destination = output.join(enclosed);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut writer = File::create(destination)?;
        io::copy(&mut entry, &mut writer)?;
    }
    Ok(())
}

fn next_dex_name(decoded: &Path) -> Result<String> {
    let mut highest = 1_u32;
    for entry in fs::read_dir(decoded)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "smali" || name == "classes.dex" {
            highest = highest.max(1);
        } else if let Some(value) = name
            .strip_prefix("smali_classes")
            .or_else(|| {
                name.strip_prefix("classes")
                    .and_then(|v| v.strip_suffix(".dex"))
            })
            .and_then(|value| value.parse::<u32>().ok())
        {
            highest = highest.max(value);
        }
    }
    Ok(format!("classes{}.dex", highest + 1))
}

fn copy_native_libraries(source: &Path, destination: &Path) -> Result<()> {
    for entry in WalkDir::new(source).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(source)?;
        let output = destination.join(relative);
        if output.exists() {
            let source_hash = crate::tools::sha256_file(entry.path())?;
            let output_hash = crate::tools::sha256_file(&output)?;
            if source_hash != output_hash {
                bail!("native library conflict: {}", output.display());
            }
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(entry.path(), output)?;
    }
    Ok(())
}

fn allocate_string_ids(public_xml: &Path, count: usize) -> Result<Vec<u32>> {
    let text = fs::read_to_string(public_xml)?;
    let regex = Regex::new(r#"<public\s+type="string"[^>]*id="0x([0-9a-fA-F]{8})""#)?;
    let last = regex
        .captures_iter(&text)
        .filter_map(|capture| u32::from_str_radix(&capture[1], 16).ok())
        .max()
        .context("public.xml не содержит string IDs")?;
    (1..=count)
        .map(|offset| {
            last.checked_add(offset as u32)
                .context("закончился диапазон string IDs")
        })
        .collect()
}

fn merge_resources(decoded: &Path, embedded: &Path) -> Result<()> {
    let public_path = decoded.join("res/values/public.xml");
    let ids = allocate_string_ids(&public_path, STRING_NAMES.len())?;
    let mut public = fs::read_to_string(&public_path)?;
    let entries = STRING_NAMES
        .iter()
        .zip(&ids)
        .map(|(name, id)| format!(r#"    <public type="string" name="{name}" id="0x{id:08x}" />"#))
        .collect::<Vec<_>>()
        .join("\n");
    public = public.replacen("</resources>", &format!("{entries}\n</resources>"), 1);
    fs::write(&public_path, public)?;
    fs::copy(
        embedded.join("res/values/values.xml"),
        decoded.join("res/values/ympresence.xml"),
    )?;
    let ru = embedded.join("res/values-ru/values-ru.xml");
    if ru.is_file() {
        fs::create_dir_all(decoded.join("res/values-ru"))?;
        fs::copy(ru, decoded.join("res/values-ru/ympresence.xml"))?;
    }
    let fields = STRING_NAMES
        .iter()
        .zip(&ids)
        .map(|(name, id)| format!(".field public static final {name}:I = 0x{id:08x}"))
        .collect::<Vec<_>>()
        .join("\n");
    let r_string = format!(
        ".class public final Ldev/pyanexy/ympresence/R$string;\n.super Ljava/lang/Object;\n.source \"Ympatcher\"\n\n{fields}\n"
    );
    let path = decoded.join("smali/dev/pyanexy/ympresence/R$string.smali");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, r_string)?;
    Ok(())
}

fn merge_manifest(decoded: &Path) -> Result<()> {
    let path = decoded.join("AndroidManifest.xml");
    let mut manifest = fs::read_to_string(&path)?;
    for permission in [
        "android.permission.INTERNET",
        "android.permission.FOREGROUND_SERVICE",
        "android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK",
    ] {
        if !manifest.contains(&format!("android:name=\"{permission}\"")) {
            manifest = manifest.replacen(
                "<application ",
                &format!("<uses-permission android:name=\"{permission}\"/>\n    <application "),
                1,
            );
        }
    }
    if !manifest.contains("com.discord.socialsdk.rpc.IDiscordRpcService") {
        let queries = "    <queries><intent><action android:name=\"com.discord.socialsdk.rpc.IDiscordRpcService\"/></intent><package android:name=\"com.discord\"/></queries>\n    ";
        manifest = manifest.replacen("<application ", &format!("{queries}<application "), 1);
    }
    if !manifest.contains("dev.pyanexy.ympresence.DiscordPresenceActivity") {
        let components = concat!(
            "        <activity android:exported=\"false\" android:name=\"dev.pyanexy.ympresence.DiscordPresenceActivity\" android:theme=\"@style/DangerBottomSheetTheme\"/>\n",
            "        <activity android:exported=\"true\" android:launchMode=\"singleTask\" android:name=\"com.discord.socialsdk.AuthenticationActivity\" android:theme=\"@android:style/Theme.Translucent.NoTitleBar\"/>\n",
            "        <service android:enabled=\"true\" android:exported=\"false\" android:foregroundServiceType=\"mediaPlayback\" android:name=\"com.discord.socialsdk.ForegroundService\"/>\n    "
        );
        manifest = manifest.replacen("</application>", &format!("{components}</application>"), 1);
    }
    fs::write(path, manifest)?;
    Ok(())
}

fn find_one(decoded: &Path, suffix: &str) -> Result<PathBuf> {
    let matches = WalkDir::new(decoded)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .to_string_lossy()
                .replace('\\', "/")
                .ends_with(suffix)
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "Discord RPC fingerprint {suffix}: найдено {}",
            matches.len()
        );
    }
    Ok(matches[0].clone())
}

fn hook_activity(decoded: &Path) -> Result<()> {
    let path = find_one(decoded, "/ru/yandex/music/main/MainScreenActivity.smali")?;
    let text = fs::read_to_string(&path)?;
    let anchor = "invoke-super {p0}, Lh3m;->onResume()V";
    if !text.contains(anchor) {
        bail!("Discord RPC: не найден MainScreenActivity.onResume fingerprint");
    }
    let replacement = format!(
        "{anchor}\n\n    # ympatcher:discord-rpc-activity:v0.5.0\n    invoke-static {{p0}}, Ldev/pyanexy/ympresence/EmbeddedPresence;->onActivity(Landroid/app/Activity;)V"
    );
    fs::write(path, text.replacen(anchor, &replacement, 1))?;
    Ok(())
}

fn hook_media_session(decoded: &Path) -> Result<()> {
    let path = find_one(
        decoded,
        "/androidx/media3/session/MediaNotificationManager.smali",
    )?;
    let text = fs::read_to_string(&path)?;
    let anchor = "check-cast p1, Landroid/media/session/MediaSession$Token;";
    if text.matches(anchor).count() != 1 {
        bail!("Discord RPC: MediaSession token fingerprint неоднозначен");
    }
    let replacement = format!(
        "{anchor}\n\n    # ympatcher:discord-rpc-session:v0.5.0\n    iget-object v0, p0, Landroidx/media3/session/MediaNotificationManager;->a:Landroidx/media3/session/MediaSessionService;\n    invoke-static {{v0, p1}}, Ldev/pyanexy/ympresence/EmbeddedPresence;->attach(Landroid/content/Context;Landroid/media/session/MediaSession$Token;)V"
    );
    fs::write(path, text.replacen(anchor, &replacement, 1))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_dex_does_not_overwrite_existing_sources() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("smali")).unwrap();
        fs::create_dir(directory.path().join("smali_classes6")).unwrap();
        assert_eq!(next_dex_name(directory.path()).unwrap(), "classes7.dex");
    }
}
