// FluxBridge — Swift wrapper around Flux C FFI.
// Uses @_silgen_name to declare C functions — no bridging header needed.

import Foundation
import Combine

// MARK: - C FFI declarations (linked from Rust static library)

@_silgen_name("flux_create")
private func _flux_create() -> OpaquePointer?

@_silgen_name("flux_free")
private func _flux_free(_ handle: OpaquePointer?)

@_silgen_name("flux_get")
private func _flux_get(_ handle: OpaquePointer?, _ path: UnsafePointer<CChar>?) -> _FluxBytes

@_silgen_name("flux_bytes_free")
private func _flux_bytes_free(_ bytes: _FluxBytes)

@_silgen_name("flux_emit")
private func _flux_emit(_ handle: OpaquePointer?, _ path: UnsafePointer<CChar>?, _ payload: UnsafePointer<CChar>?)

@_silgen_name("flux_server_url")
private func _flux_server_url(_ handle: OpaquePointer?) -> UnsafePointer<CChar>?

@_silgen_name("flux_i18n_get")
private func _flux_i18n_get(_ handle: OpaquePointer?, _ url: UnsafePointer<CChar>?) -> _FluxBytes

@_silgen_name("flux_i18n_set_locale")
private func _flux_i18n_set_locale(_ handle: OpaquePointer?, _ locale: UnsafePointer<CChar>?)

private struct _FluxBytes {
    let ptr: UnsafePointer<UInt8>?
    let len: Int
}

// MARK: - FluxStore (ObservableObject for SwiftUI)

/// Main interface to the Flux state engine from Swift.
///
/// Usage:
/// ```
/// @StateObject var store = FluxStore()
///
/// // Read state:
/// let auth: AuthState? = store.get("auth/state")
///
/// // Send request:
/// store.emit("auth/login", json: ["username": "alice"])
/// ```
final class FluxStore: ObservableObject {

    private let handle: OpaquePointer

    /// Bumped on every emit — SwiftUI observes this to trigger re-renders.
    @Published var revision: UInt64 = 0

    init() {
        guard let h = _flux_create() else {
            fatalError("flux_create() returned null")
        }
        handle = h
    }

    deinit {
        _flux_free(handle)
    }

    // MARK: - I18n

    /// Get a translated string synchronously.
    func t(_ url: String) -> String {
        let bytes = url.withCString { _flux_i18n_get(handle, $0) }
        defer { _flux_bytes_free(bytes) }
        guard bytes.len > 0, let ptr = bytes.ptr else { return url }
        return String(bytes: UnsafeBufferPointer(start: ptr, count: bytes.len), encoding: .utf8) ?? url
    }

    /// Set the i18n locale (e.g. "zh-CN", "en", "ja", "es").
    func setLocale(_ locale: String) {
        locale.withCString { _flux_i18n_set_locale(handle, $0) }
        objectWillChange.send()
        revision &+= 1
    }

    // MARK: - State

    /// Get state at a path, decoded as T.
    /// Call from SwiftUI views — reads `revision` to establish dependency.
    func get<T: Decodable>(_ path: String) -> T? {
        // Touch revision so SwiftUI knows to re-call when it changes.
        let _ = revision
        return getSync(path)
    }

    /// Get state without revision tracking (for non-SwiftUI code / tests).
    func getSync<T: Decodable>(_ path: String) -> T? {
        let bytes = path.withCString { _flux_get(handle, $0) }
        defer { _flux_bytes_free(bytes) }
        guard let ptr = bytes.ptr, bytes.len > 0 else { return nil }
        let data = Data(bytes: ptr, count: bytes.len)
        return try? JSONDecoder().decode(T.self, from: data)
    }

    // MARK: - Requests

    /// Emit a request with a JSON dictionary payload.
    func emit(_ path: String, json: [String: Any]) {
        if let data = try? JSONSerialization.data(withJSONObject: json),
           let str = String(data: data, encoding: .utf8) {
            path.withCString { p in
                str.withCString { j in
                    _flux_emit(handle, p, j)
                }
            }
        }
        objectWillChange.send()
        revision &+= 1
    }

    /// Emit a parameterless request.
    func emit(_ path: String) {
        path.withCString { _flux_emit(handle, $0, nil) }
        objectWillChange.send()
        revision &+= 1
    }

    // MARK: - Server Info

    /// The backend server URL (accessible from LAN).
    var serverURL: String {
        guard let ptr = _flux_server_url(handle) else { return "" }
        return String(cString: ptr)
    }

    /// Dashboard URL for the admin panel.
    var dashboardURL: URL? {
        URL(string: "\(serverURL)/dashboard")
    }
}
