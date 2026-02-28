/// 状态管理和持久化测试
/// 测试应用状态、导航栈、数据一致性
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('🔄 状态管理和持久化 - 严格测试套件', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
    });

    tearDown(() {
      commonTearDown();
    });

    // ============================================
    // 认证状态测试
    // ============================================

    group('Auth State', () {
      testWidgets('TC-STATE-001: 登录状态变化验证', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 初始状态：未登录
        var auth = store.get<AuthState>('auth/state');
        expect(auth, isNull);

        // 登录
        await enterTextInField(tester, 'Username', 'alice');
        await enterTextInField(tester, 'Password', 'password');
        await tapButton(tester, 'Sign In');
        await tester.pumpAndSettle();

        // 验证状态变化
        auth = store.get<AuthState>('auth/state');
        expect(auth?.phase, equals(AuthPhase.authenticated));
        expect(auth?.user?.username, equals('alice'));

        // 退出登录
        await tapTab(tester, 'Me');
        await tester.tap(find.text('Sign Out'));
        await tester.pumpAndSettle();

        // 验证状态回到未登录
        auth = store.get<AuthState>('auth/state');
        expect(auth?.phase, equals(AuthPhase.unauthenticated));
      });

      testWidgets('TC-STATE-002: 错误状态清除验证', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 输入错误密码
        await enterTextInField(tester, 'Username', 'alice');
        await enterTextInField(tester, 'Password', 'wrong');
        await tapButton(tester, 'Sign In');

        // 验证错误状态
        var auth = store.get<AuthState>('auth/state');
        expect(auth?.error, equals('Invalid credentials'));

        // 重新输入正确密码并登录
        await enterTextInField(tester, 'Password', 'password');
        await tapButton(tester, 'Sign In');
        await tester.pumpAndSettle();

        // 验证错误已清除
        auth = store.get<AuthState>('auth/state');
        expect(auth?.error, isNull);
        expect(auth?.phase, equals(AuthPhase.authenticated));
      });
    });

    // ============================================
    // 多层级导航状态
    // ============================================

    group('Navigation Stack', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('TC-NAV-001: 深层导航栈测试', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // Home -> Me -> Edit Profile -> 返回 -> 返回
        await tapTab(tester, 'Me');

        await tester.tap(find.text('Edit Profile'));
        await tester.pumpAndSettle();

        // 应该有两层导航（Me 在 Tab 中，Edit Profile 在导航栈上）
        expect(find.byType(CupertinoPageScaffold), findsWidgets);

        // 返回 Edit Profile
        await tester.tap(find.byType(CupertinoNavigationBarBackButton));
        await tester.pumpAndSettle();

        // 应该回到 Me
        expect(find.text('Me'), findsWidgets);
        expect(find.byType(EditProfileView), findsNothing);
      });

      testWidgets('TC-NAV-002: 多页面打开后返回', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 连续打开多个页面
        await tapTab(tester, 'Me');

        await tester.tap(find.text('Language'));
        await tester.pumpAndSettle();

        // 从 Language 返回
        await tester.tap(find.byType(CupertinoNavigationBarBackButton));
        await tester.pumpAndSettle();

        // 验证回到 Me
        expect(find.text('Me'), findsWidgets);
        expect(find.byType(CupertinoTabBar), findsOneWidget);
      });
    });

    // ============================================
    // 数据一致性测试
    // ============================================

    group('Data Consistency', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('TC-CONSISTENCY-001: 用户信息在各页面一致性', (
        WidgetTester tester,
      ) async {
        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        // 检查 Me 页面的用户信息
        await tapTab(tester, 'Me');

        expect(find.text('Alice'), findsOneWidget);
        expect(find.text('@alice'), findsOneWidget);

        // 修改用户信息（模拟）
        final currentAuth = store.get<AuthState>('auth/state');
        store.setState(
          'auth/state',
          AuthState(
            phase: AuthPhase.authenticated,
            user: UserProfile(
              id: currentAuth!.user!.id,
              username: currentAuth.user!.username,
              displayName: 'Alice Updated', // 修改名字
              bio: currentAuth.user!.bio,
              followerCount: currentAuth.user!.followerCount,
              followingCount: currentAuth.user!.followingCount,
              tweetCount: currentAuth.user!.tweetCount,
            ),
          ),
        );
        await tester.pumpAndSettle();

        // 验证 UI 更新了
        expect(find.text('Alice Updated'), findsOneWidget);
        expect(find.text('Alice'), findsNothing);
      });

      testWidgets('TC-CONSISTENCY-002: 时间线数据更新', (WidgetTester tester) async {
        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        // 初始状态
        var feed = store.get<TimelineFeed>('timeline/feed');
        expect(feed?.items.length, equals(3));

        // 添加新推文
        final currentItems = feed!.items;
        currentItems.insert(
          0,
          FeedItem(
            tweetId: 'new-tweet',
            author: UserProfile(
              id: 'alice-id',
              username: 'alice',
              displayName: 'Alice',
            ),
            content: 'New tweet added!',
            likeCount: 0,
            createdAt: DateTime.now().toIso8601String(),
          ),
        );

        store.setState(
          'timeline/feed',
          TimelineFeed(items: currentItems, loading: false, hasMore: false),
        );
        await tester.pumpAndSettle();

        // 验证新推文显示
        expect(find.text('New tweet added!'), findsOneWidget);
      });
    });

    // ============================================
    // Store 状态管理测试
    // ============================================

    group('FluxStore State Management', () {
      testWidgets('TC-STORE-001: 状态监听和通知', (WidgetTester tester) async {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);

        await pumpApp(tester, store: store);

        var notificationCount = 0;
        store.addListener(() {
          notificationCount++;
        });

        // 修改状态
        setupInboxState(store);
        await tester.pumpAndSettle();

        // 验证通知被触发
        expect(notificationCount, greaterThan(0));
      });

      testWidgets('TC-STORE-002: 多状态路径独立性', (WidgetTester tester) async {
        setupAuthenticatedState(store);

        await pumpApp(tester, store: store);

        // 设置多个独立状态
        store.setState('test/path1', 'value1');
        store.setState('test/path2', 'value2');
        store.setState('test/nested/path3', 'value3');

        // 验证每个状态独立
        expect(store.get<String>('test/path1'), equals('value1'));
        expect(store.get<String>('test/path2'), equals('value2'));
        expect(store.get<String>('test/nested/path3'), equals('value3'));
      });

      testWidgets('TC-STORE-003: 状态类型安全', (WidgetTester tester) async {
        setupAuthenticatedState(store);
        await pumpApp(tester, store: store);

        // 存储不同类型的数据
        store.setState('test/int', 42);
        store.setState('test/double', 3.14);
        store.setState('test/bool', true);
        store.setState('test/list', [1, 2, 3]);
        store.setState('test/map', {'key': 'value'});

        // 验证类型正确
        expect(store.get<int>('test/int'), equals(42));
        expect(store.get<double>('test/double'), equals(3.14));
        expect(store.get<bool>('test/bool'), isTrue);
        expect(store.get<List>('test/list'), equals([1, 2, 3]));
        expect(store.get<Map>('test/map'), equals({'key': 'value'}));
      });
    });

    // ============================================
    // i18n 状态测试
    // ============================================

    group('Internationalization', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('TC-I18N-001: 语言切换', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 初始语言
        expect(store.locale, equals('en'));

        // 切换到中文
        store.setLocale('zh-CN');
        await tester.pumpAndSettle();

        // 验证语言已切换
        expect(store.locale, equals('zh-CN'));

        // 验证翻译生效
        final homeTitle = store.t('ui/tab/home');
        expect(homeTitle, equals('首页'));
      });

      testWidgets('TC-I18N-002: 参数化翻译', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 测试参数化翻译
        final charCount = store.t('format/char_count?current=100&max=280');
        expect(charCount, equals('100/280'));
      });

      testWidgets('TC-I18N-003: 未知语言回退', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 切换到不存在的语言
        store.setLocale('xx-XX');
        await tester.pumpAndSettle();

        // 应该回退到英语
        final homeTitle = store.t('ui/tab/home');
        expect(homeTitle, equals('Home'));
      });
    });
  });
}
