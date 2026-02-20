"""ios_app — build an iOS .app bundle for simulator."""

load(":transition.bzl", "ios_sim_arm64_transition")
load(":providers.bzl", "IosAppInfo")

def _ios_app_impl(ctx):
    bundle_name = ctx.attr.app_name or ctx.label.name
    app_dir = ctx.actions.declare_directory(bundle_name + ".app")
    bundle_id = ctx.attr.bundle_id
    minimum_os = ctx.attr.minimum_os_version

    # Collect .a static libraries from transitioned deps
    static_libs = []
    all_dep_files = []
    for dep in ctx.attr.deps:
        if DefaultInfo in dep:
            for f in dep[DefaultInfo].files.to_list():
                all_dep_files.append(f)
                if f.path.endswith(".a"):
                    static_libs.append(f)

    # Inputs
    resource_files = ctx.files.resources
    infoplist = ctx.file.infoplist
    icon_files = ctx.files.app_icons

    # Build command
    libs_str = " ".join(["'" + f.path + "'" for f in static_libs])
    target_triple = "arm64-apple-ios{}-simulator".format(minimum_os)

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
mkdir -p "$APP_DIR"

# Link
SDK_PATH=$(xcrun --sdk iphonesimulator --show-sdk-path)
SWIFT_LIB=$(dirname $(xcrun --toolchain default --find swiftc))/../lib/swift/iphonesimulator
xcrun --sdk iphonesimulator clang -arch arm64 -target {target_triple} \
    -isysroot "$SDK_PATH" \
    -F"$SDK_PATH/System/Library/Frameworks" \
    -framework Foundation -framework UIKit -framework SwiftUI \
    -L"$SDK_PATH/usr/lib/swift" \
    -L"$SWIFT_LIB" \
    -Wl,-rpath,/usr/lib/swift \
    -Wl,-rpath,@executable_path/Frameworks \
    {libs} \
    -o "$APP_DIR/{bundle_name}"

# Info.plist
cp '{infoplist}' "$APP_DIR/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier {bundle_id}" "$APP_DIR/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string {bundle_id}" "$APP_DIR/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable {bundle_name}" "$APP_DIR/Info.plist" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string {bundle_name}" "$APP_DIR/Info.plist"

# Resources
{res_cmds}

# Ad-hoc sign (simulator)
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
        mnemonic = "IosApp",
        progress_message = "Bundling iOS app %s" % bundle_name,
        use_default_shell_env = True,
    )

    return [
        DefaultInfo(files = depset([app_dir])),
        IosAppInfo(
            app_dir = app_dir,
            ipa = None,
            bundle_id = bundle_id,
            minimum_os = minimum_os,
            team_id = "",
        ),
    ]

ios_app = rule(
    implementation = _ios_app_impl,
    attrs = {
        "deps": attr.label_list(cfg = ios_sim_arm64_transition),
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
