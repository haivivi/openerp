"""macos_ui_runner — run smoke UI launch against a Bazel-built macOS app.

This runner verifies the bundled .app can be launched on macOS and stays alive
briefly, which provides a reliable smoke check for app startup integration.
"""

def _macos_ui_runner_impl(ctx):
    app_dir = None
    bundle_id = ctx.attr.app_bundle_id

    for dep in ctx.attr.app:
        if DefaultInfo in dep:
            for f in dep[DefaultInfo].files.to_list():
                if f.path.endswith(".app"):
                    app_dir = f

    if not app_dir:
        fail("macos_ui_runner: no .app found in app deps")

    runner = ctx.actions.declare_file(ctx.label.name + ".sh")

    runner_content = """\
#!/bin/bash
set -euo pipefail

WS="${{BUILD_WORKSPACE_DIRECTORY:-$(cd "$(dirname "$0")/../.." && pwd)}}"
APP_PATH="$WS/bazel-bin/{app_short_path}"
BUNDLE_ID="{bundle_id}"
TIMEOUT_SEC={timeout_sec}

echo "=== macOS UI Smoke Runner ==="
echo "App: $APP_PATH"

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: App not found. Build first: bazel build {app_label}"
    exit 1
fi

APP_EXEC="$(ls -1 "$APP_PATH/Contents/MacOS" | head -1)"
if [ -z "$APP_EXEC" ]; then
    echo "ERROR: No executable found under $APP_PATH/Contents/MacOS"
    exit 1
fi

echo "Executable: $APP_EXEC"

"$APP_PATH/Contents/MacOS/$APP_EXEC" &
APP_PID=$!

sleep "$TIMEOUT_SEC"

if ! kill -0 "$APP_PID" 2>/dev/null; then
    wait "$APP_PID"
    RC=$?
    echo "ERROR: App exited early with code $RC"
    exit "$RC"
fi

echo "App launch smoke test passed (${{TIMEOUT_SEC}}s)"

kill -TERM "$APP_PID" 2>/dev/null || true
sleep 1
kill -KILL "$APP_PID" 2>/dev/null || true

echo "=== macOS UI Smoke complete ==="
""".format(
        app_short_path = app_dir.short_path,
        bundle_id = bundle_id,
        timeout_sec = ctx.attr.launch_timeout_sec,
        app_label = str(ctx.attr.app[0].label) if ctx.attr.app else "<app target>",
    )

    ctx.actions.write(
        output = runner,
        content = runner_content,
        is_executable = True,
    )

    return [
        DefaultInfo(
            files = depset([runner]),
            executable = runner,
            runfiles = ctx.runfiles(files = [app_dir]),
        ),
    ]

macos_ui_runner = rule(
    implementation = _macos_ui_runner_impl,
    attrs = {
        "app": attr.label_list(mandatory = True),
        "app_bundle_id": attr.string(mandatory = True),
        "test_srcs": attr.label_list(allow_files = [".swift"]),
        "launch_timeout_sec": attr.int(default = 8),
    },
    executable = True,
)
