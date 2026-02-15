// Package browser provides E2E tests for the OpenERP dashboard using
// Lightpanda (headless browser) + chromedp (CDP driver).
//
// The test starts Lightpanda and openerpd, navigates to the dashboard,
// and verifies the full CRUD flow including pagination, @count, PATCH,
// and optimistic locking (rev).
package browser

import (
	"context"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/chromedp/cdproto/cdp"
	"github.com/chromedp/chromedp"
)

const (
	rootUser = "root"
	rootPass = "openerp123"
)

// testEnv holds the running processes and URLs for the test.
type testEnv struct {
	lightpanda *exec.Cmd
	openerpd   *exec.Cmd
	baseURL    string
	wsURL      string
	tmpDir     string
}

func TestDashboard(t *testing.T) {
	env := setupEnv(t)
	defer env.cleanup(t)

	// Connect chromedp to Lightpanda.
	allocCtx, allocCancel := chromedp.NewRemoteAllocator(context.Background(), env.wsURL)
	defer allocCancel()

	ctx, cancel := chromedp.NewContext(allocCtx, chromedp.WithLogf(t.Logf))
	defer cancel()

	ctx, cancel = context.WithTimeout(ctx, 2*time.Minute)
	defer cancel()

	// ── 1. Login ──
	t.Run("login", func(t *testing.T) {
		if err := chromedp.Run(ctx,
			chromedp.Navigate(env.baseURL+"/"),
			chromedp.WaitVisible("#username", chromedp.ByID),
			chromedp.Clear("#username", chromedp.ByID),
			chromedp.SendKeys("#username", rootUser, chromedp.ByID),
			chromedp.SendKeys("#password", rootPass, chromedp.ByID),
			chromedp.Click("#submitBtn", chromedp.ByID),
			chromedp.WaitVisible("#sidebar", chromedp.ByID),
		); err != nil {
			t.Fatalf("login failed: %v", err)
		}
	})

	// ── 2. Schema loads: sidebar has resources ──
	t.Run("schema_loads", func(t *testing.T) {
		var nodes []*cdp.Node
		if err := chromedp.Run(ctx,
			chromedp.Nodes(".sidebar .nav-item", &nodes, chromedp.ByQueryAll),
		); err != nil {
			t.Fatalf("sidebar query failed: %v", err)
		}
		if len(nodes) < 2 {
			t.Fatalf("expected >= 2 sidebar items, got %d", len(nodes))
		}
	})

	// ── 3. @count badges load ──
	t.Run("count_badges", func(t *testing.T) {
		// Wait a moment for async count requests.
		time.Sleep(500 * time.Millisecond)
		var badges []*cdp.Node
		if err := chromedp.Run(ctx,
			chromedp.Nodes(".sidebar-count", &badges, chromedp.ByQueryAll),
		); err != nil {
			t.Fatalf("count badge query failed: %v", err)
		}
		// Badges exist (may be hidden if count=0).
		if len(badges) == 0 {
			t.Fatal("expected sidebar count badges to be rendered")
		}
	})

	// ── 4. Create a record via dialog ──
	var createdID string
	t.Run("create_record", func(t *testing.T) {
		// Click Users in sidebar.
		if err := chromedp.Run(ctx,
			chromedp.Evaluate(`(function(){
				const items=document.querySelectorAll('.sidebar .nav-item');
				for(const i of items){if(/user/i.test(i.textContent)){i.click();break}}
			})()`, nil),
			chromedp.Sleep(500*time.Millisecond),
			// Click Add button.
			chromedp.Click(".btn-sm-primary", chromedp.ByQuery),
			chromedp.WaitVisible("#createDlg.open", chromedp.ByQuery),
			// Fill display_name.
			chromedp.SendKeys(`#dlgForm input[name="display_name"]`, "E2E LP Test", chromedp.ByQuery),
			// Submit.
			chromedp.Click("#dlgSubmit", chromedp.ByID),
			chromedp.Sleep(1*time.Second),
		); err != nil {
			t.Fatalf("create record failed: %v", err)
		}

		// Verify via API.
		items := apiList(t, env.baseURL, "/admin/auth/users")
		found := false
		for _, item := range items {
			if dn, ok := item["displayName"].(string); ok && dn == "E2E LP Test" {
				createdID, _ = item["id"].(string)
				found = true
				break
			}
		}
		if !found {
			t.Fatal("created record not found in API response")
		}
	})

	// ── 5. Verify rev=1 on created record ──
	t.Run("rev_set_on_create", func(t *testing.T) {
		record := apiGet(t, env.baseURL, "/admin/auth/users/"+createdID)
		rev, _ := record["rev"].(float64)
		if rev != 1 {
			t.Fatalf("expected rev=1, got %v", rev)
		}
	})

	// ── 6. List uses pagination (hasMore field) ──
	t.Run("list_has_pagination", func(t *testing.T) {
		resp := apiRaw(t, env.baseURL, "/admin/auth/users?limit=1&offset=0")
		if _, ok := resp["hasMore"]; !ok {
			t.Fatal("list response missing hasMore field")
		}
		if _, ok := resp["items"]; !ok {
			t.Fatal("list response missing items field")
		}
	})

	// ── 7. @count endpoint works ──
	t.Run("count_endpoint", func(t *testing.T) {
		resp := apiRaw(t, env.baseURL, "/admin/auth/users/@count")
		count, ok := resp["count"].(float64)
		if !ok {
			t.Fatal("count response missing count field")
		}
		if count < 1 {
			t.Fatalf("expected count >= 1, got %v", count)
		}
	})

	// ── 8. PATCH partial update ──
	t.Run("patch_partial_update", func(t *testing.T) {
		record := apiGet(t, env.baseURL, "/admin/auth/users/"+createdID)
		rev := record["rev"].(float64)

		patch := map[string]interface{}{
			"displayName": "E2E LP Patched",
			"rev":         rev,
		}
		patched := apiPatch(t, env.baseURL, "/admin/auth/users/"+createdID, patch)

		if dn, _ := patched["displayName"].(string); dn != "E2E LP Patched" {
			t.Fatalf("expected patched displayName, got %v", dn)
		}
		newRev, _ := patched["rev"].(float64)
		if newRev != rev+1 {
			t.Fatalf("expected rev=%v, got %v", rev+1, newRev)
		}
	})

	// ── 9. Stale rev returns 409 ──
	t.Run("stale_rev_409", func(t *testing.T) {
		// Use rev=1 (stale, current is 2).
		patch := map[string]interface{}{
			"displayName": "Should Fail",
			"rev":         1,
		}
		status := apiPatchStatus(t, env.baseURL, "/admin/auth/users/"+createdID, patch)
		if status != 409 {
			t.Fatalf("expected 409, got %d", status)
		}
	})

	// ── 10. Pagination UI: Prev/Next buttons exist ──
	t.Run("pagination_ui", func(t *testing.T) {
		var prevDisabled, nextExists bool
		if err := chromedp.Run(ctx,
			// Reload the page to see latest data.
			chromedp.Navigate(env.baseURL+"/dashboard"),
			chromedp.WaitVisible("#sidebar", chromedp.ByID),
			chromedp.Sleep(1*time.Second),
			// Click Users.
			chromedp.Evaluate(`(function(){
				const items=document.querySelectorAll('.sidebar .nav-item');
				for(const i of items){if(/user/i.test(i.textContent)){i.click();break}}
			})()`, nil),
			chromedp.Sleep(1*time.Second),
			// Check pagination bar.
			chromedp.Evaluate(`!!document.getElementById('prevBtn')`, &prevDisabled),
			chromedp.Evaluate(`!!document.getElementById('nextBtn')`, &nextExists),
		); err != nil {
			t.Fatalf("pagination UI check failed: %v", err)
		}
		if !nextExists {
			t.Fatal("pagination Next button not found")
		}
	})

	// ── 11. Cleanup: delete test record ──
	t.Run("cleanup", func(t *testing.T) {
		if createdID != "" {
			apiDelete(t, env.baseURL, "/admin/auth/users/"+createdID)
		}
	})
}

