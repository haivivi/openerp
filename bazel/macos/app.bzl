"""macos_app — build a Mac Catalyst .app bundle."""

load(":providers.bzl", "MacosAppInfo")
load(":transition.bzl", "catalyst_arm64_transition")

def _macos_app_impl(ctx):
    bundle_name = ctx.attr.app_name or ctx.label.name
    app_dir = ctx.actions.declare_directory(bundle_name + ".app")
    bundle_id = ctx.attr.bundle_id
    minimum_os = ctx.attr.minimum_os_version

    static_libs = []
    all_dep_files = []
    for dep in ctx.attr.deps:
        if DefaultInfo in dep:
            for f in dep[DefaultInfo].files.to_list():
                all_dep_files.append(f)
                if f.path.endswith(".a"):
                    static_libs.append(f)

    resource_files = ctx.files.resources
    src_files = ctx.files.srcs
    infoplist = ctx.file.infoplist
    icon_files = ctx.files.app_icons

    if not src_files:
        fail("macos_app requires non-empty srcs for Catalyst build")

    libs_str = " ".join(["'" + f.path + "'" for f in static_libs])
    srcs_str = " ".join(["'" + f.path + "'" for f in src_files])
    target_triple = "arm64-apple-ios{}-macabi".format(minimum_os)

    res_cmds = []
    for f in resource_files:
        res_cmds.append("cp '{src}' \"$APP_DIR/Contents/Resources/\"".format(src = f.path))
    for f in icon_files:
        if f.path.endswith(".png"):
            res_cmds.append("cp '{src}' \"$APP_DIR/Contents/Resources/\"".format(src = f.path))

    script = """\
set -euo pipefail
APP_DIR="{app_dir}"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

SDK_PATH=$(xcrun --sdk macosx --show-sdk-path)
SWIFT_LIB=$(dirname $(xcrun --toolchain default --find swiftc))/../lib/swift/maccatalyst

xcrun --sdk macosx swiftc -target {target_triple} \
    -sdk "$SDK_PATH" \
    -F"$SDK_PATH/System/iOSSupport/System/Library/Frameworks" \
    -L"$SWIFT_LIB" \
    -Xlinker -rpath -Xlinker /usr/lib/swift \
    -Xlinker -rpath -Xlinker @executable_path/../Frameworks \
    -framework Foundation -framework UIKit -framework SwiftUI -framework Combine \
    {srcs} \
    {libs} \
    -o "$APP_DIR/Contents/MacOS/{bundle_name}"

cp '{infoplist}' "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier {bundle_id}" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string {bundle_id}" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable {bundle_name}" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string {bundle_name}" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleName {bundle_name}" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleName string {bundle_name}" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Delete :UIDeviceFamily" "$APP_DIR/Contents/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add :UIDeviceFamily array" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :UIDeviceFamily:0 integer 2" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :UIDesignRequiresCompatibility true" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :UIDesignRequiresCompatibility bool true" "$APP_DIR/Contents/Info.plist"

{res_cmds}

codesign --force --sign - --timestamp=none "$APP_DIR"
""".format(
        app_dir = app_dir.path,
        bundle_name = bundle_name,
        bundle_id = bundle_id,
        target_triple = target_triple,
        srcs = srcs_str,
        libs = libs_str,
        infoplist = infoplist.path,
        res_cmds = "\n".join(res_cmds),
    )

    ctx.actions.run_shell(
        outputs = [app_dir],
        inputs = static_libs + all_dep_files + src_files + resource_files + [infoplist] + icon_files,
        command = script,
        mnemonic = "MacosApp",
        progress_message = "Bundling Mac Catalyst app %s" % bundle_name,
        use_default_shell_env = True,
    )

    return [
        DefaultInfo(files = depset([app_dir])),
        MacosAppInfo(
            app_dir = app_dir,
            app_path = app_dir.path + "/Contents/MacOS/" + bundle_name,
            bundle_id = bundle_id,
            minimum_os = minimum_os,
            team_id = "",
        ),
    ]

macos_app = rule(
    implementation = _macos_app_impl,
    attrs = {
        "deps": attr.label_list(cfg = catalyst_arm64_transition),
        "srcs": attr.label_list(allow_files = [".swift"]),
        "bundle_id": attr.string(mandatory = True),
        "app_name": attr.string(default = ""),
        "minimum_os_version": attr.string(default = "18.0"),
        "infoplist": attr.label(allow_single_file = [".plist"], mandatory = True),
        "resources": attr.label_list(allow_files = True),
        "app_icons": attr.label_list(allow_files = True),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
