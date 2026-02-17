"""Public API for haivivi iOS Bazel rules."""

load(":app.bzl", _ios_app = "ios_app")
load(":archive.bzl", _ios_ipa = "ios_ipa")
load(":providers.bzl", _IosAppInfo = "IosAppInfo")
load(":provision.bzl", _ios_provision = "ios_provision")
load(":testflight.bzl", _ios_testflight = "ios_testflight")
load(":ui_test.bzl", _ios_ui_runner = "ios_ui_runner")

ios_app = _ios_app
ios_ipa = _ios_ipa
ios_provision = _ios_provision
ios_testflight = _ios_testflight
ios_ui_runner = _ios_ui_runner
IosAppInfo = _IosAppInfo
