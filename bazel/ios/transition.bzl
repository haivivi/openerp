"""iOS platform transitions — route deps through iOS toolchain."""

def _ios_sim_arm64_impl(settings, attr):
    return {
        "//command_line_option:platforms": [
            "@apple_support//platforms:ios_sim_arm64",
        ],
    }

def _ios_device_arm64_impl(settings, attr):
    return {
        "//command_line_option:platforms": [
            "@apple_support//platforms:ios_arm64",
        ],
    }

ios_sim_arm64_transition = transition(
    implementation = _ios_sim_arm64_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

ios_device_arm64_transition = transition(
    implementation = _ios_device_arm64_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)
