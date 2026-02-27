/// FluxStore — Flutter equivalent of the Swift FluxStore.
///
/// Provides state management, action dispatch, and i18n translation.
/// In production, `emit` routes through Rust FFI; in tests, state is
/// pre-populated directly via [setState].
library;

import 'package:flutter/widgets.dart';

import 'i18n.dart';

// ---------------------------------------------------------------------------
// FluxStore
// ---------------------------------------------------------------------------

class FluxStore extends ChangeNotifier {
  final Map<String, dynamic> _state = {};
  String _locale = 'en';
  String _serverURL = 'http://localhost:8080';

  // -- State access --------------------------------------------------------

  /// Read state at [path], cast to [T]. Returns `null` if absent.
  T? get<T>(String path) => _state[path] as T?;

  /// Write state directly (used by tests and action handlers).
  void setState(String path, dynamic value) {
    _state[path] = value;
    notifyListeners();
  }

  // -- Actions -------------------------------------------------------------

  /// Dispatch an action. [json] is the optional payload.
  void emit(String path, [Map<String, dynamic>? json]) {
    _handleAction(path, json);
    notifyListeners();
  }

  // -- I18n ----------------------------------------------------------------

  /// Translate [key], substituting query-string parameters if present.
  ///
  /// Example: `t("format/char_count?current=20&max=280")` →  `"20/280"`.
  String t(String key) {
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

  /// Current locale code (e.g. `"en"`, `"zh-CN"`).
  String get locale => _locale;

  /// Switch locale and notify listeners.
  void setLocale(String locale) {
    _locale = locale;
    notifyListeners();
  }

  // -- Server info ---------------------------------------------------------

  String get serverURL => _serverURL;

  void updateServerURL(String url) {
    _serverURL = url;
  }

  String? get dashboardURL => '$_serverURL/dashboard';

  // -- Action handlers (minimal, enough for integration flows) -------------

  void _handleAction(String path, Map<String, dynamic>? json) {
    switch (path) {
      case 'auth/login':
        _handleLogin(json);
      case 'auth/logout':
        _handleLogout();
      case 'app/initialize':
        break; // no-op for now
      default:
        break;
    }
  }

  void _handleLogin(Map<String, dynamic>? json) {
    if (json == null) return;
    final username = json['username'] as String? ?? '';
    final password = json['password'] as String? ?? '';

    // Simulate: only "alice" / "password" succeeds.
    if (username == 'alice' && password == 'password') {
      _state['auth/state'] = const AuthStateData(
        phase: 'authenticated',
        username: 'alice',
        displayName: 'Alice',
      );
    } else {
      // Leave phase unauthenticated, set error.
      _state['auth/state'] = const AuthStateData(
        phase: 'unauthenticated',
        error: 'Invalid credentials',
      );
    }
  }

  void _handleLogout() {
    _state['auth/state'] = const AuthStateData(phase: 'unauthenticated');
  }
}

/// Lightweight auth data used by action handlers (not the full model).
class AuthStateData {
  final String phase;
  final String? username;
  final String? displayName;
  final String? error;

  const AuthStateData({
    required this.phase,
    this.username,
    this.displayName,
    this.error,
  });
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
