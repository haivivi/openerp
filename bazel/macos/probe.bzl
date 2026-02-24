"""macOS probe — test macOS toolchain availability."""

def _macos_probe_impl(ctx):
    script = """
echo "Checking macOS toolchain..."
xcrun --sdk macosx --show-sdk-path
swiftc --version
"""

    result = ctx.actions.run_shell(
        outputs = [],
        command = script,
        mnemonic = "MacosProbe",
        progress_message = "Probing macOS toolchain",
    )

    return []

macos_probe = rule(
    implementation = _macos_probe_impl,
)
