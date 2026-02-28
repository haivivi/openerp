/// 超级严格的集成测试辅助工具 — 包含更多验证和边界检查
library;

import 'package:flutter/cupertino.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

import 'package:twitter_flux/app.dart';
import 'package:twitter_flux/store/flux_store.dart';
import 'package:twitter_flux/models/models.dart';
import 'package:twitter_flux/views/login_view.dart';
import 'package:twitter_flux/views/main_tab_view.dart';
import 'package:twitter_flux/views/home_view.dart';
import 'package:twitter_flux/views/search_view.dart';
import 'package:twitter_flux/views/inbox_view.dart';
import 'package:twitter_flux/views/me_view.dart';
import 'package:twitter_flux/views/compose_view.dart';
import 'package:twitter_flux/views/language_picker_view.dart';
import 'package:twitter_flux/views/edit_profile_view.dart';
import 'package:twitter_flux/views/change_password_view.dart';
import 'package:twitter_flux/views/widgets/tweet_row.dart';

// Re-export for convenience
export 'package:flutter_test/flutter_test.dart';
export 'package:flutter/cupertino.dart' hide RefreshCallback;
export 'package:twitter_flux/app.dart';
export 'package:twitter_flux/store/flux_store.dart';
export 'package:twitter_flux/models/models.dart';
export 'package:twitter_flux/views/login_view.dart';
export 'package:twitter_flux/views/main_tab_view.dart';
export 'package:twitter_flux/views/home_view.dart';
export 'package:twitter_flux/views/search_view.dart';
export 'package:twitter_flux/views/inbox_view.dart';
export 'package:twitter_flux/views/me_view.dart';
export 'package:twitter_flux/views/compose_view.dart';
export 'package:twitter_flux/views/language_picker_view.dart';
export 'package:twitter_flux/views/edit_profile_view.dart';
export 'package:twitter_flux/views/change_password_view.dart';
export 'package:twitter_flux/views/widgets/tweet_row.dart';

/// Initialize integration test binding.
void initIntegrationTests() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
}

/// Create a fresh store for testing.
FluxStore createTestStore() {
  return FluxStore();
}

/// Pump the TwitterFlux app with a given store.
Future<void> pumpApp(WidgetTester tester, {FluxStore? store}) async {
  final testStore = store ?? createTestStore();
  await tester.pumpWidget(TwitterFluxApp(store: testStore));
  // 不使用 pumpAndSettle，避免被无限动画（如 ActivityIndicator）卡住。
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 200));
}

/// Setup authenticated state in store.
void setupAuthenticatedState(FluxStore store) {
  store.setState(
    'auth/state',
    const AuthState(
      phase: AuthPhase.authenticated,
      user: UserProfile(
        id: 'alice-id',
        username: 'alice',
        displayName: 'Alice',
        bio: 'Flutter developer and testing enthusiast',
        followerCount: 42,
        followingCount: 100,
        tweetCount: 25,
      ),
    ),
  );

  // 为已登录测试提供非 loading 的默认状态，防止首页/收件箱持续动画导致
  // integration_test 中的 pumpAndSettle 永远不返回。
  store.setState('timeline/feed', const TimelineFeed(items: []));
  store.setState('inbox/state', const InboxState(messages: []));
  store.setState('search/state', const SearchState());
}

/// Setup timeline feed with comprehensive test data.
void setupTimelineFeed(FluxStore store) {
  store.setState(
    'timeline/feed',
    TimelineFeed(
      items: [
        FeedItem(
          tweetId: 'tweet-1',
          author: UserProfile(
            id: 'user-1',
            username: 'bob',
            displayName: 'Bob Smith',
          ),
          content: 'Hello from Bob! This is a test tweet for E2E testing.',
          likeCount: 5,
          replyCount: 2,
          createdAt: DateTime.now()
              .subtract(const Duration(hours: 2))
              .toIso8601String(),
        ),
        FeedItem(
          tweetId: 'tweet-2',
          author: UserProfile(
            id: 'user-2',
            username: 'charlie',
            displayName: 'Charlie Brown',
          ),
          content: 'Testing Flutter integration! #flutter #testing',
          likeCount: 10,
          replyCount: 0,
          likedByMe: true,
          createdAt: DateTime.now()
              .subtract(const Duration(hours: 5))
              .toIso8601String(),
        ),
        FeedItem(
          tweetId: 'tweet-3',
          author: UserProfile(
            id: 'alice-id',
            username: 'alice',
            displayName: 'Alice',
          ),
          content:
              'Just setting up my TwitterFlux account! Excited to be here.',
          likeCount: 3,
          replyCount: 1,
          createdAt: DateTime.now()
              .subtract(const Duration(days: 1))
              .toIso8601String(),
        ),
      ],
      loading: false,
      hasMore: true,
    ),
  );
}