// ── Test environment setup ──

func setupEnv(t *testing.T) *testEnv {
	t.Helper()

	// Find binaries.
	lightpandaBin := findLightpanda(t)
	openerpd := findOpenerpd(t)
	openerp := findOpenerp(t)

	tmpDir, err := os.MkdirTemp("", "openerp-e2e-*")
	if err != nil {
		t.Fatalf("create tmpdir: %v", err)
	}

	// Create test context.
	configDir := filepath.Join(tmpDir, "config")
	dataDir := filepath.Join(tmpDir, "data")
	clientConfig := filepath.Join(tmpDir, "client.toml")

	cmd := exec.Command(openerp,
		"--config", clientConfig,
		"context", "create", "e2e-lp",
		"--config-dir", configDir,
		"--data-dir", dataDir,
		"--password", rootPass,
	)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		t.Fatalf("create context: %v", err)
	}

	serverConfig := filepath.Join(configDir, "e2e-lp.toml")

	// Start openerpd.
	erpPort := freePort(t)
	erpAddr := fmt.Sprintf("127.0.0.1:%d", erpPort)
	erpURL := fmt.Sprintf("http://%s", erpAddr)

	erpCmd := exec.Command(openerpd, "-c", serverConfig, "--listen", erpAddr)
	erpCmd.Env = append(os.Environ(), "RUST_LOG=warn")
	erpCmd.Stdout = os.Stdout
	erpCmd.Stderr = os.Stderr
	if err := erpCmd.Start(); err != nil {
		t.Fatalf("start openerpd: %v", err)
	}
	waitForHealth(t, erpURL, 30*time.Second)

	// Start Lightpanda.
	lpPort := freePort(t)
	lpAddr := fmt.Sprintf("127.0.0.1:%d", lpPort)
	wsURL := fmt.Sprintf("ws://%s", lpAddr)

	lpCmd := exec.Command(lightpandaBin, "serve", "--host", "127.0.0.1", "--port", fmt.Sprintf("%d", lpPort))
	lpCmd.Env = append(os.Environ(), "LIGHTPANDA_DISABLE_TELEMETRY=true")
	lpCmd.Stdout = os.Stdout
	lpCmd.Stderr = os.Stderr
	if err := lpCmd.Start(); err != nil {
		t.Fatalf("start lightpanda: %v", err)
	}
	// Wait for Lightpanda CDP to be ready.
	waitForTCP(t, lpAddr, 15*time.Second)

	return &testEnv{
		lightpanda: lpCmd,
		openerpd:   erpCmd,
		baseURL:    erpURL,
		wsURL:      wsURL,
		tmpDir:     tmpDir,
	}
}

