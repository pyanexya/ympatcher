use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const PATCH_VERSION: &str = env!("CARGO_PKG_VERSION");
const ABOUT_MARKER: &str = "ympatcher:about-row:v0.5.0";
const DEV_MARKER: &str = "ympatcher:dev-experiments:v0.5.0";
const PASSPORT_MARKER: &str = "ympatcher:passport-signature:v0.5.0";

pub const PATCH_CATALOG: &[PatchDescriptor] = &[
    PatchDescriptor {
        id: "about",
        description: "сведения о ympatcher внутри клиента",
        default_enabled: true,
        dependencies: &[],
    },
    PatchDescriptor {
        id: "experiments",
        description: "каталог и локальные overrides feature flags",
        default_enabled: true,
        dependencies: &["about"],
    },
    PatchDescriptor {
        id: "session-compat",
        description: "совместимость Yandex Passport с постоянной подписью ympatcher",
        default_enabled: true,
        dependencies: &[],
    },
    PatchDescriptor {
        id: "discord-rpc",
        description: "Discord Rich Presence из MediaSession приложения",
        default_enabled: false,
        dependencies: &["about"],
    },
];

#[derive(Debug, Clone, Copy)]
pub struct PatchDescriptor {
    pub id: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
    pub dependencies: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct PatchSelection {
    pub about: bool,
    pub experiments: bool,
    pub session_compat: bool,
    pub discord_rpc: bool,
}

impl Default for PatchSelection {
    fn default() -> Self {
        Self {
            about: true,
            experiments: true,
            session_compat: true,
            discord_rpc: false,
        }
    }
}

impl PatchSelection {
    pub fn from_ids(ids: &[String]) -> Result<Self> {
        if ids.is_empty() {
            return Ok(Self::default());
        }
        let mut selection = Self {
            about: false,
            experiments: false,
            session_compat: false,
            discord_rpc: false,
        };
        for id in ids {
            match id.as_str() {
                "about" => selection.about = true,
                "experiments" => {
                    selection.experiments = true;
                    selection.about = true;
                }
                "session-compat" => selection.session_compat = true,
                "discord-rpc" => {
                    selection.discord_rpc = true;
                    selection.about = true;
                }
                _ => bail!("неизвестный patch id: {id}"),
            }
        }
        Ok(selection)
    }
}

#[derive(Debug)]
pub struct PatchSummary {
    pub renderer: Option<PathBuf>,
    pub experiments: usize,
    pub applied: Vec<&'static str>,
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("не удалось прочитать {}", path.display()))
}

fn write(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value.replace("\r\n", "\n"))
        .with_context(|| format!("не удалось записать {}", path.display()))
}

fn candidates(decoded: &Path, file_name: &str) -> Vec<PathBuf> {
    let relative = if file_name == "YMApplication.smali" {
        PathBuf::from("ru/yandex/music/YMApplication.smali")
    } else {
        PathBuf::from(file_name)
    };
    fs::read_dir(decoded)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
                && entry.file_name().to_string_lossy().starts_with("smali")
        })
        .map(|entry| entry.path().join(&relative))
        .filter(|path| path.is_file())
        .collect()
}

fn one_matching(
    decoded: &Path,
    file_name: &str,
    needle: &str,
    description: &str,
) -> Result<(PathBuf, String)> {
    let mut matches = Vec::new();
    for path in candidates(decoded, file_name) {
        let text = read(&path)?;
        if text.contains(needle) {
            matches.push((path, text));
        }
    }
    if matches.len() != 1 {
        bail!(
            "fingerprint {description}: ожидался один {file_name}, найдено {}",
            matches.len()
        );
    }
    Ok(matches.remove(0))
}

fn append_resource(path: &Path, entry: &str) -> Result<()> {
    let text = read(path)?;
    let updated = text.replacen("</resources>", &format!("    {entry}\n</resources>"), 1);
    if updated == text {
        bail!("некорректный resource XML: {}", path.display());
    }
    write(path, &updated)
}

