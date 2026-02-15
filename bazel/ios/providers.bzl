"""Providers for iOS build rules."""

IosAppInfo = provider(
    doc = "Information about a built iOS application.",
    fields = {
        "app_dir": "File: The .app bundle directory (a TreeArtifact).",
        "ipa": "File: The .ipa archive (if built for distribution).",
        "bundle_id": "string: The bundle identifier.",
        "minimum_os": "string: Minimum iOS version.",
        "team_id": "string: Development team ID (if signed).",
    },
)
