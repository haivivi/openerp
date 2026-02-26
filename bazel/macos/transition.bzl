"""macOS platform transitions — route deps through macOS toolchain."""

def _macos_arm64_impl(settings, attr):
    return {
        "//command_line_option:platforms": [
            "@apple_support//platforms:macos_arm64",
        ],
    }

def _macos_x86_64_impl(settings, attr):
    return {
        "//command_line_option:platforms": [
            "@apple_support//platforms:macos_x86_64",
        ],
    }

def _catalyst_arm64_impl(settings, attr):
    return {
        "//command_line_option:platforms": [
            "//bazel/macos:catalyst_arm64",
        ],
    }

macos_arm64_transition = transition(
    implementation = _macos_arm64_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

macos_x86_64_transition = transition(
    implementation = _macos_x86_64_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

catalyst_arm64_transition = transition(
    implementation = _catalyst_arm64_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)
