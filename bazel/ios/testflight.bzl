"""ios_testflight — upload IPA to TestFlight.

Usage:
  bazel run //path:upload

Requires: ASC_KEY_ID, ASC_ISSUER_ID, ASC_KEY_FILE
"""

load(":providers.bzl", "IosAppInfo")

def _ios_testflight_impl(ctx):
    app_info = ctx.attr.app[IosAppInfo]
    ipa = app_info.ipa
    if not ipa:
        fail("ios_testflight requires an ios_ipa target")

    runner = ctx.actions.declare_file(ctx.label.name + "_upload.sh")

    script = """\
#!/bin/bash
set -euo pipefail

IPA="$BUILD_WORKSPACE_DIRECTORY/bazel-bin/{ipa_short}"
BUNDLE_ID="{bundle_id}"
KEY_ID="${{ASC_KEY_ID:-}}"
ISSUER_ID="${{ASC_ISSUER_ID:-{issuer_id}}}"
KEY_FILE="${{ASC_KEY_FILE:-}}"

if [ -z "$KEY_ID" ] || [ -z "$KEY_FILE" ]; then
    echo "ERROR: App Store Connect API Key not configured."
    echo ""
    echo "Set these environment variables:"
    echo "  export ASC_KEY_ID=<your key id>"
    echo "  export ASC_ISSUER_ID=<your issuer id>"
    echo "  export ASC_KEY_FILE=<path to AuthKey_XXX.p8>"
    exit 1
fi

# Check IPA exists
if [ ! -f "$IPA" ]; then
    echo "ERROR: IPA not found at $IPA"
    echo "Run 'bazel build {ipa_target}' first."
    exit 1
fi

echo "=== TestFlight Upload ==="
echo "IPA:       $IPA ($(du -h "$IPA" | cut -f1))"
echo "Bundle ID: $BUNDLE_ID"
echo "API Key:   $KEY_ID"
echo ""

# Copy API key to expected location for xcrun
# xcrun altool looks for keys in ~/private_keys/ or ~/.private_keys/ or ~/.appstoreconnect/private_keys/
KEY_DIR="$HOME/.private_keys"
mkdir -p "$KEY_DIR"
cp "$KEY_FILE" "$KEY_DIR/AuthKey_$KEY_ID.p8" 2>/dev/null || true

# Validate
echo "--- Validating ---"
xcrun altool --validate-app \\
    --file "$IPA" \\
    --type ios \\
    --apiKey "$KEY_ID" \\
    --apiIssuer "$ISSUER_ID" \\
    2>&1 || true
echo ""

# Upload
echo "--- Uploading to TestFlight ---"
xcrun altool --upload-app \\
    --file "$IPA" \\
    --type ios \\
    --apiKey "$KEY_ID" \\
    --apiIssuer "$ISSUER_ID"

echo ""
echo "=== Upload Complete ==="
echo "Check processing status at: https://appstoreconnect.apple.com"
""".format(
        ipa_short = ipa.short_path,
        ipa_target = ctx.attr.app.label,
        bundle_id = app_info.bundle_id,
        issuer_id = ctx.attr.issuer_id,
    )

    ctx.actions.write(
        output = runner,
        content = script,
        is_executable = True,
    )

    return [DefaultInfo(
        executable = runner,
        runfiles = ctx.runfiles(files = [ipa]),
    )]

ios_testflight = rule(
    implementation = _ios_testflight_impl,
    executable = True,
    attrs = {
        "app": attr.label(mandatory = True, providers = [IosAppInfo]),
        "issuer_id": attr.string(default = ""),
    },
)
