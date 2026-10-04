//! Transport parity with the retained Kotlin `TransportTest`, over in-memory guest stdio.
use serde_json::{Value, json};
use std::{
    io::Cursor,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use workflow_chat::{
    error::ErrorKind,
    model::OpaqueJson,
    ports::SpawnSpec,
    testing::{FakeProcess, React, opaque},
    transport::{
        control::{ControlConnection, ControlInbound},
        jsonrpc::{RpcConnection, RpcInbound},
        lines::{Line, LineReader},
    },
    wire,
};

fn spec() -> SpawnSpec {
    SpawnSpec {
        argv: vec!["x".into()],
        cwd: "/".into(),
        env: Default::default(),
        label: "t".into(),
    }
}
fn process(react: impl Fn(&FakeProcess, &Value) + Send + Sync + 'static) -> Arc<FakeProcess> {
    let react: React = Arc::new(react);
    FakeProcess::new(spec(), react)
}
fn text(v: &OpaqueJson) -> String {
    wire::text(v)
}
fn rpc(p: &FakeProcess) -> (Arc<RpcConnection>, mpsc::Receiver<RpcInbound>) {
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let c = RpcConnection::open(
        p.take_stdio(),
        Box::new(move |_, m| {
            let _ = tx.lock().unwrap().send(m);
        }),
    );
    (c, rx)
}
fn recv<T>(rx: &mpsc::Receiver<T>) -> T {
    rx.recv_timeout(Duration::from_secs(5))
        .expect("inbound message")
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
fn jsonrpc_keeps_directions_apart_and_echoes_server_ids_verbatim() {
    let fake = process(|p, frame| {
        // Before answering our request 1, the server sends its own request with the same ID 1.
        if frame["method"] == "thread/start" {
            p.emit(&json!({"id":1,"method":"item/commandExecution/requestApproval","params":{"threadId":"t"}}));
            p.emit(&json!({"id":"srv-7","method":"item/tool/requestUserInput","params":{}}));
            p.emit(&json!({"id":99,"result":{}}));
            p.emit_raw("{\"method\":\"future/notification\",\"params\":{\"x\":[1,{\"y\":null}]},\"extra\":true}\n");
            p.emit(&json!({"id":1,"result":{"thread":{"id":"t"}}}));
        }
    });
    let (c, rx) = rpc(&fake);
    let result = c
        .request("thread/start", Some(opaque(json!({"cwd":"/workspace"}))))
        .unwrap();
    assert_eq!(text(&result), r#"{"thread":{"id":"t"}}"#);
    let RpcInbound::Request {
        id: numeric,
        method,
        ..
    } = recv(&rx)
    else {
        panic!()
    };
    assert_eq!(text(&numeric), "1");
    assert_eq!(method, "item/commandExecution/requestApproval");
    let RpcInbound::Request { id: string, .. } = recv(&rx) else {
        panic!()
    };
    assert_eq!(text(&string), r#""srv-7""#);
    assert!(matches!(recv(&rx), RpcInbound::Stray { .. }));
    let RpcInbound::Notification { raw, method, .. } = recv(&rx) else {
        panic!()
    };
    assert_eq!(method, "future/notification");
    // Every member of an unknown notification is preserved.
    assert_eq!(
        raw.0,
        json!({"method":"future/notification","params":{"x":[1,{"y":null}]},"extra":true})
    );
    assert_eq!(
        c.open_server_requests(),
        vec!["\"srv-7\"".to_string(), "1".to_string()]
    );
    c.respond(&string, &opaque(json!({"answers":{}}))).unwrap();
    c.respond_error(&numeric, -32601, "no", None).unwrap();
    let written: Vec<String> = fake.written().iter().map(|v| v.to_string()).collect();
    assert!(written.contains(&r#"{"id":"srv-7","result":{"answers":{}}}"#.to_string()));
    assert!(written.contains(&r#"{"error":{"code":-32601,"message":"no"},"id":1}"#.to_string()));
    assert_eq!(
        c.respond(&string, &opaque(json!({}))).unwrap_err().kind,
        ErrorKind::RequestExpired
    );
    assert!(c.open_server_requests().is_empty());
}

#[test]
fn huge_numeric_ids_string_ids_and_null_results_remain_distinct() {
    let fake = process(|p, frame| {
        if frame["method"] == "x" {
            p.emit(&json!({"id": frame["id"], "result": null}));
        }
    });
    let (c, rx) = rpc(&fake);
    for token in [
        "900719925474099312345678901",
        r#""900719925474099312345678901""#,
        "1.2300e+20",
        "-0",
    ] {
        fake.emit_raw(&format!("{{\"id\":{token},\"method\":\"future\"}}\n"));
        let RpcInbound::Request { id, .. } = recv(&rx) else {
            panic!()
        };
        assert_eq!(text(&id), token);
        c.respond(&id, &OpaqueJson::default()).unwrap();
    }
    let answers: Vec<String> = fake
        .written()
        .iter()
        .map(|v| serde_json::to_string(v).unwrap())
        .collect();
    assert_eq!(
        answers,
        [
            r#"{"id":900719925474099312345678901,"result":null}"#,
            r#"{"id":"900719925474099312345678901","result":null}"#,
            r#"{"id":1.2300e+20,"result":null}"#,
            r#"{"id":-0,"result":null}"#,
        ]
    );
    assert_eq!(text(&c.request("x", None).unwrap()), "null");
}

#[test]
fn pending_requests_fail_when_the_process_exits_and_errors_keep_vendor_data() {
    let fake = process(|p, frame| {
        if frame["method"] == "slow" {
            p.exit(1)
        } else {
            p.emit(&json!({"id":frame["id"],"error":{"code":-32602,"message":"bad params","data":{"f":1}},"future":true}))
        }
    });
    let (c, _rx) = rpc(&fake);
    let error = c.request("x", None).unwrap_err();
    assert_eq!(error.code, Some(-32602));
    assert_eq!(error.message, "bad params");
    assert_eq!(text(error.data.as_ref().unwrap()), r#"{"f":1}"#);
    assert_eq!(c.request("slow", None).unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(
        c.request("after", None).unwrap_err().kind,
        ErrorKind::Closed
    );
}

#[test]
fn malformed_frames_are_reported_and_reading_continues() {
    let fake = process(|_, _| {});
    let (_c, rx) = rpc(&fake);
    fake.emit_raw("not json\n[1,2]\n{\"method\":\"ok\"}\n");
    assert!(matches!(recv(&rx), RpcInbound::Malformed { .. }));
    assert!(matches!(recv(&rx), RpcInbound::Malformed { .. }));
    assert!(matches!(recv(&rx), RpcInbound::Notification { .. }));
}

fn control(p: &FakeProcess) -> (Arc<ControlConnection>, mpsc::Receiver<ControlInbound>) {
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let c = ControlConnection::open(
        p.take_stdio(),
        Box::new(|| "ours-1".to_string()),
        Box::new(move |_, m| {
            let _ = tx.lock().unwrap().send(m);
        }),
    );
    (c, rx)
}

#[test]
fn control_ignores_echoes_correlates_ids_and_forgets_cancelled_requests() {
    let fake = process(|p, frame| {
        if frame["type"] == "control_request" {
            let id = frame["request_id"].clone();
            p.emit(&json!({"type":"keep_alive"}));
            p.emit(&json!({"type":"control_response","response":{"subtype":"success","request_id":"someone-else","response":{}}}));
            p.emit(&json!({"type":"control_request","request_id":"cli-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{}}}));
            p.emit(&json!({"type":"control_cancel_request","request_id":"cli-1"}));
            p.emit(&json!({"type":"system","subtype":"brand_new","payload":1}));
            p.emit(&json!({"type":"control_response","response":{"subtype":"success","request_id":id,"response":{"still_queued":[]}}}));
        }
        if frame["type"] == "control_response" {
            p.emit(frame); // the CLI echoes our responses
        }
    });
    let (c, rx) = control(&fake);
    let result = c.control("interrupt", &Default::default()).unwrap();
    assert_eq!(text(&result), r#"{"still_queued":[]}"#);
    let ControlInbound::Request { id, subtype, .. } = recv(&rx) else {
        panic!()
    };
    assert_eq!((id.as_str(), subtype.as_str()), ("cli-1", "can_use_tool"));
    let ControlInbound::Cancel { id, .. } = recv(&rx) else {
        panic!()
    };
    assert_eq!(id, "cli-1");
    let ControlInbound::Message { kind, .. } = recv(&rx) else {
        panic!()
    };
    assert_eq!(kind, "system");
    assert_eq!(c.ignored_responses(), 1);
    assert_eq!(
        c.respond("cli-1", None).unwrap_err().kind,
        ErrorKind::RequestExpired
    );
    assert_eq!(
        fake.written()[0],
        json!({"type":"control_request","request_id":"ours-1","request":{"subtype":"interrupt"}})
    );
}

#[test]
fn control_errors_surface_and_undeclared_dialogs_can_be_forgotten() {
    let fake = process(|p, frame| {
        if frame["type"] == "control_request" {
            p.emit(&json!({"type":"control_response","response":{"subtype":"error","request_id":frame["request_id"],"error":"nope","future":42}}));
            p.emit(&json!({"type":"control_request","request_id":"dialog","request":{"subtype":"request_user_dialog"}}));
        }
    });
    let (c, rx) = control(&fake);
    let fields =
        workflow_chat::model::OpaqueObject::from([("model".into(), wire::string_value("x"))]);
    let error = c.control("set_model", &fields).unwrap_err();
    assert_eq!(error.message, "nope");
    assert!(matches!(recv(&rx), ControlInbound::Request { .. }));
    c.forget("dialog");
    assert!(c.respond("dialog", Some(&opaque(json!({})))).is_err());
    assert_eq!(
        fake.written()[0],
        json!({"type":"control_request","request_id":"ours-1","request":{"subtype":"set_model","model":"x"}})
    );
}
