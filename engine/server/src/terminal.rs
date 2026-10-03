//! Terminal resources outlive UI attachment. The Engine owns identity, processes and cleanup.
use crate::{
    environment::Environment,
    process::Processes,
    protocol::{Error, Result, decode_blob},
    storage,
    workspace::required,
};
use serde_json::{Value as V, json};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Terminal {
    value: V,
    process: Option<String>,
    scan: u64,
    osc: Vec<u8>,
    unreferenced: Option<Instant>,
}
pub struct Terminals {
    path: PathBuf,
    items: Mutex<HashMap<String, Terminal>>,
    failure: Option<Error>,
}
impl Terminals {
    pub fn load(root: &std::path::Path) -> Result<Self> {
        let path = root.join(".workspace/state/terminals.json");
        let loaded = (|| -> Result<HashMap<String, Terminal>> {
            let saved = if path.exists() {
                storage::read_json(&path)?
            } else {
                json!({"format":1,"terminals":[]})
            };
            if saved["format"] != 1 || !saved["terminals"].is_array() {
                return Err(Error::business(
                    "unsupported_format",
                    "unsupported terminal state",
                ));
            }
            let mut items = HashMap::new();
            for v in saved["terminals"].as_array().unwrap() {
                let id = required(v, "id")?.to_owned();
                storage::identifier(&id)?;
                let mut value = v.clone();
                value["status"] = json!("ended");
                value["exitCode"] = V::Null;
                items.insert(
                    id,
                    Terminal {
                        value,
                        process: None,
                        scan: 0,
                        osc: vec![],
                        unreferenced: Some(Instant::now()),
                    },
                );
            }
            Ok(items)
        })();
        let (items, failure) = match loaded {
            Ok(items) => (items, None),
            Err(error) => (HashMap::new(), Some(error)),
        };
        Ok(Self {
            path,
            items: Mutex::new(items),
            failure,
        })
    }
    fn save(&self, items: &HashMap<String, Terminal>) -> Result<()> {
        storage::write_json(
            &self.path,
            &json!({"format":1,"terminals":items.values().map(|t| t.value.clone()).collect::<Vec<_>>()}),
        )
    }
    pub fn call(
        &self,
        processes: &Processes,
        environment: &Arc<Mutex<Environment>>,
        method: &str,
        a: &V,
    ) -> Result<V> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if method == "terminal.create" {
            let mut items = self.items.lock().unwrap();
            let id = format!("t-{}", uuid::Uuid::new_v4());
            let ordinal = items
                .values()
                .filter_map(|t| t.value["ordinal"].as_u64())
                .max()
                .unwrap_or(0)
                + 1;
            let mut terminal = Terminal {
                value: json!({"id":id,"ordinal":ordinal,"generation":0,"cwd":cwd(a)?,"title":null,"customTitle":null,"status":"starting","rows":24,"columns":80}),
                process: None,
                scan: 0,
                osc: vec![],
                unreferenced: Some(Instant::now()),
            };
            start(&mut terminal, processes, environment, a)?;
            let result = terminal.value.clone();
            items.insert(id, terminal);
            self.save(&items)?;
            return Ok(result);
        }
        let id = required(a, "terminalId")?;
        // Never hold the registry while waiting for PTY bytes or process exit.
        if method == "terminal.read" || method == "terminal.wait" {
            self.refresh(processes)?;
            let (process, meta) = {
                let items = self.items.lock().unwrap();
                let t = items
                    .get(id)
                    .ok_or_else(|| Error::business("not_found", "terminal not found"))?;
                (t.process.clone(), t.value.clone())
            };
            if method == "terminal.wait" {
                return match process {
                    Some(p) => processes.request(
                        "process.wait",
                        &json!({"processId":p,"timeoutMs":a["timeoutMs"].as_u64().unwrap_or(1000)}),
                    ),
                    None => Ok(json!({"running":false,"exitCode":meta["exitCode"]})),
                };
            }
            let reset = a["generation"].as_u64() != meta["generation"].as_u64();
            let mut output = match process {
                Some(p) => processes.request("process.read", &json!({"processId":p,"offset":if reset {0}else{a["offset"].as_u64().unwrap_or(0)},"maxBytes":a["maxBytes"].as_u64().unwrap_or(65536),"waitMs":a["waitMs"].as_u64().unwrap_or(1000)}))?,
                None => json!({"data":"","startOffset":0,"nextOffset":0,"eof":true}),
            };
            output["terminal"] = meta;
            output["reset"] = json!(reset);
            return Ok(output);
        }
        let mut items = self.items.lock().unwrap();
        let t = items
            .get_mut(id)
            .ok_or_else(|| Error::business("not_found", "terminal not found"))?;
        match method {
            "terminal.attach" => {
                if t.process.is_none()
                    && t.value["status"] != "failed"
                    && t.value["exitCode"].is_null()
                {
                    start(t, processes, environment, a)?;
                }
            }
            "terminal.restart" => {
                if let Some(p) = &t.process {
                    processes.request("process.stop", &json!({"processId":p,"force":true}))?;
                    let stopped = processes
                        .request("process.wait", &json!({"processId":p,"timeoutMs":10000}))?;
                    if stopped["running"] == true {
                        return Err(Error::business("stop_timeout", "terminal did not stop"));
                    }
                }
                start(t, processes, environment, a)?;
            }
            "terminal.rename" => {
                t.value["customTitle"] = a["title"]
                    .as_str()
                    .map(|s| s.trim().chars().take(80).collect::<String>())
                    .filter(|s| !s.is_empty())
                    .map(V::String)
                    .unwrap_or(V::Null);
            }
            "terminal.clear" => {
                if let Some(p) = &t.process {
                    processes.clear_output(p)?;
                }
                t.scan = 0;
                t.osc.clear();
                t.value["generation"] = json!(t.value["generation"].as_u64().unwrap_or(0) + 1);
            }
            "terminal.write" | "terminal.resize" | "terminal.stop" => {
                let p = t
                    .process
                    .as_ref()
                    .ok_or_else(|| Error::business("closed", "terminal has no process"))?;
                let mut args = a.clone();
                args["processId"] = json!(p);
                let result =
                    processes.request(&method.replacen("terminal.", "process.", 1), &args)?;
                if method == "terminal.resize" {
                    t.value["rows"] = result["rows"].clone();
                    t.value["columns"] = result["columns"].clone();
                }
                return Ok(result);
            }
            "terminal.status" => {}
            _ => return Err(Error::method()),
        }
        let value = t.value.clone();
        self.save(&items)?;
        Ok(value)
    }
    pub fn refresh(&self, processes: &Processes) -> Result<()> {
        if self.failure.is_some() {
            return Ok(());
        }
        let mut items = self.items.lock().unwrap();
        let mut changed = false;
        for t in items.values_mut() {
            let Some(p) = t.process.clone() else { continue };
            let before = t.value.clone();
            for _ in 0..16 {
                let output = match processes.request(
                    "process.read",
                    &json!({"processId":p,"offset":t.scan,"maxBytes":65536}),
                ) {
                    Ok(output) => output,
                    Err(error) if error.kind == "not_found" => {
                        t.process = None;
                        if t.value["status"] == "running" {
                            t.value["status"] = json!("failed");
                            t.value["error"] = json!("terminal process record has expired");
                        }
                        break;
                    }
                    Err(error) => return Err(error),
                };
                if output["startOffset"].as_u64().unwrap_or(0) != t.scan {
                    t.osc.clear();
                }
                let bytes = decode_blob(output["data"].as_str().unwrap_or(""), 65536)?;
                t.scan = output["nextOffset"].as_u64().unwrap_or(t.scan);
                for b in bytes.iter().copied() {
                    scan_osc(t, b);
                }
                if let Some(code) = output.get("exitCode") {
                    t.value["status"] = json!("ended");
                    t.value["exitCode"] = code.clone();
                }
                if bytes.len() < 65536 {
                    break;
                }
            }
            changed |= before != t.value;
        }
        if changed {
            self.save(&items)?;
        }
        Ok(())
    }
    pub fn reap(&self, references: &HashSet<String>, processes: &Processes) -> Result<()> {
        self.refresh(processes)?;
        let mut items = self.items.lock().unwrap();
        let mut changed = false;
        for (id, t) in items.iter_mut() {
            if references.contains(id) {
                t.unreferenced = None;
                continue;
            }
            if t.process.is_none() {
                continue;
            }
            let since = t.unreferenced.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(15) {
                if let Some(p) = t.process.take() {
                    let _ = processes.request("process.stop", &json!({"processId":p,"force":true}));
                }
                t.value["status"] = json!("ended");
                t.value["exitCode"] = V::Null;
                changed = true;
            }
        }
        if changed {
            self.save(&items)?;
        }
        Ok(())
    }
    pub fn running(&self) -> Vec<String> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, t)| t.value["status"] == "running")
            .map(|(id, _)| id.clone())
            .collect()
    }
    pub fn restore(
        &self,
        ids: &[String],
        processes: &Processes,
        environment: &Arc<Mutex<Environment>>,
    ) {
        let mut items = self.items.lock().unwrap();
        for id in ids {
            if let Some(t) = items.get_mut(id) {
                t.process = None;
                if let Err(e) = start(t, processes, environment, &json!({})) {
                    t.value["status"] = json!("failed");
                    t.value["error"] = json!(e.message);
                }
            }
        }
        let _ = self.save(&items);
    }
}
fn start(
    t: &mut Terminal,
    processes: &Processes,
    environment: &Arc<Mutex<Environment>>,
    a: &V,
) -> Result<()> {
    let rows = a["rows"]
        .as_u64()
        .or(t.value["rows"].as_u64())
        .unwrap_or(24);
    let cols = a["columns"]
        .as_u64()
        .or(t.value["columns"].as_u64())
        .unwrap_or(80);
    let directory = if a.get("directory").is_some() {
        cwd(a)?
    } else {
        t.value["cwd"].as_str().unwrap_or("/workspace").into()
    };
    let spawned=processes.spawn(environment,&json!({"cwd":directory,"terminal":true,"rows":rows,"columns":cols,"env":{"TERM":"xterm-256color","COLORTERM":"truecolor","PROMPT_COMMAND":"printf '\\033]7;file://localhost%s\\007' \"$PWD\""},"label":"terminal"}))?;
    t.process = Some(required(&spawned, "processId")?.into());
    t.scan = 0;
    t.osc.clear();
    t.value["generation"] = json!(t.value["generation"].as_u64().unwrap_or(0) + 1);
    t.value["cwd"] = json!(directory);
    t.value["rows"] = json!(rows);
    t.value["columns"] = json!(cols);
    t.value["status"] = json!("running");
    t.value["exitCode"] = V::Null;
    t.value["error"] = V::Null;
    Ok(())
}
fn cwd(a: &V) -> Result<String> {
    let relative = a["directory"].as_str().unwrap_or("");
    if relative.starts_with('/')
        || relative.split('/').any(|p| p == "..")
        || relative.contains('\0')
    {
        return Err(Error::invalid("invalid terminal directory"));
    }
    Ok(if relative.is_empty() {
        "/workspace".into()
    } else {
        format!("/workspace/{relative}")
    })
}
fn scan_osc(t: &mut Terminal, b: u8) {
    if t.osc.is_empty() {
        if b == 27 {
            t.osc.push(b);
        }
        return;
    }
    if t.osc.len() == 1 && b != b']' {
        t.osc.clear();
        return;
    }
    t.osc.push(b);
    if t.osc.len() > 8192 {
        t.osc.clear();
        return;
    }
    let end = if b == 7 {
        Some(t.osc.len() - 1)
    } else if t.osc.ends_with(b"\x1b\\") {
        Some(t.osc.len() - 2)
    } else {
        None
    };
    if let Some(end) = end {
        let text = String::from_utf8_lossy(&t.osc[2..end]);
        if let Some((kind, payload)) = text.split_once(';') {
            match kind {
                "0" | "2" => {
                    t.value["title"] = json!(payload.trim().chars().take(80).collect::<String>())
                }
                "7" => {
                    if let Some(uri) = payload.strip_prefix("file://") {
                        if let Some(i) = uri.find('/') {
                            if let Some(path) = decode_cwd(&uri[i..]) {
                                t.value["cwd"] = json!(path);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        t.osc.clear();
    }
}
fn decode_cwd(path: &str) -> Option<String> {
    let input = path.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            if let (Some(a), Some(b)) = (
                (input[i + 1] as char).to_digit(16),
                (input[i + 2] as char).to_digit(16),
            ) {
                bytes.push((a * 16 + b) as u8);
                i += 3;
                continue;
            }
        }
        bytes.push(input[i]);
        i += 1;
    }
    let value = String::from_utf8(bytes).ok()?;
    if value.len() > 4096 || value.contains('\0') || value.split('/').any(|p| p == "..") {
        return None;
    }
    Some(value)
}
pub fn references(state: &V) -> HashSet<String> {
    fn walk(v: &V, out: &mut HashSet<String>) {
        match v {
            V::Object(o) => {
                if o.get("kind").and_then(V::as_str) == Some("terminal") {
                    if let Some(id) = o.get("id").and_then(V::as_str) {
                        out.insert(id.into());
                    }
                }
                for v in o.values() {
                    walk(v, out);
                }
            }
            V::Array(a) => {
                for v in a {
                    walk(v, out);
                }
            }
            _ => {}
        }
    }
    let mut refs = HashSet::new();
    if let Some(sessions) = state["sessions"].as_array() {
        for session in sessions {
            if session["archived"] != true && session["archivedAt"].is_null() {
                walk(session, &mut refs);
            }
        }
    }
    refs
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_terminal_metadata_is_preserved_without_blocking_workspace_startup() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join(".workspace/state/terminals.json");
        storage::write_json(&path, &json!({"format":99,"terminals":[]})).unwrap();
        let terminals = Terminals::load(d.path()).unwrap();
        assert_eq!(
            terminals.failure.as_ref().unwrap().kind,
            "unsupported_format"
        );
        assert_eq!(storage::read_json(&path).unwrap()["format"], 99);
        assert!(
            terminals
                .refresh(&Processes::new(Arc::new(
                    std::sync::atomic::AtomicUsize::new(0)
                )))
                .is_ok()
        );
    }
    #[test]
    fn osc_working_directories_decode_unicode_and_reject_parent_traversal() {
        assert_eq!(
            decode_cwd("/workspace/a%20b/%E4%B8%AD").unwrap(),
            "/workspace/a b/中"
        );
        assert!(decode_cwd("/workspace/%2e%2e/elsewhere").is_none());
    }
    #[test]
    fn osc_state_is_owned_without_client() {
        let mut t = Terminal {
            value: json!({}),
            process: None,
            scan: 0,
            osc: vec![],
            unreferenced: None,
        };
        for b in b"\x1b]2;Build\x07\x1b]7;file://localhost/workspace/src\x1b\\" {
            scan_osc(&mut t, *b);
        }
        assert_eq!(t.value["title"], "Build");
        assert_eq!(t.value["cwd"], "/workspace/src");
    }
    #[test]
    fn resource_registry_reloads_titles_but_not_dead_processes() {
        let d = tempfile::tempdir().unwrap();
        let registry = Terminals::load(d.path()).unwrap();
        storage::write_json(&registry.path,&json!({"format":1,"terminals":[{"id":"t-test","customTitle":"keep","status":"running"}]})).unwrap();
        let restored = Terminals::load(d.path()).unwrap();
        let items = restored.items.lock().unwrap();
        assert_eq!(items["t-test"].value["customTitle"], "keep");
        assert_eq!(items["t-test"].value["status"], "ended");
        assert!(items["t-test"].process.is_none());
    }
}
