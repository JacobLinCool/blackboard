use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppErr, Res};

pub fn parse_depends(raw: Option<String>) -> Res<Option<Vec<i64>>> {
    match raw {
        None => Ok(None),
        Some(raw) => {
            if raw.trim().is_empty() {
                return Ok(Some(Vec::new()));
            }
            let mut out = Vec::new();
            for part in raw.split(',') {
                let item = part.trim();
                if item.is_empty() {
                    continue;
                }
                out.push(
                    item.parse::<i64>()
                        .map_err(|_| AppErr("input", format!("invalid task id '{}'", item)))?,
                );
            }
            Ok(Some(out))
        }
    }
}

pub fn normalize_depends(mut deps: Vec<i64>) -> Vec<i64> {
    deps.sort_unstable();
    deps.dedup();
    deps
}

pub fn format_depends(deps: &[i64]) -> String {
    format!(
        "[{}]",
        deps.iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::{format_depends, normalize_depends, parse_depends};

    #[test]
    fn parse_depends_handles_none_and_empty() {
        assert_eq!(parse_depends(None).expect("parse should succeed"), None);
        assert_eq!(
            parse_depends(Some("".to_string())).expect("parse should succeed"),
            Some(Vec::new())
        );
        assert_eq!(
            parse_depends(Some("  ".to_string())).expect("parse should succeed"),
            Some(Vec::new())
        );
    }

    #[test]
    fn parse_depends_parses_comma_separated_ids() {
        let parsed = parse_depends(Some("3, 1,2".to_string())).expect("parse should succeed");
        assert_eq!(parsed, Some(vec![3, 1, 2]));
    }

    #[test]
    fn parse_depends_rejects_invalid_values() {
        let err = parse_depends(Some("1,x".to_string())).expect_err("parse should fail");
        assert_eq!(err.to_string(), "input: invalid task id 'x'");
    }

    #[test]
    fn normalize_depends_sorts_and_deduplicates() {
        assert_eq!(normalize_depends(vec![5, 1, 3, 1, 5]), vec![1, 3, 5]);
    }

    #[test]
    fn format_depends_matches_cli_output_style() {
        assert_eq!(format_depends(&[]), "[]");
        assert_eq!(format_depends(&[2, 4, 8]), "[2, 4, 8]");
    }
}
