"""macOS app providers."""

MacosAppInfo = provider(
    doc = "Provider for macOS application bundles",
    fields = {
        "app_dir": "The .app bundle directory",
        "app_path": "Path to the executable inside the app bundle",
        "bundle_id": "The application bundle identifier",
        "minimum_os": "Minimum macOS version",
        "team_id": "Development team ID",
    },
)
