use std::collections::HashMap;

static PLAN: &str = include_str!("../docs/PLAN.md");

/// Extract `PR-NN` -> status from the summary table rows (`| PR-NN | ... | \`✅ Done\` |`).
fn summary_statuses() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for row in PLAN.lines().filter(|l| l.starts_with("| PR-")) {
        // cells: | PR-NN | title | phase | deps | `✅ Done` |
        let cells: Vec<&str> = row.split('|').map(str::trim).filter(|c| !c.is_empty()).collect();
        if cells.len() < 5 {
            continue;
        }
        let pr = cells[0].to_string();
        // last cell holds the status token
        let status =
            cells.last().expect("summary row has a status cell").trim_matches('`').to_string();
        map.insert(pr, status);
    }
    map
}

/// Extract `PR-NN` (or `PR-NNx`, e.g. `PR-04a`) -> status from the per-PR
/// section headers (`### PR-NN — ...` followed by `**ステータス:** \`✅ Done\``).
fn section_statuses() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_pr: Option<String> = None;
    for line in PLAN.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("### ") {
            // any H3 resets the current PR context
            current_pr = None;
            if let Some(rest) = trimmed.strip_prefix("### PR-") {
                // capture the leading PR id token: digits + optional trailing letter (PR-04a)
                let id_token: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
                    .collect();
                if !id_token.is_empty() {
                    current_pr = Some(format!("PR-{id_token}"));
                }
            }
        } else if let Some(pr) = &current_pr {
            if let Some(rest) = trimmed.strip_prefix("**ステータス:**") {
                let status = rest.trim().trim_matches('`').to_string();
                if !status.is_empty() {
                    map.insert(pr.clone(), status);
                }
            }
        }
    }
    map
}

#[test]
fn plan_summary_marks_all_completed_prs_done() {
    let summary = summary_statuses();
    assert_eq!(summary.len(), 17, "expected 17 summary-table PR rows");

    for (pr, status) in &summary {
        assert_eq!(status, "✅ Done", "summary row {pr} is not marked done: {status:?}");
    }
}

#[test]
fn plan_summary_and_section_counts_match() {
    let summary = summary_statuses();
    let sections = section_statuses();

    assert!(!sections.is_empty(), "no `### PR-` sections parsed; parser may be broken");
    assert_eq!(
        summary.len(),
        sections.len(),
        "summary table has {} PR rows but {} `### PR-` sections exist",
        summary.len(),
        sections.len()
    );

    for (pr, summary_status) in &summary {
        let section_status = sections.get(pr).unwrap_or_else(|| {
            panic!("summary lists {pr} but no matching `### PR-` section exists")
        });
        assert_eq!(
            summary_status, section_status,
            "{pr} status diverges: summary={summary_status:?} section={section_status:?}"
        );
    }
}

#[test]
fn plan_has_fresh_last_updated_date() {
    // The "最終更新: YYYY-MM-DD" line must exist and be a parseable date.
    // It is not required to equal today (CI may run off-cache), but it must be
    // present and well-formed so staleness is detectable.
    let date_line = PLAN
        .lines()
        .find_map(|l| l.trim().strip_prefix("最終更新:").map(str::trim))
        .expect("PLAN.md must have a `最終更新: YYYY-MM-DD` line");

    let date = date_line.trim();
    assert!(
        chrono::DateTime::parse_from_rfc3339(&format!("{date}T00:00:00+00:00")).is_ok()
            || date.len() == 10,
        "`最終更新` must be YYYY-MM-DD, got {date:?}"
    );
}
