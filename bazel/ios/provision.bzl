"""ios_provision — register Bundle ID + create App on App Store Connect.

Usage:
  bazel run //path:provision

Requires App Store Connect API Key via environment:
  ASC_KEY_ID, ASC_ISSUER_ID, ASC_KEY_FILE
"""

def _ios_provision_impl(ctx):
    runner = ctx.actions.declare_file(ctx.label.name + "_provision.sh")

    bundle_id = ctx.attr.bundle_id
    app_name = ctx.attr.app_name
    team_id = ctx.attr.team_id

    script = """\
#!/bin/bash
set -euo pipefail

BUNDLE_ID="{bundle_id}"
APP_NAME="{app_name}"
TEAM_ID="{team_id}"
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
    echo ""
    echo "Create a key at: https://appstoreconnect.apple.com/access/integrations/api"
    exit 1
fi

echo "=== iOS Provision ==="
echo "Bundle ID: $BUNDLE_ID"
echo "App Name:  $APP_NAME"
echo "Team ID:   $TEAM_ID"
echo "API Key:   $KEY_ID"
echo ""

# Generate JWT via python3 (handles ES256 correctly)
TOKEN=$(python3 -c "
import json, time, base64, hashlib, subprocess, struct

def b64url(data):
    return base64.urlsafe_b64encode(data).rstrip(b'=').decode()

header = b64url(json.dumps({{'alg':'ES256','kid':'$KEY_ID','typ':'JWT'}}).encode())
now = int(time.time())
payload = b64url(json.dumps({{'iss':'$ISSUER_ID','iat':now,'exp':now+1200,'aud':'appstoreconnect-v1'}}).encode())
unsigned = f'{{header}}.{{payload}}'.encode()

# Sign with openssl, get DER signature
import tempfile, os
msg_file = tempfile.mktemp()
with open(msg_file, 'wb') as f: f.write(unsigned)
der_sig = subprocess.check_output(['openssl', 'dgst', '-sha256', '-sign', '$KEY_FILE', msg_file])
os.unlink(msg_file)

# Convert DER to raw r||s (64 bytes)
# DER: 30 len 02 rlen r 02 slen s
i = 2
rlen = der_sig[3]
r = der_sig[4:4+rlen]
slen = der_sig[5+rlen]
s = der_sig[6+rlen:6+rlen+slen]
# Pad/trim to 32 bytes each
r = r[-32:].rjust(32, b'\\x00')
s = s[-32:].rjust(32, b'\\x00')

sig = b64url(r + s)
print(f'{{header}}.{{payload}}.{{sig}}')
")
API="https://api.appstoreconnect.apple.com/v1"
AUTH="Authorization: Bearer $TOKEN"

# Step 1: Check if Bundle ID exists
echo "--- Checking Bundle ID ---"
EXISTING=$(curl -s -H "$AUTH" "$API/bundleIds?filter[identifier]=$BUNDLE_ID" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('data',[])))" 2>/dev/null || echo "0")

if [ "$EXISTING" != "0" ]; then
    echo "Bundle ID $BUNDLE_ID already registered."
    BUNDLE_ID_RESOURCE=$(curl -s -H "$AUTH" "$API/bundleIds?filter[identifier]=$BUNDLE_ID" | python3 -c "import sys,json; print(json.load(sys.stdin)['data'][0]['id'])")
else
    echo "Registering Bundle ID $BUNDLE_ID..."
    RESULT=$(curl -s -X POST -H "$AUTH" -H "Content-Type: application/json" "$API/bundleIds" -d "$(cat <<JSONEOF
{{
  "data": {{
    "type": "bundleIds",
    "attributes": {{
      "identifier": "$BUNDLE_ID",
      "name": "$APP_NAME",
      "platform": "IOS"
    }}
  }}
}}
JSONEOF
)")
    BUNDLE_ID_RESOURCE=$(echo "$RESULT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{{}}).get('id',''))" 2>/dev/null || echo "")
    if [ -z "$BUNDLE_ID_RESOURCE" ]; then
        echo "ERROR: Failed to register Bundle ID"
        echo "$RESULT" | python3 -m json.tool 2>/dev/null || echo "$RESULT"
        exit 1
    fi
    echo "Registered! Resource ID: $BUNDLE_ID_RESOURCE"
fi

# Step 2: Check if App exists
echo ""
echo "--- Checking App ---"
APP_EXISTS=$(curl -s -H "$AUTH" "$API/apps?filter[bundleId]=$BUNDLE_ID" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('data',[])))" 2>/dev/null || echo "0")

if [ "$APP_EXISTS" != "0" ]; then
    echo "App $APP_NAME already exists on App Store Connect."
else
    echo "Creating App $APP_NAME..."
    RESULT=$(curl -s -X POST -H "$AUTH" -H "Content-Type: application/json" "$API/apps" -d "$(cat <<JSONEOF
{{
  "data": {{
    "type": "apps",
    "attributes": {{
      "name": "$APP_NAME",
      "primaryLocale": "zh-Hans",
      "bundleId": "$BUNDLE_ID_RESOURCE",
      "sku": "$(echo $BUNDLE_ID | tr '.' '-')"
    }},
    "relationships": {{
      "bundleId": {{
        "data": {{
          "type": "bundleIds",
          "id": "$BUNDLE_ID_RESOURCE"
        }}
      }}
    }}
  }}
}}
JSONEOF
)")
    APP_ID=$(echo "$RESULT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('data',{{}}).get('id',''))" 2>/dev/null || echo "")
    if [ -z "$APP_ID" ]; then
        echo "ERROR: Failed to create App"
        echo "$RESULT" | python3 -m json.tool 2>/dev/null || echo "$RESULT"
        exit 1
    fi
    echo "Created! App ID: $APP_ID"
fi

echo ""
echo "=== Provision Complete ==="
echo "Bundle ID: $BUNDLE_ID"
echo "You can now upload builds with: bazel run //prototype/swift/Moca:upload"
""".format(
        bundle_id = bundle_id,
        app_name = app_name,
        team_id = team_id,
        issuer_id = ctx.attr.issuer_id,
    )

    ctx.actions.write(
        output = runner,
        content = script,
        is_executable = True,
    )

    return [DefaultInfo(executable = runner)]

ios_provision = rule(
    implementation = _ios_provision_impl,
    executable = True,
    attrs = {
        "bundle_id": attr.string(mandatory = True),
        "app_name": attr.string(mandatory = True),
        "team_id": attr.string(mandatory = True),
        "issuer_id": attr.string(default = ""),
    },
)
