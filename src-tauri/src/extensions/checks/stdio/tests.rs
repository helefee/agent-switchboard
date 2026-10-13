use super::*;

#[test]
fn relative_program_uses_the_selected_working_directory() {
    let temp = tempfile::tempdir().unwrap();
    let program = program_path("./server", &BTreeMap::new(), temp.path()).unwrap();
    assert_eq!(program, temp.path().join("./server"));
}

#[test]
fn oversized_or_unterminated_output_is_bounded() {
    let mut input = std::io::Cursor::new(b"12345".as_slice());
    assert!(read_line_capped(&mut input, 4).is_err());
}

#[cfg(windows)]
#[test]
fn windows_shims_resolve_only_in_the_target_environment_path() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("fixture.cmd"), "@exit /b 0").unwrap();
    let env = BTreeMap::from([
        ("PATH".into(), temp.path().to_string_lossy().into()),
        ("PATHEXT".into(), ".CMD;.EXE".into()),
    ]);
    assert_eq!(program_path("fixture", &env, temp.path()).unwrap(), temp.path().join("fixture.CMD"));
    assert!(program_path("missing-fixture-command", &env, temp.path()).is_err());
}
