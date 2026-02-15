//! E2E browser tests for the OpenERP dashboard.
//!
//! Drives Lightpanda (headless browser with CDP) using chromiumoxide.
//! Tests the full dashboard flow: login → schema → CRUD → pagination → PATCH → rev.
//!
//! Usage:
//!   bazel test //e2e/browser:browser_test
//!
//! Or manually:
//!   LIGHTPANDA_PATH=/path/to/lightpanda \
//!   OPENERPD_PATH=/path/to/openerpd \
//!   OPENERP_PATH=/path/to/openerp \
//!   cargo test --test browser_test

#[cfg(test)]
mod tests {
    use chromiumoxide::browser::Browser;
    use futures::StreamExt;
    use serde_json::Value;
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::time::Duration;

    const ROOT_USER: &str = "root";
    const ROOT_PASS: &str = "openerp123";

    /// Test environment: running Lightpanda + openerpd processes.
    struct TestEnv {
        lightpanda: Child,
        openerpd: Child,
        base_url: String,
        /// CDP endpoint (http:// URL — chromiumoxide discovers ws:// via /json/version).
        cdp_url: String,
        _tmp: tempfile::TempDir,
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = self.lightpanda.kill();
            let _ = self.lightpanda.wait();
            let _ = self.openerpd.kill();
            let _ = self.openerpd.wait();
        }
    }

    // ── Main test ──

    /// Wrap an async operation with a timeout. Panics with a clear message on timeout.
    async fn timed<F, T>(label: &str, secs: u64, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        match tokio::time::timeout(Duration::from_secs(secs), f).await {
            Ok(v) => v,
            Err(_) => panic!("[E2E] TIMEOUT after {secs}s: {label}"),
        }
    }

    #[tokio::test]
    async fn dashboard_e2e() {
        eprintln!("[E2E] Setting up environment...");
        let env = setup_env().await;

        // Connect to Lightpanda via CDP (http:// → auto-discovers ws:// via /json/version).
        eprintln!("[E2E] Connecting to Lightpanda CDP at {}...", env.cdp_url);
        let (browser, mut handler) = timed("Browser::connect", 30, async {
            Browser::connect(&env.cdp_url)
                .await
                .expect("connect to Lightpanda CDP")
        })
        .await;

        // Spawn CDP event handler.
        tokio::spawn(async move { while handler.next().await.is_some() {} });

        eprintln!("[E2E] Creating new page...");
        let page = timed("new_page", 15, async {
            browser.new_page("about:blank").await.expect("new page")
        })
        .await;

        // ── 1. Login ──
        eprintln!("[E2E] Step 1: Login...");
        timed("goto login", 15, async {
            page.goto(format!("{}/", env.base_url))
                .await
                .expect("goto login")
        })
        .await;
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Fill login form.
        timed("fill login form", 15, async {
            page.find_element("#username").await.expect("find username")
                .click().await.expect("click username")
                .type_str(ROOT_USER).await.expect("type username");
            page.find_element("#password").await.expect("find password")
                .click().await.expect("click password")
                .type_str(ROOT_PASS).await.expect("type password");
            page.find_element("#submitBtn").await.expect("find submit")
                .click().await.expect("click submit");
        })
        .await;

        tokio::time::sleep(Duration::from_secs(3)).await;

        // Verify we're on dashboard.
        let url = page.url().await.expect("get url");
        assert!(
            url.as_deref().unwrap_or("").contains("/dashboard"),
            "expected /dashboard, got: {:?}",
            url
        );

        // ── 2. Schema loads: sidebar has resources ──
        eprintln!("[E2E] Step 2: Schema loads...");
        let sidebar_count: i64 = page
            .evaluate("document.querySelectorAll('.sidebar .nav-item').length")
            .await
            .expect("eval sidebar count")
            .into_value()
            .expect("parse sidebar count");
        assert!(sidebar_count >= 2, "expected >= 2 sidebar items, got {sidebar_count}");

        // ── 3. @count badges exist ──
        eprintln!("[E2E] Step 3: @count badges...");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let badge_count: i64 = page
            .evaluate("document.querySelectorAll('.sidebar-count').length")
            .await
            .expect("eval badge count")
            .into_value()
            .expect("parse badge count");
        assert!(badge_count > 0, "expected sidebar count badges, got {badge_count}");

        // ── 4. Navigate to Users ──
        eprintln!("[E2E] Step 4: Navigate to Users...");
        page.evaluate(
            r#"(function(){
                const items=document.querySelectorAll('.sidebar .nav-item');
                for(const i of items){if(/user/i.test(i.textContent)){i.click();break}}
            })()"#,
        )
        .await
        .expect("click Users");
        tokio::time::sleep(Duration::from_secs(1)).await;

        // ── 5. Create a record via dialog ──
        eprintln!("[E2E] Step 5: Create record...");
        page.evaluate("document.querySelector('.btn-sm-primary').click()")
            .await
            .expect("click Add");
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Type into display_name field.
        page.evaluate(
            r#"(function(){
                const inp=document.querySelector('#dlgForm input[name="display_name"]');
                if(inp){inp.value='E2E Rust Test';inp.dispatchEvent(new Event('input'))}
            })()"#,
        )
        .await
        .expect("fill display_name");

        page.evaluate("document.getElementById('dlgSubmit').click()")
            .await
            .expect("click Create");
        tokio::time::sleep(Duration::from_secs(1)).await;

        // ── 6. Verify via API: record created with rev=1 ──
        eprintln!("[E2E] Step 6-12: API verification...");
        let token = api_login(&env.base_url).await;
        let items = api_list(&env.base_url, "/admin/auth/users", &token).await;
        let created = items
            .iter()
            .find(|i| i.get("displayName").and_then(|v| v.as_str()) == Some("E2E Rust Test"))
            .expect("created record not found in API");
        let id = created["id"].as_str().expect("id");
        let rev = created["rev"].as_f64().expect("rev");
        assert_eq!(rev, 1.0, "rev should be 1 after create");

        // ── 7. List API returns hasMore field ──
        let list_resp = api_get_raw(&env.base_url, "/admin/auth/users?limit=1&offset=0", &token).await;
        assert!(list_resp.get("hasMore").is_some(), "list response missing hasMore");
        assert!(list_resp.get("items").is_some(), "list response missing items");

        // ── 8. @count endpoint works ──
        let count_resp = api_get_raw(&env.base_url, "/admin/auth/users/@count", &token).await;
        let count = count_resp["count"].as_f64().expect("count field");
        assert!(count >= 1.0, "expected count >= 1, got {count}");

        // ── 9. PATCH partial update ──
        let patch = serde_json::json!({"displayName": "E2E Rust Patched", "rev": 1});
        let patched = api_patch(&env.base_url, &format!("/admin/auth/users/{id}"), &patch, &token).await;
        assert_eq!(patched["displayName"], "E2E Rust Patched");
        assert_eq!(patched["rev"], 2, "rev should be 2 after patch");

        // ── 10. Stale rev → 409 ──
        let stale_patch = serde_json::json!({"displayName": "Should Fail", "rev": 1});
        let status = api_patch_status(
            &env.base_url,
            &format!("/admin/auth/users/{id}"),
            &stale_patch,
            &token,
        )
        .await;
        assert_eq!(status, 409, "stale rev should return 409");

        // ── 11. Pagination UI: Prev/Next buttons exist ──
        page.goto(format!("{}/dashboard", env.base_url))
            .await
            .expect("goto dashboard");
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Click Users again.
        page.evaluate(
            r#"(function(){
                const items=document.querySelectorAll('.sidebar .nav-item');
                for(const i of items){if(/user/i.test(i.textContent)){i.click();break}}
            })()"#,
        )
        .await
        .expect("click Users again");
        tokio::time::sleep(Duration::from_secs(1)).await;

        let has_prev: bool = page
            .evaluate("!!document.getElementById('prevBtn')")
            .await
            .expect("eval prevBtn")
            .into_value()
            .expect("parse prevBtn");
        let has_next: bool = page
            .evaluate("!!document.getElementById('nextBtn')")
            .await
            .expect("eval nextBtn")
            .into_value()
            .expect("parse nextBtn");
        assert!(has_prev, "Prev button should exist");
        assert!(has_next, "Next button should exist");

        // ── 12. Cleanup ──
        api_delete(&env.base_url, &format!("/admin/auth/users/{id}"), &token).await;
    }

    // ── Environment setup ──

    async fn setup_env() -> TestEnv {
        let lightpanda_bin = find_lightpanda();
        let openerpd_bin = find_binary("openerpd", "OPENERPD_PATH");
        let openerp_bin = find_binary("openerp", "OPENERP_PATH");

        let tmp = tempfile::tempdir().expect("create tmpdir");
        let config_dir = tmp.path().join("config");
        let data_dir = tmp.path().join("data");
        let client_config = tmp.path().join("client.toml");

        // Create context.
        let status = Command::new(&openerp_bin)
            .args([
                "--config",
                client_config.to_str().unwrap(),
                "context",
                "create",
                "e2e-lp",
                "--config-dir",
                config_dir.to_str().unwrap(),
                "--data-dir",
                data_dir.to_str().unwrap(),
                "--password",
                ROOT_PASS,
            ])
            .status()
            .expect("run openerp context create");
        assert!(status.success(), "openerp context create failed");

        let server_config = config_dir.join("e2e-lp.toml");

        // Start openerpd.
        eprintln!("[E2E] Starting openerpd...");
        let erp_port = free_port();
        let erp_addr = format!("127.0.0.1:{erp_port}");
        let base_url = format!("http://{erp_addr}");

        let openerpd = Command::new(&openerpd_bin)
            .args(["-c", server_config.to_str().unwrap(), "--listen", &erp_addr])
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start openerpd");

        wait_for_health(&base_url, Duration::from_secs(30)).await;
        eprintln!("[E2E] openerpd ready at {base_url}");

        eprintln!("[E2E] Starting Lightpanda...");
        let lp_port = free_port();
        let lp_addr = format!("127.0.0.1:{lp_port}");
        // Use http:// — chromiumoxide discovers ws:// via /json/version.
        let cdp_url = format!("http://{lp_addr}");

        let lightpanda = Command::new(&lightpanda_bin)
            .args([
                "serve",
                "--host",
                "127.0.0.1",
                "--port",
                &lp_port.to_string(),
            ])
            .env("LIGHTPANDA_DISABLE_TELEMETRY", "true")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start lightpanda");

        // Wait for Lightpanda's CDP HTTP endpoint (not just TCP).
        wait_for_cdp(&cdp_url, Duration::from_secs(30)).await;
        eprintln!("[E2E] Lightpanda ready at {cdp_url}");

        TestEnv {
            lightpanda,
            openerpd,
            base_url,
            cdp_url,
            _tmp: tmp,
        }
    }

    // ── Binary discovery ──

    /// Find a file in Bazel runfiles or env override.
    /// Checks multiple candidate paths to handle different Bazel versions
    /// and both in-runfiles and standalone execution.
    fn find_in_runfiles(candidates: &[&str], env_key: &str, desc: &str) -> PathBuf {
        // 1. Explicit env override.
        if let Ok(p) = std::env::var(env_key) {
            let pb = PathBuf::from(&p);
            if pb.exists() {
                return pb;
            }
        }

        // 2. Bazel runfiles: TEST_SRCDIR / RUNFILES_DIR set by bazel test.
        let runfiles_dirs: Vec<PathBuf> = [
            std::env::var("TEST_SRCDIR").ok(),
            std::env::var("RUNFILES_DIR").ok(),
            Some(".".to_string()),
        ]
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .collect();

        for base in &runfiles_dirs {
            for c in candidates {
                let p = base.join(c);
                if p.exists() {
                    return p;
                }
            }
        }

        // 3. PATH lookup (for standalone execution).
        let bin_name = candidates
            .last()
            .and_then(|c| c.rsplit('/').next())
            .unwrap_or(desc);
        if let Ok(output) = Command::new("which").arg(bin_name).output() {
            if output.status.success() {
                let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !p.is_empty() {
                    return PathBuf::from(p);
                }
            }
        }

        panic!(
            "{desc} not found. Candidates: {:?}. Set {env_key} or run via bazel test.",
            candidates
        );
    }

    fn find_lightpanda() -> PathBuf {
        find_in_runfiles(
            &[
                // Bazel bzlmod http_file runfiles paths.
                "+http_file+lightpanda_macos_arm64/file/downloaded",
                "+http_file+lightpanda_linux_x86_64/file/downloaded",
                "+http_file+lightpanda_linux_arm64/file/downloaded",
            ],
            "LIGHTPANDA_PATH",
            "lightpanda",
        )
    }

    fn find_binary(name: &str, env_key: &str) -> PathBuf {
        let candidates = [
            // Bazel runfiles path (bzlmod: _main/ prefix).
            format!("_main/rust/bin/{name}/{name}"),
            // Fallback: no prefix.
            format!("rust/bin/{name}/{name}"),
        ];
        let refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
        find_in_runfiles(&refs, env_key, name)
    }

    // ── Network helpers ──

    fn free_port() -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind free port");
        l.local_addr().unwrap().port()
    }

    async fn wait_for_health(base_url: &str, timeout: Duration) {
        let url = format!("{base_url}/health");
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if let Ok(resp) = reqwest::get(&url).await {
                if resp.status().is_success() {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        panic!("openerpd not healthy after {timeout:?}");
    }

    /// Wait for Lightpanda's CDP server to respond to /json/version.
    async fn wait_for_cdp(base_url: &str, timeout: Duration) {
        let url = format!("{base_url}/json/version");
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if let Ok(resp) = reqwest::get(&url).await {
                if resp.status().is_success() {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        panic!("[E2E] Lightpanda CDP not ready at {base_url} after {timeout:?}");
    }

    // ── API helpers ──

    async fn api_login(base_url: &str) -> String {
        let client = reqwest::Client::new();
        let body = serde_json::json!({"username": ROOT_USER, "password": ROOT_PASS});
        let resp = client
            .post(format!("{base_url}/auth/login"))
            .json(&body)
            .send()
            .await
            .expect("login request");
        let data: Value = resp.json().await.expect("login json");
        data["token"].as_str().expect("token field").to_string()
    }

    async fn api_get_raw(base_url: &str, path: &str, token: &str) -> Value {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{base_url}{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("api get");
        resp.json().await.expect("api json")
    }

    async fn api_list(base_url: &str, path: &str, token: &str) -> Vec<Value> {
        let raw = api_get_raw(base_url, path, token).await;
        raw["items"]
            .as_array()
            .expect("items array")
            .to_vec()
    }

    async fn api_patch(base_url: &str, path: &str, body: &Value, token: &str) -> Value {
        let client = reqwest::Client::new();
        let resp = client
            .patch(format!("{base_url}{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .expect("api patch");
        assert_eq!(resp.status().as_u16(), 200, "PATCH should succeed");
        resp.json().await.expect("patch json")
    }

    async fn api_patch_status(base_url: &str, path: &str, body: &Value, token: &str) -> u16 {
        let client = reqwest::Client::new();
        let resp = client
            .patch(format!("{base_url}{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .expect("api patch status");
        resp.status().as_u16()
    }

    async fn api_delete(base_url: &str, path: &str, token: &str) {
        let client = reqwest::Client::new();
        client
            .delete(format!("{base_url}{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("api delete");
    }
}
