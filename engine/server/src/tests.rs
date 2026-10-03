use super::*;
use crate::state as workspace;
use environment::persist as storage;
use serde_json::{Value as V, json};
use std::{
    fs::{self, File},
    io,
};
fn j(value: &impl serde::Serialize) -> V {
    serde_json::to_value(value).unwrap()
}
fn valid_spec(value: &V) -> environment::Result<()> {
    let spec = environment::json::strict_json::<environment::EnvironmentSpec>(
        &serde_json::to_vec(value).unwrap(),
    )
    .map_err(|e| environment::Error::invalid(&e.to_string()))?;
    environment::validate(&spec)
}
mod layout {
    use super::*;
    pub fn apply(before: &V, op: &V) -> V {
        j(&workflow_workspace::layout::apply(
            &environment::json::strict_json(&serde_json::to_vec(before).unwrap()).unwrap(),
            &environment::json::strict_json(&serde_json::to_vec(op).unwrap()).unwrap(),
        ))
    }
    pub fn normalize(value: &mut V) {
        let mut typed: workflow_workspace::Workbench =
            environment::json::strict_json(&serde_json::to_vec(value).unwrap()).unwrap();
        workflow_workspace::layout::normalize(&mut typed);
        *value = j(&typed);
    }
    pub fn empty() -> V {
        j(&workflow_workspace::Workbench::default())
    }
}
use std::io::BufRead;
#[test]
fn files_only_connection_does_not_install_an_unused_environment() {
    let (temp, _workspace) = temp_workspace();
    let options = environment::Options {
        root: temp.path().to_owned(),
        runtime: Some(temp.path().join("runtime-must-not-be-started")),
        image: Some(temp.path().join("unused-image")),
        ..Default::default()
    };
    let environment = Arc::new(Mutex::new(
        environment::Environment::load(options, Arc::new(AtomicUsize::new(0))).unwrap(),
    ));
    environment::Environment::reconcile_changed(&environment);
    assert!(!environment.lock().unwrap().building);
    assert!(
        environment
            .lock()
            .unwrap()
            .state
            .requested_spec_hash
            .is_none()
    );
    assert_eq!(
        j(&environment.lock().unwrap().status())["phase"],
        "not_installed"
    );
}
#[test]
fn atomic_create_and_upload_publication_never_replace_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("file.txt");
    storage::create_atomic(&destination, b"original").unwrap();
    assert!(storage::create_atomic(&destination, b"replacement").is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"original");
    let staged = dir.path().join("staged");
    fs::write(&staged, b"upload").unwrap();
    assert_eq!(
        storage::publish_new(&staged, &destination)
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(&staged).unwrap(), b"upload");
    assert_eq!(fs::read(&destination).unwrap(), b"original");
    let published = dir.path().join("new.txt");
    storage::publish_new(&staged, &published).unwrap();
    assert!(!staged.exists());
    assert_eq!(fs::read(&published).unwrap(), b"upload");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}
