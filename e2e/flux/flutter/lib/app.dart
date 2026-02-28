/// TwitterFluxApp — entry point (mirrors Swift TwitterFluxApp + RootView).
///
/// Flux owns all state. Flutter only renders.
library;

import 'package:flutter/cupertino.dart';

import 'models/models.dart';
import 'store/flux_store.dart';
import 'views/login_view.dart';
import 'views/main_tab_view.dart';

class TwitterFluxApp extends StatelessWidget {
  final FluxStore store;

  const TwitterFluxApp({super.key, required this.store});

  @override
  Widget build(BuildContext context) {
    return FluxStoreScope(
      store: store,
      child: const CupertinoApp(
        title: 'TwitterFlux',
        theme: CupertinoThemeData(brightness: Brightness.light),
        home: _RootView(),
        debugShowCheckedModeBanner: false,
      ),
    );
  }
}

/// Root view — routes based on auth state (login vs main tabs).
class _RootView extends StatelessWidget {
  const _RootView();

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final auth = store.get<AuthState>('auth/state');
    final isLoggedIn = auth?.phase == AuthPhase.authenticated;

    if (isLoggedIn) {
      return const MainTabView();
    }
    return const LoginView();
  }
}
