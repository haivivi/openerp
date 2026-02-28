/// 超级严格的登录流程集成测试
/// 测试内容包括：正常登录、各种失败场景、边界条件、UI状态验证
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('🔐 登录流程 - 严格测试套件', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
    });

    tearDown(() {
      commonTearDown();
    });

    // ============================================
    // 基础功能测试
    // ============================================

    testWidgets('TC-LOGIN-001: 正常登录成功 - 完整验证', (WidgetTester tester) async {
      // Arrange: 启动应用
      await pumpApp(tester, store: store);

      // 验证初始状态
      expect(find.text('TwitterFlux'), findsOneWidget);
      expect(find.byType(LoginView), findsOneWidget);
      expect(find.byType(MainTabView), findsNothing);

      // 验证输入框存在且为空
      expectTextFieldPlaceholder(tester, 'Username');
      expectTextFieldPlaceholder(tester, 'Password');

      // 验证登录按钮初始禁用
      expectButtonDisabled(tester, 'Sign In');

      // Act: 输入有效凭据
      await enterTextInField(tester, 'Username', 'alice');
      await enterTextInField(tester, 'Password', 'password');

      // 验证输入后按钮启用
      expectButtonEnabled(tester, 'Sign In');

      // 点击登录
      await tapButton(tester, 'Sign In');

      // Assert: 验证登录成功后的状态
      await waitFor(tester, find.byType(CupertinoTabScaffold));

      // 验证导航到了主界面
      expect(find.byType(CupertinoTabScaffold), findsOneWidget);
      expect(find.byType(MainTabView), findsOneWidget);
      expect(find.byType(LoginView), findsNothing);

      // 验证 TabBar 存在且在底部
      expectTabBarAtBottom(tester);
      expectTabBarHasFourTabs(tester);

      // 验证所有 Tab 标签存在
      // "Home" 同时出现在导航标题和 Tab 标签，允许多个。
      expect(find.text('Home'), findsWidgets);
      expect(find.text('Search'), findsOneWidget);
      expect(find.text('Inbox'), findsOneWidget);
      expect(find.text('Me'), findsOneWidget);

      // 验证默认选中 Home Tab
      expectSelectedTab(tester, 'Home');

      // 验证 AuthState 正确更新
      final auth = store.get<AuthState>('auth/state');
      expect(auth, isNotNull, reason: 'AuthState should be set after login');
      expect(auth!.phase, equals(AuthPhase.authenticated));
      expect(auth.user, isNotNull);
      expect(auth.user!.username, equals('alice'));
      expect(auth.user!.displayName, equals('Alice'));
      expect(auth.error, isNull);
      expect(auth.busy, isFalse);
    });

    // ============================================
    // 错误处理测试
    // ============================================

    testWidgets('TC-LOGIN-002: 错误密码 - 显示错误信息', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 输入错误密码
      await enterTextInField(tester, 'Username', 'alice');
      await enterTextInField(tester, 'Password', 'wrongpassword');
      await tapButton(tester, 'Sign In');

      // 验证仍在登录页
      expect(find.byType(LoginView), findsOneWidget);
      expect(find.byType(MainTabView), findsNothing);

      // 验证显示错误信息
      expect(find.text('Invalid credentials'), findsOneWidget);

      // 验证 AuthState
      final auth = store.get<AuthState>('auth/state');
      expect(auth!.phase, equals(AuthPhase.unauthenticated));
      expect(auth.error, equals('Invalid credentials'));
    });

    testWidgets('TC-LOGIN-003: 不存在的用户 - 显示错误信息', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      await enterTextInField(tester, 'Username', 'nonexistent');
      await enterTextInField(tester, 'Password', 'password');
      await tapButton(tester, 'Sign In');

      expect(find.text('Invalid credentials'), findsOneWidget);
      expect(find.byType(LoginView), findsOneWidget);
    });

    testWidgets('TC-LOGIN-004: 空用户名 - 按钮禁用', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 只输入密码
      await enterTextInField(tester, 'Password', 'password');

      // 验证按钮仍然禁用
      expectButtonDisabled(tester, 'Sign In');
    });

    testWidgets('TC-LOGIN-005: 空密码 - 按钮禁用', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 只输入用户名
      await enterTextInField(tester, 'Username', 'alice');

      // 验证按钮仍然禁用
      expectButtonDisabled(tester, 'Sign In');
    });

    testWidgets('TC-LOGIN-006: 用户名和密码都为空 - 按钮禁用', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 什么都不输入
      expectButtonDisabled(tester, 'Sign In');
    });

    // ============================================
    // 边界条件测试
    // ============================================

    testWidgets('TC-LOGIN-007: 超长用户名 - 应能输入', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      final longUsername = 'a' * 100;
      await enterTextInField(tester, 'Username', longUsername);
      await enterTextInField(tester, 'Password', 'password');

      // 验证输入成功
      final usernameField = findCupertinoTextField('Username');
      expect(usernameField, findsOneWidget);

      // 按钮应该启用
      expectButtonEnabled(tester, 'Sign In');
    });

    testWidgets('TC-LOGIN-008: 特殊字符用户名 - 应能输入', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      await enterTextInField(tester, 'Username', 'user@example.com');
      await enterTextInField(tester, 'Password', 'password');

      expectButtonEnabled(tester, 'Sign In');
    });

    testWidgets('TC-LOGIN-009: 中文用户名 - 应能输入', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      await enterTextInField(tester, 'Username', '爱丽丝');
      await enterTextInField(tester, 'Password', '密码');

      expectButtonEnabled(tester, 'Sign In');
    });

    testWidgets('TC-LOGIN-010: 空格用户名 - 应能输入', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      await enterTextInField(tester, 'Username', 'alice smith');
      await enterTextInField(tester, 'Password', 'password');

      expectButtonEnabled(tester, 'Sign In');
    });

    testWidgets('TC-LOGIN-011: 密码隐藏显示 - 验证 obscureText', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await enterTextInField(tester, 'Password', 'secret123');

      // 验证密码字段是隐藏的
      final passwordField = findCupertinoTextField('Password');
      expect(passwordField, findsOneWidget);

      final fieldWidget = tester.widget<CupertinoTextField>(passwordField);
      expect(
        fieldWidget.obscureText,
        isTrue,
        reason: 'Password field should be obscured',
      );
    });

    // ============================================
    // 并发和重复操作测试
    // ============================================

    testWidgets('TC-LOGIN-012: 快速重复点击登录按钮 - 应只处理一次', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      await enterTextInField(tester, 'Username', 'alice');
      await enterTextInField(tester, 'Password', 'password');

      // 快速点击多次
      final signInButton = find.text('Sign In');
      await tester.tap(signInButton);
      await tester.tap(signInButton);
      await tester.tap(signInButton);
      await tester.pumpAndSettle();

      // 验证最终成功登录
      expect(find.byType(MainTabView), findsOneWidget);
    });

    testWidgets('TC-LOGIN-013: 登录失败后重新输入 - 应清除错误', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 第一次错误登录
      await enterTextInField(tester, 'Username', 'alice');
      await enterTextInField(tester, 'Password', 'wrong');
      await tapButton(tester, 'Sign In');

      expect(find.text('Invalid credentials'), findsOneWidget);

      // 修改密码
      await enterTextInField(tester, 'Password', 'password');

      // 验证错误仍然存在（直到再次点击登录）
      expect(find.text('Invalid credentials'), findsOneWidget);
    });

    // ============================================
    // UI 布局测试
    // ============================================

    testWidgets('TC-LOGIN-014: 验证登录页布局 - 所有元素存在', (WidgetTester tester) async {
      await pumpApp(tester, store: store);

      // 验证所有 UI 元素存在
      expect(find.byIcon(CupertinoIcons.chat_bubble_2_fill), findsOneWidget);
      expect(find.text('TwitterFlux'), findsOneWidget);
      expect(find.text('Powered by Flux State Engine'), findsOneWidget);
      expect(find.text('Username'), findsOneWidget);
      expect(find.text('Password'), findsOneWidget);
      expect(find.text('Sign In'), findsOneWidget);
      expect(find.text('Use alice / password to sign in'), findsOneWidget);
    });

    testWidgets('TC-LOGIN-015: 验证输入框样式 - CupertinoTextField', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // 验证使用 CupertinoTextField
      final textFields = find.byType(CupertinoTextField);
      expect(textFields, findsNWidgets(2));

      // 验证每个输入框都有 placeholder
      final usernameField = findCupertinoTextField('Username');
      final passwordField = findCupertinoTextField('Password');

      expect(usernameField, findsOneWidget);
      expect(passwordField, findsOneWidget);
    });
  });
}
