// E2E test runner — cross-platform Go binary.
//
// 1. Builds openerpd and openerp CLI via Bazel
// 2. Creates a test context using `openerp context create --password`
// 3. Starts openerpd server on a random port
// 4. Waits for /health to return 200
// 5. Runs Rust unit tests
// 6. Runs Node.js E2E tests
// 7. Kills server, cleans up
package main

import (
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"
)

const rootPass = "openerp123"

func main() {
	root := findWorkspaceRoot()
	if root == "" {
		fatal("cannot find workspace root (MODULE.bazel)")
	}
	fmt.Println("Workspace:", root)

	// Step 1: Build both binaries.
	fmt.Println("\n=== Step 1: Build binaries ===")
	if err := bazel(root, "build", "//rust/bin/openerpd", "//rust/bin/openerp"); err != nil {
		fatal("Build failed: %v", err)
	}

	ext := ""
	if runtime.GOOS == "windows" {
		ext = ".exe"
	}
	openerpd := filepath.Join(root, "bazel-bin/rust/bin/openerpd/openerpd"+ext)
	openerp := filepath.Join(root, "bazel-bin/rust/bin/openerp/openerp"+ext)

	// Step 2: Run Rust tests.
	fmt.Println("\n=== Step 2: Rust tests ===")
	rustTests := []string{
		"//rust/lib/dsl/golden:golden_test",
		"//rust/lib/dsl/store:store_test",
		"//rust/lib/dsl/types:types_test",
		"//rust/lib/dsl/macro_test:macro_test",
		"//rust/lib/core:core_test",
		"//rust/mod/auth:auth_test",
	}
	if err := bazel(root, append([]string{"test"}, rustTests...)...); err != nil {
		fatal("Rust tests failed: %v", err)
	}
	fmt.Println("Rust tests passed.")

	// Step 3: Create test context via CLI.
	fmt.Println("\n=== Step 3: Create test context ===")
	tmpDir, err := os.MkdirTemp("", "openerp-e2e-*")
	if err != nil {
		fatal("temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	configDir := filepath.Join(tmpDir, "config")
	dataDir := filepath.Join(tmpDir, "data")
	clientConfig := filepath.Join(tmpDir, "client.toml")

	cmd := exec.Command(openerp,
		"--config", clientConfig,
		"context", "create", "e2e-test",
		"--config-dir", configDir,
		"--data-dir", dataDir,
		"--password", rootPass,
	)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		fatal("context create: %v", err)
	}

	// Find the generated server config.
	serverConfig := filepath.Join(configDir, "e2e-test.toml")
	if _, err := os.Stat(serverConfig); os.IsNotExist(err) {
		fatal("server config not found at %s", serverConfig)
	}
	fmt.Printf("Server config: %s\n", serverConfig)

	// Step 4: Start server on random port.
	fmt.Println("\n=== Step 4: Start server ===")
	port := freePort()
	listen := fmt.Sprintf("127.0.0.1:%d", port)
	baseURL := fmt.Sprintf("http://%s", listen)

	srv := exec.Command(openerpd, "-c", serverConfig, "--listen", listen)
	srv.Env = append(os.Environ(), "RUST_LOG=warn")
	srv.Stdout = os.Stdout
	srv.Stderr = os.Stderr
	if err := srv.Start(); err != nil {
		fatal("start server: %v", err)
	}
	defer func() {
		srv.Process.Kill()
		srv.Wait()
	}()

	if !waitForHealth(baseURL, 30*time.Second) {
		fatal("server did not start within 30s")
	}
	fmt.Printf("Server running on %s\n", baseURL)

	// Step 5: Install E2E deps if needed.
	fmt.Println("\n=== Step 5: Install E2E deps ===")
	e2eDir := filepath.Join(root, "e2e")
	if _, err := os.Stat(filepath.Join(e2eDir, "node_modules")); os.IsNotExist(err) {
		npm := exec.Command("npm", "install")
		npm.Dir = e2eDir
		npm.Env = append(os.Environ(), "PUPPETEER_SKIP_DOWNLOAD=true")
		npm.Stdout = os.Stdout
		npm.Stderr = os.Stderr
		if err := npm.Run(); err != nil {
			fatal("npm install: %v", err)
		}
	}

	// Step 6: Run E2E tests.
	fmt.Println("\n=== Step 6: Run E2E tests ===")
	testFiles := []string{
		"tests/01-login.test.mjs",
		"tests/02-dashboard-crud.test.mjs",
		"tests/03-api-auth.test.mjs",
		"tests/04-user-login.test.mjs",
		"tests/05-facet-api.test.mjs",
	}
	args := append([]string{"--test"}, testFiles...)
	node := exec.Command("node", args...)
	node.Dir = e2eDir
	node.Env = append(os.Environ(),
		"BASE_URL="+baseURL,
		"ROOT_PASS="+rootPass,
	)
	node.Stdout = os.Stdout
	node.Stderr = os.Stderr
	if err := node.Run(); err != nil {
		fatal("E2E tests failed: %v", err)
	}

	fmt.Println("\n=== All tests passed! ===")
}

func findWorkspaceRoot() string {
	dir, _ := os.Getwd()
	for {
		if _, err := os.Stat(filepath.Join(dir, "MODULE.bazel")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			return ""
		}
		dir = parent
	}
}

func bazel(dir string, args ...string) error {
	cmd := exec.Command("bazel", args...)
	cmd.Dir = dir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

func freePort() int {
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		fatal("free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

func waitForHealth(baseURL string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(baseURL + "/health")
		if err == nil && resp.StatusCode == 200 {
			resp.Body.Close()
			return true
		}
		time.Sleep(500 * time.Millisecond)
	}
	return false
}

func fatal(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	if !strings.HasSuffix(msg, "\n") {
		msg += "\n"
	}
	fmt.Fprintf(os.Stderr, "FATAL: %s", msg)
	os.Exit(1)
}
