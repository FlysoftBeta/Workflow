use super::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct LayoutFixture {
    seed: usize,
    step: usize,
    before: Workbench,
    op: LayoutAction,
    after: Workbench,
}
#[test]
fn layout_matches_kotlin_oracle_and_is_idempotent() {
    let fixtures = include_str!("../../server/fixtures/layout.jsonl");
    for line in fixtures.lines() {
        let fixture: LayoutFixture =
            workflow_environment::json::strict_json(line.as_bytes()).unwrap();
        let actual = layout::apply(&fixture.before, &fixture.op);
        assert_eq!(
            actual, fixture.after,
            "seed {} step {}",
            fixture.seed, fixture.step
        );
        let mut normalized = actual.clone();
        layout::normalize(&mut normalized);
        assert_eq!(actual, normalized);
    }
}
#[test]
fn layout_retains_unknown_fields_and_resets_malformed_session_layout() {
    let source = r#"{"id":"s","createdAt":1,"lastUsedAt":1,"futureSession":{"a":1},"workbench":{"futureWorkbench":{"b":2}}}"#;
    let session: Session = serde_json::from_str(source).unwrap();
    assert!(session.extra.contains_key("futureSession"));
    assert!(session.workbench.extra.contains_key("futureWorkbench"));
    let malformed = source.replace("{\"futureWorkbench\":{\"b\":2}}", "12");
    let session: Session = serde_json::from_str(&malformed).unwrap();
    assert_eq!(session.workbench, Workbench::default());
}
#[test]
fn tagged_layout_actions_accept_decimal_and_exponent_numbers() {
    for source in [
        r#"{"type":"resizeRegion","region":"bottom","size":0.3500}"#,
        r#"{"type":"resizeRegion","region":"bottom","size":3.5e-1}"#,
    ] {
        let op: LayoutAction = workflow_environment::json::strict_json(source.as_bytes()).unwrap();
        assert!(matches!(op,LayoutAction::ResizeRegion{size,..}if size==0.35));
    }
    let invalid=br#"{"type":"resizeRegion","region":"bottom","size":{"$serde_json::private::Number":"0.35"}}"#;
    assert!(workflow_environment::json::strict_json::<LayoutAction>(invalid).is_err());
}
#[test]
fn normalizing_and_replacing_views_preserves_unrelated_extensions() {
    let marker = workflow_environment::json::strict_json::<OpaqueJson>(
        br#"{"precision":123456789012345678901234567890}"#,
    )
    .unwrap();
    let mut w = layout::apply(
        &Workbench::default(),
        &LayoutAction::Open {
            target: Target::file("a"),
            placement: Placement::Auto,
            focus: true,
        },
    );
    w.extra.insert("future".into(), marker.clone());
    w.panels[0].extra.insert("future".into(), marker.clone());
    w.panels[0]
        .target
        .extra
        .insert("future".into(), marker.clone());
    w.stacks[0].extra.insert("future".into(), marker.clone());
    w.files.bottom.extra.insert("future".into(), marker.clone());
    let w = layout::apply(
        &w,
        &LayoutAction::UpdateView {
            panel_id: "p1".into(),
            view: PanelView::default(),
        },
    );
    for extra in [
        &w.extra,
        &w.panels[0].extra,
        &w.panels[0].target.extra,
        &w.stacks[0].extra,
        &w.files.bottom.extra,
    ] {
        assert_eq!(extra.get("future"), Some(&marker));
    }
}
