use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Core {
    process: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Core {
    fn start(home: &std::path::Path, port: u16) -> Self {
        let mut process = Command::new(env!("CARGO_BIN_EXE_codex-minus-core"))
            .arg("--stdio")
            .env("HOME", home)
            .env("USERPROFILE", home)
            .env("CODEX_HOME", home.join(".codex"))
            .env("CODEX_MINUS_TEST_HOME", home)
            .env("CODEX_PLUS_MANAGER_GUARD_PORT", port.to_string())
            .env_remove("CODEX_PLUS_GUARD_PORT")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = process.stdin.take();
        let output = BufReader::new(process.stdout.take().unwrap());
        Self {
            process,
            input,
            output,
        }
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        assert!(self.output.read_line(&mut line).unwrap() > 0);
        serde_json::from_str(&line).unwrap()
    }

    fn call(&mut self, id: u64, command: &str, args: Value) -> Value {
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({"id":id,"command":command,"args":args})
        )
        .unwrap();
        loop {
            let result = self.read();
            if result.get("id").is_some() {
                assert_eq!(result["id"], id);
                return result;
            }
        }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn real_binary_uses_isolated_data_and_rejects_a_competing_writer() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".codex")).unwrap();
    let sentinel = home.path().join(".codex/auth.json");
    std::fs::write(&sentinel, "official-auth-sentinel").unwrap();
    let port = port();
    let mut first = Core::start(home.path(), port);
    assert_eq!(first.read()["event"], "ready");
    let health = first.call(1, "health", json!({}));
    assert_eq!(health["result"]["protocolVersion"], 1);
    assert_eq!(
        std::path::PathBuf::from(health["result"]["settingsPath"].as_str().unwrap()),
        home.path().join(".codex-session-delete/settings.json")
    );
    let mut second = Core::start(home.path(), port);
    assert_eq!(second.read()["code"], "AlreadyRunning");
    assert!(!second.process.wait().unwrap().success());
    assert_eq!(
        first.call(
            2,
            "save_relay_file",
            json!({"apiKey":"rpc-secret-sentinel"})
        )["error"]["code"],
        "UnknownCommand"
    );
    assert_eq!(
        first.call(3, "load_settings", json!({}))["result"]["status"],
        "ok"
    );
    assert_eq!(
        std::fs::read_to_string(sentinel).unwrap(),
        "official-auth-sentinel"
    );
    first.input.take();
    assert!(first.process.wait().unwrap().success());
}

#[test]
fn isolation_mismatch_fails_before_creating_manager_state() {
    let home = tempfile::tempdir().unwrap();
    // CODEX_HOME must exist or the pinned core falls back. Both resolved roots must be proven.
    let outside = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_codex-minus-core"))
        .arg("--stdio")
        .env("HOME", outside.path())
        .env("USERPROFILE", outside.path())
        .env("CODEX_MINUS_TEST_HOME", home.path())
        .env("CODEX_HOME", outside.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("IsolationInvalid")
    );
    assert!(!outside.path().join(".codex-session-delete").exists());
    assert!(!home.path().join(".codex-session-delete").exists());
}
