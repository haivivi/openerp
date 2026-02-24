"""macos_app — build a macOS .app bundle."""

load(":transition.bzl", "macos_arm64_transition")
load(":providers.bzl", "MacosAppInfo")

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
    infoplist = ctx.file.infoplist
    icon_files = ctx.files.app_icons

    libs_str = " ".join(["'" + f.path + "'" for f in static_libs])
    target_triple = "aarch64-apple-darwin"

    res_cmds = []
    for f in resource_files:
        res_cmds.append("cp '{src}' \"$APP_DIR/\"".format(src = f.path))
    for f in icon_files:
        if f.path.endswith(".png"):
            res_cmds.append("cp '{src}' \"$APP_DIR/Contents/Resources/\"".format(src = f.path))

    script = """\
set -euo pipefail
APP_DIR="{app_dir}"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

SDK_PATH=$(xcrun --sdk macosx --show-sdk-path)
SWIFT_LIB=$(dirname $(xcrun --toolchain default --find swiftc))/../lib/swift/macosx
xcrun --sdk macosx clang -arch arm64 -target {target_triple} \
    -isysroot "$SDK_PATH" \
    -F"$SDK_PATH/System/Library/Frameworks" \
    -framework Foundation -framework AppKit -framework SwiftUI -framework Combine \
    -L"$SDK_PATH/usr/lib/swift" \
    -L"$SWIFT_LIB" \
    -Wl,-rpath,/usr/lib/swift \
    -Wl,-rpath,@executable_path/../Frameworks \
    {libs} \
    -o "$APP_DIR/Contents/MacOS/{bundle_name}"

cp '{infoplist}' "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier {bundle_id}" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string {bundle_id}" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable {bundle_name}" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string {bundle_name}" "$APP_DIR/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleName {bundle_name}" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleName string {bundle_name}" "$APP_DIR/Contents/Info.plist"

{res_cmds}

codesign --force --sign - --timestamp=none "$APP_DIR"
""".format(
        app_dir = app_dir.path,
        bundle_name = bundle_name,
        bundle_id = bundle_id,
        target_triple = target_triple,
        libs = libs_str,
        infoplist = infoplist.path,
        res_cmds = "\n".join(res_cmds),
    )

    ctx.actions.run_shell(
        outputs = [app_dir],
        inputs = static_libs + all_dep_files + resource_files + [infoplist] + icon_files,
        command = script,
        mnemonic = "MacosApp",
        progress_message = "Bundling macOS app %s" % bundle_name,
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
        "deps": attr.label_list(cfg = macos_arm64_transition),
        "bundle_id": attr.string(mandatory = True),
        "app_name": attr.string(default = ""),
        "minimum_os_version": attr.string(default = "14.0"),
        "infoplist": attr.label(allow_single_file = [".plist"], mandatory = True),
        "resources": attr.label_list(allow_files = True),
        "app_icons": attr.label_list(allow_files = True),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