fn temp_workspace() -> (tempfile::TempDir, workspace::Workspace) {
    let temp = tempfile::tempdir().unwrap();
    let w = workspace::Workspace::load(temp.path().to_owned()).unwrap();
    (temp, w)
}
fn cmd(w: &mut workspace::Workspace, name: &str, a: V) -> V {
    j(&w.command(name, &a).unwrap().value)
}
#[test]
fn draft_detects_external_change_before_first_keystroke_and_survives_restart() {
    let (t, mut w) = temp_workspace();
    fs::write(t.path().join("a.txt"), "base").unwrap();
    let snap = cmd(&mut w, "openFile", json!({"path":"a.txt"}));
    fs::write(t.path().join("a.txt"), "external").unwrap();
    cmd(
        &mut w,
        "editFile",
        json!({"path":"a.txt","text":"mine","shown":snap["disk"]}),
    );
    assert_eq!(
        cmd(&mut w, "saveFile", json!({"path":"a.txt"}))["kind"],
        "conflict"
    );
    drop(w);
    let mut w = workspace::Workspace::load(t.path().to_owned()).unwrap();
    assert_eq!(j(&w.state)["drafts"]["a.txt"]["text"], "mine");
    cmd(
        &mut w,
        "resolveConflict",
        json!({"path":"a.txt","resolution":"keep_mine"}),
    );
    assert_eq!(
        cmd(&mut w, "saveFile", json!({"path":"a.txt"}))["kind"],
        "saved"
    );
    assert_eq!(fs::read_to_string(t.path().join("a.txt")).unwrap(), "mine");
}
#[test]
fn archive_save_preflights_all_files_and_closing_panels_keeps_drafts() {
    let (t, mut w) = temp_workspace();
    let id = cmd(&mut w, "createSession", json!({}));
    for p in ["a", "b"] {
        fs::write(t.path().join(p), "base").unwrap();
        cmd(
            &mut w,
            "applyLayout",
            json!({"sessionId":id,"op":{"type":"open","target":{"kind":"file","path":p}}}),
        );
        let snap = cmd(&mut w, "openFile", json!({"path":p}));
        cmd(
            &mut w,
            "editFile",
            json!({"path":p,"text":"draft","shown":snap["disk"]}),
        );
    }
    assert_eq!(
        cmd(&mut w, "archiveSession", json!({"id":id}))["kind"],
        "needsDecision"
    );
    fs::write(t.path().join("b"), "external").unwrap();
    assert_eq!(
        cmd(
            &mut w,
            "archiveSession",
            json!({"id":id,"decision":"save_all"})
        )["kind"],
        "saveConflict"
    );
    assert_eq!(fs::read_to_string(t.path().join("a")).unwrap(), "base");
    cmd(
        &mut w,
        "applyLayout",
        json!({"sessionId":id,"op":{"type":"close","panelIds":["p1"]}}),
    );
    assert_eq!(j(&w.state)["drafts"]["a"]["text"], "draft");
    assert_eq!(
        cmd(
            &mut w,
            "archiveSession",
            json!({"id":id,"decision":"keep_drafts"})
        )["kind"],
        "archived"
    );
    assert!(j(&w.state)["drafts"]["b"].is_object());
}
#[test]
fn composer_acknowledge_never_clears_a_newer_edit() {
    let (_t, mut w) = temp_workspace();
    let first = json!({"conversationId":"c","revision":1,"text":"first","attachments":[]});
    cmd(
        &mut w,
        "editComposer",
        json!({"draft":first,"expectedRevision":0}),
    );
    let next = json!({"conversationId":"c","revision":2,"text":"second","attachments":[]});
    cmd(
        &mut w,
        "editComposer",
        json!({"draft":next,"expectedRevision":1}),
    );
    assert_eq!(
        cmd(&mut w, "acknowledgeComposer", json!({"submitted":first}))["text"],
        "second"
    );
    assert!(
        w.command("editComposer", &json!({"draft":first,"expectedRevision":1}))
            .is_err()
    );
    let cleared = cmd(&mut w, "acknowledgeComposer", json!({"submitted":next}));
    assert_eq!(cleared["text"], "");
    assert_eq!(cleared["revision"], 3);
}
#[test]
fn shared_resource_only_protects_most_recent_live_session() {
    let (_t, mut w) = temp_workspace();
    let a = cmd(&mut w, "createSession", json!({}));
    let b = cmd(&mut w, "createSession", json!({}));
    for id in [&a, &b] {
        cmd(
            &mut w,
            "applyLayout",
            json!({"sessionId":id,"op":{"type":"open","target":{"kind":"file","path":"dirty"}}}),
        );
    }
    cmd(
        &mut w,
        "editFile",
        json!({"path":"dirty","text":"draft","shown":workflow_filework::FileVersion::default()}),
    );
    let time = storage::now() - 2 * 86_400_000;
    w.state.workspace.sessions[0].last_used_at = time - 1;
    w.state.workspace.sessions[1].last_used_at = time;
    w.maintenance().unwrap();
    assert!(j(&w.state)["sessions"][0]["archivedAt"].is_number());
    assert!(j(&w.state)["sessions"][1]["archivedAt"].is_null());
}
#[test]
fn internal_paths_symlinks_and_no_overwrite_are_enforced() {
    let (t, mut w) = temp_workspace();
    assert!(w.command("openFile", &json!({"path":"../escape"})).is_err());
    assert!(
        w.command(
            "openFile",
            &json!({"path":".workspace/state/workspace.json"})
        )
        .is_err()
    );
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "x").unwrap();
    std::os::unix::fs::symlink(outside.path(), t.path().join("link")).unwrap();
    assert!(
        w.command("openFile", &json!({"path":"link/secret"}))
            .is_err()
    );
    assert_eq!(
        cmd(
            &mut w,
            "createFile",
            json!({"path":"x","data":encode_blob(b"original")})
        )["kind"],
        "done"
    );
    assert_eq!(
        cmd(
            &mut w,
            "createFile",
            json!({"path":"x","data":encode_blob(b"replace")})
        )["kind"],
        "failed"
    );
    assert_eq!(fs::read(t.path().join("x")).unwrap(), b"original");
}
#[test]
fn trash_restore_move_and_drafts_follow_paths() {
    let (t, mut w) = temp_workspace();
    fs::create_dir(t.path().join("dir")).unwrap();
    fs::write(t.path().join("dir/a.txt"), "a").unwrap();
    let snap = cmd(&mut w, "openFile", json!({"path":"dir/a.txt"}));
    cmd(
        &mut w,
        "editFile",
        json!({"path":"dir/a.txt","text":"draft","shown":snap["disk"]}),
    );
    assert_eq!(
        cmd(&mut w, "movePath", json!({"from":"dir","to":"renamed"}))["kind"],
        "done"
    );
    assert_eq!(j(&w.state)["drafts"]["renamed/a.txt"]["text"], "draft");
    let trash = cmd(&mut w, "trashPath", json!({"path":"renamed"}));
    assert_eq!(trash["entry"]["path"], "renamed");
    assert!(j(&w.state)["drafts"]["renamed/a.txt"].is_object());
    let restored = cmd(
        &mut w,
        "restoreFromTrash",
        json!({"id":trash["entry"]["id"]}),
    );
    assert_eq!(restored["kind"], "restored");
    assert!(t.path().join("renamed/a.txt").is_file());
}
#[test]
fn corrupt_state_is_preserved_and_newer_schema_never_overwritten() {
    let (t, mut w) = temp_workspace();
    cmd(&mut w, "createSession", json!({"name":"retained"}));
    cmd(&mut w, "flush", json!({}));
    let path = t.path().join(".workspace/state/workspace.json");
    fs::write(&path, b"{truncated").unwrap();
    let recovered = workspace::Workspace::load(t.path().to_owned()).unwrap();
    assert_eq!(j(&recovered.state)["sessions"][0]["name"], "retained");
    assert!(
        fs::read_dir(t.path().join(".workspace/corrupt"))
            .unwrap()
            .count()
            > 0
    );
    fs::write(&path, b"{\"format\":77}").unwrap();
    let mut unknown = workspace::Workspace::load(t.path().to_owned()).unwrap();
    assert_eq!(j(&unknown.state)["status"], "failed");
    assert!(unknown.command("createSession", &json!({})).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"{\"format\":77}");
}
#[test]
fn environment_schema_accepts_arrays_and_rejects_invalid_or_duplicate_entries() {
    assert!(valid_spec(&json!({"version":1,"python":["3.14","3.13.5"],"node":[],"post_scripts":[{"id":"setup","run":"true","user":"root"}]})).is_ok());
    for v in [
        json!({"version":1,"python":"3.14"}),
        json!({"version":1,"node":["24","24"]}),
        json!({"version":1,"python":["3;rm"]}),
        json!({"version":1,"post_scripts":[{"id":"x","run":"true","user":"device_root"}]}),
    ] {
        assert!(valid_spec(&v).is_err());
    }
}
fn approximately(a: &V, b: &V) -> bool {
    match (a, b) {
        (V::Number(a), V::Number(b)) => (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-8,
        (V::Array(a), V::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| approximately(a, b))
        }
        (V::Object(a), V::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|b| approximately(v, b)))
        }
        _ => a == b,
    }
}
#[test]
fn layout_matches_kotlin_oracle_and_normalization_is_idempotent() {
    let fixture = std::env::var("WORKFLOW_LAYOUT_ORACLE").ok();
    let input: Box<dyn BufRead> = if let Some(path) = fixture {
        Box::new(io::BufReader::new(File::open(path).unwrap()))
    } else {
        Box::new(io::Cursor::new(include_bytes!("../fixtures/layout.jsonl")))
    };
    let mut count = 0;
    for line in input.lines() {
        let record: V = serde_json::from_str(&line.unwrap()).unwrap();
        let result = layout::apply(&record["before"], &record["op"]);
        assert!(
            approximately(&result, &record["after"]),
            "seed={} step={} op={}\nactual={}\nexpected={}",
            record["seed"],
            record["step"],
            record["op"],
            result,
            record["after"]
        );
        let mut normalized = result.clone();
        layout::normalize(&mut normalized);
        assert!(
            approximately(&result, &normalized),
            "normalization not idempotent"
        );
        count += 1;
    }
    assert!(count >= 40);
}