fn allocate_public_id(public_xml: &Path, kind: &str) -> Result<u32> {
    let text = read(public_xml)?;
    let regex = Regex::new(&format!(
        r#"<public\s+type="{}"[^>]*id="0x([0-9a-fA-F]{{8}})""#,
        regex::escape(kind)
    ))?;
    let mut values = regex
        .captures_iter(&text)
        .filter_map(|capture| u32::from_str_radix(&capture[1], 16).ok())
        .collect::<Vec<_>>();
    values.sort_unstable();
    let last = values
        .last()
        .context(format!("в public.xml нет типа {kind}"))?;
    let candidate = last
        .checked_add(1)
        .context("закончился диапазон resource ID")?;
    if candidate >> 16 != last >> 16 {
        bail!("закончился диапазон resource ID для {kind}");
    }
    Ok(candidate)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RendererApi {
    Legacy,
    Compose161,
    Compose162,
}

fn about_row_legacy(drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    format!(
        r#"

    # {ABOUT_MARKER}
    sget-object v22, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v23, 0x{title_id:08x}
    invoke-virtual/range {{v22 .. v23}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v22
    sget-object v23, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v24, 0x{subtitle_id:08x}
    invoke-virtual/range {{v23 .. v24}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v23
    sget-object v24, Ldanger/OpenAbout;->a:Ldanger/OpenAbout;
    sget-object v25, Lhhi;->a:Lhhi;
    const v26, 0x{drawable_id:08x}
    invoke-static/range {{v26 .. v26}}, Ljava/lang/Integer;->valueOf(I)Ljava/lang/Integer;
    move-result-object v26
    move-object/from16 v27, v1
    const/16 v28, 0xc00
    const/16 v29, 0x0
    invoke-static/range {{v22 .. v29}}, Lj3g;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lkhi;Ljava/lang/Integer;Lkv5;II)V
"#
    )
}

fn about_row_compose161(drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    format!(
        r#"

    # {ABOUT_MARKER}
    sget-object v2, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v3, 0x{title_id:08x}
    invoke-virtual {{v2, v3}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v2
    sget-object v3, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v4, 0x{subtitle_id:08x}
    invoke-virtual {{v3, v4}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v3
    sget-object v4, Ldanger/OpenAbout;->a:Ldanger/OpenAbout;
    sget-object v5, Lzmi;->a:Lzmi;
    const v6, 0x{drawable_id:08x}
    invoke-static {{v6}}, Ljava/lang/Integer;->valueOf(I)Ljava/lang/Integer;
    move-result-object v6
    move-object v7, v0
    const/16 v8, 0xc00
    const/4 v9, 0x0
    invoke-static/range {{v2 .. v9}}, Lh2h;->c(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lcni;Ljava/lang/Integer;Ldx5;II)V
"#
    )
}

fn about_row_compose162(drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    format!(
        r#"

    # {ABOUT_MARKER}
    sget-object v2, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v3, 0x{title_id:08x}
    invoke-virtual {{v2, v3}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v2
    sget-object v3, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v4, 0x{subtitle_id:08x}
    invoke-virtual {{v3, v4}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v3
    sget-object v4, Ldanger/OpenAbout;->a:Ldanger/OpenAbout;
    move-object v5, v10
    const v6, 0x{drawable_id:08x}
    invoke-static {{v6}}, Ljava/lang/Integer;->valueOf(I)Ljava/lang/Integer;
    move-result-object v6
    move-object v7, v0
    const/16 v8, 0xc00
    const/16 v9, 0x10
    invoke-static/range {{v2 .. v9}}, Ln1e;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lv0j;Ljava/lang/Integer;Lx26;II)V
"#
    )
}

fn apply_about(
    decoded: &Path,
    version_name: &str,
    version_code: &str,
) -> Result<(PathBuf, u32, u32, u32, RendererApi)> {
    let (file_name, anchor, api) = if version_code == "24026442" {
        (
            "wee.smali",
            Regex::new(
                r#"(?s)const-string v2, "settings_about_button".{0,900}?invoke-static/range \{v2 \.\. v9\}, Ln1e;->a\(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lv0j;Ljava/lang/Integer;Lx26;II\)V"#,
            )?,
            RendererApi::Compose162,
        )
    } else if version_code == "24026431" {
        (
            "uc6.smali",
            Regex::new(
                r#"(?s)const-string v2, "settings_about_button".{0,1200}?invoke-static/range \{v2 \.\. v9\}, Lh2h;->c\(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lcni;Ljava/lang/Integer;Ldx5;II\)V"#,
            )?,
            RendererApi::Compose161,
        )
    } else {
        (
            "u2.smali",
            Regex::new(
                r#"(?s)const-string v15, "version_info".{0,4000}?invoke-static \{v3, v1, v2, v5\}, Lexj;->y\(ILkv5;Lkhi;Ljava/lang/String;\)V"#,
            )?,
            RendererApi::Legacy,
        )
    };
    let mut matches = Vec::new();
    for path in candidates(decoded, file_name) {
        let text = read(&path)?;
        if anchor.is_match(&text) {
            matches.push((path, text));
        }
    }
    if matches.len() != 1 {
        bail!(
            "fingerprint About: ожидался один renderer {file_name}, найдено {}",
            matches.len()
        );
    }
    let (renderer, text) = matches.remove(0);
    if text.contains(ABOUT_MARKER) {
        bail!("APK уже содержит ympatcher patch");
    }
    let public = decoded.join("res/values/public.xml");
    let info_icon_id = allocate_public_id(&public, "drawable")?;
    let science_icon_id = info_icon_id.checked_add(1).context("нет drawable ID")?;
    let string_id = allocate_public_id(&public, "string")?;
    let string_ids = [
        string_id,
        string_id.checked_add(1).context("нет string ID")?,
        string_id.checked_add(2).context("нет string ID")?,
        string_id.checked_add(3).context("нет string ID")?,
    ];
    let found = anchor
        .find(&text)
        .context("не найден anchor версии About")?;
    let mut patched = String::with_capacity(text.len() + 1000);
    patched.push_str(&text[..found.end()]);
    patched.push_str(&match api {
        RendererApi::Legacy => about_row_legacy(info_icon_id, string_ids[0], string_ids[1]),
        RendererApi::Compose161 => about_row_compose161(info_icon_id, string_ids[0], string_ids[1]),
        RendererApi::Compose162 => about_row_compose162(info_icon_id, string_ids[0], string_ids[1]),
    });
    patched.push_str(&text[found.end()..]);
    write(&renderer, &patched)?;

    append_resource(
        &public,
        &format!(
            r#"<public type="drawable" name="danger_info_icon" id="0x{info_icon_id:08x}" />
    <public type="drawable" name="danger_science_icon" id="0x{science_icon_id:08x}" />"#
        ),
    )?;
    for (name, id) in [
        ("danger_about_title", string_ids[0]),
        ("danger_about_subtitle", string_ids[1]),
        ("danger_about_experiments_title", string_ids[2]),
        ("danger_about_experiments_subtitle", string_ids[3]),
    ] {
        append_resource(
            &public,
            &format!(r#"<public type="string" name="{name}" id="0x{id:08x}" />"#),
        )?;
    }
    for (name, payload) in [
        (
            "AboutActivity.smali",
            include_str!("../payload/smali/AboutActivity.smali"),
        ),
        (
            "OpenAbout.smali",
            include_str!("../payload/smali/OpenAbout.smali"),
        ),
    ] {
        write(&decoded.join("smali/danger").join(name), payload)?;
    }
    write(
        &decoded.join("smali/danger/DevExperiments.smali"),
        include_str!("../payload/smali/DevExperiments.smali"),
    )?;
    copy_payload_resources(decoded)?;
    let manifest = decoded.join("AndroidManifest.xml");
    let manifest_text = read(&manifest)?;
    if manifest_text.contains("danger.AboutActivity") {
        bail!("AboutActivity уже зарегистрирована");
    }
    let activity = "        <activity android:exported=\"false\" android:name=\"danger.AboutActivity\" android:theme=\"@style/DangerBottomSheetTheme\" />\n    ";
    let patched_manifest =
        manifest_text.replacen("</application>", &format!("{activity}</application>"), 1);
    if patched_manifest == manifest_text {
        bail!("в AndroidManifest.xml не найден application");
    }
    write(&manifest, &patched_manifest)?;
    let (application, app_text) = one_matching(
        decoded,
        "YMApplication.smali",
        ".method public final onCreate()V",
        "YMApplication",
    )?;
    let app_anchor = [
        "invoke-super {v0}, Landroid/app/Application;->onCreate()V",
        "invoke-super {v1}, Landroid/app/Application;->onCreate()V",
    ]
    .into_iter()
    .find(|anchor| app_text.contains(anchor))
    .context("не найден Application.onCreate anchor")?;
    let position = app_text
        .find(app_anchor)
        .context("не найден Application.onCreate anchor")?
        + app_anchor.len();
    let injection = "\n\n    invoke-static/range {p0 .. p0}, Ldanger/DevExperiments;->init(Landroid/content/Context;)V";
    write(
        &application,
        &format!(
            "{}{}{}",
            &app_text[..position],
            injection,
            &app_text[position..]
        ),
    )?;
    let metadata = json!({
        "patchVersion": PATCH_VERSION,
        "baseVersionName": version_name,
        "baseVersionCode": version_code,
        "author": "Pyanexya aka Pyanexy",
        "github": "pyanexu",
        "ui": "localized-adaptive-bottom-sheet"
    });
    write(
        &decoded.join("assets/danger-patch.json"),
        &(serde_json::to_string_pretty(&metadata)? + "\n"),
    )?;
    Ok((renderer, science_icon_id, string_ids[2], string_ids[3], api))
}

fn extract_experiments(decoded: &Path, api: RendererApi) -> Result<Vec<String>> {
    let string = Regex::new(r#"const-string\s+(v\d+),\s+"([^"]+)""#)?;
    let valid = Regex::new(r#"(?i)^android[a-z0-9_]{3,}$"#)?;
    let mut names = BTreeSet::new();
    let registry_classes = Regex::new(r"new-instance\s+v\d+,\s+L([A-Za-z0-9_$]+);")?;
    let mut referenced_classes = BTreeSet::new();
    let registry_file = match api {
        RendererApi::Legacy => "yu0.smali",
        RendererApi::Compose161 => "cv0.smali",
        RendererApi::Compose162 => "kw0.smali",
    };
    let registries = candidates(decoded, registry_file);
    for path in &registries {
        let registry = read(path)?;
        for capture in string.captures_iter(&registry) {
            let value = &capture[2];
            if valid.is_match(value) {
                names.insert(value.to_owned());
            }
            let after = &registry[capture.get(0).unwrap().end()..];
            let nearby = &after[..after.len().min(280)];
            let constructor = Regex::new(&format!(
                r"invoke-direct\s+\{{[^}}]*\b{}\b[^}}]*\}},\s+L(?:m0c|xq7);-><init>",
                regex::escape(&capture[1])
            ))?;
            if constructor.is_match(nearby) {
                names.insert(value.to_owned());
            }
        }
        for capture in registry_classes.captures_iter(&registry) {
            referenced_classes.insert(capture[1].to_owned());
        }
    }
    let roots = fs::read_dir(decoded)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
                && entry.file_name().to_string_lossy().starts_with("smali")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    for class_name in referenced_classes {
        let Some(path) = roots
            .iter()
            .map(|root| root.join(format!("{class_name}.smali")))
            .find(|path| path.is_file())
        else {
            continue;
        };
        let text = read(&path)?;
        if api != RendererApi::Compose162
            && !(text.contains(".super Lm0c;") || text.contains(".super Lxq7;"))
        {
            continue;
        }
        for capture in string.captures_iter(&text) {
            let value = &capture[2];
            if valid.is_match(value) {
                names.insert(value.to_owned());
            }
        }
    }
    if names.len() < 100 {
        bail!(
            "каталог Experiment SDK подозрительно мал: {} имён",
            names.len()
        );
    }
    Ok(names.into_iter().collect())
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn copy_payload_resources(decoded: &Path) -> Result<()> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("payload/res");
    for entry in WalkDir::new(&source).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(&source)?;
        let destination = decoded.join("res").join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(entry.path(), destination)?;
    }
    Ok(())
}

fn dev_row_legacy(drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    format!(
        r#"

    # {DEV_MARKER}
    sget-object v22, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v23, 0x{title_id:08x}
    invoke-virtual/range {{v22 .. v23}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v22
    sget-object v23, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v24, 0x{subtitle_id:08x}
    invoke-virtual/range {{v23 .. v24}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v23
    sget-object v24, Ldanger/OpenDevExperiments;->a:Ldanger/OpenDevExperiments;
    sget-object v25, Lhhi;->a:Lhhi;
    const v26, 0x{drawable_id:08x}
    invoke-static/range {{v26 .. v26}}, Ljava/lang/Integer;->valueOf(I)Ljava/lang/Integer;
    move-result-object v26
    move-object/from16 v27, v1
    const/16 v28, 0xc00
    const/16 v29, 0x0
    invoke-static/range {{v22 .. v29}}, Lj3g;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lkhi;Ljava/lang/Integer;Lkv5;II)V
"#
    )
}

fn dev_row_compose161(drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    format!(
        r#"

    # {DEV_MARKER}
    sget-object v2, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v3, 0x{title_id:08x}
    invoke-virtual {{v2, v3}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v2
    sget-object v3, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v4, 0x{subtitle_id:08x}
    invoke-virtual {{v3, v4}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v3
    sget-object v4, Ldanger/OpenDevExperiments;->a:Ldanger/OpenDevExperiments;
    sget-object v5, Lzmi;->a:Lzmi;
    const v6, 0x{drawable_id:08x}
    invoke-static {{v6}}, Ljava/lang/Integer;->valueOf(I)Ljava/lang/Integer;
    move-result-object v6
    move-object v7, v0
    const/16 v8, 0xc00
    const/4 v9, 0x0
    invoke-static/range {{v2 .. v9}}, Lh2h;->c(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lcni;Ljava/lang/Integer;Ldx5;II)V
"#
    )
}

fn dev_row_compose162(drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    format!(
        r#"

    # {DEV_MARKER}
    sget-object v2, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v3, 0x{title_id:08x}
    invoke-virtual {{v2, v3}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v2
    sget-object v3, Ldanger/DevExperiments;->a:Landroid/content/Context;
    const v4, 0x{subtitle_id:08x}
    invoke-virtual {{v3, v4}}, Landroid/content/Context;->getString(I)Ljava/lang/String;
    move-result-object v3
    sget-object v4, Ldanger/OpenDevExperiments;->a:Ldanger/OpenDevExperiments;
    move-object v5, v10
    const v6, 0x{drawable_id:08x}
    invoke-static {{v6}}, Ljava/lang/Integer;->valueOf(I)Ljava/lang/Integer;
    move-result-object v6
    move-object v7, v0
    const/16 v8, 0xc00
    const/16 v9, 0x10
    invoke-static/range {{v2 .. v9}}, Ln1e;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lv0j;Ljava/lang/Integer;Lx26;II)V
"#
    )
}

fn presence_row(api: RendererApi, drawable_id: u32, title_id: u32, subtitle_id: u32) -> String {
    match api {
        RendererApi::Legacy => about_row_legacy(drawable_id, title_id, subtitle_id)
            .replace(ABOUT_MARKER, "ympatcher:discord-rpc:v0.5.0")
            .replace("Ldanger/OpenAbout;", "Ldanger/OpenDiscordPresence;"),
        RendererApi::Compose161 => about_row_compose161(drawable_id, title_id, subtitle_id)
            .replace(ABOUT_MARKER, "ympatcher:discord-rpc:v0.5.0")
            .replace("Ldanger/OpenAbout;", "Ldanger/OpenDiscordPresence;"),
        RendererApi::Compose162 => about_row_compose162(drawable_id, title_id, subtitle_id)
            .replace(ABOUT_MARKER, "ympatcher:discord-rpc:v0.5.0")
            .replace("Ldanger/OpenAbout;", "Ldanger/OpenDiscordPresence;"),
    }
}

fn apply_discord_row(decoded: &Path, renderer: &Path, api: RendererApi) -> Result<()> {
    let public = decoded.join("res/values/public.xml");
    let icon_id = allocate_public_id(&public, "drawable")?;
    let title_id = allocate_public_id(&public, "string")?;
    let subtitle_id = title_id.checked_add(1).context("нет string ID")?;
    append_resource(
        &public,
        &format!(r#"<public type="drawable" name="danger_discord_icon" id="0x{icon_id:08x}" />"#,),
    )?;
    for (name, id) in [
        ("danger_presence_title", title_id),
        ("danger_presence_subtitle", subtitle_id),
    ] {
        append_resource(
            &public,
            &format!(r#"<public type="string" name="{name}" id="0x{id:08x}" />"#),
        )?;
    }
    write(
        &decoded.join("smali/danger/OpenDiscordPresence.smali"),
        include_str!("../payload/smali/OpenDiscordPresence.smali"),
    )?;
    let text = read(renderer)?;
    let marker = if text.contains(DEV_MARKER) {
        DEV_MARKER
    } else {
        ABOUT_MARKER
    };
    let marker_at = text.find(marker).context("не найден settings row marker")?;
    let call = match api {
        RendererApi::Legacy => {
            "invoke-static/range {v22 .. v29}, Lj3g;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lkhi;Ljava/lang/Integer;Lkv5;II)V"
        }
        RendererApi::Compose161 => {
            "invoke-static/range {v2 .. v9}, Lh2h;->c(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lcni;Ljava/lang/Integer;Ldx5;II)V"
        }
        RendererApi::Compose162 => {
            "invoke-static/range {v2 .. v9}, Ln1e;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lv0j;Ljava/lang/Integer;Lx26;II)V"
        }
    };
    let end = text[marker_at..]
        .find(call)
        .context("не найден конец settings row")?
        + marker_at
        + call.len();
    write(
        renderer,
        &format!(
            "{}{}{}",
            &text[..end],
            presence_row(api, icon_id, title_id, subtitle_id),
            &text[end..]
        ),
    )
}

fn apply_dev_experiments(
    decoded: &Path,
    renderer: &Path,
    drawable_id: u32,
    title_id: u32,
    subtitle_id: u32,
    api: RendererApi,
) -> Result<usize> {
    let experiments = extract_experiments(decoded, api)?;
    let (lookup_file, lookup_fingerprint) = match api {
        RendererApi::Legacy => (
            "f1c.smali",
            ".method public static h(Lf1c;Ljava/lang/String;)Ljava/lang/String;",
        ),
        RendererApi::Compose161 => (
            "l4c.smali",
            ".method public static h(Ll4c;Ljava/lang/String;)Ljava/lang/String;",
        ),
        RendererApi::Compose162 => (
            "iec.smali",
            ".method public static h(Liec;Ljava/lang/String;)Ljava/lang/String;",
        ),
    };
    let (lookup, lookup_text) =
        one_matching(decoded, lookup_file, lookup_fingerprint, "Experiment SDK")?;
    let lookup_anchor = "invoke-virtual {p1}, Ljava/lang/Object;->getClass()Ljava/lang/Class;";
    let position = lookup_text
        .find(lookup_anchor)
        .context("не найден lookup anchor")?
        + lookup_anchor.len();
    let injection = "\n\n    invoke-static {p1}, Ldanger/DevExperiments;->getOverride(Ljava/lang/String;)Ljava/lang/String;\n    move-result-object v0\n    if-eqz v0, :danger_server_experiment\n    return-object v0\n    :danger_server_experiment\n";
    let patched = format!(
        "{}{}{}",
        &lookup_text[..position],
        injection,
        &lookup_text[position..]
    );
    write(&lookup, &patched)?;

    let renderer_text = read(renderer)?;
    let marker = renderer_text
        .find(ABOUT_MARKER)
        .context("не найден About marker")?;
    let call = match api {
        RendererApi::Legacy => {
            "invoke-static/range {v22 .. v29}, Lj3g;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lkhi;Ljava/lang/Integer;Lkv5;II)V"
        }
        RendererApi::Compose161 => {
            "invoke-static/range {v2 .. v9}, Lh2h;->c(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lcni;Ljava/lang/Integer;Ldx5;II)V"
        }
        RendererApi::Compose162 => {
            "invoke-static/range {v2 .. v9}, Ln1e;->a(Ljava/lang/String;Ljava/lang/String;Lkotlin/jvm/functions/Function0;Lv0j;Ljava/lang/Integer;Lx26;II)V"
        }
    };
    let end = renderer_text[marker..]
        .find(call)
        .context("не найден конец About row")?
        + marker
        + call.len();
    write(
        renderer,
        &format!(
            "{}{}{}",
            &renderer_text[..end],
            match api {
                RendererApi::Legacy => dev_row_legacy(drawable_id, title_id, subtitle_id),
                RendererApi::Compose161 => {
                    dev_row_compose161(drawable_id, title_id, subtitle_id)
                }
                RendererApi::Compose162 => {
                    dev_row_compose162(drawable_id, title_id, subtitle_id)
                }
            },
            &renderer_text[end..]
        ),
    )?;

    let items = experiments
        .iter()
        .map(|name| format!("        <item>{}</item>", escape_xml(name)))
        .collect::<Vec<_>>()
        .join("\n");
    let array_xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <string-array name=\"danger_experiment_names\">\n{items}\n    </string-array>\n</resources>\n"
    );
    write(
        &decoded.join("res/values/danger_experiment_names.xml"),
        &array_xml,
    )?;

    for (name, payload) in [
        (
            "DevExperimentsActivity.smali",
            include_str!("../payload/smali/DevExperimentsActivity.smali"),
        ),
        (
            "ChoiceClick.smali",
            include_str!("../payload/smali/ChoiceClick.smali"),
        ),
        (
            "OpenDevExperiments.smali",
            include_str!("../payload/smali/OpenDevExperiments.smali"),
        ),
    ] {
        write(&decoded.join("smali/danger").join(name), payload)?;
    }

    let manifest = decoded.join("AndroidManifest.xml");
    let text = read(&manifest)?;
    if text.contains("danger.DevExperimentsActivity") {
        bail!("DevExperimentsActivity уже зарегистрирована");
    }
    let activity = "        <activity android:exported=\"false\" android:name=\"danger.DevExperimentsActivity\" android:theme=\"@style/DangerBottomSheetTheme\" />\n    ";
    let patched = text.replacen("</application>", &format!("{activity}</application>"), 1);
    write(&manifest, &patched)?;
    Ok(experiments.len())
}

fn apply_passport(decoded: &Path, version_code: &str) -> Result<()> {
    let file_name = match version_code {
        "24026442" => "dx0.smali",
        "24026431" => "f6v.smali",
        _ => "vjw.smali",
    };
    let (path, text) = one_matching(
        decoded,
        file_name,
        "Internal error, application signature mismatch",
        "Yandex Passport signature",
    )?;
    if text.contains(PASSPORT_MARKER) {
        bail!("APK уже содержит Passport signature patch");
    }
    let error = text
        .find("Internal error, application signature mismatch")
        .context("не найден Passport error")?;
    let labels_re = Regex::new(r"(?m)\n\s*(:cond_[^\n]+)\n\s*(:goto_[^\n]+)\n")?;
    let entry = labels_re
        .find_iter(&text[..error])
        .last()
        .context("не найден вход в Passport error branch")?;
    let unit_offset = text[error..]
        .find("Lkotlin/Unit;->a:Lkotlin/Unit;")
        .context("не найдена Passport success branch")?
        + error;
    let success = text[..unit_offset]
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with(':'))
        .context("перед Passport success branch отсутствует label")?;
    let labels = &text[entry.start()..entry.end()];
    let insertion = format!("{labels}    # {PASSPORT_MARKER}\n    goto {success}\n\n");
    write(
        &path,
        &format!(
            "{}{}{}",
            &text[..entry.start()],
            insertion,
            &text[entry.end()..]
        ),
    )?;
    Ok(())
}

pub fn apply_all(
    decoded: &Path,
    version_name: &str,
    version_code: &str,
    selection: PatchSelection,
) -> Result<PatchSummary> {
    let mut renderer = None;
    let mut experiments = 0;
    let mut applied = Vec::new();
    if selection.about {
        let (path, drawable_id, dev_title_id, dev_subtitle_id, api) =
            apply_about(decoded, version_name, version_code)?;
        applied.push("about");
        if selection.experiments {
            experiments = apply_dev_experiments(
                decoded,
                &path,
                drawable_id,
                dev_title_id,
                dev_subtitle_id,
                api,
            )?;
            applied.push("experiments");
        }
        if selection.discord_rpc {
            apply_discord_row(decoded, &path, api)?;
            applied.push("discord-rpc");
        }
        renderer = Some(path);
    }
    if selection.session_compat {
        apply_passport(decoded, version_code)?;
        applied.push("session-compat");
    }
    Ok(PatchSummary {
        renderer,
        experiments,
        applied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_experiment_names() {
        assert_eq!(escape_xml("a&b<c>"), "a&amp;b&lt;c&gt;");
    }

    #[test]
    fn experiment_selection_enables_about_dependency() {
        let selection = PatchSelection::from_ids(&["experiments".to_owned()]).unwrap();
        assert!(selection.about);
        assert!(selection.experiments);
        assert!(!selection.session_compat);
    }

    #[test]
    fn payload_contains_bottom_sheet_and_search() {
        let activity = include_str!("../payload/smali/DevExperimentsActivity.smali");
        assert!(activity.contains("OnTouchListener"));
        assert!(activity.contains("afterTextChanged"));
        assert!(activity.contains("resetDefaults"));
        assert!(
            include_str!("../payload/smali/AboutActivity.smali").contains("pyanexya/ympatcher")
        );
        let sheet = include_str!("../payload/res/layout/danger_dev_sheet.xml");
        assert!(sheet.contains("@string/danger_search_hint"));
        assert!(sheet.contains("@+id/danger_reset"));
        assert!(
            include_str!("../payload/res/values-ru/danger_strings.xml").contains("Эксперименты")
        );
        assert!(
            include_str!("../payload/res/values-night/danger_colors.xml")
                .contains("danger_background")
        );
        assert!(
            include_str!("../payload/res/drawable/danger_science_icon.xml")
                .contains("viewportWidth=\"960\"")
        );
    }
}
