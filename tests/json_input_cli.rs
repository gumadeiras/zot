use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn zot_bin() -> String {
    env::var("CARGO_BIN_EXE_zot").expect("cargo should set CARGO_BIN_EXE_zot for integration tests")
}

fn temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    env::temp_dir().join(format!("zot-integration-{nanos}-{name}"))
}

fn write_temp_json(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdirs");
    }
    fs::write(path, contents).expect("write temp json");
}

fn run_zot(args: &[&str], stdin_input: Option<&str>) -> String {
    let mut command = Command::new(zot_bin());
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().expect("spawn zot");

    if let Some(stdin_input) = stdin_input {
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(stdin_input.as_bytes())
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait for zot");
    assert!(
        output.status.success(),
        "zot failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn dry_run_json_reads_from_stdin_without_credentials() {
    let stdout = run_zot(
        &["add", "--dry-run", "json"],
        Some("{\"itemType\":\"webpage\",\"title\":\"stdin\"}\n"),
    );
    assert!(stdout.contains("\"title\": \"stdin\""));
}

#[test]
fn dry_run_json_reads_from_file_without_credentials() {
    let path = temp_path("item.json");
    write_temp_json(&path, "{\"itemType\":\"webpage\",\"title\":\"file\"}\n");

    let stdout = run_zot(
        &[
            "add",
            "--dry-run",
            "json",
            path.to_str().expect("utf8 path"),
        ],
        None,
    );
    assert!(stdout.contains("\"title\": \"file\""));

    fs::remove_file(path).expect("cleanup");
}

#[test]
fn dry_run_json_reads_from_existing_bracketed_file_path() {
    let path = temp_path("[draft].json");
    write_temp_json(&path, "{\"itemType\":\"webpage\",\"title\":\"bracket\"}\n");

    let stdout = run_zot(
        &[
            "add",
            "--dry-run",
            "json",
            path.to_str().expect("utf8 path"),
        ],
        None,
    );
    assert!(stdout.contains("\"title\": \"bracket\""));

    fs::remove_file(path).expect("cleanup");
}

#[test]
fn dry_run_json_reads_inline_argument() {
    let stdout = run_zot(
        &[
            "add",
            "--dry-run",
            "json",
            "{\"itemType\":\"webpage\",\"title\":\"inline\"}",
        ],
        None,
    );
    assert!(stdout.contains("\"title\": \"inline\""));
}

#[test]
fn explicit_value_overrides_file_input() {
    let path = temp_path("item.json");
    write_temp_json(&path, "{\"itemType\":\"webpage\",\"title\":\"file\"}\n");

    let stdout = run_zot(
        &[
            "add",
            "--dry-run",
            "json",
            "--value",
            "{\"itemType\":\"webpage\",\"title\":\"value\"}",
            path.to_str().expect("utf8 path"),
        ],
        None,
    );
    assert!(stdout.contains("\"title\": \"value\""));
    assert!(!stdout.contains("\"title\": \"file\""));

    fs::remove_file(path).expect("cleanup");
}

#[test]
fn json_output_wraps_dry_run_payload() {
    let stdout = run_zot(
        &[
            "--json",
            "add",
            "--dry-run",
            "json",
            "{\"itemType\":\"webpage\",\"title\":\"wrapped\"}",
        ],
        None,
    );
    assert!(stdout.contains("\"dry_run\": true"));
    assert!(stdout.contains("\"title\": \"wrapped\""));
}
