"""ios_ipa — build a signed .ipa for device / App Store / TestFlight."""

load(":transition.bzl", "ios_device_arm64_transition")
load(":providers.bzl", "IosAppInfo")

def _ios_ipa_impl(ctx):
    bundle_name = ctx.attr.app_name or ctx.label.name
    # Use a label-scoped staging .app to avoid output conflicts when an
    # ios_app target and an ios_ipa target share the same app_name.
    app_dir = ctx.actions.declare_directory(ctx.label.name + ".stage.app")
    ipa_file = ctx.actions.declare_file(bundle_name + ".ipa")
    bundle_id = ctx.attr.bundle_id
    team_id = ctx.attr.team_id
    minimum_os = ctx.attr.minimum_os_version

    # Collect .a from transitioned deps
    static_libs = []
    all_dep_files = []
    for dep in ctx.attr.deps:
        if DefaultInfo in dep:
            for f in dep[DefaultInfo].files.to_list():
                all_dep_files.append(f)
                if f.path.endswith(".a"):
                    static_libs.append(f)

    resource_files = ctx.files.resources
    infoplist = ctx.file.infoplist
    icon_files = ctx.files.app_icons
    profile = ctx.file.provisioning_profile

    extra_inputs = []
    if profile:
        profile_cmd = "echo 'Embedding profile' && cp '{p}' \"$APP_DIR/embedded.mobileprovision\"".format(p = profile.path)
        extra_inputs.append(profile)
    else:
        profile_cmd = "echo 'WARNING: No provisioning profile provided'"

    entitlements = ctx.file.entitlements
    if entitlements:
        entitlements_cmd = ""
        entitlements_flag = "--entitlements '{e}'".format(e = entitlements.path)
        extra_inputs.append(entitlements)
    else:
        entitlements_cmd = ""
        entitlements_flag = ""

    libs_str = " ".join(["'" + f.path + "'" for f in static_libs])
    target_triple = "arm64-apple-ios{}".format(minimum_os)

    # Resource copy commands
    res_cmds = []
    for f in resource_files:
        res_cmds.append("cp '{src}' \"$APP_DIR/\"".format(src = f.path))
    for f in icon_files:
        if f.path.endswith(".png"):
            res_cmds.append("cp '{src}' \"$APP_DIR/\"".format(src = f.path))

    script = """\
set -euo pipefail
APP_DIR="{app_dir}"
IPA="{ipa}"
BUNDLE_NAME="{bundle_name}"
BUNDLE_ID="{bundle_id}"
TEAM_ID="{team_id}"

mkdir -p "$APP_DIR"

# Link for device (iphoneos)
SDK_PATH=$(xcrun --sdk iphoneos --show-sdk-path)
SWIFT_LIB=$(dirname $(xcrun --toolchain default --find swiftc))/../lib/swift/iphoneos
xcrun --sdk iphoneos clang -arch arm64 -target {target_triple} \
    -isysroot "$SDK_PATH" \
    -F"$SDK_PATH/System/Library/Frameworks" \
    -framework Foundation -framework UIKit -framework SwiftUI \
    -L"$SDK_PATH/usr/lib/swift" \
    -L"$SWIFT_LIB" \
    -Wl,-rpath,/usr/lib/swift \
    -Wl,-rpath,@executable_path/Frameworks \
    {libs} \
    -o "$APP_DIR/$BUNDLE_NAME"

# Info.plist
cp '{infoplist}' "$APP_DIR/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$APP_DIR/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string $BUNDLE_ID" "$APP_DIR/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $BUNDLE_NAME" "$APP_DIR/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string $BUNDLE_NAME" "$APP_DIR/Info.plist"

# Xcode/SDK metadata (required for App Store validation)
XCODE_BUILD=$(xcodebuild -version | tail -1 | awk '{{print $3}}')
XCODE_VER=$(xcodebuild -version | head -1 | awk '{{print $2}}' | tr -d '.')0
SDK_BUILD=$(xcrun --sdk iphoneos --show-sdk-build-version)
SDK_VER=$(xcrun --sdk iphoneos --show-sdk-version)
/usr/libexec/PlistBuddy -c "Add :DTSDKName string iphoneos$SDK_VER" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTSDKBuild string $SDK_BUILD" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTXcode string $XCODE_VER" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTXcodeBuild string $XCODE_BUILD" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTPlatformName string iphoneos" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTPlatformVersion string $SDK_VER" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTPlatformBuild string $SDK_BUILD" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :DTCompiler string com.apple.compilers.llvm.clang.1_0" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :CFBundleSupportedPlatforms array" "$APP_DIR/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :CFBundleSupportedPlatforms:0 string iPhoneOS" "$APP_DIR/Info.plist" 2>/dev/null || true

# Resources
{res_cmds}

# Compile app icon asset catalog
XCASSETS_DIR=$(find . -path "*/Assets.xcassets" -type d | head -1)
if [ -n "$XCASSETS_DIR" ]; then
    xcrun actool "$XCASSETS_DIR" \
        --compile "$APP_DIR" \
        --platform iphoneos \
        --minimum-deployment-target {minimum_os} \
        --app-icon AppIcon \
        --output-partial-info-plist "$APP_DIR/assetcatalog_generated_info.plist" \
        2>/dev/null || echo "actool warning (non-fatal)"
    # Merge generated plist into Info.plist
    if [ -f "$APP_DIR/assetcatalog_generated_info.plist" ]; then
        /usr/libexec/PlistBuddy -c "Merge $APP_DIR/assetcatalog_generated_info.plist" "$APP_DIR/Info.plist" 2>/dev/null || true
        rm -f "$APP_DIR/assetcatalog_generated_info.plist"
    fi
fi

# Embed provisioning profile
{profile_cmd}

# Sign — prefer Distribution, fallback to any
HASH=$(security find-identity -v -p codesigning 2>/dev/null | grep Distribution | head -1 | awk '{{print $2}}')
if [ -z "$HASH" ]; then
    HASH=$(security find-identity -v -p codesigning 2>/dev/null | grep -v "valid identities" | head -1 | awk '{{print $2}}')
fi
if [ -n "$HASH" ]; then
    echo "Signing with hash: $HASH"
    {entitlements_cmd}
    codesign --force --sign "$HASH" {entitlements_flag} --timestamp=none "$APP_DIR"
else
    echo "WARNING: No signing identity found, using ad-hoc"
    codesign --force --sign - --timestamp=none "$APP_DIR"
fi

# Create IPA (Payload/Name.app → zip)
PAYLOAD=$(mktemp -d)
mkdir -p "$PAYLOAD/Payload/$BUNDLE_NAME.app"
cp -R "$APP_DIR/" "$PAYLOAD/Payload/$BUNDLE_NAME.app/"
(cd "$PAYLOAD" && zip -qr - Payload) > "$IPA"
rm -rf "$PAYLOAD"

echo "Created $IPA ($(du -h "$IPA" | cut -f1))"
""".format(
        app_dir = app_dir.path,
        ipa = ipa_file.path,
        bundle_name = bundle_name,
        bundle_id = bundle_id,
        team_id = team_id,
        target_triple = target_triple,
        libs = libs_str,
        infoplist = infoplist.path,
        res_cmds = "\n".join(res_cmds),
        minimum_os = minimum_os,
        profile_cmd = profile_cmd,
        entitlements_cmd = entitlements_cmd,
        entitlements_flag = entitlements_flag,
    )

    ctx.actions.run_shell(
        outputs = [app_dir, ipa_file],
        inputs = static_libs + all_dep_files + resource_files + [infoplist] + icon_files + extra_inputs,
        command = script,
        mnemonic = "IosIpa",
        progress_message = "Building iOS IPA %s" % bundle_name,
        use_default_shell_env = True,
        execution_requirements = {
            "no-sandbox": "1",
            "local": "1",
            "requires-network": "1",
        },
    )

    return [
        DefaultInfo(files = depset([ipa_file])),
        IosAppInfo(
            app_dir = app_dir,
            ipa = ipa_file,
            bundle_id = bundle_id,
            minimum_os = minimum_os,
            team_id = team_id,
        ),
    ]

ios_ipa = rule(
    implementation = _ios_ipa_impl,
    attrs = {
        "deps": attr.label_list(cfg = ios_device_arm64_transition),
        "bundle_id": attr.string(mandatory = True),
        "app_name": attr.string(default = ""),
        "minimum_os_version": attr.string(default = "18.0"),
        "team_id": attr.string(mandatory = True),
        "infoplist": attr.label(allow_single_file = [".plist"], mandatory = True),
        "provisioning_profile": attr.label(allow_single_file = [".mobileprovision"]),
        "entitlements": attr.label(allow_single_file = [".plist"]),
        "resources": attr.label_list(allow_files = True),
        "app_icons": attr.label_list(allow_files = True),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
