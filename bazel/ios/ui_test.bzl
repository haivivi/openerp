"""ios_ui_runner — run XCUITest against a Bazel-built iOS app.

Strategy:
  1. Bazel builds the .app bundle.
  2. Runner generates a minimal Xcode project wrapping the test sources.
  3. xcodebuild compiles + runs the XCUITest on a booted simulator.

This is the only reliable way — XCTest assertions are ObjC macros
that require xcodebuild's Clang module bridging.
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

    runner = ctx.actions.declare_file(ctx.label.name + ".sh")

    # Collect Swift test file names.
    swift_files = [f.short_path for f in test_sources if f.short_path.endswith(".swift")]

    runner_content = """\
#!/bin/bash
set -euo pipefail

WS="${{BUILD_WORKSPACE_DIRECTORY:-$(cd "$(dirname "$0")/../.." && pwd)}}"
APP_PATH="$WS/bazel-bin/{app_short_path}"
BUNDLE_ID="{bundle_id}"

echo "=== Flux XCUITest Runner ==="
echo "App: $APP_PATH"

# Verify app exists.
if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: App not found at $APP_PATH"
    echo "Run: bazel build //ios/TwitterFlux:TwitterFlux"
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
DEVICE_NAME=$(xcrun simctl list devices booted -j | python3 -c "
import json, sys
data = json.load(sys.stdin)
for runtime, devices in data.get('devices', {{}}).items():
    for d in devices:
        if d.get('state') == 'Booted':
            print(d.get('name',''))
            sys.exit(0)
")
echo "Simulator: $DEVICE_NAME ($DEVICE_ID)"

# Install app.
TMP_APP="/tmp/xcuitest_app_$$.app"
cp -r "$APP_PATH" "$TMP_APP"
chmod -R u+w "$TMP_APP"
xattr -cr "$TMP_APP" 2>/dev/null || true
codesign --force --sign - --deep --timestamp=none "$TMP_APP"
xcrun simctl install "$DEVICE_ID" "$TMP_APP"
rm -rf "$TMP_APP"
echo "App installed."

# Generate minimal Xcode project for xcodebuild.
PROJ_DIR=$(mktemp -d)/UITestProject
mkdir -p "$PROJ_DIR/UITestProject.xcodeproj"
mkdir -p "$PROJ_DIR/UITests"

# Copy test sources.
{src_copies}

# Write pbxproj with proper file references.
{write_pbxproj}

# Write xcscheme.
mkdir -p "$PROJ_DIR/UITestProject.xcodeproj/xcshareddata/xcschemes"
cat > "$PROJ_DIR/UITestProject.xcodeproj/xcshareddata/xcschemes/UITests.xcscheme" << 'SCHEME'
<?xml version="1.0" encoding="UTF-8"?>
<Scheme LastUpgradeVersion="1600" version="1.7">
    <TestAction buildConfiguration="Debug" selectedDebuggerIdentifier="Xcode.DebuggerFoundation.Debugger.LLDB" selectedLauncherIdentifier="Xcode.DebuggerFoundation.Launcher.LLDB" shouldUseLaunchSchemeArgsEnv="YES">
        <Testables>
            <TestableReference skipped="NO">
                <BuildableReference BuildableIdentifier="primary" BlueprintIdentifier="UI_TEST_TARGET" BuildableName="UITests.xctest" BlueprintName="UITests" ReferencedContainer="container:UITestProject.xcodeproj"/>
            </TestableReference>
        </Testables>
    </TestAction>
</Scheme>
SCHEME

echo "Running xcodebuild test..."
cd "$PROJ_DIR"
xcodebuild test \
    -project UITestProject.xcodeproj \
    -scheme UITests \
    -destination "platform=iOS Simulator,id=$DEVICE_ID" \
    -only-testing:UITests \
    TEST_TARGET_NAME=TwitterFlux \
    TEST_HOST_APP_BUNDLE_IDENTIFIER="$BUNDLE_ID" \
    UITARGET_APP_BUNDLE_IDENTIFIER="$BUNDLE_ID" \
    2>&1 | tail -80

echo "=== XCUITest complete ==="
""".format(
        app_short_path = app_dir.short_path,
        bundle_id = bundle_id,
        src_copies = "\n".join([
            'cp "$WS/{}" "$PROJ_DIR/UITests/"'.format(f)
            for f in swift_files
        ]),
        write_pbxproj = _gen_pbxproj_script(swift_files, bundle_id),
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

def _gen_pbxproj_script(swift_files, bundle_id):
    """Generate shell code that writes a minimal pbxproj."""
    # Each Swift file needs a PBXFileReference + PBXBuildFile entry.
    file_refs = []
    build_files = []
    children = []
    for i, f in enumerate(swift_files):
        name = f.split("/")[-1]
        fref_id = "FREF_" + str(i)
        bfile_id = "BFILE_" + str(i)
        file_refs.append(
            '{fref} = {{ isa = PBXFileReference; path = "{name}"; sourceTree = "<group>"; }};'.format(
                fref = fref_id, name = name,
            ),
        )
        build_files.append(
            '{bfile} = {{ isa = PBXBuildFile; fileRef = {fref}; }};'.format(
                bfile = bfile_id, fref = fref_id,
            ),
        )
        children.append(fref_id)

    return """\
cat > "$PROJ_DIR/UITestProject.xcodeproj/project.pbxproj" << 'PBXEOF'
// !$*UTF8*$!
{{
    archiveVersion = 1;
    objectVersion = 56;
    rootObject = ROOT_OBJ;
    objects = {{
        ROOT_OBJ = {{ isa = PBXProject; buildConfigurationList = PROJ_CFGLIST; mainGroup = MAIN_GRP; targets = (UI_TEST_TARGET); }};
        MAIN_GRP = {{ isa = PBXGroup; children = (TESTS_GRP); sourceTree = "<group>"; }};
        TESTS_GRP = {{ isa = PBXGroup; children = ({children}); path = UITests; sourceTree = "<group>"; }};
        {file_refs}
        {build_files}
        PROJ_CFGLIST = {{ isa = XCConfigurationList; buildConfigurations = (PROJ_CFG); }};
        PROJ_CFG = {{ isa = XCBuildConfiguration; name = Debug; buildSettings = {{
            SDKROOT = iphonesimulator;
            SWIFT_VERSION = 5.0;
            IPHONEOS_DEPLOYMENT_TARGET = 18.0;
        }}; }};
        UI_TEST_TARGET = {{ isa = PBXNativeTarget; name = UITests; buildConfigurationList = TEST_CFGLIST; buildPhases = (SOURCES_PHASE); productType = "com.apple.product-type.bundle.ui-testing"; productReference = PRODUCT_REF; }};
        PRODUCT_REF = {{ isa = PBXFileReference; explicitFileType = "wrapper.cfbundle"; path = UITests.xctest; sourceTree = BUILT_PRODUCTS_DIR; }};
        TEST_CFGLIST = {{ isa = XCConfigurationList; buildConfigurations = (TEST_CFG); }};
        TEST_CFG = {{ isa = XCBuildConfiguration; name = Debug; buildSettings = {{
            PRODUCT_BUNDLE_IDENTIFIER = "{bundle_id}.uitests";
            PRODUCT_NAME = UITests;
            SWIFT_VERSION = 5.0;
            IPHONEOS_DEPLOYMENT_TARGET = 18.0;
            LD_RUNPATH_SEARCH_PATHS = "$(inherited) @executable_path/Frameworks @loader_path/Frameworks";
            FRAMEWORK_SEARCH_PATHS = "$(PLATFORM_DIR)/Developer/Library/Frameworks";
            TEST_TARGET_NAME = TwitterFlux;
        }}; }};
        SOURCES_PHASE = {{ isa = PBXSourcesBuildPhase; files = ({bfile_ids}); }};
    }};
}}
PBXEOF""".format(
        children = ", ".join(children),
        file_refs = "\n        ".join(file_refs),
        build_files = "\n        ".join(build_files),
        bfile_ids = ", ".join(["BFILE_" + str(i) for i in range(len(swift_files))]),
        bundle_id = bundle_id,
    )

ios_ui_runner = rule(
    implementation = _ios_ui_runner_impl,
    attrs = {
        "app": attr.label_list(mandatory = True),
        "test_srcs": attr.label_list(allow_files = [".swift"]),
        "app_bundle_id": attr.string(mandatory = True),
    },
    executable = True,
)
