/// Unit tests for FluxStore (mock mode) and i18n translation.
///
/// Tests the store's state management, i18n lookup, locale switching,
/// and the InheritedNotifier propagation.
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:twitter_flux/models/models.dart';
import 'package:twitter_flux/store/flux_store.dart';

void main() {
  group('FluxStore — mock mode', () {
    late FluxStore store;

    setUp(() {
      store = FluxStore();
    });

    test('isFFI returns false in mock mode', () {
      expect(store.isFFI, isFalse);
    });

    test('get returns null for absent path', () {
      expect(store.get<AuthState>('auth/state'), isNull);
    });

    test('setState + get round-trip', () {
      const auth = AuthState(
        phase: AuthPhase.authenticated,
        user: UserProfile(id: 'alice', username: 'alice', displayName: 'Alice'),
      );
      store.setState('auth/state', auth);

      final result = store.get<AuthState>('auth/state');
      expect(result, isNotNull);
      expect(result!.phase, AuthPhase.authenticated);
      expect(result.user!.username, 'alice');
    });

    test('setState notifies listeners', () {
      var notified = false;
      store.addListener(() => notified = true);
      store.setState(
        'auth/state',
        const AuthState(phase: AuthPhase.unauthenticated),
      );
      expect(notified, isTrue);
    });

    test('emit notifies listeners', () {
      var notified = false;
      store.addListener(() => notified = true);
      store.emit('app/initialize');
      expect(notified, isTrue);
    });
  });

  group('FluxStore — i18n (mock mode)', () {
    late FluxStore store;

    setUp(() {
      store = FluxStore();
    });

    test('translates known key in English', () {
      expect(store.t('ui/tab/home'), 'Home');
      expect(store.t('ui/login/button'), 'Sign In');
      expect(store.t('ui/me/sign_out'), 'Sign Out');
    });

    test('returns key for unknown translation', () {
      expect(store.t('ui/nonexistent/key'), 'ui/nonexistent/key');
    });

    test('substitutes format parameters', () {
      final result = store.t('format/char_count?current=20&max=280');
      expect(result, '20/280');
    });

    test('format tweet count', () {
      final result = store.t('format/tweet_count?count=42');
      expect(result, '42 tweets');
    });

    test('switches locale to Chinese', () {
      store.setLocale('zh-CN');
      expect(store.t('ui/tab/home'), '首页');
      expect(store.t('ui/login/button'), '登录');
      expect(store.t('ui/me/sign_out'), '退出登录');
    });

    test('switches locale to Japanese', () {
      store.setLocale('ja');
      expect(store.t('ui/tab/home'), 'ホーム');
      expect(store.t('ui/login/button'), 'サインイン');
    });

    test('switches locale to Spanish', () {
      store.setLocale('es');
      expect(store.t('ui/tab/home'), 'Inicio');
      expect(store.t('ui/login/button'), 'Iniciar sesión');
    });

    test('Chinese format parameters', () {
      store.setLocale('zh-CN');
      final result = store.t('format/tweet_count?count=7');
      expect(result, '7 条推文');
    });

    test('setLocale notifies listeners', () {
      var notified = false;
      store.addListener(() => notified = true);
      store.setLocale('ja');
      expect(notified, isTrue);
    });
  });

  group('FluxStore — server info (mock mode)', () {
    test('default server URL', () {
      final store = FluxStore();
      expect(store.serverURL, 'http://localhost:8080');
    });

    test('dashboard URL', () {
      final store = FluxStore();
      expect(store.dashboardURL, 'http://localhost:8080/dashboard');
    });
  });
}
