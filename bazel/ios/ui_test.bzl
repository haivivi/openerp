"""ios_ui_runner — run XCUITest against a Bazel-built iOS app.

Uses xcodebuild with a generated .xctestrun file.
This is the most reliable approach:
  1. Bazel builds the .app
  2. xcodebuild compiles the XCUITest .xctest bundle
  3. A .xctestrun plist tells xcodebuild test-without-building where everything is
"""

def _ios_ui_runner_impl(ctx):
    app_dir = None
    bundle_id = ctx.attr.app_bundle_id
    for dep in ctx.attr.app:
        if DefaultInfo in dep:
            for f in dep[DefaultInfo].files.to_list():
                if f.path.endswith(".app"):
                    app_dir = f

    if not app_dir:
        fail("ios_ui_runner: no .app found in deps")

    test_sources = ctx.files.test_srcs
    swift_files = [f.short_path for f in test_sources if f.short_path.endswith(".swift")]

    runner = ctx.actions.declare_file(ctx.label.name + ".sh")

    runner_content = """\
#!/bin/bash
set -euo pipefail

WS="${{BUILD_WORKSPACE_DIRECTORY:-$(cd "$(dirname "$0")/../.." && pwd)}}"
APP_PATH="$WS/bazel-bin/{app_short_path}"
BUNDLE_ID="{bundle_id}"

echo "=== Flux XCUITest Runner ==="

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: App not found. Run: bazel build //e2e/flux/swift:ios_app"
    exit 1
fi

# Find booted simulator.
DEVICE_ID=$(xcrun simctl list devices booted -j | python3 -c "
import json, sys
data = json.load(sys.stdin)
for runtime, devices in data.get('devices', {{}}).items():
    for d in devices:
        if d.get('state') == 'Booted':
            print(d['udid'])
            sys.exit(0)
print('')
")
if [ -z "$DEVICE_ID" ]; then
    echo "ERROR: No booted simulator."
    exit 1
fi
echo "Simulator: $DEVICE_ID"

# Install app on simulator.
TMP_APP="/tmp/xcuitest_$$.app"
cp -r "$APP_PATH" "$TMP_APP"
chmod -R u+w "$TMP_APP"
xattr -cr "$TMP_APP" 2>/dev/null || true
codesign --force --sign - --deep --timestamp=none "$TMP_APP"
xcrun simctl install "$DEVICE_ID" "$TMP_APP"
echo "App installed."

# Create workspace.
WORK=$(mktemp -d)
trap "rm -rf $WORK" EXIT

# Copy test Swift sources.
mkdir -p "$WORK/UITests"
{src_copies}

# Compile xctest bundle using xcodebuild via a minimal project.
# The trick: create a project with ONLY the UI test target,
# and set TEST_HOST to empty + provide the app path via xctestrun.
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
            SDKROOT = iphonesimulator;
            SWIFT_VERSION = 5.0;
            IPHONEOS_DEPLOYMENT_TARGET = 18.0;
            SUPPORTS_MACCATALYST = NO;
        }}; }};
        T = {{ isa = PBXNativeTarget; name = UITests; buildConfigurationList = TCL; buildPhases = (SP); productType = "com.apple.product-type.bundle.ui-testing"; productReference = PR; }};
        PR = {{ isa = PBXFileReference; explicitFileType = "wrapper.cfbundle"; path = UITests.xctest; sourceTree = BUILT_PRODUCTS_DIR; }};
        TCL = {{ isa = XCConfigurationList; buildConfigurations = (TC); }};
        TC = {{ isa = XCBuildConfiguration; name = Debug; buildSettings = {{
            PRODUCT_BUNDLE_IDENTIFIER = "{bundle_id}.uitests";
            PRODUCT_NAME = UITests;
            SWIFT_VERSION = 5.0;
            IPHONEOS_DEPLOYMENT_TARGET = 18.0;
            FRAMEWORK_SEARCH_PATHS = "$(PLATFORM_DIR)/Developer/Library/Frameworks";
            LD_RUNPATH_SEARCH_PATHS = "$(inherited) @executable_path/Frameworks @loader_path/Frameworks";
            TEST_TARGET_NAME = "";
            SUPPORTS_MACCATALYST = NO;
            GENERATE_INFOPLIST_FILE = YES;
            ALWAYS_SEARCH_USER_PATHS = NO;
        }}; }};
        SP = {{ isa = PBXSourcesBuildPhase; files = ({bfile_list}); }};
    }};
}}
PBXEOF

echo "Building xctest bundle..."
cd "$WORK"
xcodebuild build-for-testing \
    -project Proj.xcodeproj \
    -scheme UITests \
    -destination "generic/platform=iOS Simulator" \
    -derivedDataPath "$WORK/dd" \
    DSTROOT="$WORK/dd" \
    2>&1 | tail -5

# Find the built xctest bundle.
XCTEST_BUNDLE=$(find "$WORK/dd" -name "UITests.xctest" -type d | head -1)
if [ -z "$XCTEST_BUNDLE" ]; then
    echo "ERROR: xctest bundle not found after build."
    exit 1
fi
echo "XCTest bundle: $XCTEST_BUNDLE"

# Get the installed app path on the simulator.
INSTALLED_APP=$(xcrun simctl get_app_container "$DEVICE_ID" "$BUNDLE_ID" 2>/dev/null || echo "")
if [ -z "$INSTALLED_APP" ]; then
    echo "ERROR: App not installed on simulator."
    exit 1
fi
echo "Installed app: $INSTALLED_APP"

# Also find the test runner app that xcodebuild built.
RUNNER_APP=$(find "$WORK/dd" -name "UITests-Runner.app" -type d | head -1)
echo "Runner app: $RUNNER_APP"

# Create .xctestrun file.
cat > "$WORK/run.xctestrun" << XCTESTRUN
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>__xctestrun_metadata__</key>
    <dict>
        <key>FormatVersion</key>
        <integer>1</integer>
    </dict>
    <key>UITests</key>
    <dict>
        <key>TestBundlePath</key>
        <string>$RUNNER_APP/PlugIns/UITests.xctest</string>
        <key>TestHostPath</key>
        <string>$RUNNER_APP</string>
        <key>TestHostBundleIdentifier</key>
        <string>{bundle_id}.uitests.xctrunner</string>
        <key>UITargetAppPath</key>
        <string>$INSTALLED_APP</string>
        <key>UITargetAppBundleIdentifier</key>
        <string>$BUNDLE_ID</string>
        <key>IsUITestBundle</key>
        <true/>
        <key>DependentProductPaths</key>
        <array>
            <string>$RUNNER_APP</string>
        </array>
    </dict>
</dict>
</plist>
XCTESTRUN

echo "Running XCUITests..."
xcodebuild test-without-building \
    -xctestrun "$WORK/run.xctestrun" \
    -destination "platform=iOS Simulator,id=$DEVICE_ID" \
    2>&1 | tail -30

rm -rf "$TMP_APP"
echo "=== XCUITest complete ==="
""".format(
        app_short_path = app_dir.short_path,
        bundle_id = bundle_id,
        src_copies = "\n".join([
            'cp "$WS/{}" "$WORK/UITests/"'.format(f)
            for f in swift_files
        ]),
        fref_list = ", ".join(["FR" + str(i) for i in range(len(swift_files))]),
        file_ref_entries = "\n        ".join([
            'FR{i} = {{ isa = PBXFileReference; path = "{name}"; sourceTree = "<group>"; }};'.format(
                i = i, name = f.split("/")[-1],
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
            files = depset([runner]),
            executable = runner,
            runfiles = ctx.runfiles(files = [app_dir] + test_sources),
        ),
    ]

ios_ui_runner = rule(
    implementation = _ios_ui_runner_impl,
    attrs = {
        "app": attr.label_list(mandatory = True),
        "test_srcs": attr.label_list(allow_files = [".swift"]),
        "app_bundle_id": attr.string(mandatory = True),
    },
    executable = True,
)