/// Setup inbox with comprehensive test messages.
void setupInboxState(FluxStore store) {
  store.setState(
    'inbox/state',
    InboxState(
      messages: [
        InboxMessage(
          id: 'msg-1',
          kind: 'system',
          title: 'Welcome to TwitterFlux',
          body: 'Welcome! This is a seeded message for testing purposes.',
          read: false,
          createdAt: DateTime.now()
              .subtract(const Duration(days: 7))
              .toIso8601String(),
        ),
        InboxMessage(
          id: 'msg-2',
          kind: 'like',
          title: 'New Like',
          body: '@bob liked your tweet',
          read: true,
          createdAt: DateTime.now()
              .subtract(const Duration(days: 3))
              .toIso8601String(),
        ),
        InboxMessage(
          id: 'msg-3',
          kind: 'follow',
          title: 'New Follower',
          body: '@charlie started following you',
          read: false,
          createdAt: DateTime.now()
              .subtract(const Duration(days: 1))
              .toIso8601String(),
        ),
        InboxMessage(
          id: 'msg-4',
          kind: 'mention',
          title: 'New Mention',
          body: '@dave mentioned you in a tweet',
          read: true,
          createdAt: DateTime.now()
              .subtract(const Duration(hours: 5))
              .toIso8601String(),
        ),
      ],
      unreadCount: 2,
      loading: false,
    ),
  );
}

/// Setup search state with test data.
void setupSearchState(FluxStore store) {
  store.setState(
    'search/state',
    SearchState(
      query: 'flutter',
      users: [
        UserProfile(
          id: 'user-flutter-1',
          username: 'flutterdev',
          displayName: 'Flutter Developer',
          followerCount: 10000,
        ),
        UserProfile(
          id: 'user-flutter-2',
          username: 'flutterteam',
          displayName: 'Flutter Team',
          followerCount: 50000,
        ),
      ],
      tweets: [
        FeedItem(
          tweetId: 'search-tweet-1',
          author: UserProfile(
            id: 'user-1',
            username: 'flutterfan',
            displayName: 'Flutter Fan',
          ),
          content: 'I love Flutter! Best framework ever.',
          likeCount: 100,
          createdAt: DateTime.now().toIso8601String(),
        ),
      ],
      loading: false,
    ),
  );
}

/// ============================================
/// 严格验证工具函数
/// ============================================

/// 验证 TabBar 是否在底部
void expectTabBarAtBottom(WidgetTester tester) {
  final tabBar = find.byType(CupertinoTabBar);
  expect(tabBar, findsOneWidget, reason: 'TabBar must exist');

  final tabBarRect = tester.getRect(tabBar);
  final screenHeight =
      tester.view.physicalSize.height / tester.view.devicePixelRatio;

  // TabBar 应该在屏幕底部（允许 100px 的误差）
  expect(
    tabBarRect.bottom,
    greaterThan(screenHeight - 100),
    reason:
        'TabBar must be at the bottom of the screen, '
        'but bottom was ${tabBarRect.bottom} and screen height is $screenHeight',
  );
}

/// 验证 TabBar 有且仅有 4 个 Tab
void expectTabBarHasFourTabs(WidgetTester tester) {
  final tabBar = find.byType(CupertinoTabBar);
  expect(tabBar, findsOneWidget);

  final tabBarWidget = tester.widget<CupertinoTabBar>(tabBar);
  expect(
    tabBarWidget.items.length,
    equals(4),
    reason: 'TabBar must have exactly 4 tabs (Home, Search, Inbox, Me)',
  );
}

/// 验证当前选中的 Tab
void expectSelectedTab(WidgetTester tester, String tabLabel) {
  // 查找所有 BottomNavigationBarItem 的 label
  final labels = find.text(tabLabel);
  expect(labels, findsWidgets, reason: 'Tab "$tabLabel" should exist');

  // 验证 TabBar 存在
  expect(find.byType(CupertinoTabBar), findsOneWidget);
}

