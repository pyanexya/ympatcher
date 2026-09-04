use super::json::atomic_write;
use crate::dataminer::core::{ChangeCategory, DatamineReport};
use anyhow::Result;
use std::fmt::Write;
use std::path::Path;

pub fn render(report: &DatamineReport) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "# Yandex Music Android {}",
        report.metadata.version_name
    )
    .unwrap();
    writeln!(output).unwrap();
    writeln!(output, "- Канал: `{}`", report.metadata.channel).unwrap();
    writeln!(output, "- Version code: `{}`", report.metadata.version_code).unwrap();
    writeln!(output, "- APK SHA-256: `{}`", report.metadata.source_sha256).unwrap();
    writeln!(output, "- Content SHA-256: `{}`", report.content_sha256).unwrap();
    writeln!(
        output,
        "- Smali classes: `{}`",
        report.metadata.smali_classes
    )
    .unwrap();
    if let Some(comparison) = &report.comparison {
        writeln!(
            output,
            "\n## Изменения {} → {}\n",
            comparison.base_version_name, comparison.target_version_name
        )
        .unwrap();
        for category in [
            ChangeCategory::Added,
            ChangeCategory::Removed,
            ChangeCategory::Changed,
            ChangeCategory::Breaking,
            ChangeCategory::SecurityRelevant,
            ChangeCategory::Unknown,
        ] {
            let changes = comparison
                .changes
                .iter()
                .filter(|change| change.category == category)
                .collect::<Vec<_>>();
            if changes.is_empty() {
                continue;
            }
            writeln!(output, "### {:?}\n", category).unwrap();
            for change in changes {
                writeln!(
                    output,
                    "- `{}/{}` — `{}` → `{}` ({:?}, `{}`)",
                    change.section,
                    change.key,
                    change.before.as_deref().unwrap_or("∅"),
                    change.after.as_deref().unwrap_or("∅"),
                    change.evidence.confidence,
                    change.evidence.source
                )
                .unwrap();
            }
            writeln!(output).unwrap();
        }
    }
    writeln!(output, "\n## Snapshot\n").unwrap();
    let mut current = "";
    for finding in &report.snapshot {
        if finding.section != current {
            current = &finding.section;
            writeln!(output, "### {}\n", current).unwrap();
        }
        writeln!(
            output,
            "- `{}` = `{}` ({:?}, `{}`)",
            finding.key, finding.value, finding.evidence.confidence, finding.evidence.source
        )
        .unwrap();
    }
    output
}

pub fn write(path: &Path, report: &DatamineReport) -> Result<()> {
    atomic_write(path, render(report).as_bytes())
}
