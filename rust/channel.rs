use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Dev,
    Canary,
    Nightly,
    Unknown,
}

impl ReleaseChannel {
    pub fn classify(version_name: &str, declared: Option<Self>) -> Self {
        if let Some(channel) = declared
            && channel != Self::Unknown
        {
            return channel;
        }
        let normalized = version_name.to_ascii_lowercase();
        for (marker, channel) in [
            ("nightly", Self::Nightly),
            ("canary", Self::Canary),
            ("beta", Self::Beta),
            ("dev", Self::Dev),
        ] {
            if normalized
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|part| part == marker)
            {
                return channel;
            }
        }
        if normalized
            .chars()
            .any(|character| character.is_ascii_digit())
        {
            Self::Stable
        } else {
            Self::Unknown
        }
    }

    pub fn manifest_lane(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta | Self::Dev | Self::Canary | Self::Nightly | Self::Unknown => "dev",
        }
    }
}

impl fmt::Display for ReleaseChannel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Dev => "dev",
            Self::Canary => "canary",
            Self::Nightly => "nightly",
            Self::Unknown => "unknown",
        })
    }
}

impl FromStr for ReleaseChannel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            "dev" => Ok(Self::Dev),
            "canary" => Ok(Self::Canary),
            "nightly" => Ok(Self::Nightly),
            "unknown" => Ok(Self::Unknown),
            _ => bail!("неизвестный release channel: {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_version_markers() {
        assert_eq!(
            ReleaseChannel::classify("2026.09 beta 2", None),
            ReleaseChannel::Beta
        );
        assert_eq!(
            ReleaseChannel::classify("canary-1042", None),
            ReleaseChannel::Canary
        );
        assert_eq!(
            ReleaseChannel::classify("nightly.17", None),
            ReleaseChannel::Nightly
        );
        assert_eq!(
            ReleaseChannel::classify("2026.08.3 #161rur", None),
            ReleaseChannel::Stable
        );
    }

    #[test]
    fn explicit_metadata_wins_over_name_heuristic() {
        assert_eq!(
            ReleaseChannel::classify("2026.09 beta", Some(ReleaseChannel::Stable)),
            ReleaseChannel::Stable
        );
    }

    #[test]
    fn branch_lane_is_not_a_release_channel_parser() {
        assert_eq!(ReleaseChannel::Beta.manifest_lane(), "dev");
        assert_eq!(ReleaseChannel::Canary.manifest_lane(), "dev");
        assert_eq!(ReleaseChannel::Stable.manifest_lane(), "stable");
    }
}
