use super::json::atomic_write;
use crate::channel::ReleaseChannel;
use crate::dataminer::core::{Change, ChangeCategory, DatamineReport};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Payload {
    pub username: String,
    pub embeds: Vec<Embed>,
}

#[derive(Debug, Serialize)]
pub struct Embed {
    pub title: String,
    pub description: String,
    pub color: u32,
    pub footer: Footer,
}

#[derive(Debug, Serialize)]
pub struct Footer {
    pub text: String,
}

fn label(category: ChangeCategory) -> &'static str {
    match category {
        ChangeCategory::Added => "Добавлено",
        ChangeCategory::Removed => "Удалено",
        ChangeCategory::Changed => "Изменено",
        ChangeCategory::Unchanged => "Без изменений",
        ChangeCategory::Unknown => "Неизвестно",
        ChangeCategory::Breaking => "Breaking",
        ChangeCategory::SecurityRelevant => "Security relevant",
    }
}

fn color(category: ChangeCategory, channel: ReleaseChannel) -> u32 {
    match category {
        ChangeCategory::Added => 0x57F287,
        ChangeCategory::Removed => 0xED4245,
        ChangeCategory::Changed => 0xFEE75C,
        _ if channel == ReleaseChannel::Stable => 0xFFD400,
        _ => 0x8B5CF6,
    }
}

fn relevant_section(section: &str) -> Option<&'static str> {
    match section {
        "endpoints" | "http_methods" => Some("Endpoints"),
        "routes" | "deep_links" | "components" => Some("Страницы и routes"),
        "experiments" => Some("Эксперименты"),
        "localizations" | "strings" => Some("Локализация"),
        _ => None,
    }
}

fn line(change: &Change) -> String {
    format!(
        "**{}** `{}`{}",
        label(change.category),
        change.key,
        change
            .after
            .as_ref()
            .filter(|value| *value != "present" && *value != "referenced")
            .map(|value| format!(" → `{value}`"))
            .unwrap_or_default()
    )
}

fn chunks(lines: &[String], limit: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    for line in lines {
        let mut safe = line.clone();
        if safe.chars().count() > limit {
            safe = safe
                .chars()
                .take(limit.saturating_sub(1))
                .collect::<String>()
                + "…";
        }
        if !current.is_empty() && current.len() + safe.len() + 1 > limit {
            result.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(&safe);
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

pub fn build_payloads(report: &DatamineReport) -> Vec<Payload> {
    let Some(comparison) = &report.comparison else {
        return Vec::new();
    };
    if comparison.changes.is_empty() {
        return Vec::new();
    }
    let title = format!(
        "Эксперименты изменились: {} → {}",
        comparison.base_version_name, comparison.target_version_name
    );
    let mut embeds = vec![Embed {
        title: title.clone(),
        description: format!(
            "Android `{}` · {} change(s) · content `{}`",
            report.metadata.channel,
            comparison.changes.len(),
            &report.content_sha256[..report.content_sha256.len().min(12)]
        ),
        color: if report.metadata.channel == ReleaseChannel::Stable {
            0xFFD400
        } else {
            0x8B5CF6
        },
        footer: Footer {
            text: format!("diff {}", comparison.diff_sha256),
        },
    }];
    for section in [
        "Endpoints",
        "Страницы и routes",
        "Эксперименты",
        "Локализация",
    ] {
        for category in [
            ChangeCategory::Added,
            ChangeCategory::Removed,
            ChangeCategory::Changed,
            ChangeCategory::Breaking,
            ChangeCategory::SecurityRelevant,
            ChangeCategory::Unknown,
        ] {
            let lines = comparison
                .changes
                .iter()
                .filter(|change| {
                    change.category == category
                        && relevant_section(&change.section) == Some(section)
                })
                .map(line)
                .collect::<Vec<_>>();
            for (index, description) in chunks(&lines, 3900).into_iter().enumerate() {
                embeds.push(Embed {
                    title: if index == 0 {
                        format!("{section} · {}", label(category))
                    } else {
                        format!("{section} · {} (продолжение)", label(category))
                    },
                    description,
                    color: color(category, report.metadata.channel),
                    footer: Footer {
                        text: format!("diff {}", comparison.diff_sha256),
                    },
                });
            }
        }
    }
    embeds
        .chunks(10)
        .map(|items| Payload {
            username: "ympatcher dataminer".into(),
            embeds: items
                .iter()
                .map(|embed| Embed {
                    title: if embed.title == title {
                        title.clone()
                    } else {
                        format!("{title} · {}", embed.title)
                            .chars()
                            .take(256)
                            .collect()
                    },
                    description: embed.description.clone(),
                    color: embed.color,
                    footer: Footer {
                        text: embed.footer.text.clone(),
                    },
                })
                .collect(),
        })
        .collect()
}

pub fn write_dry_run(path: &Path, report: &DatamineReport) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(&build_payloads(report))?;
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataminer::core::{Comparison, Confidence, Evidence};

    #[test]
    fn payload_skips_empty_sections_and_never_exceeds_embed_limit() {
        let report = DatamineReport {
            schema_version: 2,
            generated_at_unix: 0,
            target: "android".into(),
            metadata: crate::dataminer::core::Metadata {
                package_name: "ru.yandex.music".into(),
                version_name: "2".into(),
                version_code: "2".into(),
                channel: ReleaseChannel::Stable,
                source_sha256: "a".repeat(64),
                source_certificate_sha256: "b".repeat(64),
                smali_classes: 1,
            },
            content_sha256: String::new(),
            snapshot: vec![],
            comparison: Some(Comparison {
                base_version_name: "1".into(),
                target_version_name: "2".into(),
                diff_sha256: "c".repeat(64),
                changes: vec![Change {
                    category: ChangeCategory::Added,
                    section: "experiments".into(),
                    key: "android_new_wave".into(),
                    before: None,
                    after: Some("on".into()),
                    evidence: Evidence {
                        source: "fixture.smali".into(),
                        confidence: Confidence::High,
                    },
                }],
            }),
        };
        let payloads = build_payloads(&report);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].embeds.len(), 2);
        assert!(
            payloads[0]
                .embeds
                .iter()
                .all(|embed| embed.description.len() <= 4096)
        );
        assert!(
            !serde_json::to_string(&payloads)
                .unwrap()
                .contains("webhook")
        );
    }
}
