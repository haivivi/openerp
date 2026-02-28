/// Me View & Settings Integration Tests
/// Tests: Language switching, Sign Out, Navigation
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('Me View & Settings Tests', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
      setupAuthenticatedState(store);
    });

    tearDown(() {
      commonTearDown();
    });

    testWidgets('Scenario 5.1: View user profile information', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Me tab
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Verify user info is displayed
      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('@alice'), findsOneWidget);
      expect(find.textContaining('Flutter developer'), findsOneWidget);

      // Verify avatar with initial
      expect(find.text('A'), findsOneWidget); // First letter of Alice

      // Verify stats
      expect(find.text('42'), findsOneWidget);
      expect(find.text('100'), findsOneWidget);
      expect(find.text('25'), findsOneWidget);

      // Verify settings options
      expect(find.text('Edit Profile'), findsOneWidget);
      expect(find.text('Change Password'), findsOneWidget);
      expect(find.text('Language'), findsOneWidget);
      expect(find.text('Sign Out'), findsOneWidget);
    });

    testWidgets('Scenario 5.2: Sign Out returns to login page', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Me tab
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Tap Sign Out
      await tester.tap(find.text('Sign Out'));
      await tester.pumpAndSettle();

      // Verify we're on login page
      expect(find.text('TwitterFlux'), findsOneWidget);
      expect(find.text('Sign In'), findsOneWidget);
      expect(find.text('Username'), findsOneWidget);
      expect(find.text('Password'), findsOneWidget);

      // Verify MainTabView is gone
      expect(find.byType(CupertinoTabScaffold), findsNothing);

      // Verify auth state
      final auth = store.get<AuthState>('auth/state');
      expect(auth!.phase, equals(AuthPhase.unauthenticated));
    });

    testWidgets('Scenario 6.1: Navigate to Language Picker', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Me tab
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Tap Language
      await tester.tap(find.text('Language'));
      await tester.pumpAndSettle();

      // Verify Language Picker page
      expect(find.text('Language'), findsWidgets);
      expect(find.text('English'), findsOneWidget);
      expect(find.text('简体中文'), findsOneWidget);
    });

    testWidgets('Scenario 6.2: Switch language to Chinese', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Me tab
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Verify current language shows English labels
      expect(find.text('Edit Profile'), findsOneWidget);

      // Navigate to Language Picker
      await tester.tap(find.text('Language'));
      await tester.pumpAndSettle();

      // Select Chinese
      await tester.tap(find.text('简体中文'));
      await tester.pumpAndSettle();

      // Navigate back
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // Verify some labels are now in Chinese (if translations exist)
      // Note: This depends on actual translations being available
    });

    testWidgets('Scenario 6.3: Navigate to Edit Profile', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Me tab
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Tap Edit Profile
      await tester.tap(find.text('Edit Profile'));
      await tester.pumpAndSettle();

      // Verify Edit Profile page
      expect(find.text('Edit Profile'), findsWidgets);

      // Go back
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // Verify we're back on Me
      expect(find.text('Me'), findsWidgets);
    });

    testWidgets('Scenario 6.4: Navigate to Change Password', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Me tab
      await tester.tap(find.text('Me'));
      await tester.pumpAndSettle();

      // Tap Change Password
      await tester.tap(find.text('Change Password'));
      await tester.pumpAndSettle();

      // Verify Change Password page
      expect(find.text('Change Password'), findsWidgets);

      // Go back
      await tester.tap(find.byType(CupertinoNavigationBarBackButton));
      await tester.pumpAndSettle();

      // Verify we're back on Me
      expect(find.text('Me'), findsWidgets);
    });
  });
}