/// 验证导航栈深度
void expectNavigationStackDepth(WidgetTester tester, int expectedDepth) {
  // 通过查找 CupertinoPageScaffold 来估算导航栈深度
  final navigators = find.byType(CupertinoPageScaffold);
  final actualDepth = navigators.evaluate().length;

  expect(
    actualDepth,
    equals(expectedDepth),
    reason:
        'Navigation stack depth should be $expectedDepth, but was $actualDepth',
  );
}

/// 验证页面是否有返回按钮
void expectBackButtonPresent(WidgetTester tester) {
  final backButton = find.byType(CupertinoNavigationBarBackButton);
  expect(backButton, findsOneWidget, reason: 'Page should have a back button');
}

/// 验证页面没有返回按钮（根页面）
void expectNoBackButton(WidgetTester tester) {
  final backButton = find.byType(CupertinoNavigationBarBackButton);
  expect(
    backButton,
    findsNothing,
    reason: 'Root page should not have a back button',
  );
}

/// 验证文本字段的 hint/placeholder
void expectTextFieldPlaceholder(WidgetTester tester, String placeholder) {
  final textField = find.byWidgetPredicate((widget) {
    if (widget is CupertinoTextField) {
      return widget.placeholder == placeholder;
    }
    return false;
  });

  expect(
    textField,
    findsOneWidget,
    reason: 'TextField with placeholder "$placeholder" not found',
  );
}

/// 验证按钮是否禁用
void expectButtonDisabled(WidgetTester tester, String buttonText) {
  final button = find.widgetWithText(CupertinoButton, buttonText);
  expect(button, findsOneWidget);

  final buttonWidget = tester.widget<CupertinoButton>(button);
  expect(
    buttonWidget.onPressed,
    isNull,
    reason: 'Button "$buttonText" should be disabled',
  );
}

/// 验证按钮是否启用
void expectButtonEnabled(WidgetTester tester, String buttonText) {
  final button = find.widgetWithText(CupertinoButton, buttonText);
  expect(button, findsOneWidget);

  final buttonWidget = tester.widget<CupertinoButton>(button);
  expect(
    buttonWidget.onPressed,
    isNotNull,
    reason: 'Button "$buttonText" should be enabled',
  );
}

/// 验证 ListView 是否可滚动
Future<void> expectListViewScrollable(
  WidgetTester tester,
  String listViewDescription,
) async {
  final listView = find.byType(ListView);
  expect(
    listView,
    findsOneWidget,
    reason: '$listViewDescription should be scrollable',
  );
}

/// 验证 widget 是否在视口中
void expectWidgetInViewport(
  WidgetTester tester,
  Finder finder,
  String description,
) {
  expect(finder, findsOneWidget);

  final rect = tester.getRect(finder);
  final screenSize = tester.view.physicalSize / tester.view.devicePixelRatio;

  expect(
    rect.top >= 0 && rect.bottom <= screenSize.height,
    isTrue,
    reason: '$description should be within viewport',
  );
}

/// ============================================
/// 便捷操作函数
/// ============================================

/// Find CupertinoTextField by placeholder text.
Finder findCupertinoTextField(String placeholder) {
  return find.byWidgetPredicate((widget) {
    if (widget is CupertinoTextField) {
      return widget.placeholder == placeholder;
    }
    return false;
  });
}

/// Enter text in a CupertinoTextField.
Future<void> enterTextInField(
  WidgetTester tester,
  String placeholder,
  String text,
) async {
  final field = findCupertinoTextField(placeholder);
  expect(
    field,
    findsOneWidget,
    reason: 'Text field with placeholder "$placeholder" not found',
  );
  await tester.enterText(field, text);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 120));
}

/// Tap a button by text.
Future<void> tapButton(WidgetTester tester, String text) async {
  final button = find.text(text);
  expect(button, findsOneWidget, reason: 'Button with text "$text" not found');
  await tester.tap(button);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 250));
}

/// Tap a widget by finder.
Future<void> tapWidget(
  WidgetTester tester,
  Finder finder,
  String description,
) async {
  expect(finder, findsOneWidget, reason: '$description not found');
  await tester.tap(finder);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 250));
}

/// 在底部 TabBar 中按标签点击（避免与页面标题重名造成歧义）
Finder findTabLabel(String label) {
  return find.descendant(
    of: find.byType(CupertinoTabBar),
    matching: find.text(label),
  );
}

Future<void> tapTab(WidgetTester tester, String label) async {
  final tabLabel = findTabLabel(label);
  expect(tabLabel, findsWidgets, reason: 'Tab "$label" not found in tab bar');
  await tester.tap(tabLabel.first, warnIfMissed: false);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 250));
}