func (e *testEnv) cleanup(t *testing.T) {
	t.Helper()
	if e.lightpanda != nil && e.lightpanda.Process != nil {
		_ = e.lightpanda.Process.Kill()
		_ = e.lightpanda.Wait()
	}
	if e.openerpd != nil && e.openerpd.Process != nil {
		_ = e.openerpd.Process.Kill()
		_ = e.openerpd.Wait()
	}
	if e.tmpDir != "" {
		_ = os.RemoveAll(e.tmpDir)
	}
}

// ── Binary discovery ──

func findLightpanda(t *testing.T) string {
	t.Helper()
	// 1. LIGHTPANDA_PATH env.
	if p := os.Getenv("LIGHTPANDA_PATH"); p != "" {
		return p
	}
	// 2. Bazel runfiles.
	candidates := []string{
		"external/lightpanda_macos_arm64/file/lightpanda-aarch64-macos",
		"external/lightpanda_linux_x86_64/file/lightpanda-x86_64-linux",
		"external/lightpanda_linux_arm64/file/lightpanda-aarch64-linux",
	}
	for _, c := range candidates {
		if _, err := os.Stat(c); err == nil {
			return c
		}
	}
	// 3. PATH.
	if p, err := exec.LookPath("lightpanda"); err == nil {
		return p
	}
	t.Skip("lightpanda binary not found; set LIGHTPANDA_PATH or install lightpanda")
	return ""
}

func findOpenerpd(t *testing.T) string {
	t.Helper()
	// Bazel runfiles.
	candidates := []string{
		"rust/bin/openerpd/openerpd",
		"../rust/bin/openerpd/openerpd",
	}
	// Also check BUILD_WORKSPACE_DIRECTORY.
	if ws := os.Getenv("BUILD_WORKSPACE_DIRECTORY"); ws != "" {
		candidates = append(candidates, filepath.Join(ws, "bazel-bin/rust/bin/openerpd/openerpd"))
	}
	for _, c := range candidates {
		if _, err := os.Stat(c); err == nil {
			return c
		}
	}
	// Try bazel-bin from workspace root.
	if p := os.Getenv("OPENERPD_PATH"); p != "" {
		return p
	}
	t.Skip("openerpd binary not found; build with bazel or set OPENERPD_PATH")
	return ""
}