#[test]
fn malformed_layout_is_reset_without_losing_session_or_unknown_drafts() {
    let (t, mut w) = temp_workspace();
    cmd(&mut w, "createSession", json!({"name":"retained"}));
    w.persist().unwrap();
    let state_path = t.path().join(".workspace/state/workspace.json");
    let mut broken: V = storage::read_json(&state_path).unwrap();
    broken["state"]["sessions"][0]["workbench"] = json!({"files":"damaged","panels":[]});
    storage::write_json(&state_path, &broken).unwrap();
    let loaded = workspace::Workspace::load(t.path().to_owned()).unwrap();
    assert_eq!(
        j(&loaded.state)["sessions"][0]["workbench"],
        layout::empty()
    );
    let path = t.path().join(".workspace/state/workspace.json");
    let mut document: V = storage::read_json(&path).unwrap();
    document["state"]["drafts"]["a"] = json!({"format":2,"path":"a","text":"future"});
    storage::write_json(&path, &document).unwrap();
    let loaded = workspace::Workspace::load(t.path().to_owned()).unwrap();
    assert!(!loaded.writable);
    assert_eq!(storage::read_json::<V>(&path).unwrap(), document);
}

#[test]
fn environment_corruption_does_not_block_files_and_unknown_format_is_preserved() {
    let (t, _) = temp_workspace();
    let options = environment::Options {
        root: t.path().to_owned(),
        ..Default::default()
    };
    let running = Arc::new(AtomicUsize::new(0));
    environment::Environment::load(options.clone(), running.clone()).unwrap();
    let path = t.path().join(".workspace/environment/environment.json");
    fs::write(&path, b"{truncated").unwrap();
    let recovered = environment::Environment::load(options.clone(), running.clone()).unwrap();
    assert_eq!(j(&recovered.status())["phase"], "failed");
    assert!(
        fs::read_dir(t.path().join(".workspace/corrupt"))
            .unwrap()
            .count()
            > 0
    );
    fs::write(&path, b"{\"format\":4}").unwrap();
    let unknown = environment::Environment::load(options, running).unwrap();
    assert_eq!(j(&unknown.status())["phase"], "failed");
    assert_eq!(fs::read(&path).unwrap(), b"{\"format\":4}");
}

