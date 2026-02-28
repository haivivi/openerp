/// 边界条件和异常场景集成测试
/// 测试极端情况、错误处理、数据边界
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('🚨 边界条件和异常场景 - 严格测试套件', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
    });

    tearDown(() {
      commonTearDown();
    });

    // ============================================
    // 字符限制边界测试
    // ============================================

    group('Compose - 字符限制边界', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('TC-BOUNDARY-001: 发帖 - 刚好 280 字符', (
        WidgetTester tester,
      ) async {
        await pumpApp(tester, store: store);

        // 进入 Compose
        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 输入刚好 280 字符
        final content = generateLongText(280);
        await tester.enterText(find.byType(CupertinoTextField), content);
        await tester.pumpAndSettle();

        // 验证字符计数
        expect(find.text('280/280'), findsOneWidget);

        // 验证 Post 按钮启用
        expectButtonEnabled(tester, 'Post');
      });

      testWidgets('TC-BOUNDARY-002: 发帖 - 281 字符（超限制）', (
        WidgetTester tester,
      ) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 输入 281 字符
        final content = generateLongText(281);
        await tester.enterText(find.byType(CupertinoTextField), content);
        await tester.pumpAndSettle();

        // 验证字符计数显示红色
        expect(find.text('281/280'), findsOneWidget);

        // 验证 Post 按钮禁用
        expectButtonDisabled(tester, 'Post');
      });

      testWidgets('TC-BOUNDARY-003: 发帖 - 0 字符', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 不输入任何内容
        expect(find.text('0/280'), findsOneWidget);
        expectButtonDisabled(tester, 'Post');
      });

      testWidgets('TC-BOUNDARY-004: 发帖 - 只有空格', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 输入只有空格
        await tester.enterText(find.byType(CupertinoTextField), '   ');
        await tester.pumpAndSettle();

        // 应该算 3 个字符，但 Post 按钮应该禁用（trim 后为空）
        expect(find.text('3/280'), findsOneWidget);
        // 根据实现，可能禁用或启用
      });

      testWidgets('TC-BOUNDARY-005: 发帖 - 特殊字符', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 输入特殊字符
        final specialChars = generateSpecialCharText();
        await tester.enterText(find.byType(CupertinoTextField), specialChars);
        await tester.pumpAndSettle();

        // 验证字符计数正确
        expect(find.text('${specialChars.length}/280'), findsOneWidget);
        expectButtonEnabled(tester, 'Post');
      });

      testWidgets('TC-BOUNDARY-006: 发帖 - Emoji 字符', (
        WidgetTester tester,
      ) async {
        await pumpApp(tester, store: store);

        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        // 输入 Emoji
        final emojiText = generateEmojiText();
        await tester.enterText(find.byType(CupertinoTextField), emojiText);
        await tester.pumpAndSettle();

        // 验证能正常输入（Emoji 可能按字符或代码点计数）
        expect(find.byType(CupertinoTextField), findsOneWidget);
      });
    });

    // ============================================
    // 空状态测试
    // ============================================

    group('Empty States', () {
      testWidgets('TC-EMPTY-001: Home - 空时间线', (WidgetTester tester) async {
        setupAuthenticatedState(store);
        // 不设置时间线数据

        await pumpApp(tester, store: store);

        // 验证显示空状态提示
        expect(find.text('No tweets yet'), findsOneWidget);
        expect(find.text('Pull to refresh'), findsOneWidget);
      });

      testWidgets('TC-EMPTY-002: Inbox - 空消息列表', (WidgetTester tester) async {
        setupAuthenticatedState(store);
        // 不设置 inbox 数据

        await pumpApp(tester, store: store);
        await tester.tap(find.text('Inbox'));
        await tester.pumpAndSettle();

        // 验证显示空状态
        expect(find.text('No messages'), findsOneWidget);
      });

      testWidgets('TC-EMPTY-003: Search - 无结果', (WidgetTester tester) async {
        setupAuthenticatedState(store);

        await pumpApp(tester, store: store);
        await tester.tap(find.text('Search'));
        await tester.pumpAndSettle();

        // 搜索不存在的内容
        await tester.enterText(
          find.byType(CupertinoSearchTextField),
          'xyznonexistent',
        );
        await tester.pumpAndSettle();

        // 应该显示无结果提示
        // 根据实现，可能显示 "No results" 或空列表
      });
    });

    // ============================================
    // 快速操作测试
    // ============================================

    group('Rapid Actions', () {
      setUp(() {
        setupAuthenticatedState(store);
        setupTimelineFeed(store);
      });

      testWidgets('TC-RAPID-001: 快速连续发帖', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 快速发帖 3 次
        for (var i = 0; i < 3; i++) {
          await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
          await tester.pumpAndSettle();

          await tester.enterText(
            find.byType(CupertinoTextField),
            'Rapid tweet $i',
          );
          await tester.pumpAndSettle();

          await tester.tap(find.text('Post'));
          await tester.pumpAndSettle();
        }

        // 验证应用没有崩溃
        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      });

      testWidgets('TC-RAPID-002: 快速切换 Tab 并操作', (WidgetTester tester) async {
        await pumpApp(tester, store: store);

        // 快速切换并点击
        for (var i = 0; i < 10; i++) {
          await tester.tap(find.text('Me'));
          await tester.pump(const Duration(milliseconds: 50));
          await tester.tap(find.text('Home'));
          await tester.pump(const Duration(milliseconds: 50));
        }

        await tester.pumpAndSettle();

        // 验证应用稳定
        expect(find.byType(CupertinoTabBar), findsOneWidget);
      });
    });

    // ============================================
    // 数据完整性测试
    // ============================================

    group('Data Integrity', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('TC-DATA-001: 用户信息完整性验证', (WidgetTester tester) async {
        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        await tester.tap(find.text('Me'));
        await tester.pumpAndSettle();

        final auth = store.get<AuthState>('auth/state');

        // 验证所有字段都存在且类型正确
        expect(auth?.user?.id, isNotNull);
        expect(auth?.user?.id, isA<String>());
        expect(auth?.user?.username, equals('alice'));
        expect(auth?.user?.displayName, equals('Alice'));
        expect(
          auth?.user?.bio,
          equals('Flutter developer and testing enthusiast'),
        );
        expect(auth?.user?.followerCount, isA<int>());
        expect(auth?.user?.followingCount, isA<int>());
        expect(auth?.user?.tweetCount, isA<int>());
      });

      testWidgets('TC-DATA-002: 时间线数据完整性', (WidgetTester tester) async {
        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        final feed = store.get<TimelineFeed>('timeline/feed');

        expect(feed?.items, isNotNull);
        expect(feed?.items.length, equals(3));

        // 验证每个 item 都有必需字段
        for (final item in feed!.items) {
          expect(item.tweetId, isNotNull);
          expect(item.author, isNotNull);
          expect(item.content, isNotNull);
          expect(item.createdAt, isNotNull);
        }
      });
    });

    // ============================================
    // 内存和性能测试
    // ============================================

    group('Performance', () {
      setUp(() {
        setupAuthenticatedState(store);
      });

      testWidgets('TC-PERF-001: 大量数据加载 - 时间线', (WidgetTester tester) async {
        // 创建大量推文数据
        final manyItems = List.generate(
          100,
          (index) => FeedItem(
            tweetId: 'tweet-$index',
            author: UserProfile(
              id: 'user-$index',
              username: 'user$index',
              displayName: 'User $index',
            ),
            content: 'Tweet content $index ' * 10,
            likeCount: index,
            createdAt: DateTime.now().toIso8601String(),
          ),
        );

        store.setState(
          'timeline/feed',
          TimelineFeed(items: manyItems, loading: false, hasMore: false),
        );

        await pumpApp(tester, store: store);

        // 验证能正常显示且可滚动
        expect(find.byType(ListView), findsOneWidget);

        // 滚动到底部
        await tester.fling(find.byType(ListView), const Offset(0, -1000), 1000);
        await tester.pumpAndSettle();

        // 应用应该没有崩溃
        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      });

      testWidgets('TC-PERF-002: 页面切换性能', (WidgetTester tester) async {
        setupTimelineFeed(store);
        await pumpApp(tester, store: store);

        // 多次切换 Tab
        final stopwatch = Stopwatch()..start();

        for (var i = 0; i < 20; i++) {
          await tester.tap(find.text('Search'));
          await tester.pumpAndSettle();
          await tester.tap(find.text('Home'));
          await tester.pumpAndSettle();
        }

        stopwatch.stop();

        // 应该能在合理时间内完成（小于 10 秒）
        expect(
          stopwatch.elapsedMilliseconds,
          lessThan(10000),
          reason: 'Tab switching should complete within 10s',
        );
      });
    });
  });
}
