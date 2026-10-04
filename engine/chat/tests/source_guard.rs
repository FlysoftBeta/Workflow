//! Chat performs no direct filesystem, subprocess or libc access: guest execution and persistence
//! go through the Environment-backed ports that Server composes.
#[test]
fn chat_sources_use_no_direct_io_or_process_apis() {
    fn visit(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    visit(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(files.len() > 20);
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for forbidden in ["std::fs", "std::process", "libc::", "Command::new", "File::open", "unsafe "] {
            assert!(!text.contains(forbidden), "{} uses {forbidden}", file.display());
        }
    }
}