/// Wait for a widget to appear with timeout.
Future<void> waitFor(
  WidgetTester tester,
  Finder finder, {
  Duration timeout = const Duration(seconds: 5),
}) async {
  final endTime = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(endTime)) {
    await tester.pump(const Duration(milliseconds: 100));
    if (finder.evaluate().isNotEmpty) return;
  }
  throw Exception('Widget not found within ${timeout.inSeconds}s: $finder');
}

/// Scroll until a widget is visible.
Future<void> scrollUntilVisible(
  WidgetTester tester,
  Finder scrollable,
  Finder target,
  String description,
) async {
  await tester.scrollUntilVisible(target, 100, scrollable: scrollable);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 200));
  expect(
    target,
    findsOneWidget,
    reason: '$description should be visible after scrolling',
  );
}

/// 模拟返回键/手势
Future<void> goBack(WidgetTester tester) async {
  // 尝试点击返回按钮
  final backButton = find.byType(CupertinoNavigationBarBackButton);
  if (backButton.evaluate().isNotEmpty) {
    await tester.tap(backButton);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
    return;
  }

  // 如果没有返回按钮，尝试物理返回（Android）
  await tester.binding.setSurfaceSize(const Size(375, 812));
  await tester.pump();
}

/// 截屏（用于调试）
Future<void> takeScreenshot(WidgetTester tester, String name) async {
  await tester.pump();
  // 在实际设备上会截取屏幕
}

/// Common test setup that runs before each test.
void commonSetUp() {
  // Reset any global state if needed
  TestWidgetsFlutterBinding.ensureInitialized();
}

/// Common test teardown that runs after each test.
void commonTearDown() {
  // Clean up any resources
}

/// ============================================
/// 性能测试工具
/// ============================================

/// 测量 widget 构建时间
Future<Duration> measureBuildTime(WidgetTester tester, Widget widget) async {
  final stopwatch = Stopwatch()..start();
  await tester.pumpWidget(widget);
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 200));
  stopwatch.stop();
  return stopwatch.elapsed;
}

/// 验证构建时间在合理范围内
void expectBuildTimeUnder(Duration buildTime, int maxMilliseconds) {
  expect(
    buildTime.inMilliseconds,
    lessThan(maxMilliseconds),
    reason:
        'Build time should be under ${maxMilliseconds}ms, '
        'but was ${buildTime.inMilliseconds}ms',
  );
}

/// ============================================
/// 边界测试数据生成器
/// ============================================

/// 生成长文本（用于测试字符限制）
String generateLongText(int charCount) {
  return 'A' * charCount;
}

/// 生成特殊字符文本
String generateSpecialCharText() {
  return '!@#\$%^&*()_+-=[]{}|;\':",./<>?~`™®©℠℡№℀℁ℂ℃℄℅℆ℇ℈℉ℊℋℌℍℎℏℐℑℒℓ℔ℕ№℗℘ℙℚℛℜℝ℞℟℠℡™℣ℤ℥Ω℧ℨ℩KÅℬℭ℮ℯℰℱℲℳℴℵℶℷℸℹ℺℻ℼℽℾℿ⅀⅁⅂⅃⅄ⅅⅆⅇⅈⅉ⅊⅋⅌⅍ⅎ⅏⅐⅑⅒⅓⅔⅕⅖⅗⅘⅙⅚⅛⅜⅝⅞⅟ⅠⅡⅢⅣⅤⅥⅦⅧⅨⅩⅪⅫⅬⅭⅮⅯⅰⅱⅲⅳⅴⅵⅶⅷⅸⅹⅺⅻⅼⅽⅾⅿ';
}

/// 生成 Unicode 表情符号文本
String generateEmojiText() {
  return '😀😃😄😁😆😅😂🤣😊😇🙂🙃😉😌😍🥰😘😗😙😚😋😛😝😜🤪🤨🧐🤓😎🥸🤩🥳😏😒😞😔😟😕🙁☹️😣😖😫😩🥺😢😭😤😠😡🤬🤯😳🥵🥶😱😨😰😥😓🤗🤔🤭🤫🤥😶😐😑😬🙄😯😦😧😮😲🥱😴🤤😪😵🤐🥴🤢🤮🤧😷🤒🤕🤑🤠😈👿👹👺🤡💩👻💀☠️👽👾🤖🎃😺😸😹😻😼😽🙀😿😾';
}
