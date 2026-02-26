"""macos_ui_runner — validate macOS app launch and UITest source compilation.

This rule is a Bazel *test rule* (usable via `bazel test`) and performs:
1) Compile-check of provided UITest Swift sources via xcodebuild on macOS.
2) Smoke launch of Bazel-built app and short liveness verification.
"""

def _macos_ui_runner_impl(ctx):
    app_dir = None
    bundle_id = ctx.attr.app_bundle_id
    test_sources = ctx.files.test_srcs

    for dep in ctx.attr.app:
        if DefaultInfo in dep:
            for f in dep[DefaultInfo].files.to_list():
                if f.path.endswith(".app"):
                    app_dir = f

    if not app_dir:
        fail("macos_ui_runner: no .app found in app deps")

    if not test_sources:
        fail("macos_ui_runner: test_srcs must contain at least one .swift file")

    swift_files = [f.short_path for f in test_sources if f.short_path.endswith(".swift")]
    if not swift_files:
        fail("macos_ui_runner: no .swift files found in test_srcs")

    runner = ctx.actions.declare_file(ctx.label.name + ".sh")

    runner_content = """\
#!/bin/bash
set -euo pipefail

WS="${{BUILD_WORKSPACE_DIRECTORY:-$(pwd)}}"
BUNDLE_ID="{bundle_id}"
TIMEOUT_SEC={timeout_sec}

if [ -n "${{RUNFILES_DIR:-}}" ]; then
    SRC_BASE="$RUNFILES_DIR/_main"
elif [ -n "${{TEST_SRCDIR:-}}" ]; then
    SRC_BASE="$TEST_SRCDIR/_main"
else
    SRC_BASE="$WS"
fi

APP_PATH="$SRC_BASE/{app_short_path}"

echo "=== macOS UI Runner ==="
echo "App path: $APP_PATH"

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: App not found at $APP_PATH"
    echo "Hint: bazel build {app_label}"
    exit 1
fi

INFO_PLIST="$APP_PATH/Contents/Info.plist"
if [ ! -f "$INFO_PLIST" ]; then
    echo "ERROR: Missing Info.plist at $INFO_PLIST"
    exit 1
fi

ACTUAL_BUNDLE_ID=$(/usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" "$INFO_PLIST" 2>/dev/null || true)
if [ "$ACTUAL_BUNDLE_ID" != "$BUNDLE_ID" ]; then
    echo "ERROR: Bundle ID mismatch. expected=$BUNDLE_ID actual=$ACTUAL_BUNDLE_ID"
    exit 1
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/UITests"
{src_copies}

mkdir -p "$WORK/Proj.xcodeproj"
cat > "$WORK/Proj.xcodeproj/project.pbxproj" << 'PBXEOF'
// !$*UTF8*$!
{{
    archiveVersion = 1;
    objectVersion = 56;
    rootObject = R;
    objects = {{
        R = {{ isa = PBXProject; buildConfigurationList = PCL; mainGroup = MG; targets = (T); }};
        MG = {{ isa = PBXGroup; children = (TG); sourceTree = "<group>"; }};
        TG = {{ isa = PBXGroup; children = ({fref_list}); path = UITests; sourceTree = "<group>"; }};
        {file_ref_entries}
        {build_file_entries}
        PCL = {{ isa = XCConfigurationList; buildConfigurations = (PC); }};
        PC = {{ isa = XCBuildConfiguration; name = Debug; buildSettings = {{
            SDKROOT = macosx;
            SWIFT_VERSION = 5.0;
            MACOSX_DEPLOYMENT_TARGET = 14.0;
        }}; }};
        T = {{ isa = PBXNativeTarget; name = UITests; buildConfigurationList = TCL; buildPhases = (SP); productType = "com.apple.product-type.bundle.ui-testing"; productReference = PR; }};
        PR = {{ isa = PBXFileReference; explicitFileType = "wrapper.cfbundle"; path = UITests.xctest; sourceTree = BUILT_PRODUCTS_DIR; }};
        TCL = {{ isa = XCConfigurationList; buildConfigurations = (TC); }};
        TC = {{ isa = XCBuildConfiguration; name = Debug; buildSettings = {{
            PRODUCT_BUNDLE_IDENTIFIER = "{bundle_id}.uitests";
            PRODUCT_NAME = UITests;
            SWIFT_VERSION = 5.0;
            MACOSX_DEPLOYMENT_TARGET = 14.0;
            GENERATE_INFOPLIST_FILE = YES;
            ALWAYS_SEARCH_USER_PATHS = NO;
        }}; }};
        SP = {{ isa = PBXSourcesBuildPhase; files = ({bfile_list}); }};
    }};
}}
PBXEOF

echo "Compiling UITest sources with xcodebuild..."
cd "$WORK"
xcodebuild build-for-testing \
    -project Proj.xcodeproj \
    -scheme UITests \
    -destination "platform=macOS" \
    -derivedDataPath "$WORK/dd" \
    2>&1 | tail -10

RUNNER_APP=$(find "$WORK/dd" -name "UITests-Runner.app" -type d | head -1)
if [ -z "$RUNNER_APP" ]; then
    echo "ERROR: UITests-Runner.app not found"
    exit 1
fi

XCTEST_BUNDLE="$RUNNER_APP/Contents/PlugIns/UITests.xctest"
if [ ! -d "$XCTEST_BUNDLE" ]; then
    echo "ERROR: xctest bundle not found at $XCTEST_BUNDLE"
    exit 1
fi

echo "UITest sources compile-check passed: $XCTEST_BUNDLE"

APP_EXEC="$(ls -1 "$APP_PATH/Contents/MacOS" | head -1)"
if [ -z "$APP_EXEC" ]; then
    echo "ERROR: No executable found under $APP_PATH/Contents/MacOS"
    exit 1
fi

echo "Launching app executable: $APP_EXEC"
"$APP_PATH/Contents/MacOS/$APP_EXEC" &
APP_PID=$!

sleep "$TIMEOUT_SEC"

if ! kill -0 "$APP_PID" 2>/dev/null; then
    set +e
    wait "$APP_PID"
    RC=$?
    set -e
    echo "ERROR: App exited early with code $RC"
    exit 1
fi

echo "App launch smoke test passed (${{TIMEOUT_SEC}}s)"

kill -TERM "$APP_PID" 2>/dev/null || true
sleep 1
kill -KILL "$APP_PID" 2>/dev/null || true

echo "=== macOS UI Runner complete ==="
""".format(
        app_short_path = app_dir.short_path,
        bundle_id = bundle_id,
        app_label = str(ctx.attr.app[0].label) if ctx.attr.app else "<app target>",
        timeout_sec = ctx.attr.launch_timeout_sec,
        src_copies = "\n".join([
            'cp "$SRC_BASE/{}" "$WORK/UITests/"'.format(f)
            for f in swift_files
        ]),
        fref_list = ", ".join(["FR" + str(i) for i in range(len(swift_files))]),
        file_ref_entries = "\n        ".join([
            'FR{i} = {{ isa = PBXFileReference; path = "{name}"; sourceTree = "<group>"; }};'.format(
                i = i,
                name = f.split("/")[-1],
            )
            for i, f in enumerate(swift_files)
        ]),
        build_file_entries = "\n        ".join([
            'BF{i} = {{ isa = PBXBuildFile; fileRef = FR{i}; }};'.format(i = i)
            for i in range(len(swift_files))
        ]),
        bfile_list = ", ".join(["BF" + str(i) for i in range(len(swift_files))]),
    )

    ctx.actions.write(
        output = runner,
        content = runner_content,
        is_executable = True,
    )

    return [
        DefaultInfo(
            executable = runner,
            runfiles = ctx.runfiles(files = [app_dir] + test_sources),
        ),
    ]

_macos_ui_test = rule(
    implementation = _macos_ui_runner_impl,
    attrs = {
        "app": attr.label_list(mandatory = True),
        "app_bundle_id": attr.string(mandatory = True),
        "test_srcs": attr.label_list(allow_files = [".swift"], mandatory = True),
        "launch_timeout_sec": attr.int(default = 8),
    },
    test = True,
)

def macos_ui_runner(name, app, app_bundle_id, test_srcs, tags = None):
    """Macro wrapper exposing the historical macos_ui_runner API as test rule."""
    _macos_ui_test(
        name = name,
        app = app,
        app_bundle_id = app_bundle_id,
        test_srcs = test_srcs,
        tags = tags,
    )
