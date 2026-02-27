/// FluxBridge — Dart FFI wrapper around Flux C API.
///
/// Mirrors Swift's FluxBridge.swift exactly:
/// - flux_create / flux_free — lifecycle
/// - flux_get / flux_bytes_free — read state (JSON)
/// - flux_emit — dispatch request
/// - flux_i18n_get / flux_i18n_set_locale — translation
/// - flux_server_url — embedded server URL
library;

import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';

// ---------------------------------------------------------------------------
// C struct: FluxBytes { const uint8_t *ptr; size_t len; }
// ---------------------------------------------------------------------------

final class FluxBytes extends Struct {
  external Pointer<Uint8> ptr;

  @Size()
  external int len;
}

// ---------------------------------------------------------------------------
// FluxBridge
// ---------------------------------------------------------------------------

class FluxBridge {
  final DynamicLibrary _lib;
  final Pointer<Void> _handle;

  // Cached function pointers.
  late final FluxBytes Function(Pointer<Void>, Pointer<Utf8>) _getBytes;
  late final void Function(FluxBytes) _bytesFree;
  late final void Function(Pointer<Void>, Pointer<Utf8>, Pointer<Utf8>) _emit;
  late final FluxBytes Function(Pointer<Void>, Pointer<Utf8>) _i18nGet;
  late final void Function(Pointer<Void>, Pointer<Utf8>) _i18nSetLocale;
  late final Pointer<Utf8> Function(Pointer<Void>) _serverUrl;

  FluxBridge._(this._lib, this._handle) {
    _getBytes = _lib
        .lookupFunction<
          FluxBytes Function(Pointer<Void>, Pointer<Utf8>),
          FluxBytes Function(Pointer<Void>, Pointer<Utf8>)
        >('flux_get');

    _bytesFree = _lib
        .lookupFunction<Void Function(FluxBytes), void Function(FluxBytes)>(
          'flux_bytes_free',
        );

    _emit = _lib
        .lookupFunction<
          Void Function(Pointer<Void>, Pointer<Utf8>, Pointer<Utf8>),
          void Function(Pointer<Void>, Pointer<Utf8>, Pointer<Utf8>)
        >('flux_emit');

    _i18nGet = _lib
        .lookupFunction<
          FluxBytes Function(Pointer<Void>, Pointer<Utf8>),
          FluxBytes Function(Pointer<Void>, Pointer<Utf8>)
        >('flux_i18n_get');

    _i18nSetLocale = _lib
        .lookupFunction<
          Void Function(Pointer<Void>, Pointer<Utf8>),
          void Function(Pointer<Void>, Pointer<Utf8>)
        >('flux_i18n_set_locale');

    _serverUrl = _lib
        .lookupFunction<
          Pointer<Utf8> Function(Pointer<Void>),
          Pointer<Utf8> Function(Pointer<Void>)
        >('flux_server_url');
  }

  /// Open the Flux shared library at [libraryPath] and create a new engine.
  factory FluxBridge.open(String libraryPath) {
    final lib = DynamicLibrary.open(libraryPath);
    final create = lib
        .lookupFunction<Pointer<Void> Function(), Pointer<Void> Function()>(
          'flux_create',
        );

    final handle = create();
    if (handle == nullptr) {
      throw StateError('flux_create() returned null');
    }
    return FluxBridge._(lib, handle);
  }

  /// Free the Flux engine handle.
  void dispose() {
    final free = _lib
        .lookupFunction<
          Void Function(Pointer<Void>),
          void Function(Pointer<Void>)
        >('flux_free');
    free(_handle);
  }

  // -- State ---------------------------------------------------------------

  /// Read state at [path] as a JSON string. Returns `null` if not found.
  String? getJson(String path) {
    final pathPtr = path.toNativeUtf8();
    try {
      final bytes = _getBytes(_handle, pathPtr);
      if (bytes.ptr == nullptr || bytes.len == 0) return null;
      try {
        return utf8.decode(bytes.ptr.asTypedList(bytes.len));
      } finally {
        _bytesFree(bytes);
      }
    } finally {
      malloc.free(pathPtr);
    }
  }

  // -- Requests ------------------------------------------------------------

  /// Emit a request. [payloadJson] is optional (null for parameterless).
  void emit(String path, [String? payloadJson]) {
    final pathPtr = path.toNativeUtf8();
    final Pointer<Utf8> payloadPtr = payloadJson != null
        ? payloadJson.toNativeUtf8()
        : nullptr.cast<Utf8>();
    try {
      _emit(_handle, pathPtr, payloadPtr);
    } finally {
      malloc.free(pathPtr);
      if (payloadJson != null) malloc.free(payloadPtr);
    }
  }

  // -- I18n ----------------------------------------------------------------

  /// Get a translated string. Synchronous.
  /// [url] can be `"path"` or `"path?key=value&key2=value2"`.
  String t(String url) {
    final urlPtr = url.toNativeUtf8();
    try {
      final bytes = _i18nGet(_handle, urlPtr);
      if (bytes.ptr == nullptr || bytes.len == 0) return url;
      try {
        return utf8.decode(bytes.ptr.asTypedList(bytes.len));
      } finally {
        _bytesFree(bytes);
      }
    } finally {
      malloc.free(urlPtr);
    }
  }

  /// Set the i18n locale (e.g. "zh-CN", "en", "ja", "es").
  void setLocale(String locale) {
    final localePtr = locale.toNativeUtf8();
    try {
      _i18nSetLocale(_handle, localePtr);
    } finally {
      malloc.free(localePtr);
    }
  }

  // -- Server info ---------------------------------------------------------

  /// The embedded backend server URL (e.g. "http://192.168.1.100:3000").
  String get serverURL {
    final ptr = _serverUrl(_handle);
    if (ptr == nullptr) return '';
    return ptr.toDartString();
  }
}
