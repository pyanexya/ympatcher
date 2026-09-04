use super::model::{Distribution, ParsedVersion, Platform};
use regex::Regex;
use std::cmp::Ordering;
use std::sync::OnceLock;

fn version_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"^(\d{4})\.(\d{1,2})\.(\d+)(?:-([A-Za-z0-9_-]+))?\s+#(\d+(?:\.\d+)*)([A-Za-z][A-Za-z0-9_-]*)?$",
        )
        .expect("version pattern is valid")
    })
}

pub fn parse_version(input: &str) -> Option<ParsedVersion> {
    let captures = version_pattern().captures(input.trim())?;
    let build = captures
        .get(5)?
        .as_str()
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u32>, _>>()
        .ok()?;
    Some(ParsedVersion {
        year: captures.get(1)?.as_str().parse().ok()?,
        month: captures.get(2)?.as_str().parse().ok()?,
        patch: captures.get(3)?.as_str().parse().ok()?,
        variant: captures.get(4).map(|value| value.as_str().to_owned()),
        build,
        suffix: captures.get(6).map(|value| value.as_str().to_owned()),
    })
}

pub fn compare_versions(left: &ParsedVersion, right: &ParsedVersion) -> Ordering {
    (left.year, left.month, left.patch)
        .cmp(&(right.year, right.month, right.patch))
        .then_with(|| compare_build(&left.build, &right.build))
}

fn compare_build(left: &[u32], right: &[u32]) -> Ordering {
    let length = left.len().max(right.len());
    (0..length)
        .map(|index| {
            left.get(index)
                .copied()
                .unwrap_or_default()
                .cmp(&right.get(index).copied().unwrap_or_default())
        })
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

pub fn inferred_distribution(parsed: Option<&ParsedVersion>) -> Distribution {
    match parsed.and_then(|value| value.suffix.as_deref()) {
        Some("gpr") => Distribution::GooglePlay,
        Some("rur") => Distribution::RuStore,
        _ => Distribution::Unknown,
    }
}

pub fn inferred_platform(parsed: Option<&ParsedVersion>) -> Platform {
    if parsed
        .and_then(|value| value.variant.as_deref())
        .is_some_and(|variant| variant.eq_ignore_ascii_case("wear"))
    {
        Platform::WearOs
    } else {
        Platform::AndroidPhone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_names() {
        for name in [
            "2026.08.4 #162.1gpr",
            "2026.08.3 #161gpr",
            "2026.08.3 #161rur",
            "2026.08.4-wear #162.1gpr",
        ] {
            assert!(parse_version(name).is_some(), "{name}");
        }
        let parsed = parse_version("2026.08.4-wear #162.1gpr").unwrap();
        assert_eq!(parsed.variant.as_deref(), Some("wear"));
        assert_eq!(parsed.build, [162, 1]);
    }

    #[test]
    fn compares_components_numerically() {
        for (newer, older) in [
            ("2026.08.4 #1gpr", "2026.08.3 #999gpr"),
            ("2026.09.1 #1gpr", "2026.08.99 #999gpr"),
            ("2027.01.1 #1gpr", "2026.12.99 #999gpr"),
        ] {
            assert_eq!(
                compare_versions(
                    &parse_version(newer).unwrap(),
                    &parse_version(older).unwrap()
                ),
                Ordering::Greater
            );
        }
    }

    #[test]
    fn malformed_inputs_are_rejected_without_panicking() {
        for input in ["", "garbage", "2026.08.4", "2026.08.4 #"] {
            assert!(parse_version(input).is_none());
        }
        let unknown = parse_version("2026.08.4 #162xyz").unwrap();
        assert_eq!(unknown.suffix.as_deref(), Some("xyz"));
        assert_eq!(inferred_distribution(Some(&unknown)), Distribution::Unknown);
    }
}
