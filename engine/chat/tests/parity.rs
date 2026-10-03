use serde::Deserialize;
use workflow_chat::{model::*, reducer};
use workflow_environment::json::strict_json;
#[derive(Deserialize)]
struct Case {
    case: String,
    before: AgentState,
    events: Vec<AgentEvent>,
    after: AgentState,
}
#[test]
fn kotlin_reducer_golden_corpus() {
    let cases: Vec<Case> = strict_json(include_bytes!("golden/reducer.json")).unwrap();
    assert_eq!(cases.len(), 26);
    for case in cases {
        let mut actual = case.before;
        for event in &case.events {
            reducer::reduce(&mut actual, event);
        }
        assert_eq!(actual, case.after, "{}", case.case);
        // Verify encoding too: variants, explicit null defaults and structured maps.
        let bytes = serde_json::to_vec(&actual).unwrap();
        assert_eq!(
            strict_json::<AgentState>(&bytes).unwrap(),
            case.after,
            "{} codec",
            case.case
        );
    }
}
#[test]
fn raw_numeric_and_string_request_ids_stay_distinct() {
    let numeric: RequestKey =
        strict_json(br#"{"backend":"CODEX","rawId":9007199254740993123456789}"#).unwrap();
    let string: RequestKey =
        strict_json(br#"{"backend":"CODEX","rawId":"9007199254740993123456789"}"#).unwrap();
    assert_ne!(numeric, string);
    assert!(
        serde_json::to_string(&numeric)
            .unwrap()
            .contains("9007199254740993123456789")
    );
}
