//! E2E test runner — cross-platform Rust binary.
//!
//! 1. Builds openerpd + openerp CLI via Bazel
//! 2. Creates a test context using `openerp context create --password`
//! 3. Starts openerpd on a random port
//! 4. Waits for /health to return 200
//! 5. Runs Rust unit tests via Bazel
//! 6. Runs Node.js E2E tests
//! 7. Kills server, cleans up

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ROOT_PASS: &str = "openerp123";

fn main() {
    let root = find_workspace_root().unwrap_or_else(|| {
        fatal("Cannot find workspace root (MODULE.bazel). Run via: bazel run //e2e/runner");
    });
    // When launched by `bazel run`, cwd is the sandbox. Switch to real workspace.
    std::env::set_current_dir(&root).expect("chdir to workspace");
    println!("Workspace: {}", root.display());

    // Step 1: Build binaries.
    step("Step 1: Build binaries");
    bazel(&root, &["build", "//rust/bin/openerpd", "//rust/bin/openerp"]);

    let ext = if cfg!(windows) { ".exe" } else { "" };
    let openerpd = root.join(format!("bazel-bin/rust/bin/openerpd/openerpd{ext}"));
    let openerp = root.join(format!("bazel-bin/rust/bin/openerp/openerp{ext}"));

    // Step 2: Rust unit tests.
    step("Step 2: Rust tests");
    bazel(
        &root,
        &[
            "test",
            "//rust/lib/dsl/golden:golden_test",
            "//rust/lib/dsl/store:store_test",
            "//rust/lib/dsl/types:types_test",
            "//rust/lib/dsl/macro_test:macro_test",
            "//rust/lib/core:core_test",
            "//rust/mod/auth:auth_test",
            "//rust/mod/pms:pms_test",
            "//rust/mod/task:task_test",
        ],
    );
    println!("Rust tests passed.");

    // Step 3: Create test context via CLI.
    step("Step 3: Create test context");
    let tmp_dir = tempfile::tempdir().expect("create temp dir");
    let config_dir = tmp_dir.path().join("config");
    let data_dir = tmp_dir.path().join("data");
    let client_config = tmp_dir.path().join("client.toml");

    run(
        &openerp,
        &[
            "--config",
            client_config.to_str().unwrap(),
            "context",
            "create",
            "e2e-test",
            "--config-dir",
            config_dir.to_str().unwrap(),
            "--data-dir",
            data_dir.to_str().unwrap(),
            "--password",
            ROOT_PASS,
        ],
    );

    let server_config = config_dir.join("e2e-test.toml");
    assert!(
        server_config.exists(),
        "Server config not found: {}",
        server_config.display()
    );
    println!("Server config: {}", server_config.display());

    // Step 4: Start server on a random port.
    step("Step 4: Start server");
    let port = free_port();
    let listen = format!("127.0.0.1:{port}");
    let base_url = format!("http://{listen}");

    let mut server = Command::new(&openerpd)
        .args(["-c", server_config.to_str().unwrap(), "--listen", &listen])
        .env("RUST_LOG", "warn")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start openerpd");

    // Stream server stderr in a background thread so we can see logs.
    let stderr = server.stderr.take().unwrap();
    let log_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("[openerpd] {line}");
            }
        }
    });

    let _guard = ServerGuard {
        child: server,
        _log_thread: log_thread,
    };

    wait_for_health(&base_url, Duration::from_secs(30));
    println!("Server running on {base_url}");

    // Step 5: Install E2E Node deps if needed.
    step("Step 5: Install E2E deps");
    let e2e_dir = root.join("e2e");
    if !e2e_dir.join("node_modules").exists() {
        let mut cmd = Command::new("npm");
        cmd.arg("install")
            .current_dir(&e2e_dir)
            .env("PUPPETEER_SKIP_DOWNLOAD", "true");
        let status = cmd.status().expect("npm install");
        if !status.success() {
            fatal("npm install failed");
        }
    }

    // Step 6: Run E2E tests.
    step("Step 6: Run E2E tests");
    let test_files = [
        "tests/01-login.test.mjs",
        "tests/02-dashboard-crud.test.mjs",
        "tests/03-api-auth.test.mjs",
        "tests/04-user-login.test.mjs",
        "tests/05-facet-api.test.mjs",
        "tests/06-pms-actions.test.mjs",
        "tests/07-task-actions.test.mjs",
        "tests/08-put-edit-api.test.mjs",
        "tests/09-user-password-login.test.mjs",
    ];
    let mut args: Vec<&str> = vec!["--test"];
    args.extend(test_files.iter());

    let status = Command::new("node")
        .args(&args)
        .current_dir(&e2e_dir)
        .env("BASE_URL", &base_url)
        .env("ROOT_PASS", ROOT_PASS)
        .status()
        .expect("run node tests");
    if !status.success() {
        fatal("E2E tests failed");
    }

    println!("\n=== All tests passed! ===");
}

// ── Helpers ──

fn find_workspace_root() -> Option<PathBuf> {
    // Prefer BUILD_WORKSPACE_DIRECTORY (set by `bazel run`).
    if let Ok(dir) = std::env::var("BUILD_WORKSPACE_DIRECTORY") {
        let p = PathBuf::from(&dir);
        if p.join("MODULE.bazel").exists() {
            return Some(p);
        }
    }
    // Fallback: walk up from cwd.
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("MODULE.bazel").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn step(name: &str) {
    println!("\n=== {name} ===");
}

fn bazel(dir: &Path, args: &[&str]) {
    let status = Command::new("bazel")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("run bazel");
    if !status.success() {
        fatal(&format!("bazel {} failed", args.join(" ")));
    }
}

fn run(bin: &Path, args: &[&str]) {
    let status = Command::new(bin)
        .args(args)
        .status()
        .unwrap_or_else(|e| fatal(&format!("run {}: {e}", bin.display())));
    if !status.success() {
        fatal(&format!("{} failed", bin.display()));
    }
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind free port");
    listener.local_addr().unwrap().port()
}

fn wait_for_health(base_url: &str, timeout: Duration) {
    let url = format!("{base_url}/health");
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(resp) = reqwest::blocking::get(&url) {
            if resp.status().is_success() {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    fatal(&format!(
        "Server did not become healthy within {}s",
        timeout.as_secs()
    ));
}

fn fatal(msg: &str) -> ! {
    eprintln!("FATAL: {msg}");
    std::process::exit(1);
}

/// RAII guard — kills the server process on drop.
struct ServerGuard {
    child: Child,
    _log_thread: std::thread::JoinHandle<()>,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
