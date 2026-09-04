use crate::channel::ReleaseChannel;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatamineReport {
    pub schema_version: u32,
    pub generated_at_unix: u64,
    pub target: String,
    pub metadata: Metadata,
    pub content_sha256: String,
    pub snapshot: Vec<Finding>,
    pub comparison: Option<Comparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub package_name: String,
    pub version_name: String,
    pub version_code: String,
    pub channel: ReleaseChannel,
    pub source_sha256: String,
    pub source_certificate_sha256: String,
    pub smali_classes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub source: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub section: String,
    pub key: String,
    pub value: String,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ChangeCategory {
    Added,
    Removed,
    Changed,
    Unchanged,
    Unknown,
    Breaking,
    SecurityRelevant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub category: ChangeCategory,
    pub section: String,
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub base_version_name: String,
    pub target_version_name: String,
    pub changes: Vec<Change>,
    pub diff_sha256: String,
}

impl DatamineReport {
    pub fn deterministic_hash(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Stable<'a> {
            schema_version: u32,
            target: &'a str,
            metadata: &'a Metadata,
            snapshot: &'a [Finding],
        }
        let bytes = serde_json::to_vec(&Stable {
            schema_version: self.schema_version,
            target: &self.target,
            metadata: &self.metadata,
            snapshot: &self.snapshot,
        })?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }
}

pub fn compare(previous: &DatamineReport, current: &DatamineReport) -> Comparison {
    let before = previous
        .snapshot
        .iter()
        .map(|item| ((item.section.as_str(), item.key.as_str()), item))
        .collect::<BTreeMap<_, _>>();
    let after = current
        .snapshot
        .iter()
        .map(|item| ((item.section.as_str(), item.key.as_str()), item))
        .collect::<BTreeMap<_, _>>();
    let keys = before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    for key in keys {
        let old = before.get(&key);
        let new = after.get(&key);
        let (category, evidence) = match (old, new) {
            (None, Some(value)) => (ChangeCategory::Added, value.evidence.clone()),
            (Some(value), None) => (ChangeCategory::Removed, value.evidence.clone()),
            (Some(old), Some(new)) if old.value != new.value => {
                (ChangeCategory::Changed, new.evidence.clone())
            }
            (Some(_), Some(_)) => continue,
            (None, None) => unreachable!(),
        };
        changes.push(Change {
            category,
            section: key.0.to_owned(),
            key: key.1.to_owned(),
            before: old.map(|item| item.value.clone()),
            after: new.map(|item| item.value.clone()),
            evidence,
        });
    }
    let bytes = serde_json::to_vec(&changes).expect("serializing changes cannot fail");
    Comparison {
        base_version_name: previous.metadata.version_name.clone(),
        target_version_name: current.metadata.version_name.clone(),
        changes,
        diff_sha256: hex::encode(Sha256::digest(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(value: &str) -> DatamineReport {
        DatamineReport {
            schema_version: 2,
            generated_at_unix: 1,
            target: "android".into(),
            metadata: Metadata {
                package_name: "ru.yandex.music".into(),
                version_name: value.into(),
                version_code: "1".into(),
                channel: ReleaseChannel::Stable,
                source_sha256: "a".repeat(64),
                source_certificate_sha256: "b".repeat(64),
                smali_classes: 1,
            },
            content_sha256: String::new(),
            snapshot: vec![Finding {
                section: "experiments".into(),
                key: "flag".into(),
                value: value.into(),
                evidence: Evidence {
                    source: "fixture.smali".into(),
                    confidence: Confidence::High,
                },
            }],
            comparison: None,
        }
    }

    #[test]
    fn hash_ignores_generation_time_and_comparison() {
        let mut a = report("on");
        let mut b = a.clone();
        b.generated_at_unix = 99;
        b.comparison = Some(compare(&report("off"), &b));
        assert_eq!(
            a.deterministic_hash().unwrap(),
            b.deterministic_hash().unwrap()
        );
        a.snapshot[0].value = "off".into();
        assert_ne!(
            a.deterministic_hash().unwrap(),
            b.deterministic_hash().unwrap()
        );
    }

    #[test]
    fn diff_is_sorted_and_classified() {
        let comparison = compare(&report("off"), &report("on"));
        assert_eq!(comparison.changes.len(), 1);
        assert_eq!(comparison.changes[0].category, ChangeCategory::Changed);
        assert_eq!(comparison.changes[0].before.as_deref(), Some("off"));
    }
}
