use std::{io::Cursor, time::Duration};
use workflow_chat::{
    error::ErrorKind,
    transport::{
        control::{ControlRouter, Inbound as ControlInbound},
        jsonrpc::{Inbound, RpcRouter},
        lines::{Line, LineReader},
        raw::{RawJson, RequestId},
    },
};
fn raw(s: &str) -> RawJson {
    RawJson::parse(s).unwrap()
}
#[test]
fn line_reader_handles_crlf_oversize_invalid_utf8_and_final_unterminated_line() {
    let mut bytes = b"{\"a\":1}\r\n\n".to_vec();
    bytes.extend_from_slice(&[b'x'; 50]);
    bytes.push(b'\n');
    bytes.extend_from_slice(&[0xc3, 0x28, b'\n']);
    bytes.extend_from_slice("{\"b\":\"é\"}".as_bytes());
    let mut reader = LineReader::new(Cursor::new(bytes), 20);
    assert_eq!(reader.next_line().unwrap(), Line::Text("{\"a\":1}".into()));
    assert_eq!(reader.next_line().unwrap(), Line::TooLarge(50));
    assert_eq!(reader.next_line().unwrap(), Line::BadEncoding(2));
    assert_eq!(
        reader.next_line().unwrap(),
        Line::Text("{\"b\":\"é\"}".into())
    );
    assert_eq!(reader.next_line().unwrap(), Line::Eof);
}
#[test]
fn jsonrpc_separates_directions_and_echoes_raw_ids_without_auto_answering() {
    let mut router = RpcRouter::default();
    let (_, _, waiter) = router.begin("thread/start", Some(&raw("{}"))).unwrap();
    let first=router.route(raw(r#"{"id":1,"method":"item/commandExecution/requestApproval","params":{"threadId":"t"}}"#)).unwrap();
    assert!(matches!(first, Some(Inbound::Request { .. })));
    let second = router
        .route(raw(
            r#"{"id":"srv-7","method":"item/tool/requestUserInput","params":{}}"#,
        ))
        .unwrap();
    assert!(matches!(second, Some(Inbound::Request { .. })));
    assert!(matches!(
        router.route(raw(r#"{"id":99,"result":{}}"#)).unwrap(),
        Some(Inbound::Stray { .. })
    ));
    let unknown =
        r#"{ "method":"future/notification", "params":{"x":[1,{"y":null}]}, "extra":true }"#;
    let Some(Inbound::Notification { raw: preserved, .. }) = router.route(raw(unknown)).unwrap()
    else {
        panic!()
    };
    assert_eq!(preserved.text(), unknown);
    router
        .route(raw(r#"{"id":1,"result":{"thread":{"id":"t"}}}"#))
        .unwrap();
    assert_eq!(
        waiter
            .recv_timeout(Duration::from_millis(100))
            .unwrap()
            .unwrap()
            .text(),
        r#"{"thread":{"id":"t"}}"#
    );
    assert_eq!(router.open_requests().len(), 2);
    assert_eq!(
        router
            .respond(&RequestId::string("srv-7"), &raw(r#"{"answers":{}}"#))
            .unwrap()
            .text(),
        r#"{"id":"srv-7","result":{"answers":{}}}"#
    );
    assert_eq!(
        router
            .reject(&RequestId::number(1), -32601, "no")
            .unwrap()
            .text(),
        r#"{"id":1,"error":{"code":-32601,"message":"no"}}"#
    );
    assert!(
        router
            .respond(&RequestId::string("srv-7"), &raw("{}"))
            .is_err()
    );
    assert!(router.open_requests().is_empty());
}
#[test]
fn huge_numeric_ids_string_ids_and_null_results_remain_distinct() {
    let mut router = RpcRouter::default();
    for token in [
        "900719925474099312345678901",
        r#""900719925474099312345678901""#,
        "1.2300e+20",
    ] {
        let Some(Inbound::Request { id, .. }) = router
            .route(raw(&format!("{{\"id\":{token},\"method\":\"future\"}}")))
            .unwrap()
        else {
            panic!()
        };
        let response = router.respond(&id, &RawJson::null()).unwrap();
        assert_eq!(
            response.text(),
            format!("{{\"id\":{token},\"result\":null}}")
        );
    }
    let (_, _, waiter) = router.begin("x", None).unwrap();
    router.route(raw(r#"{"id":1,"result":null}"#)).unwrap();
    assert_eq!(waiter.recv().unwrap().unwrap().text(), "null");
}
#[test]
fn pending_requests_fail_on_exit_and_errors_retain_vendor_data() {
    let mut router = RpcRouter::default();
    let (_, _, waiter) = router.begin("slow", None).unwrap();
    router.close();
    assert_eq!(waiter.recv().unwrap().unwrap_err().kind, ErrorKind::Closed);
    let mut router = RpcRouter::default();
    let (_, _, waiter) = router.begin("x", None).unwrap();
    let failure =
        r#"{"id":1,"error":{"code":-32602,"message":"bad params","data":{"f":1}},"future":true}"#;
    router.route(raw(failure)).unwrap();
    let error = waiter.recv().unwrap().unwrap_err();
    assert_eq!(error.message, "bad params");
    assert_eq!(error.vendor.unwrap().text(), failure);
}
#[test]
fn control_ignores_echoes_correlates_ids_and_forgets_cancelled_requests() {
    let mut router = ControlRouter::default();
    let (sent, waiter) = router
        .begin("ours-1", &raw(r#"{"subtype":"interrupt"}"#))
        .unwrap();
    assert_eq!(
        sent.text(),
        r#"{"type":"control_request","request_id":"ours-1","request":{"subtype":"interrupt"}}"#
    );
    assert!(
        router
            .route(raw(r#"{"type":"keep_alive"}"#))
            .unwrap()
            .is_none()
    );
    router.route(raw(r#"{"type":"control_response","response":{"subtype":"success","request_id":"someone-else","response":{}}}"#)).unwrap();
    assert!(matches!(router.route(raw(r#"{"type":"control_request","request_id":"cli-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{}}}"#)).unwrap(),Some(ControlInbound::Request {..})));
    assert_eq!(router.open_requests(), vec!["cli-1"]);
    assert!(matches!(
        router
            .route(raw(
                r#"{"type":"control_cancel_request","request_id":"cli-1"}"#
            ))
            .unwrap(),
        Some(ControlInbound::Cancel { .. })
    ));
    assert!(matches!(
        router
            .route(raw(
                r#"{"type":"system","subtype":"brand_new","payload":1}"#
            ))
            .unwrap(),
        Some(ControlInbound::Message { .. })
    ));
    router.route(raw(r#"{"type":"control_response","response":{"subtype":"success","request_id":"ours-1","response":{"still_queued":[]}}}"#)).unwrap();
    assert_eq!(
        waiter.recv().unwrap().unwrap().text(),
        r#"{"still_queued":[]}"#
    );
    assert_eq!(router.ignored_responses, 1);
    assert!(router.respond("cli-1", None).is_err());
}
#[test]
fn control_errors_preserve_raw_frame_and_undeclared_dialogs_can_be_forgotten() {
    let mut router = ControlRouter::default();
    let (_, waiter) = router
        .begin("ours", &raw(r#"{"subtype":"set_model"}"#))
        .unwrap();
    let failure = r#"{"type":"control_response","response":{"subtype":"error","request_id":"ours","error":"nope","future":42}}"#;
    router.route(raw(failure)).unwrap();
    let error = waiter.recv().unwrap().unwrap_err();
    assert_eq!(error.message, "nope");
    assert_eq!(error.vendor.unwrap().text(), failure);
    router.route(raw(r#"{"type":"control_request","request_id":"dialog","request":{"subtype":"request_user_dialog"}}"#)).unwrap();
    router.forget("dialog");
    assert!(router.respond("dialog", Some(&raw("{}"))).is_err());
}
