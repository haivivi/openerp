"""Public API for haivivi macOS Bazel rules."""

load(":app.bzl", _macos_app = "macos_app")
load(":providers.bzl", _MacosAppInfo = "MacosAppInfo")
load(":probe.bzl", _macos_probe = "macos_probe")
load(":ui_test.bzl", _macos_ui_runner = "macos_ui_runner")

macos_app = _macos_app
macos_probe = _macos_probe
macos_ui_runner = _macos_ui_runner
MacosAppInfo = _MacosAppInfo
