//! Real Server process checks for the unchanged Android JSONL boundary.
#[test]
fn black_box_protocol_and_restart() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/test-protocol.py");
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg("import runpy,sys,tempfile,pathlib; m=runpy.run_path(sys.argv[1]); t=tempfile.TemporaryDirectory(prefix='workflow-contract-'); m['protocol'](pathlib.Path(sys.argv[2]),pathlib.Path(t.name))")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_workflow-engine"))
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
