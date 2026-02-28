import 'dart:io';

import 'package:flutter/cupertino.dart';

import 'app.dart';
import 'store/flux_store.dart';

/// Locate the Flux shared library for the current platform.
String? _findFluxLibrary() {
  // Bazel build output (host development).
  const bazelPath = 'bazel-bin/rust/lib/flux_ffi/libflux_ffi.dylib';

  // Android: loaded from APK's lib/ directory (name varies by platform).
  // macOS/Linux: look for dylib/so next to the executable or in known paths.
  for (final candidate in [
    bazelPath,
    'libflux_ffi.dylib', // macOS
    'libflux_ffi.so', // Linux / Android
  ]) {
    if (File(candidate).existsSync()) return candidate;
  }
  return null;
}

void main() {
  final libPath = _findFluxLibrary();
  final FluxStore store;

  if (libPath != null) {
    // Production: driven by the Rust Flux engine.
    store = FluxStore.ffi(libPath);
    store.emit('app/initialize');
  } else {
    // Fallback: mock mode (no Rust library available).
    store = FluxStore();
  }

  runApp(TwitterFluxApp(store: store));
}
