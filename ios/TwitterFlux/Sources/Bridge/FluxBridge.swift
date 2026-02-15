// FluxBridge — Swift wrapper around Flux C FFI.
// Uses @_silgen_name to declare C functions — no bridging header needed in Bazel.

import Foundation

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

private struct _FluxBytes {
    let ptr: UnsafePointer<UInt8>?
    let len: Int
}

// MARK: - FluxStore (ObservableObject for SwiftUI)

/// Main interface to the Flux state engine from Swift.
/// Wraps the C FFI and provides typed, SwiftUI-friendly APIs.
@MainActor
final class FluxStore: ObservableObject {

    private let handle: OpaquePointer

    /// Revision counter — bumped on every emit to trigger SwiftUI updates.
    @Published private(set) var revision: UInt64 = 0

    init() {
        guard let h = _flux_create() else {
            fatalError("flux_create() returned null")
        }
        handle = h
    }

    deinit {
        _flux_free(handle)
    }

    // MARK: - State

    /// Get state at a path, decoded as T.
    func get<T: Decodable>(_ path: String) -> T? {
        let _ = revision // observe revision for SwiftUI reactivity
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
        revision += 1
    }

    /// Emit a parameterless request.
    func emit(_ path: String) {
        path.withCString { _flux_emit(handle, $0, nil) }
        revision += 1
    }
}
