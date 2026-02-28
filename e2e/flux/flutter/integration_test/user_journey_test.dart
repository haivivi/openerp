/// Complete User Journey Integration Test
/// Full end-to-end flow: Login → Tabs → Compose → Settings → Sign Out
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('Complete User Journey Test', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
    });

    tearDown(() {
      commonTearDown();
    });

    testWidgets(
      'Scenario 7.1: Full user journey - Login through Sign Out',
      (WidgetTester tester) async {
        // ===== STEP 1: Login =====
        print('Step 1: Logging in...');
        await pumpApp(tester, store: store);

        // Verify on login page
        expect(find.text('TwitterFlux'), findsOneWidget);
        expect(find.text('Sign In'), findsOneWidget);

        // Enter credentials
        await enterTextInField(tester, 'Username', 'alice');
        await enterTextInField(tester, 'Password', 'password');
        await tapButton(tester, 'Sign In');

        // Verify login success
        await waitFor(tester, find.byType(CupertinoTabScaffold));
        expect(find.byType(CupertinoTabScaffold), findsOneWidget);
        print('✓ Login successful');

        // ===== STEP 2: Setup test data =====
        setupTimelineFeed(store);
        setupInboxState(store);
        await tester.pumpAndSettle();

        // ===== STEP 3: Traverse all tabs =====
        print('Step 3: Traversing tabs...');

        // Home Tab (already active)
        expect(find.text('Home'), findsWidgets);
        expect(find.byType(ListView), findsOneWidget);
        print('✓ Home tab OK');

        // Search Tab
        await tapTab(tester, 'Search');
        expect(find.byType(CupertinoSearchTextField), findsOneWidget);
        print('✓ Search tab OK');

        // Inbox Tab
        await tapTab(tester, 'Inbox');
        expect(find.text('Welcome to TwitterFlux'), findsOneWidget);
        expect(find.text('New Like'), findsOneWidget);
        print('✓ Inbox tab OK');

        // Me Tab
        await tapTab(tester, 'Me');
        expect(find.text('Alice'), findsOneWidget);
        expect(find.text('@alice'), findsOneWidget);
        print('✓ Me tab OK');

        // Return to Home
        await tapTab(tester, 'Home');
        print('✓ All tabs traversed');

        // ===== STEP 4: Compose a tweet =====
        print('Step 4: Composing tweet...');
        await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
        await tester.pumpAndSettle();

        expect(find.text('New Tweet'), findsOneWidget);

        // Enter content
        await tester.enterText(
          find.byType(CupertinoTextField),
          'E2E test tweet from user journey',
        );
        await tester.pumpAndSettle();

        // Post
        await tester.tap(find.text('Post'));
        await tester.pumpAndSettle();

        // Verify back on Home
        expect(find.text('Home'), findsWidgets);
        print('✓ Tweet posted');

        // ===== STEP 5: Navigate to Language Picker =====
        print('Step 5: Testing language settings...');
        await tapTab(tester, 'Me');

        await tester.tap(find.text('Language'));
        await tester.pumpAndSettle();

        expect(find.text('English'), findsOneWidget);
        expect(find.text('简体中文'), findsOneWidget);
        print('✓ Language picker opened');

        // Go back to Me
        await tester.tap(find.byType(CupertinoNavigationBarBackButton));
        await tester.pumpAndSettle();

        // ===== STEP 6: Sign Out =====
        print('Step 6: Signing out...');
        await tester.tap(find.text('Sign Out'));
        await tester.pumpAndSettle();

        // Verify back on login page
        expect(find.text('TwitterFlux'), findsOneWidget);
        expect(find.text('Sign In'), findsOneWidget);
        expect(find.byType(CupertinoTabScaffold), findsNothing);
        print('✓ Signed out successfully');

        // ===== FINAL VERIFICATION =====
        print('\n✅ Full user journey completed successfully!');
        print('   - Login: PASSED');
        print('   - Tab Navigation: PASSED');
        print('   - Compose Tweet: PASSED');
        print('   - Language Settings: PASSED');
        print('   - Sign Out: PASSED');
      },
      timeout: const Timeout(Duration(minutes: 2)),
    );

    testWidgets(
      'Scenario 7.2: Quick smoke test - Verify all major components',
      (WidgetTester tester) async {
        // Quick test to verify all major components work
        await pumpApp(tester, store: store);

        // Login
        await enterTextInField(tester, 'Username', 'alice');
        await enterTextInField(tester, 'Password', 'password');
        await tapButton(tester, 'Sign In');

        await waitFor(tester, find.byType(CupertinoTabScaffold));

        // Setup data
        setupTimelineFeed(store);
        setupInboxState(store);
        await tester.pumpAndSettle();

        // Quick traverse
        for (final tab in ['Search', 'Inbox', 'Me', 'Home']) {
          await tapTab(tester, tab);
        }

        // Sign out
        await tapTab(tester, 'Me');
        await tester.tap(find.text('Sign Out'));
        await tester.pumpAndSettle();

        // Verify
        expect(find.text('Sign In'), findsOneWidget);
      },
      timeout: const Timeout(Duration(minutes: 1)),
    );
  });
}
