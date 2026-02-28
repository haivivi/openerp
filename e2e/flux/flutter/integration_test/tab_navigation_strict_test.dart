/// 超级严格的 Tab 导航集成测试
/// 测试内容包括：Tab 切换、导航栈、页面状态、UI 验证
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('🧭 Tab 导航 - 严格测试套件', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
      setupAuthenticatedState(store);
      setupTimelineFeed(store);
      setupInboxState(store);
      setupSearchState(store);
    });

    tearDown(() {
      commonTearDown();
    });

    // ============================================
    // TabBar 基础验证
    // ============================================

    testWidgets('TC-TAB-001: TabBar 必须在底部且有4个 Tab', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // 验证 TabBar 存在
      expect(find.byType(CupertinoTabBar), findsOneWidget);

      // 验证 TabBar 在底部
      expectTabBarAtBottom(tester);

      // 验证有且仅有 4 个 Tab
      expectTabBarHasFourTabs(tester);
    });

    testWidgets('TC-TAB-002: 验证所有 Tab 标签和图标', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      final tabBar = tester.widget<CupertinoTabBar>(
        find.byType(CupertinoTabBar),
      );

      // 验证 Tab 标签
      expect(tabBar.items[0].label, equals('Home'));
      expect(tabBar.items[1].label, equals('Search'));
      expect(tabBar.items[2].label, equals('Inbox'));
      expect(tabBar.items[3].label, equals('Me'));

      // 验证每个 Tab 都有图标
      for (var i = 0; i < 4; i++) {
        expect(
          tabBar.items[i].icon,
          isNotNull,
          reason: 'Tab $i should have an icon',
        );
      }
    });

    testWidgets('TC-TAB-003: 默认选中 Home Tab', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 验证默认在 Home
      // Home 会同时出现在导航标题与 Tab 标签。
      expect(find.text('Home'), findsWidgets);
      expect(find.byType(HomeView), findsOneWidget);

      // 验证 Home Tab 是当前选中
      final tabBar = tester.widget<CupertinoTabBar>(
        find.byType(CupertinoTabBar),
      );
      expect(tabBar.currentIndex, equals(0));
    });

    // ============================================
    // Tab 切换测试
    // ============================================

    testWidgets('TC-TAB-004: Home → Search → Inbox → Me → Home 完整切换', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Home
      expect(find.byType(HomeView), findsOneWidget);

      // Switch to Search
      await tester.tap(find.text('Search'));
      await tester.pumpAndSettle();
      expect(find.byType(SearchView), findsOneWidget);
      expect(find.byType(HomeView), findsNothing);

      // Switch to Inbox
      await tester.tap(find.text('Inbox'));
      await tester.pumpAndSettle();
      expect(find.byType(InboxView), findsOneWidget);
      expect(find.byType(SearchView), findsNothing);

      // Switch to Me
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();
      expect(find.byType(MeView), findsOneWidget);
      expect(find.byType(InboxView), findsNothing);

      // Back to Home
      await tester.tap(find.text('Home'));
      await tester.pumpAndSettle();
      expect(find.byType(HomeView), findsOneWidget);
      expect(find.byType(MeView), findsNothing);
    });

    testWidgets('TC-TAB-005: 快速连续切换 Tab - 应保持稳定', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 快速切换多次
      for (var i = 0; i < 5; i++) {
        await tester.tap(find.text('Search'));
        await tester.pump(const Duration(milliseconds: 50));
        await tester.tap(find.text('Inbox'));
        await tester.pump(const Duration(milliseconds: 50));
        await tester.tap(find.text('Me'));
        await tester.pump(const Duration(milliseconds: 50));
        await tester.tap(find.text('Home'));
        await tester.pump(const Duration(milliseconds: 50));
      }

      await tester.pumpAndSettle();

      // 验证应用没有崩溃
      expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      expect(find.byType(CupertinoTabBar), findsOneWidget);
    });

    // ============================================
    // Home Tab 详细测试
    // ============================================

    testWidgets('TC-TAB-006: Home Tab - 验证时间线内容完整', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // 验证标题
      expect(find.text('Home'), findsWidgets);

      // 验证列表存在
      expect(find.byType(ListView), findsOneWidget);

      // 验证所有推文都显示
      expect(
        find.text('Hello from Bob! This is a test tweet for E2E testing.'),
        findsOneWidget,
      );
      expect(
        find.text('Testing Flutter integration! #flutter #testing'),
        findsOneWidget,
      );
      expect(
        find.text(
          'Just setting up my TwitterFlux account! Excited to be here.',
        ),
        findsOneWidget,
      );

      // 验证作者信息
      expect(find.text('Bob Smith'), findsOneWidget);
      expect(find.text('Charlie Brown'), findsOneWidget);
      expect(find.text('Alice'), findsOneWidget);

      // 验证统计数据
      expect(find.text('5'), findsWidgets); // likes
      expect(find.text('10'), findsWidgets);
      expect(find.text('3'), findsWidgets);
    });

    testWidgets('TC-TAB-007: Home Tab - 验证发帖按钮存在且可点击', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // 验证发帖按钮
      final composeButton = find.byIcon(CupertinoIcons.square_pencil);
      expect(composeButton, findsOneWidget);

      // 点击发帖按钮
      await tester.tap(composeButton);
      await tester.pumpAndSettle();

      // 验证导航到了 Compose 页面
      expect(find.byType(ComposeView), findsOneWidget);
      expect(find.text('New Tweet'), findsOneWidget);
    });

    // ============================================
    // Search Tab 详细测试
    // ============================================

    testWidgets('TC-TAB-008: Search Tab - 验证搜索界面完整', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Search'));
      await tester.pumpAndSettle();

      // 验证标题
      expect(find.text('Search'), findsWidgets);

      // 验证搜索框存在
      expect(find.byType(CupertinoSearchTextField), findsOneWidget);

      // 验证返回按钮不存在（这是根页面）
      expectNoBackButton(tester);
    });

    // ============================================
    // Inbox Tab 详细测试
    // ============================================

    testWidgets('TC-TAB-009: Inbox Tab - 验证消息列表完整', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Inbox'));
      await tester.pumpAndSettle();

      // 验证标题
      expect(find.text('Inbox'), findsWidgets);

      // 验证所有消息都显示
      expect(find.text('Welcome to TwitterFlux'), findsOneWidget);
      expect(
        find.text('Welcome! This is a seeded message for testing purposes.'),
        findsOneWidget,
      );
      expect(find.text('New Like'), findsOneWidget);
      expect(find.text('@bob liked your tweet'), findsOneWidget);
      expect(find.text('New Follower'), findsOneWidget);
      expect(find.text('@charlie started following you'), findsOneWidget);
      expect(find.text('New Mention'), findsOneWidget);
      expect(find.text('@dave mentioned you in a tweet'), findsOneWidget);
    });

    // ============================================
    // Me Tab 详细测试
    // ============================================

    testWidgets('TC-TAB-010: Me Tab - 验证用户信息完整', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // 验证标题
      expect(find.text('Me'), findsWidgets);

      // 验证用户信息
      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('@alice'), findsOneWidget);
      expect(
        find.text('Flutter developer and testing enthusiast'),
        findsOneWidget,
      );

      // 验证头像（首字母）
      expect(find.text('A'), findsOneWidget);

      // 验证统计数据
      expect(find.text('42'), findsOneWidget); // followers
      expect(find.text('100'), findsOneWidget); // following
      expect(find.text('25'), findsOneWidget); // tweets

      // 验证统计标签
      expect(find.text('Followers'), findsOneWidget);
      expect(find.text('Following'), findsOneWidget);
      expect(find.text('Tweets'), findsOneWidget);
    });

    testWidgets('TC-TAB-011: Me Tab - 验证设置选项存在', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // 验证所有设置选项
      expect(find.text('Edit Profile'), findsOneWidget);
      expect(find.text('Change Password'), findsOneWidget);
      expect(find.text('Language'), findsOneWidget);
      expect(find.text('Sign Out'), findsOneWidget);

      // 验证 Developer 区域
      expect(find.text('Developer'), findsOneWidget);
      expect(find.text('Admin Dashboard'), findsOneWidget);
      expect(find.text('http://localhost:8080'), findsOneWidget);
    });

    testWidgets('TC-TAB-012: Me Tab - 导航到 Edit Profile', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // 点击 Edit Profile
      await tester.tap(find.text('Edit Profile'));
      await tester.pumpAndSettle();

      // 验证导航到了 Edit Profile 页面
      expect(find.byType(EditProfileView), findsOneWidget);
      expect(find.text('Edit Profile'), findsWidgets);

      // 验证有返回按钮
      expectBackButtonPresent(tester);

      // 返回
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // 验证回到了 Me
      expect(find.byType(MeView), findsOneWidget);
    });

    testWidgets('TC-TAB-013: Me Tab - 导航到 Change Password', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // 点击 Change Password
      await tester.tap(find.text('Change Password'));
      await tester.pumpAndSettle();

      // 验证导航到了 Change Password 页面
      expect(find.byType(ChangePasswordView), findsOneWidget);
      expect(find.text('Change Password'), findsWidgets);

      // 验证有返回按钮
      expectBackButtonPresent(tester);

      // 返回
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // 验证回到了 Me
      expect(find.byType(MeView), findsOneWidget);
    });

    testWidgets('TC-TAB-014: Me Tab - 导航到 Language Picker', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // 点击 Language
      await tester.tap(find.text('Language'));
      await tester.pumpAndSettle();

      // 验证导航到了 Language Picker 页面
      expect(find.byType(LanguagePickerView), findsOneWidget);
      expect(find.text('Language'), findsWidgets);

      // 验证语言选项
      expect(find.text('English'), findsOneWidget);
      expect(find.text('简体中文'), findsOneWidget);

      // 验证有返回按钮
      expectBackButtonPresent(tester);

      // 返回
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // 验证回到了 Me
      expect(find.byType(MeView), findsOneWidget);
    });

    // ============================================
    // 深层导航栈测试
    // ============================================

    testWidgets('TC-TAB-015: 深层导航栈 - Me → Edit Profile → 返回', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Me
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Edit Profile
      await tester.tap(find.text('Edit Profile'));
      await tester.pumpAndSettle();

      // 返回
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // 验证回到了 Me，且 Me 还在 TabBar 中
      expect(find.byType(MeView), findsOneWidget);
      expect(find.byType(CupertinoTabBar), findsOneWidget);
      expect(find.text('Sign Out'), findsOneWidget);
    });
  });
}
