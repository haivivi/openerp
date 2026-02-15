//! E2E Browser Test Runner.
//!
//! Starts openerpd, installs npm deps, then runs the Puppeteer test.
//! Puppeteer auto-downloads and manages Chromium.
//!
//! Usage:
//!   bazel run //e2e/browser:runner

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ROOT_PASS: &str = "openerp123";

fn main() {
    eprintln!("[runner] Starting E2E browser test...");

    let openerpd_bin = find_binary("openerpd", "OPENERPD_PATH");
    let openerp_bin = find_binary("openerp", "OPENERP_PATH");
    let test_file = find_test_file();

    eprintln!("[runner] openerpd:   {}", openerpd_bin.display());
    eprintln!("[runner] openerp:    {}", openerp_bin.display());
    eprintln!("[runner] test file:  {}", test_file.display());

    // Create test context.
    let tmp = tempfile::tempdir().expect("create tmpdir");
    let config_dir = tmp.path().join("config");
    let data_dir = tmp.path().join("data");
    let client_config = tmp.path().join("client.toml");

    let status = Command::new(&openerp_bin)
        .args([
            "--config", client_config.to_str().unwrap(),
            "context", "create", "e2e-br",
            "--config-dir", config_dir.to_str().unwrap(),
            "--data-dir", data_dir.to_str().unwrap(),
            "--password", ROOT_PASS,
        ])
        .status()
        .expect("run openerp context create");
    assert!(status.success(), "openerp context create failed");

    let server_config = config_dir.join("e2e-br.toml");

    // Start openerpd.
    let erp_port = free_port();
    let erp_addr = format!("127.0.0.1:{erp_port}");
    let base_url = format!("http://{erp_addr}");
    eprintln!("[runner] Starting openerpd on {erp_addr}...");

    let mut openerpd = Command::new(&openerpd_bin)
        .args(["-c", server_config.to_str().unwrap(), "--listen", &erp_addr])
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start openerpd");

    wait_for_health(&base_url, Duration::from_secs(30));
    eprintln!("[runner] openerpd ready");

    // Resolve real paths (Bazel runfiles are symlinks).
    let test_dir = test_file.parent().unwrap().to_path_buf();
    let src_test_dir = std::fs::canonicalize(&test_dir)
        .unwrap_or_else(|_| test_dir.clone());

    // Install npm deps in a temp location to avoid polluting workspace.
    let npm_dir = tmp.path().join("npm");
    std::fs::create_dir_all(&npm_dir).expect("create npm dir");
    std::fs::copy(src_test_dir.join("package.json"), npm_dir.join("package.json"))
        .expect("copy package.json");
    std::fs::copy(
        src_test_dir.join("dashboard.test.mjs"),
        npm_dir.join("dashboard.test.mjs"),
    )
    .expect("copy test file");

    eprintln!("[runner] Installing npm deps in {}...", npm_dir.display());
    let status = Command::new("npm")
        .args(["install"])
        .current_dir(&npm_dir)
        .status()
        .expect("npm install");
    assert!(status.success(), "npm install failed");

    let real_test_dir = npm_dir.clone();
    let real_test_file = npm_dir.join("dashboard.test.mjs");

    // Run Puppeteer test from the real directory (where node_modules is).
    let real_test_file = real_test_dir.join("dashboard.test.mjs");
    eprintln!("[runner] Running Puppeteer test...");
    let status = Command::new("node")
        .args(["--test", real_test_file.to_str().unwrap()])
        .current_dir(&real_test_dir)
        .env("BASE_URL", &base_url)
        .env("ROOT_PASS", ROOT_PASS)
        .status()
        .expect("run node test");

    // Kill openerpd.
    let _ = openerpd.kill();
    let _ = openerpd.wait();

    if status.success() {
        eprintln!("[runner] All tests passed!");
    } else {
        eprintln!("[runner] Tests FAILED");
    }

    std::process::exit(status.code().unwrap_or(1));
}

// ── Binary discovery ──

fn find_in_runfiles(candidates: &[&str], env_key: &str, desc: &str) -> PathBuf {
    if let Ok(p) = std::env::var(env_key) {
        let pb = PathBuf::from(&p);
        if pb.exists() { return pb; }
    }
    let exe_runfiles = std::env::current_exe().ok().and_then(|p| {
        let name = p.file_name()?.to_str()?;
        let rf = p.parent()?.join(format!("{name}.runfiles"));
        if rf.exists() { Some(rf.to_string_lossy().to_string()) } else { None }
    });
    let runfiles_dirs: Vec<PathBuf> = [
        std::env::var("RUNFILES_DIR").ok(),
        std::env::var("TEST_SRCDIR").ok(),
        exe_runfiles,
        Some(".".to_string()),
    ]
    .into_iter()
    .flatten()
    .map(PathBuf::from)
    .collect();

    for base in &runfiles_dirs {
        for c in candidates {
            let p = base.join(c);
            if p.exists() { return p; }
        }
    }
    if let Ok(output) = Command::new("which").arg(desc).output() {
        if output.status.success() {
            let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !p.is_empty() { return PathBuf::from(p); }
        }
    }
    panic!("{desc} not found. Candidates: {candidates:?}. Set {env_key}.");
}

fn find_binary(name: &str, env_key: &str) -> PathBuf {
    let candidates = [
        format!("_main/rust/bin/{name}/{name}"),
        format!("rust/bin/{name}/{name}"),
    ];
    let refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
    find_in_runfiles(&refs, env_key, name)
}

fn find_test_file() -> PathBuf {
    let candidates = [
        "_main/e2e/browser/dashboard.test.mjs",
        "e2e/browser/dashboard.test.mjs",
    ];
    find_in_runfiles(&candidates, "TEST_FILE", "dashboard.test.mjs")
}

// ── Network helpers ──

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind free port")
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_health(base_url: &str, timeout: Duration) {
    let url = format!("{base_url}/health");
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(resp) = reqwest::blocking::get(&url) {
            if resp.status().is_success() { return; }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    panic!("openerpd not healthy after {timeout:?}");
}