#[test]
fn external_config_changes_win_compare_and_swap_and_invalid_files_stay_untouched() {
    let (t, mut w) = temp_workspace();
    let path = t.path().join(".workspace/config.json");
    let mut externally_changed = j(&w.state)["config"].clone();
    externally_changed["appearance"]["theme"] = json!("dark");
    storage::write_json(&path, &externally_changed).unwrap();
    let stale = j(&w.state)["config"].clone();
    let revision = w.revision;
    assert_eq!(
        cmd(
            &mut w,
            "updateConfig",
            json!({"config":stale,"expectedRevision":revision})
        )["kind"],
        "conflict"
    );
    assert_eq!(j(&w.state)["config"]["appearance"]["theme"], "dark");
    fs::write(&path, "{invalid").unwrap();
    let revision = w.revision;
    assert_eq!(
        cmd(
            &mut w,
            "updateConfig",
            json!({"config":stale,"expectedRevision":revision})
        )["kind"],
        "blocked"
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "{invalid");
}

#[test]
fn saving_preserves_executable_mode_and_move_never_overwrites_orphan_drafts() {
    use std::os::unix::fs::PermissionsExt;
    let (t, mut w) = temp_workspace();
    let file = t.path().join("run.sh");
    fs::write(&file, "old").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o751)).unwrap();
    let opened = cmd(&mut w, "openFile", json!({"path":"run.sh"}));
    cmd(
        &mut w,
        "editFile",
        json!({"path":"run.sh","shown":opened["disk"],"text":"new"}),
    );
    assert_eq!(
        cmd(&mut w, "saveFile", json!({"path":"run.sh"}))["kind"],
        "saved"
    );
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o751
    );
    cmd(
        &mut w,
        "editFile",
        json!({"path":"destination","shown":workflow_filework::FileVersion::default(),"text":"orphan"}),
    );
    let result = cmd(
        &mut w,
        "movePath",
        json!({"from":"run.sh","to":"destination"}),
    );
    // A clean source still must not silently shadow an orphan destination draft.
    assert_eq!(result["kind"], "failed");
    assert!(file.exists());
    assert_eq!(j(&w.state)["drafts"]["destination"]["text"], "orphan");
}

#[test]
fn nested_service_assets_are_explicit_safe_paths() {
    let (_t, mut w) = temp_workspace();
    assert_eq!(
        cmd(
            &mut w,
            "createFile",
            json!({"path":".workspace/services/example/providers/list.yaml","data":encode_blob(b"items: []\n")})
        )["kind"],
        "done"
    );
    assert!(
        w.filework
            .validate_path(".workspace/services/example/providers/../state")
            .is_err()
    );
    let file = cmd(
        &mut w,
        "openFile",
        json!({"path":".workspace/services/example/providers/list.yaml"}),
    );
    assert_eq!(file["diskText"], "items: []\n");
}
