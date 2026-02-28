/// Integration test for FluxStore in FFI mode — talks to the real Rust engine.
///
/// Requires the shared library built by:
///   bazel build //rust/lib/flux_ffi:flux_ffi_dylib
///
/// Run with:
///   flutter test test/flux_ffi_test.dart
@TestOn('mac-os')
library;

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:twitter_flux/models/models.dart';
import 'package:twitter_flux/store/flux_store.dart';

/// Resolve the dylib path relative to the repo root.
String? _findDylib() {
  // Walk up from the flutter project dir to find bazel-bin.
  var dir = Directory.current;
  for (var i = 0; i < 5; i++) {
    final candidate = File(
      '${dir.path}/bazel-bin/rust/lib/flux_ffi/libflux_ffi.dylib',
    );
    if (candidate.existsSync()) return candidate.path;
    dir = dir.parent;
  }
  return null;
}

void main() {
  final dylibPath = _findDylib();

  group(
    'FluxStore — FFI mode (real Rust engine)',
    () {
      late FluxStore store;

      setUp(() {
        if (dylibPath == null) {
          fail(
            'libflux_ffi.dylib not found. '
            'Run: bazel build //rust/lib/flux_ffi:flux_ffi_dylib',
          );
        }
        store = FluxStore.ffi(dylibPath!);
      });

      tearDown(() {
        store.dispose();
      });

      test('isFFI returns true', () {
        expect(store.isFFI, isTrue);
      });

      test('serverURL is non-empty after creation', () {
        expect(store.serverURL, isNotEmpty);
        expect(store.serverURL, startsWith('http://'));
      });

      test('i18n translates known keys in English', () {
        expect(store.t('ui/tab/home'), 'Home');
        expect(store.t('ui/login/button'), isNotEmpty);
      });

      test('i18n switches locale to Chinese', () {
        store.setLocale('zh-CN');
        final home = store.t('ui/tab/home');
        // Should be Chinese after switching.
        expect(home, isNot('Home'));
        expect(home, isNotEmpty);
      });

      test('app/initialize populates auth state', () {
        store.emit('app/initialize');
        final auth = store.get<AuthState>('auth/state');
        expect(auth, isNotNull);
        expect(auth!.phase, AuthPhase.unauthenticated);
      });

      test('auth/login with alice/password succeeds', () {
        store.emit('app/initialize');
        store.emit('auth/login', {'username': 'alice', 'password': 'password'});
        final auth = store.get<AuthState>('auth/state');
        expect(auth, isNotNull);
        expect(auth!.phase, AuthPhase.authenticated);
        expect(auth.user, isNotNull);
        expect(auth.user!.username, 'alice');
      });

      test('auth/login with wrong password fails', () {
        store.emit('app/initialize');
        store.emit('auth/login', {'username': 'alice', 'password': 'wrong'});
        final auth = store.get<AuthState>('auth/state');
        expect(auth, isNotNull);
        expect(auth!.phase, AuthPhase.unauthenticated);
        expect(auth.error, isNotNull);
      });

      test('after login, timeline/load returns feed', () {
        store.emit('app/initialize');
        store.emit('auth/login', {'username': 'alice', 'password': 'password'});
        store.emit('timeline/load');
        final feed = store.get<TimelineFeed>('timeline/feed');
        expect(feed, isNotNull);
        expect(feed!.items, isNotEmpty);
      });

      test('after login, inbox/load returns messages', () {
        store.emit('app/initialize');
        store.emit('auth/login', {'username': 'alice', 'password': 'password'});
        store.emit('inbox/load');
        final inbox = store.get<InboxState>('inbox/state');
        expect(inbox, isNotNull);
        expect(inbox!.messages, isNotEmpty);
      });

      test('auth/logout returns to unauthenticated', () {
        store.emit('app/initialize');
        store.emit('auth/login', {'username': 'alice', 'password': 'password'});
        store.emit('auth/logout');
        final auth = store.get<AuthState>('auth/state');
        expect(auth, isNotNull);
        expect(auth!.phase, AuthPhase.unauthenticated);
      });
    },
    skip: dylibPath == null
        ? 'libflux_ffi.dylib not found (run bazel build first)'
        : null,
  );
}
