/// FluxStore — Flutter equivalent of the Swift FluxStore / FluxBridge.
///
/// Two modes:
/// 1. **FFI mode** (`FluxStore.ffi(libPath)`) — all state, i18n, and actions
///    go through the Rust Flux engine via dart:ffi, exactly like Swift.
/// 2. **Mock mode** (`FluxStore()`) — state set directly via [setState],
///    i18n from Dart tables. Used for golden tests that don't need the
///    Rust library.
library;

import 'dart:convert';

import 'package:flutter/widgets.dart';

import '../models/models.dart';
import 'flux_bridge.dart';
import 'i18n.dart';

// ---------------------------------------------------------------------------
// JSON decoder registry — maps Dart types to fromJson factories.
// Mirrors Swift's JSONDecoder.decode(T.self, from: data).
// ---------------------------------------------------------------------------

typedef _JsonFactory<T> = T Function(Map<String, dynamic>);

final Map<Type, _JsonFactory<dynamic>> _decoders = {
  AuthState: (j) => AuthState.fromJson(j),
  TimelineFeed: (j) => TimelineFeed.fromJson(j),
  ComposeState: (j) => ComposeState.fromJson(j),
  SearchState: (j) => SearchState.fromJson(j),
  SettingsState: (j) => SettingsState.fromJson(j),
  PasswordState: (j) => PasswordState.fromJson(j),
  InboxState: (j) => InboxState.fromJson(j),
  // ProfilePage and TweetDetailState use dynamic paths;
  // resolved by path prefix in _decodeByPath.
  ProfilePage: (j) => ProfilePage.fromJson(j),
  TweetDetailState: (j) => TweetDetailState.fromJson(j),
};

// ---------------------------------------------------------------------------
// FluxStore
// ---------------------------------------------------------------------------

class FluxStore extends ChangeNotifier {
  final FluxBridge? _bridge;

  /// Mock state — used when [_bridge] is null (golden tests).
  final Map<String, dynamic> _mockState = {};
  String _locale = 'en';

  /// Create a store in mock mode (for golden tests).
  FluxStore() : _bridge = null;

  /// Create a store backed by the Rust FFI engine (production / integration).
  FluxStore.ffi(String libraryPath) : _bridge = FluxBridge.open(libraryPath);

  /// Internal constructor for dependency injection.
  FluxStore.withBridge(FluxBridge bridge) : _bridge = bridge;

  /// Whether this store is connected to the Rust engine.
  bool get isFFI => _bridge != null;

  @override
  void dispose() {
    _bridge?.dispose();
    super.dispose();
  }

  // -- State access --------------------------------------------------------

  /// Read state at [path], decoded as [T].
  /// In FFI mode, calls `flux_get` → JSON → fromJson.
  /// In mock mode, returns from the in-memory map.
  T? get<T>(String path) {
    if (_bridge != null) {
      return _getViaFFI<T>(path);
    }
    return _mockState[path] as T?;
  }

  /// Write state directly (mock mode only, for golden tests).
  void setState(String path, dynamic value) {
    _mockState[path] = value;
    notifyListeners();
  }

  // -- Actions -------------------------------------------------------------

  /// Dispatch an action.
  /// In FFI mode, calls `flux_emit` with JSON payload → Rust handles everything.
  /// In mock mode, no-op (golden tests pre-populate state).
  void emit(String path, [Map<String, dynamic>? json]) {
    final bridge = _bridge;
    if (bridge != null) {
      final payload = json != null ? jsonEncode(json) : null;
      bridge.emit(path, payload);
    }
    notifyListeners();
  }

  // -- I18n ----------------------------------------------------------------

  /// Translate [key].
  /// In FFI mode, calls `flux_i18n_get` — translations come from Rust.
  /// In mock mode, looks up the Dart translation table.
  String t(String key) {
    final bridge = _bridge;
    if (bridge != null) return bridge.t(key);
    return _translateLocal(key);
  }

  /// Current locale code (e.g. `"en"`, `"zh-CN"`).
  String get locale {
    final bridge = _bridge;
    if (bridge != null) return bridge.t('ui/lang/code');
    return _locale;
  }

  /// Switch locale.
  void setLocale(String locale) {
    final bridge = _bridge;
    if (bridge != null) {
      bridge.setLocale(locale);
    } else {
      _locale = locale;
    }
    notifyListeners();
  }

  // -- Server info ---------------------------------------------------------

  String get serverURL {
    final bridge = _bridge;
    if (bridge != null) return bridge.serverURL;
    return 'http://localhost:8080';
  }

  String? get dashboardURL => '$serverURL/dashboard';

  // -- FFI helpers ---------------------------------------------------------

  T? _getViaFFI<T>(String path) {
    final jsonStr = _bridge!.getJson(path);
    if (jsonStr == null) return null;

    final dynamic decoded = jsonDecode(jsonStr);
    if (decoded is! Map<String, dynamic>) return null;

    // Try exact type match first.
    final factory = _decoders[T];
    if (factory != null) return factory(decoded) as T;

    // Dynamic paths: profile/{id} → ProfilePage, tweet/{id} → TweetDetailState.
    if (path.startsWith('profile/')) {
      return ProfilePage.fromJson(decoded) as T;
    }
    if (path.startsWith('tweet/')) {
      return TweetDetailState.fromJson(decoded) as T;
    }

    return null;
  }

  // -- Mock i18n (fallback when no FFI) ------------------------------------

  String _translateLocal(String key) {
    final qIndex = key.indexOf('?');
    if (qIndex == -1) {
      return kTranslations[_locale]?[key] ?? kTranslations['en']?[key] ?? key;
    }

    final basePath = key.substring(0, qIndex);
    final queryStr = key.substring(qIndex + 1);
    var template =
        kTranslations[_locale]?[basePath] ??
        kTranslations['en']?[basePath] ??
        basePath;

    for (final pair in queryStr.split('&')) {
      final eqIndex = pair.indexOf('=');
      if (eqIndex > 0) {
        final name = pair.substring(0, eqIndex);
        final value = pair.substring(eqIndex + 1);
        template = template.replaceAll('{$name}', value);
      }
    }
    return template;
  }
}

// ---------------------------------------------------------------------------
// FluxStoreScope — InheritedNotifier for propagating the store down the tree.
// ---------------------------------------------------------------------------

class FluxStoreScope extends InheritedNotifier<FluxStore> {
  const FluxStoreScope({
    super.key,
    required FluxStore store,
    required super.child,
  }) : super(notifier: store);

  static FluxStore of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<FluxStoreScope>();
    assert(scope != null, 'No FluxStoreScope found in context');
    return scope!.notifier!;
  }
}
