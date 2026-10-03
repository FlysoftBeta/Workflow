use super::*;
fn fixture() -> (tempfile::TempDir, FileWork, FileWorkState) {
    let d = tempfile::tempdir().unwrap();
    let store = Store::open(d.path()).unwrap();
    let files = FileWork::new(d.path().to_owned(), store);
    (d, files, FileWorkState::default())
}
fn valid(_: &str, _: &str) -> Result<()> {
    Ok(())
}
#[test]
fn first_edit_uses_shown_version_and_conflict_requires_explicit_resolution() {
    let (d, files, mut state) = fixture();
    stdfs::write(d.path().join("a"), "base").unwrap();
    let open = files.open(&mut state, "a").unwrap();
    stdfs::write(d.path().join("a"), "external").unwrap();
    files.edit(&mut state, "a", "mine", &open.disk).unwrap();
    assert!(matches!(
        files.save(&mut state, "a", None, valid).unwrap(),
        SaveOutcome::Conflict { .. }
    ));
    let encoded = serde_json::to_vec(&state).unwrap();
    let mut state: FileWorkState = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(state.drafts["a"].text, "mine");
    files
        .resolve_conflict(&mut state, "a", ConflictResolution::KeepMine)
        .unwrap();
    assert!(matches!(
        files.save(&mut state, "a", None, valid).unwrap(),
        SaveOutcome::Saved { .. }
    ));
    assert_eq!(stdfs::read_to_string(d.path().join("a")).unwrap(), "mine");
    assert!(state.drafts.is_empty())
}
#[test]
fn archive_preflights_every_file_before_writing() {
    let (d, files, mut state) = fixture();
    for p in ["a", "b"] {
        stdfs::write(d.path().join(p), "base").unwrap();
        let open = files.open(&mut state, p).unwrap();
        files.edit(&mut state, p, "draft", &open.disk).unwrap();
    }
    let refs = vec![
        ResourceRef::File { path: "a".into() },
        ResourceRef::File { path: "b".into() },
    ];
    assert!(matches!(
        files.archive(&mut state, &refs, None, valid).unwrap(),
        ArchiveOutcome::NeedsDecision { .. }
    ));
    stdfs::write(d.path().join("b"), "external").unwrap();
    assert!(matches!(
        files
            .archive(&mut state, &refs, Some(ArchiveDecision::SaveAll), valid)
            .unwrap(),
        ArchiveOutcome::SaveConflict { .. }
    ));
    assert_eq!(stdfs::read_to_string(d.path().join("a")).unwrap(), "base");
    assert!(matches!(
        files
            .archive(&mut state, &refs, Some(ArchiveDecision::KeepDrafts), valid)
            .unwrap(),
        ArchiveOutcome::Archived { .. }
    ));
    assert_eq!(state.drafts.len(), 2);
    files
        .archive(&mut state, &refs, Some(ArchiveDecision::Discard), valid)
        .unwrap();
    assert!(state.drafts.is_empty())
}
#[test]
fn composer_acknowledge_never_clears_newer_revision() {
    let (_, files, mut state) = fixture();
    let mut first = ComposerDraft::empty("c");
    first.revision = 1;
    first.text = "first".into();
    files.edit_composer(&mut state, first.clone(), 0).unwrap();
    let mut next = first.clone();
    next.revision = 2;
    next.text = "second".into();
    files.edit_composer(&mut state, next.clone(), 1).unwrap();
    assert_eq!(
        state.acknowledge_composer(first.clone()).unwrap().text,
        "second"
    );
    assert!(files.edit_composer(&mut state, first, 1).is_err());
    let cleared = state.acknowledge_composer(next).unwrap();
    assert_eq!(cleared.revision, 3);
    assert!(!cleared.has_content())
}
#[test]
fn archive_protects_only_newest_live_reference_with_deterministic_ties() {
    let (_, files, mut state) = fixture();
    files
        .edit(&mut state, "dirty", "draft", &FileVersion::default())
        .unwrap();
    let a = ArchiveCandidate {
        id: "a".into(),
        last_used_at: 1,
        created_at: 1,
        resources: vec![ResourceRef::File {
            path: "dirty".into(),
        }],
    };
    let b = ArchiveCandidate {
        id: "b".into(),
        last_used_at: 2,
        ..a.clone()
    };
    assert_eq!(
        protected_sessions(&state, &[a, b]),
        ["b".to_owned()].into_iter().collect()
    );
}
#[test]
fn private_paths_symlinks_and_atomic_create_are_enforced() {
    let (d, files, mut state) = fixture();
    for p in [
        "../escape",
        ".workspace/state/workspace.json",
        ".workspace/services/x/providers/../state",
    ] {
        assert!(files.open(&mut state, p).is_err())
    }
    let outside = tempfile::tempdir().unwrap();
    stdfs::write(outside.path().join("secret"), "x").unwrap();
    std::os::unix::fs::symlink(outside.path(), d.path().join("link")).unwrap();
    assert!(files.open(&mut state, "link/secret").is_err());
    let op = FileOperation::CreateFile {
        path: "x".into(),
        data: b"original".to_vec(),
    };
    assert_eq!(files.file_operation(&mut state, &op), FileOutcome::Done);
    assert!(matches!(
        files.file_operation(&mut state, &op),
        FileOutcome::Failed { .. }
    ));
    assert_eq!(stdfs::read(d.path().join("x")).unwrap(), b"original")
}
#[test]
fn moves_preserve_drafts_attachments_and_trash_contents() {
    let (d, files, mut state) = fixture();
    stdfs::create_dir(d.path().join("dir")).unwrap();
    stdfs::write(d.path().join("dir/a.txt"), "a").unwrap();
    let open = files.open(&mut state, "dir/a.txt").unwrap();
    files
        .edit(&mut state, "dir/a.txt", "draft", &open.disk)
        .unwrap();
    let mut composer = ComposerDraft::empty("c");
    composer.revision = 1;
    composer.attachments.push(Attachment {
        path: "dir/a.txt".into(),
        mime_type: None,
        extra: Default::default(),
    });
    files.edit_composer(&mut state, composer, 0).unwrap();
    assert_eq!(
        files.file_operation(
            &mut state,
            &FileOperation::Move {
                from: "dir".into(),
                to: "renamed".into()
            }
        ),
        FileOutcome::Done
    );
    assert_eq!(state.drafts["renamed/a.txt"].text, "draft");
    assert_eq!(state.composers["c"].attachments[0].path, "renamed/a.txt");
    let FileOutcome::Trashed { entry } = files.file_operation(
        &mut state,
        &FileOperation::Trash {
            path: "renamed".into(),
        },
    ) else {
        panic!("trash failed")
    };
    assert!(state.drafts.contains_key("renamed/a.txt"));
    assert!(
        matches!(files.file_operation(&mut state,&FileOperation::Restore{id:entry.id}),FileOutcome::Restored{path}if path=="renamed")
    );
    assert!(d.path().join("renamed/a.txt").is_file())
}
#[test]
fn saves_preserve_executable_mode_and_moves_reject_orphan_drafts() {
    use std::os::unix::fs::PermissionsExt;
    let (d, files, mut state) = fixture();
    let p = d.path().join("run.sh");
    stdfs::write(&p, "old").unwrap();
    stdfs::set_permissions(&p, stdfs::Permissions::from_mode(0o751)).unwrap();
    let open = files.open(&mut state, "run.sh").unwrap();
    files.edit(&mut state, "run.sh", "new", &open.disk).unwrap();
    files.save(&mut state, "run.sh", None, valid).unwrap();
    assert_eq!(
        stdfs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o751
    );
    files
        .edit(&mut state, "destination", "orphan", &FileVersion::default())
        .unwrap();
    assert!(matches!(
        files.file_operation(
            &mut state,
            &FileOperation::Move {
                from: "run.sh".into(),
                to: "destination".into()
            }
        ),
        FileOutcome::Failed { .. }
    ));
    assert!(p.exists());
    assert_eq!(state.drafts["destination"].text, "orphan")
}
#[test]
fn imports_allocate_at_commit_and_uploads_enforce_lengths_offsets_and_limits() {
    let (d, files, _) = fixture();
    let mut uploads = Uploads::default();
    let dest = UploadDestination::Allocate {
        directory: "".into(),
        name: "bundle.tar.gz".into(),
    };
    let a = files.begin_upload(&mut uploads, dest.clone(), 3).unwrap();
    let b = files.begin_upload(&mut uploads, dest, 3).unwrap();
    assert!(
        files
            .upload_chunk(&mut uploads, &a.upload_id, 1, b"bad")
            .is_err()
    );
    assert!(files.commit_upload(&mut uploads, &a.upload_id).is_err());
    for id in [&a.upload_id, &b.upload_id] {
        files.upload_chunk(&mut uploads, id, 0, b"new").unwrap();
    }
    stdfs::write(d.path().join("bundle.tar.gz"), "original").unwrap();
    assert_eq!(
        files.commit_upload(&mut uploads, &a.upload_id).unwrap(),
        UploadOutcome::Done {
            path: "bundle (1).tar.gz".into()
        }
    );
    assert_eq!(
        files.commit_upload(&mut uploads, &b.upload_id).unwrap(),
        UploadOutcome::Done {
            path: "bundle (2).tar.gz".into()
        }
    );
    assert_eq!(
        stdfs::read_to_string(d.path().join("bundle.tar.gz")).unwrap(),
        "original"
    );
    assert!(uploads.pending.is_empty());
    let chunk = files.read_chunk("bundle (1).tar.gz", 1, 2).unwrap();
    assert_eq!(chunk.data, b"ew");
    assert!(chunk.eof);
    assert!(files.read_chunk("bundle (1).tar.gz", 4, 1).is_err());
    assert!(files.read_chunk("bundle (1).tar.gz", 0, 65537).is_err());
}
#[test]
fn exact_upload_does_not_replace_raced_destination_and_service_assets_use_store() {
    let (d, files, mut state) = fixture();
    let mut uploads = Uploads::default();
    let exact = files
        .begin_upload(
            &mut uploads,
            UploadDestination::Exact {
                path: "race".into(),
            },
            1,
        )
        .unwrap();
    files
        .upload_chunk(&mut uploads, &exact.upload_id, 0, b"x")
        .unwrap();
    stdfs::write(d.path().join("race"), "old").unwrap();
    assert!(matches!(
        files.commit_upload(&mut uploads, &exact.upload_id).unwrap(),
        UploadOutcome::Failed { .. }
    ));
    files.cancel_upload(&mut uploads, &exact.upload_id);
    assert_eq!(files.store.list("uploads").unwrap(), Vec::<String>::new());
    let path = ".workspace/services/example/providers/list.yaml";
    let upload = files
        .begin_upload(
            &mut uploads,
            UploadDestination::Exact { path: path.into() },
            4,
        )
        .unwrap();
    files
        .upload_chunk(&mut uploads, &upload.upload_id, 0, b"test")
        .unwrap();
    assert!(matches!(
        files
            .commit_upload(&mut uploads, &upload.upload_id)
            .unwrap(),
        UploadOutcome::Done { .. }
    ));
    assert_eq!(files.read_chunk(path, 1, 2).unwrap().data, b"es");
    assert_eq!(
        files.open(&mut state, path).unwrap().disk_text.as_deref(),
        Some("test")
    );
}
#[test]
fn explicit_service_directories_are_listable_and_skip_symlinks() {
    let (d, files, mut state) = fixture();
    let path = ".workspace/services/example/providers";
    files
        .store
        .write_text("services/example/seed", "x")
        .unwrap();
    assert_eq!(
        files.file_operation(
            &mut state,
            &FileOperation::CreateDirectory { path: path.into() }
        ),
        FileOutcome::Done
    );
    files
        .store
        .write_text("services/example/providers/a", "visible")
        .unwrap();
    files
        .store
        .write_text("services/example/providers/.hidden", "hidden")
        .unwrap();
    // This test fixture intentionally injects an unsafe store child through the host filesystem.
    std::os::unix::fs::symlink(
        d.path(),
        d.path().join(".workspace/services/example/providers/link"),
    )
    .unwrap();
    let entries = files.list_directory(path, false).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, format!("{path}/a"));
    assert_eq!(files.list_directory(path, true).unwrap().len(), 2);
}

#[test]
fn terminal_link_classification_checks_existence_private_paths_and_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let store = workflow_environment::Store::open(dir.path()).unwrap();
    let files = crate::FileWork::new(dir.path().to_owned(), store.clone());
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::os::unix::fs::symlink("main.rs", dir.path().join("link.rs")).unwrap();
    assert_eq!(
        files.existing_path_kind("main.rs").unwrap(),
        Some(crate::ExistingPathKind::File)
    );
    assert_eq!(
        files.existing_path_kind("src").unwrap(),
        Some(crate::ExistingPathKind::Directory)
    );
    assert_eq!(files.existing_path_kind("absent.rs").unwrap(), None);
    assert!(files.existing_path_kind("link.rs").is_err());
    assert!(
        files
            .existing_path_kind(".workspace/state/workspace.json")
            .is_err()
    );
    assert!(files.existing_path_kind("../outside").is_err());
    store
        .write_text("proxy/config.yaml", "mode: rule\n")
        .unwrap();
    assert_eq!(
        files
            .existing_path_kind(".workspace/proxy/config.yaml")
            .unwrap(),
        Some(crate::ExistingPathKind::File)
    );
}
