/// MainTabView — bottom tab bar scaffold (mirrors Swift MainTabView.iOS).
library;

import 'package:flutter/cupertino.dart';

import '../store/flux_store.dart';
import 'home_view.dart';
import 'inbox_view.dart';
import 'me_view.dart';
import 'search_view.dart';

class MainTabView extends StatelessWidget {
  const MainTabView({super.key});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);

    return CupertinoTabScaffold(
      tabBar: CupertinoTabBar(
        items: [
          BottomNavigationBarItem(
            icon: const Icon(CupertinoIcons.house_fill),
            label: store.t('ui/tab/home'),
          ),
          BottomNavigationBarItem(
            icon: const Icon(CupertinoIcons.search),
            label: store.t('ui/tab/search'),
          ),
          BottomNavigationBarItem(
            icon: const Icon(CupertinoIcons.tray_fill),
            label: store.t('ui/tab/inbox'),
          ),
          BottomNavigationBarItem(
            icon: const Icon(CupertinoIcons.person_fill),
            label: store.t('ui/tab/me'),
          ),
        ],
      ),
      tabBuilder: (context, index) {
        return CupertinoTabView(
          builder: (context) {
            switch (index) {
              case 0:
                return const HomeView();
              case 1:
                return const SearchView();
              case 2:
                return const InboxView();
              case 3:
                return const MeView();
              default:
                return const HomeView();
            }
          },
        );
      },
    );
  }
}
