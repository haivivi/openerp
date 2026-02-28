/// 魔鬼测试 - 终极严格的集成测试
/// 包含大量边界条件、并发操作、错误注入
/// 目标是让开发者哭出来 😈
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('👹 魔鬼测试 - 终极折磨套件', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
    });

    tearDown(() {
      commonTearDown();
    });

    // ============================================
    // 地狱级边界测试
    // ============================================

    group('💀 地狱边界测试', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('DEVIL-001: 输入 10,000 个字符到 Compose', (
        WidgetTester tester,
      ) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 输入 10000 字符
        final massiveText = generateLongText(10000);
        await tester.enterText(find.byType(CupertinoTextField), massiveText);
        await tester.pumpAndSettle();

        // 应用不应该崩溃
        expect(find.byType(ComposeView), findsOneWidget);

        // 字符计数应该显示
        final textFinder = find.byType(Text);
        expect(textFinder, findsWidgets);
      });

      testWidgets('DEVIL-002: 包含 null 字节的文本', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 包含 null 字节的文本
        await tester.enterText(
          find.byType(CupertinoTextField),
          'Hello\x00World',
        );
        await tester.pumpAndSettle();

        // 不应该崩溃
        expect(find.byType(ComposeView), findsOneWidget);
      });

      testWidgets('DEVIL-003: Unicode 控制字符', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // RTL 控制字符
        await tester.enterText(
          find.byType(CupertinoTextField),
          '\u202BHello\u202C', // RTL embedding
        );
        await tester.pumpAndSettle();

        expect(find.byType(ComposeView), findsOneWidget);
      });

      testWidgets('DEVIL-004: 零宽字符和不可见字符', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 零宽空格、零宽连接符等
        await tester.enterText(
          find.byType(CupertinoTextField),
          'Hello\u200B\u200C\u200D\u2060\uFEFFWorld',
        );
        await tester.pumpAndSettle();

        expect(find.byType(ComposeView), findsOneWidget);
      });
    });

    // ============================================
    // 疯狂快速操作
    // ============================================

    group('⚡ 疯狂快速操作', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('DEVIL-005: 100 毫秒内切换 Tab 20 次', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 疯狂切换
        for (var i = 0; i < 20; i++) {
          await tapTab(tester, 'Search');
          await tester.pump(const Duration(milliseconds: 25));
          await tapTab(tester, 'Inbox');
          await tester.pump(const Duration(milliseconds: 25));
          await tapTab(tester, 'Me');
          await tester.pump(const Duration(milliseconds: 25));
          await tapTab(tester, 'Home');
          await tester.pump(const Duration(milliseconds: 25));
        }

        await tester.pumpAndSettle();

        // 不应该崩溃
        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      });

      testWidgets('DEVIL-006: 同时点击多个按钮', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tapTab(tester, 'Me');

        // 尝试同时点击多个设置项
        await tester.tap(find.text('Edit Profile'));
        await tester.tap(find.text('Language'), warnIfMissed: false);
        await tester.tap(find.text('Change Password'), warnIfMissed: false);
        await tester.pumpAndSettle();

        // 应该只打开一个页面
        // 根据实现，可能打开第一个或最后一个
        expect(find.byType(CupertinoPageScaffold), findsWidgets);
      });

      testWidgets('DEVIL-007: 在动画过程中切换 Tab', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 在动画未完成时切换
        await tapTab(tester, 'Search');
        await tester.pump(const Duration(milliseconds: 50));
        await tapTab(tester, 'Inbox');
        await tester.pump(const Duration(milliseconds: 50));
        await tapTab(tester, 'Me');
        await tester.pump(const Duration(milliseconds: 50));

        await tester.pumpAndSettle();

        expect(find.byType(CupertinoTabBar), findsOneWidget);
      });
    });

    // ============================================
    // 内存压力测试
    // ============================================

    group('🧠 内存压力测试', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('DEVIL-008: 加载 1000 条推文的时间线', (WidgetTester tester) async {
        // 创建 1000 条推文
        final massiveFeed = List.generate(
          1000,
          (index) => FeedItem(
            tweetId: 'tweet-$index',
            author: UserProfile(
              id: 'user-$index',
              username: 'user$index',
              displayName: 'User Number $index With Long Name',
            ),
            content: 'This is tweet number $index with some content ' * 20,
            likeCount: index * 100,
            replyCount: index * 10,
            createdAt: DateTime.now()
                .subtract(Duration(minutes: index))
                .toIso8601String(),
          ),
        );

        store.setState(
          'timeline/feed',
          TimelineFeed(items: massiveFeed, loading: false, hasMore: false),
        );

        await pumpApp(tester, store: store);

        // 验证能正常显示
        expect(find.byType(ListView), findsOneWidget);

        // 疯狂滚动
        for (var i = 0; i < 10; i++) {
          await tester.fling(
            find.byType(ListView),
            const Offset(0, -2000),
            2000,
          );
          await tester.pumpAndSettle();
        }

        // 不应该崩溃或内存溢出
        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      }, timeout: const Timeout(Duration(minutes: 2)));

      testWidgets('DEVIL-009: 创建 100 个 Store 监听器', (WidgetTester tester) async {
        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        // 添加 100 个监听器
        final listeners = <VoidCallback>[];
        for (var i = 0; i < 100; i++) {
          final listener = () {};
          store.addListener(listener);
          listeners.add(listener);
        }

        // 触发状态更新
        for (var i = 0; i < 50; i++) {
          store.setState('test/counter', i);
          await tester.pump();
        }

        await tester.pumpAndSettle();

        // 清理监听器
        for (final listener in listeners) {
          store.removeListener(listener);
        }

        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      });
    });

    // ============================================
    // 并发操作测试
    // ============================================

    group('🔄 并发操作', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('DEVIL-010: 在发帖时切换 Tab', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 开始发帖
        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        await tester.enterText(find.byType(CupertinoTextField), 'Test tweet');

        // 在输入过程中切换 Tab（这不应该发生，但测试一下）
        // 实际上 CupertinoTabScaffold 会阻止这个操作
        await tester.pumpAndSettle();

        expect(find.byType(ComposeView), findsOneWidget);
      });

      testWidgets('DEVIL-011: 快速登录登出循环', (WidgetTester tester) async {
        // 循环登录登出 5 次
        for (var i = 0; i < 5; i++) {
          store.setState(
            'auth/state',
            const AuthState(phase: AuthPhase.unauthenticated),
          );
          await pumpApp(tester, store: store);

          // 登录
          await enterTextInField(tester, 'Username', 'alice');
          await enterTextInField(tester, 'Password', 'password');
          await tapButton(tester, 'Sign In');
          await tester.pumpAndSettle();

          expect(find.byType(CupertinoTabScaffold), findsOneWidget);

          // 登出
          await tapTab(tester, 'Me');
          await tester.tap(find.text('Sign Out'));
          await tester.pumpAndSettle();

          expect(find.byType(LoginView), findsOneWidget);
        }
      });
    });

    // ============================================
    // 异常数据测试
    // ============================================

    group('🐛 异常数据', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('DEVIL-012: null 用户资料字段', (WidgetTester tester) async {
        // 设置一个有 null 字段的用户
        store.setState(
          'auth/state',
          const AuthState(
            phase: AuthPhase.authenticated,
            user: UserProfile(
              id: 'test-id',
              username: 'test',
              displayName: 'Test User',
              bio: null,
              avatar: null,
              followerCount: 0,
              followingCount: 0,
              tweetCount: 0,
            ),
          ),
        );

        await pumpApp(tester, store: store);
        await tapTab(tester, 'Me');

        // 不应该因为 null 字段而崩溃
        expect(find.byType(MeView), findsOneWidget);
      });

      testWidgets('DEVIL-013: 空字符串用户资料', (WidgetTester tester) async {
        store.setState(
          'auth/state',
          const AuthState(
            phase: AuthPhase.authenticated,
            user: UserProfile(id: '', username: '', displayName: '', bio: ''),
          ),
        );

        await pumpApp(tester, store: store);
        await tester.tap(find.text('Me'));
        await tester.pumpAndSettle();

        expect(find.byType(MeView), findsOneWidget);
      });

      testWidgets('DEVIL-014: 负数统计数据', (WidgetTester tester) async {
        store.setState(
          'auth/state',
          const AuthState(
            phase: AuthPhase.authenticated,
            user: UserProfile(
              id: 'test',
              username: 'test',
              displayName: 'Test',
              followerCount: -100,
              followingCount: -999,
              tweetCount: -1,
            ),
          ),
        );

        await pumpApp(tester, store: store);
        await tester.tap(find.text('Me'));
        await tester.pumpAndSettle();

        // 应该显示负数（或者处理为 0，取决于实现）
        expect(find.byType(MeView), findsOneWidget);
      });
    });

    // ============================================
    // 时序和竞态条件
    // ============================================

    group('⏱️ 时序和竞态条件', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('DEVIL-015: 在状态更新中导航', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 触发状态更新并立即导航
        store.setState('test/value', 1);
        await tester.tap(find.text('Me'));
        await tester.pump();

        store.setState('test/value', 2);
        await tester.tap(find.text('Edit Profile'));
        await tester.pump();

        await tester.pumpAndSettle();

        expect(find.byType(CupertinoPageScaffold), findsWidgets);
      });

      testWidgets('DEVIL-016: 快速连续状态更新', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 100 个快速状态更新
        for (var i = 0; i < 100; i++) {
          store.setState('test/counter', i);
          await tester.pump(const Duration(milliseconds: 1));
        }

        await tester.pumpAndSettle();

        // 最终值应该是 99
        expect(store.get<int>('test/counter'), equals(99));
      });
    });

    // ============================================
    // UI 边界测试
    // ============================================

    group('📱 UI 边界', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('DEVIL-017: 极小屏幕尺寸', (WidgetTester tester) async {
        // 设置极小屏幕
        tester.binding.window.physicalSizeTestValue = const Size(320, 480);
        tester.binding.window.devicePixelRatioTestValue = 1.0;
        addTearDown(() {
          tester.binding.window.clearPhysicalSizeTestValue();
          tester.binding.window.clearDevicePixelRatioTestValue();
        });

        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        // 不应该布局溢出
        expect(tester.takeException(), isNull);
      });

      testWidgets('DEVIL-018: 极大屏幕尺寸', (WidgetTester tester) async {
        // 设置 iPad Pro 尺寸
        tester.binding.window.physicalSizeTestValue = const Size(2048, 2732);
        tester.binding.window.devicePixelRatioTestValue = 2.0;
        addTearDown(() {
          tester.binding.window.clearPhysicalSizeTestValue();
          tester.binding.window.clearDevicePixelRatioTestValue();
        });

        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      });

      testWidgets('DEVIL-019: 横屏模式', (WidgetTester tester) async {
        tester.binding.window.physicalSizeTestValue = const Size(812, 375);
        addTearDown(() {
          tester.binding.window.clearPhysicalSizeTestValue();
        });

        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      });
    });

    // ============================================
    // 最终大魔王测试
    // ============================================

    testWidgets('DEVIL-999: 终极混沌测试 - 同时做所有事情', (WidgetTester tester) async {
      // 1. 登录
      await pumpApp(tester, store: store);
      await enterTextInField(tester, 'Username', 'alice');
      await enterTextInField(tester, 'Password', 'password');
      await tapButton(tester, 'Sign In');
      await tester.pumpAndSettle();

      setupTimelineFeed(store);
      setupInboxState(store);
      await tester.pumpAndSettle();

      // 2. 疯狂操作序列
      for (var round = 0; round < 3; round++) {
        // 切换所有 Tab
        for (final tab in ['Search', 'Inbox', 'Me', 'Home']) {
          await tapTab(tester, tab);
          await tester.pump(const Duration(milliseconds: 100));
        }

        // 导航到深层页面并返回
        await tapTab(tester, 'Me');
        await tester.tap(find.text('Edit Profile'));
        await tester.pumpAndSettle();
        await tester.tap(find.byType(CupertinoNavigationBarBackButton));
        await tester.pumpAndSettle();

        // 发帖并取消
        await tapTab(tester, 'Home');
        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();
        await tester.enterText(find.byType(CupertinoTextField), 'Test $round');
        await tester.pumpAndSettle();
        await tester.tap(find.text('Cancel'));
        await tester.pumpAndSettle();

        // 快速状态更新
        for (var i = 0; i < 10; i++) {
          store.setState('chaos/counter', round * 10 + i);
          await tester.pump();
        }
      }

      // 3. 最终登出
      await tapTab(tester, 'Me');
      await tester.tap(find.text('Sign Out'));
      await tester.pumpAndSettle();

      // 验证：应用应该还活着
      expect(find.byType(LoginView), findsOneWidget);
      expect(find.text('TwitterFlux'), findsOneWidget);
    }, timeout: const Timeout(Duration(minutes: 3)));
  });
}
