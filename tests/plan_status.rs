#[test]
fn plan_summary_marks_all_completed_prs_done() {
    let plan = include_str!("../docs/PLAN.md");
    let summary_rows = plan.lines().filter(|line| line.starts_with("| PR-")).collect::<Vec<_>>();

    assert_eq!(summary_rows.len(), 17);

    for row in summary_rows {
        assert!(row.contains("`✅ Done`"), "summary row is not marked done: {row}");
    }
}
