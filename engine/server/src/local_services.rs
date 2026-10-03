//! Engine-owned intent/configuration and capability executor receipts. No local device side effects.
use crate::{
    protocol::{Error, Result},
    storage,
    workspace::{self, Workspace},
};
use serde_json::{Value as V, json};
use std::{fs::File, io::Read, sync::OnceLock};

const MAX_CONFIG: usize = 16 * 1024 * 1024;
static BOOT: OnceLock<String> = OnceLock::new();
fn boot() -> &'static str {
    BOOT.get_or_init(|| uuid::Uuid::new_v4().to_string())
}
fn load(w: &Workspace) -> Result<V> {
    let path = w.root.join(".workspace/state/local-services/proxy.json");
    if !path.exists() {
        return Ok(
            json!({"format":1,"desired":{"running":false,"mode":null,"selections":{}},"executor":null,"operation":null,"measuredEpoch":null}),
        );
    }
    let value = storage::read_json(&path)?;
    if value["format"] != 1 || !value["desired"].is_object() {
        return Err(Error::business(
            "unsupported_format",
            "invalid local service state",
        ));
    }
    Ok(value)
}
fn save(w: &mut Workspace, value: &V) -> Result<()> {
    storage::write_json(
        &w.root.join(".workspace/state/local-services/proxy.json"),
        value,
    )?;
    w.commit(w.clone())
}
fn active(value: &V) -> bool {
    value["executor"]["boot"].as_str() == Some(boot()) && value["executor"]["epoch"].is_string()
}
fn validate(value: &V, a: &V) -> Result<()> {
    if !active(value) || a["epoch"] != value["executor"]["epoch"] {
        return Err(Error::business(
            "stale_executor",
            "proxy executor lease is no longer active",
        ));
    }
    Ok(())
}
fn view(value: &V) -> V {
    json!({"desired":value["desired"],"operation":value["operation"],
        "executor":{"epoch":value["executor"]["epoch"],"active":active(value)},
        "measuredConfirmed":active(value) && value["measuredEpoch"] == value["executor"]["epoch"]})
}
pub fn projection(w: &Workspace) -> Result<V> {
    Ok(view(&load(w)?))
}
fn bounded(a: &V, key: &str, max: usize) -> Result<String> {
    let value = workspace::required(a, key)?;
    if value.is_empty() || value.len() > max || value.contains('\0') {
        return Err(Error::invalid("invalid service command argument"));
    }
    Ok(value.to_string())
}
fn template() -> Result<String> {
    let mut bytes = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let secret = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(include_str!("../resources/proxy-template.yaml").replace("@SECRET@", &secret))
}
fn measured(w: &mut Workspace, value: &mut V, a: &V) -> Result<()> {
    if !a["state"].is_object() {
        return Err(Error::invalid("service state must be an object"));
    }
    if serde_json::to_vec(&a["state"])
        .map_err(|_| Error::invalid("invalid measured state"))?
        .len()
        > 1024 * 1024
    {
        return Err(Error::invalid("proxy report exceeds 1 MiB"));
    }
    storage::write_json(
        &w.root.join(".workspace/state/services/proxy.json"),
        &json!({"format":1,"state":a["state"],"reportedAt":storage::now(),"epoch":a["epoch"]}),
    )?;
    value["measuredEpoch"] = a["epoch"].clone();
    Ok(())
}
/// Caller holds the workspace lock, routes the canonical service-document callback and notifies watches.
/// The callback is (workspace, documents method, filename, arguments); service is always proxy.
pub fn call(
    w: &mut Workspace,
    method: &str,
    a: &V,
    document: &mut dyn FnMut(&mut Workspace, &str, &str, &V) -> Result<V>,
) -> Result<V> {
    if a["serviceId"] != "proxy" {
        return Err(Error::invalid("unsupported local service"));
    }
    if !w.writable {
        return Err(Error::business("read_only", "workspace is not writable"));
    }
    let mut value = load(w)?;
    if method == "services.executor.register" {
        let executor = bounded(a, "executorId", 200)?;
        // Repeated registration by this instance is idempotent, but another instance retires it.
        if active(&value) && value["executor"]["id"] == executor {
            return Ok(json!({"epoch":value["executor"]["epoch"],"control":view(&value)}));
        }
        if value["operation"]["status"] == "pending" {
            value["operation"]["status"] = json!("interrupted");
        }
        let epoch = uuid::Uuid::new_v4().to_string();
        value["executor"] = json!({"id":executor,"epoch":epoch,"boot":boot()});
        value["measuredEpoch"] = V::Null;
        save(w, &value)?;
        return Ok(json!({"epoch":epoch,"control":view(&value)}));
    }
    validate(&value, a)?;
    match method {
        "services.executor.retire" => {
            if value["operation"]["status"] == "pending" {
                value["operation"]["status"] = json!("interrupted");
            }
            value["executor"] = V::Null;
            value["measuredEpoch"] = V::Null;
            save(w, &value)?;
            Ok(json!({"control":view(&value)}))
        }
        "services.report" => {
            measured(w, &mut value, a)?;
            save(w, &value)?;
            Ok(
                json!({"serviceId":"proxy","state":a["state"],"revision":w.revision,"control":view(&value)}),
            )
        }
        "services.complete" => {
            if value["operation"]["status"] != "pending"
                || value["operation"]["id"] != a["operationId"]
                || value["operation"]["epoch"] != a["epoch"]
            {
                return Err(Error::business(
                    "stale_operation",
                    "proxy operation is no longer pending",
                ));
            }
            let success = a["success"]
                .as_bool()
                .ok_or_else(|| Error::invalid("success must be boolean"))?;
            if success {
                match value["operation"]["name"].as_str().unwrap_or("") {
                    "start"
                        if a["state"]["phase"] != "running"
                            || a["state"]["pid"].as_u64().unwrap_or(0) == 0 =>
                    {
                        return Err(Error::business(
                            "unconfirmed",
                            "executor did not measure a running proxy",
                        ));
                    }
                    "stop"
                        if a["state"]["phase"] != "stopped"
                            || !a["state"]["pid"].is_null()
                            || a["state"]["stopUnconfirmed"] == true =>
                    {
                        return Err(Error::business(
                            "unconfirmed",
                            "executor did not confirm proxy shutdown",
                        ));
                    }
                    "setMode" if a["state"]["mode"] != value["desired"]["mode"] => {
                        return Err(Error::business(
                            "unconfirmed",
                            "executor did not confirm selected mode",
                        ));
                    }
                    "select" => {
                        let group = value["operation"]["targetGroup"].as_str().unwrap_or("");
                        let expected = &value["desired"]["selections"][group];
                        let normal = a["state"]["groups"]["groups"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .find(|entry| entry["name"] == group);
                        let global = &a["state"]["groups"]["global"];
                        let observed = normal.unwrap_or(global);
                        if observed["name"] != group || &observed["now"] != expected {
                            return Err(Error::business(
                                "unconfirmed",
                                "executor did not confirm selected node",
                            ));
                        }
                    }
                    _ => {}
                }
            }
            measured(w, &mut value, a)?;
            value["operation"]["status"] = json!(if success { "completed" } else { "failed" });
            value["operation"]["completedAt"] = json!(storage::now());
            // Deliberately retain no executor exception or command parameters (URLs can carry secrets).
            save(w, &value)?;
            Ok(
                json!({"serviceId":"proxy","state":a["state"],"revision":w.revision,"control":view(&value)}),
            )
        }
        "services.command" => {
            let name = workspace::required(a, "name")?;
            let args = a.get("args").cloned().unwrap_or_else(|| json!({}));
            if !args.is_object() {
                return Err(Error::invalid("command args must be object"));
            }
            if name == "publishLog" {
                let text = workspace::required(&args, "text")?;
                if text.len() > 1024 * 1024 || !args["expectedRevision"].is_u64() {
                    return Err(Error::invalid("invalid bounded log publication"));
                }
                let receipt = document(
                    w,
                    "documents.write",
                    "runtime.log",
                    &json!({"document":text,"expectedRevision":args["expectedRevision"]}),
                )?;
                return Ok(
                    json!({"operation":null,"revision":receipt["revision"],"control":view(&value)}),
                );
            }
            if name == "ensureConfig" || name == "importConfig" {
                let mut config = document(w, "documents.read", "config.yaml", &json!({}))?;
                if name == "ensureConfig" && config["document"].is_null() {
                    document(
                        w,
                        "documents.write",
                        "config.yaml",
                        &json!({"document":template()?,"expectedRevision":config["revision"]}),
                    )?;
                    config = document(w, "documents.read", "config.yaml", &json!({}))?;
                } else if name == "importConfig" {
                    let text = workspace::required(&args, "text")?;
                    if text.is_empty() || text.len() > MAX_CONFIG {
                        return Err(Error::invalid(
                            "proxy configuration must be 1 byte to 16 MiB",
                        ));
                    }
                    if !args["expectedRevision"].is_u64() {
                        return Err(Error::invalid("expectedRevision is required"));
                    }
                    document(
                        w,
                        "documents.write",
                        "config.yaml",
                        &json!({"document":text,"expectedRevision":args["expectedRevision"]}),
                    )?;
                    config = document(w, "documents.read", "config.yaml", &json!({}))?;
                }
                return Ok(
                    json!({"operation":null,"config":{"text":config["document"],"revision":config["revision"]},"control":view(&value)}),
                );
            }
            if value["operation"]["status"] == "pending" {
                return Err(Error::business(
                    "busy",
                    "a proxy operation still needs a measured completion",
                ));
            }
            let mut canonical = json!({});
            match name {
                "start" => value["desired"]["running"] = json!(true),
                "stop" => value["desired"]["running"] = json!(false),
                "checkConfig" | "refreshProviders" => {}
                "setMode" => {
                    let mode = workspace::required(&args, "mode")?;
                    if !["rule", "global", "direct"].contains(&mode) {
                        return Err(Error::invalid("invalid proxy mode"));
                    }
                    canonical["mode"] = json!(mode);
                    value["desired"]["mode"] = json!(mode);
                }
                "select" => {
                    let group = bounded(&args, "group", 1024)?;
                    let node = bounded(&args, "node", 1024)?;
                    canonical = json!({"group":group,"node":node});
                    value["desired"]["selections"][group] = json!(node);
                }
                "testNode" | "testGroup" => {
                    let field = if name == "testNode" { "name" } else { "group" };
                    canonical[field] = json!(bounded(&args, field, 1024)?);
                    if let Some(url) = args.get("url").filter(|v| !v.is_null()) {
                        let url = url
                            .as_str()
                            .ok_or_else(|| Error::invalid("invalid test URL"))?;
                        if url.len() > 8192
                            || !(url.starts_with("https://") || url.starts_with("http://"))
                            || url.contains(['\r', '\n', '\0'])
                        {
                            return Err(Error::invalid("invalid test URL"));
                        }
                        canonical["url"] = json!(url);
                    }
                    let timeout = match args.get("timeoutMs") {
                        None => 5000,
                        Some(value) => value
                            .as_u64()
                            .ok_or_else(|| Error::invalid("invalid test timeout"))?,
                    };
                    if !(1..=60000).contains(&timeout) {
                        return Err(Error::invalid("invalid test timeout"));
                    }
                    canonical["timeoutMs"] = json!(timeout);
                }
                _ => return Err(Error::invalid("unsupported proxy command")),
            }
            let config = if ["start", "checkConfig", "refreshProviders"].contains(&name) {
                let config = document(w, "documents.read", "config.yaml", &json!({}))?;
                if !config["document"].is_string() {
                    return Err(Error::business(
                        "missing_config",
                        "proxy configuration does not exist",
                    ));
                }
                json!({"text":config["document"],"revision":config["revision"]})
            } else {
                V::Null
            };
            let id = uuid::Uuid::new_v4().to_string();
            value["operation"] = json!({"id":id,"name":name,"status":"pending","epoch":a["epoch"],"requestedAt":storage::now()});
            if name == "select" {
                value["operation"]["targetGroup"] = canonical["group"].clone();
            }
            save(w, &value)?;
            Ok(
                json!({"operation":{"id":id,"name":name,"args":canonical,"config":config},"control":view(&value)}),
            )
        }
        _ => Err(Error::method()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    struct Harness {
        _dir: tempfile::TempDir,
        workspace: Workspace,
    }
    impl Harness {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let workspace = Workspace::load(dir.path().to_owned()).unwrap();
            Self {
                _dir: dir,
                workspace,
            }
        }
        fn call(&mut self, method: &str, args: V) -> Result<V> {
            call(&mut self.workspace, method, &args, &mut |w, m, key, a| {
                let path = w.root.join(".workspace/services/proxy").join(key);
                let sidecar = w
                    .root
                    .join(".workspace/documents/test")
                    .join(format!("{key}.json"));
                let body = fs::read_to_string(&path).ok();
                let digest = body.as_ref().map(|text| storage::hash(text.as_bytes()));
                let mut meta = if sidecar.exists() {
                    storage::read_json(&sidecar)?
                } else {
                    json!({"revision":0,"hash":null})
                };
                if meta["hash"] != json!(digest) {
                    meta["revision"] = json!(meta["revision"].as_u64().unwrap() + 1);
                    meta["hash"] = json!(digest);
                    storage::write_json(&sidecar, &meta)?;
                }
                if m == "documents.read" {
                    return Ok(json!({"document":body,"revision":meta["revision"]}));
                }
                if a["expectedRevision"] != meta["revision"] {
                    return Err(Error::business("conflict", "service document changed"));
                }
                let text = workspace::required(a, "document")?;
                storage::atomic(&path, text.as_bytes())?;
                meta["revision"] = json!(meta["revision"].as_u64().unwrap() + 1);
                meta["hash"] = json!(storage::hash(text.as_bytes()));
                storage::write_json(&sidecar, &meta)?;
                Ok(json!({"revision":meta["revision"]}))
            })
        }
        fn register(&mut self, id: &str) -> V {
            self.call(
                "services.executor.register",
                json!({"serviceId":"proxy","executorId":id}),
            )
            .unwrap()["epoch"]
                .clone()
        }
        fn command(&mut self, epoch: &V, name: &str, args: V) -> Result<V> {
            self.call(
                "services.command",
                json!({"serviceId":"proxy","epoch":epoch,"name":name,"args":args}),
            )
        }
    }
    #[test]
    fn config_creation_is_inactive_unique_and_cas_preserves_external_edits() {
        let mut h = Harness::new();
        let epoch = h.register("one");
        let first = h.command(&epoch, "ensureConfig", json!({})).unwrap();
        let text = first["config"]["text"].as_str().unwrap();
        assert!(text.contains("enable: false"));
        assert!(text.contains("127.0.0.1:19090"));
        assert!(text.contains("device: workflow-tun"));
        assert!(text.contains("iproute2-table-index: 9500"));
        assert!(text.contains("auto-redirect: false"));
        assert!(!text.contains("@SECRET@"));
        assert!(first["operation"].is_null());
        assert_eq!(
            first["config"],
            h.command(&epoch, "ensureConfig", json!({})).unwrap()["config"]
        );
        assert!(
            !h.workspace
                .root
                .join(".workspace/state/services/proxy.json")
                .exists()
        );
        fs::write(
            h.workspace
                .root
                .join(".workspace/services/proxy/config.yaml"),
            "mode: direct\n",
        )
        .unwrap();
        let error = h
            .command(
                &epoch,
                "importConfig",
                json!({"text":"mode: rule\n","expectedRevision":first["config"]["revision"]}),
            )
            .unwrap_err();
        assert_eq!(error.kind, "conflict");
        assert_eq!(
            fs::read_to_string(
                h.workspace
                    .root
                    .join(".workspace/services/proxy/config.yaml")
            )
            .unwrap(),
            "mode: direct\n"
        );
        assert_ne!(template().unwrap(), template().unwrap());
    }
    #[test]
    fn pending_intent_is_not_a_measured_success_and_stale_executors_cannot_complete() {
        let mut h = Harness::new();
        let one = h.register("one");
        assert_eq!(
            h.command(&one, "start", json!({})).unwrap_err().kind,
            "missing_config"
        );
        h.command(&one, "ensureConfig", json!({})).unwrap();
        let ticket = h.command(&one, "start", json!({})).unwrap();
        assert_eq!(ticket["control"]["desired"]["running"], true);
        assert_eq!(ticket["control"]["operation"]["status"], "pending");
        assert_eq!(ticket["control"]["measuredConfirmed"], false);
        assert_eq!(
            h.command(&one, "start", json!({})).unwrap_err().kind,
            "busy"
        );
        let bad = json!({"serviceId":"proxy","epoch":one,"operationId":ticket["operation"]["id"],"success":true,"state":{"phase":"stopped","pid":null}});
        assert_eq!(
            h.call("services.complete", bad).unwrap_err().kind,
            "unconfirmed"
        );
        let two = h.register("two");
        assert_ne!(one, two);
        assert_eq!(
            h.call(
                "services.report",
                json!({"serviceId":"proxy","epoch":one,"state":{"phase":"running"}})
            )
            .unwrap_err()
            .kind,
            "stale_executor"
        );
        assert_eq!(h.call("services.complete",json!({"serviceId":"proxy","epoch":one,"operationId":ticket["operation"]["id"],"success":true,"state":{"phase":"running","pid":99}})).unwrap_err().kind,"stale_executor");
        assert_eq!(
            projection(&h.workspace).unwrap()["operation"]["status"],
            "interrupted"
        );
    }
    #[test]
    fn previous_boot_epoch_is_inactive_and_invalid_commands_do_not_create_tickets() {
        let mut h = Harness::new();
        let epoch = h.register("one");
        assert!(
            h.command(&epoch, "setMode", json!({"mode":"arbitrary"}))
                .is_err()
        );
        assert!(
            h.command(
                &epoch,
                "testNode",
                json!({"name":"node","url":"file:///secret"})
            )
            .is_err()
        );
        assert!(
            h.command(&epoch, "testNode", json!({"name":"node","timeoutMs":60001}))
                .is_err()
        );
        let mut value = load(&h.workspace).unwrap();
        value["executor"]["boot"] = json!("previous-engine-process");
        save(&mut h.workspace, &value).unwrap();
        assert_eq!(
            projection(&h.workspace).unwrap()["executor"]["active"],
            false
        );
        assert_eq!(
            h.command(&epoch, "stop", json!({})).unwrap_err().kind,
            "stale_executor"
        );
    }
    #[test]
    fn failure_receipt_records_measurement_without_erasing_desired_intent() {
        let mut h = Harness::new();
        let epoch = h.register("one");
        h.command(&epoch, "ensureConfig", json!({})).unwrap();
        let ticket = h.command(&epoch, "start", json!({})).unwrap();
        let receipt=h.call("services.complete",json!({"serviceId":"proxy","epoch":epoch,"operationId":ticket["operation"]["id"],"success":false,"state":{"phase":"error","pid":null}})).unwrap();
        assert_eq!(receipt["control"]["operation"]["status"], "failed");
        assert_eq!(receipt["control"]["desired"]["running"], true);
        assert_eq!(receipt["state"]["phase"], "error");
        assert_eq!(receipt["control"]["measuredConfirmed"], true);
    }
    #[test]
    fn selected_node_requires_matching_readback_and_test_urls_are_not_persisted() {
        let mut h = Harness::new();
        let epoch = h.register("one");
        let ticket = h
            .command(&epoch, "select", json!({"group":"choice","node":"chosen"}))
            .unwrap();
        let receipt = |node: &str| json!({"serviceId":"proxy","epoch":epoch,"operationId":ticket["operation"]["id"],"success":true,"state":{"groups":{"groups":[{"name":"choice","now":node}],"global":null}}});
        assert_eq!(
            h.call("services.complete", receipt("other"))
                .unwrap_err()
                .kind,
            "unconfirmed"
        );
        assert_eq!(
            h.call("services.complete", receipt("chosen")).unwrap()["control"]["operation"]["status"],
            "completed"
        );
        let url = "https://example.invalid/ping?token=fixture-private";
        let test = h
            .command(
                &epoch,
                "testNode",
                json!({"name":"node","url":url,"timeoutMs":1000}),
            )
            .unwrap();
        assert_eq!(test["operation"]["args"]["url"], url);
        assert!(
            !fs::read_to_string(
                h.workspace
                    .root
                    .join(".workspace/state/local-services/proxy.json")
            )
            .unwrap()
            .contains("fixture-private")
        );
    }
}
