//! affected-targets — find Bazel targets affected by git changes.
//!
//! Usage:
//!   bazel run //devops/tools/affected -- --base=origin/main
//!   bazel run //devops/tools/affected -- --base=HEAD~1 --mode=test
//!   bazel run //devops/tools/affected -- --base=origin/main --check=//rust/mod/auth:auth

use std::collections::{BTreeSet, HashSet};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    let args = Args::parse();

    // Bazel runs binaries in a sandbox; chdir to the real workspace.
    if let Ok(ws) = env::var("BUILD_WORKSPACE_DIRECTORY") {
        std::env::set_current_dir(&ws).expect("failed to chdir to workspace");
    }

    let changed = git_changed_files(&args.base);

    if changed.is_empty() {
        if args.verbose {
            eprintln!("no changed files");
        }
        if args.check.is_some() {
            println!("not-affected");
        }
        return;
    }

    if args.verbose {
        eprintln!("changed files ({}):", changed.len());
        for f in &changed {
            eprintln!("  {f}");
        }
        eprintln!();
    }

    let affected = find_affected_targets(&changed, args.verbose);

    // Filter by mode (build = libraries + binaries, test = tests, all = everything)
    let filtered = filter_by_mode(&affected, &args.mode);

    if let Some(ref target) = args.check {
        if filtered.contains(target.as_str()) {
            println!("affected");
            std::process::exit(0);
        } else {
            println!("not-affected");
            std::process::exit(1);
        }
    }

    let out: Box<dyn Write> = if let Some(ref path) = args.output {
        Box::new(std::fs::File::create(path).expect("failed to create output file"))
    } else {
        Box::new(io::stdout().lock())
    };
    let mut out = io::BufWriter::new(out);

    if args.oneline {
        let line = filtered.iter().cloned().collect::<Vec<_>>().join(" ");
        writeln!(out, "{line}").ok();
    } else {
        for t in &filtered {
            writeln!(out, "{t}").ok();
        }
    }
}

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

struct Args {
    base: String,
    mode: String,
    check: Option<String>,
    output: Option<String>,
    oneline: bool,
    verbose: bool,
}

impl Args {
    fn parse() -> Self {
        let mut base = String::new();
        let mut mode = "all".to_string();
        let mut check = None;
        let mut output = None;
        let mut oneline = false;
        let mut verbose = false;

        let args: Vec<String> = env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if let Some(val) = arg.strip_prefix("--base=") {
                base = val.to_string();
            } else if arg == "--base" {
                i += 1;
                base = args.get(i).cloned().unwrap_or_default();
            } else if let Some(val) = arg.strip_prefix("--mode=") {
                mode = val.to_string();
            } else if arg == "--mode" {
                i += 1;
                mode = args.get(i).cloned().unwrap_or("all".into());
            } else if let Some(val) = arg.strip_prefix("--check=") {
                check = Some(val.to_string());
            } else if arg == "--check" {
                i += 1;
                check = args.get(i).cloned();
            } else if let Some(val) = arg.strip_prefix("--output=") {
                output = Some(val.to_string());
            } else if arg == "--output" {
                i += 1;
                output = args.get(i).cloned();
            } else if arg == "--oneline" {
                oneline = true;
            } else if arg == "-v" || arg == "--verbose" {
                verbose = true;
            }
            i += 1;
        }

        if base.is_empty() {
            eprintln!("error: --base is required");
            eprintln!("usage: affected --base=<commit> [--mode=build|test|all] [--check=<target>]");
            std::process::exit(1);
        }

        Self { base, mode, check, output, oneline, verbose }
    }
}

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

fn git_changed_files(base: &str) -> Vec<String> {
    // Resolve merge-base for divergent branches.
    let actual_base = run_cmd("git", &["merge-base", base, "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| base.to_string());

    let output = run_cmd("git", &["diff", "--name-only", &format!("{actual_base}..HEAD"), "--"])
        .or_else(|_| run_cmd("git", &["diff", "--name-only", &actual_base, "HEAD", "--"]))
        .unwrap_or_default();

    output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Affected targets
// ---------------------------------------------------------------------------

fn find_affected_targets(changed_files: &[String], verbose: bool) -> BTreeSet<String> {
    // Deduplicate by Bazel package.
    let mut packages = HashSet::new();
    for file in changed_files {
        // Skip bazel output dirs and non-source directories.
        if file.starts_with("bazel-")
            || file.starts_with(".github/")
            || file.starts_with(".git/")
            || file.starts_with("docs/")
        {
            if verbose {
                eprintln!("  skip (non-source): {file}");
            }
            continue;
        }
        if let Some(pkg) = find_package_for_file(file) {
            packages.insert(pkg);
        }
    }

    if verbose {
        eprintln!("affected packages ({}):", packages.len());
        let mut sorted: Vec<_> = packages.iter().collect();
        sorted.sort();
        for p in &sorted {
            eprintln!("  {p}");
        }
        eprintln!();
    }

    let mut targets = BTreeSet::new();
    for pkg in &packages {
        match bazel_query(&format!("rdeps(//..., {pkg}:all)")) {
            Ok(ts) => {
                for t in ts {
                    targets.insert(t);
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  warning: query failed for {pkg}: {e}");
                }
            }
        }
    }

    targets
}

fn find_package_for_file(file: &str) -> Option<String> {
    let mut dir = PathBuf::from(file);
    dir.pop(); // remove filename

    loop {
        let candidate = if dir.as_os_str().is_empty() {
            PathBuf::new()
        } else {
            dir.clone()
        };

        for build in &["BUILD.bazel", "BUILD"] {
            let build_path = if candidate.as_os_str().is_empty() {
                PathBuf::from(build)
            } else {
                candidate.join(build)
            };
            if build_path.exists() {
                let pkg = if candidate.as_os_str().is_empty() {
                    "//".to_string()
                } else {
                    format!("//{}", candidate.display())
                };
                return Some(pkg);
            }
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

fn filter_by_mode<'a>(targets: &'a BTreeSet<String>, mode: &str) -> BTreeSet<&'a str> {
    match mode {
        "build" => targets
            .iter()
            .filter(|t| !t.contains("_test") && !t.contains("_bench"))
            .map(|s| s.as_str())
            .collect(),
        "test" => targets
            .iter()
            .filter(|t| t.contains("_test"))
            .map(|s| s.as_str())
            .collect(),
        "bench" => targets
            .iter()
            .filter(|t| t.contains("_bench"))
            .map(|s| s.as_str())
            .collect(),
        _ => targets.iter().map(|s| s.as_str()).collect(),
    }
}

// ---------------------------------------------------------------------------
// Bazel query
// ---------------------------------------------------------------------------

fn bazel_query(query: &str) -> Result<Vec<String>, String> {
    let output = Command::new("bazel")
        .args(["query", query, "--keep_going", "--noshow_progress"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run bazel query: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // bazel query with --keep_going returns exit code 3 for partial results
        // which is acceptable.
        if output.status.code() != Some(3) {
            return Err(format!("bazel query failed: {stderr}"));
        }
    }

    let targets: Vec<String> = output
        .stdout
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let trimmed = line.trim().to_string();
            // Filter out external targets.
            if trimmed.is_empty() || trimmed.starts_with('@') {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect();

    Ok(targets)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("{program}: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "{program} exited with {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("invalid utf8: {e}"))
}