func findOpenerp(t *testing.T) string {
	t.Helper()
	candidates := []string{
		"rust/bin/openerp/openerp",
		"../rust/bin/openerp/openerp",
	}
	if ws := os.Getenv("BUILD_WORKSPACE_DIRECTORY"); ws != "" {
		candidates = append(candidates, filepath.Join(ws, "bazel-bin/rust/bin/openerp/openerp"))
	}
	for _, c := range candidates {
		if _, err := os.Stat(c); err == nil {
			return c
		}
	}
	if p := os.Getenv("OPENERP_PATH"); p != "" {
		return p
	}
	t.Skip("openerp binary not found; build with bazel or set OPENERP_PATH")
	return ""
}

// ── Network helpers ──

func freePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("find free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

func waitForHealth(t *testing.T, baseURL string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(baseURL + "/health")
		if err == nil && resp.StatusCode == 200 {
			resp.Body.Close()
			return
		}
		if resp != nil {
			resp.Body.Close()
		}
		time.Sleep(300 * time.Millisecond)
	}
	t.Fatalf("openerpd not healthy after %s", timeout)
}

func waitForTCP(t *testing.T, addr string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, time.Second)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("TCP %s not ready after %s", addr, timeout)
}

// ── API helpers ──

func apiRaw(t *testing.T, baseURL, path string) map[string]interface{} {
	t.Helper()
	token := getAPIToken(t, baseURL)
	req, _ := http.NewRequest("GET", baseURL+path, nil)
	req.Header.Set("Authorization", "Bearer "+token)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("API GET %s: %v", path, err)
	}
	defer resp.Body.Close()
	var data map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&data); err != nil {
		t.Fatalf("decode %s: %v", path, err)
	}
	return data
}

func apiList(t *testing.T, baseURL, path string) []map[string]interface{} {
	t.Helper()
	raw := apiRaw(t, baseURL, path)
	items, ok := raw["items"].([]interface{})
	if !ok {
		t.Fatalf("expected items array in %s", path)
	}
	result := make([]map[string]interface{}, len(items))
	for i, item := range items {
		result[i], _ = item.(map[string]interface{})
	}
	return result
}

func apiGet(t *testing.T, baseURL, path string) map[string]interface{} {
	t.Helper()
	return apiRaw(t, baseURL, path)
}

func apiPatch(t *testing.T, baseURL, path string, body map[string]interface{}) map[string]interface{} {
	t.Helper()
	token := getAPIToken(t, baseURL)
	b, _ := json.Marshal(body)
	req, _ := http.NewRequest("PATCH", baseURL+path, strings.NewReader(string(b)))
	req.Header.Set("Authorization", "Bearer "+token)
	req.Header.Set("Content-Type", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("API PATCH %s: %v", path, err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		t.Fatalf("PATCH %s: status %d", path, resp.StatusCode)
	}
	var data map[string]interface{}
	json.NewDecoder(resp.Body).Decode(&data)
	return data
}

func apiPatchStatus(t *testing.T, baseURL, path string, body map[string]interface{}) int {
	t.Helper()
	token := getAPIToken(t, baseURL)
	b, _ := json.Marshal(body)
	req, _ := http.NewRequest("PATCH", baseURL+path, strings.NewReader(string(b)))
	req.Header.Set("Authorization", "Bearer "+token)
	req.Header.Set("Content-Type", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("API PATCH %s: %v", path, err)
	}
	resp.Body.Close()
	return resp.StatusCode
}

func apiDelete(t *testing.T, baseURL, path string) {
	t.Helper()
	token := getAPIToken(t, baseURL)
	req, _ := http.NewRequest("DELETE", baseURL+path, nil)
	req.Header.Set("Authorization", "Bearer "+token)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("API DELETE %s: %v", path, err)
	}
	resp.Body.Close()
}

// getAPIToken performs a login to get JWT. Cached per test run.
var cachedToken string

func getAPIToken(t *testing.T, baseURL string) string {
	t.Helper()
	if cachedToken != "" {
		return cachedToken
	}
	body := fmt.Sprintf(`{"username":"%s","password":"%s"}`, rootUser, rootPass)
	resp, err := http.Post(baseURL+"/auth/login", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("login API: %v", err)
	}
	defer resp.Body.Close()
	var data map[string]interface{}
	json.NewDecoder(resp.Body).Decode(&data)
	token, ok := data["token"].(string)
	if !ok {
		t.Fatalf("login response missing token: %+v", data)
	}
	cachedToken = token
	return token
}

// init sets GOOS/GOARCH-aware defaults.
func init() {
	_ = runtime.GOOS
}
