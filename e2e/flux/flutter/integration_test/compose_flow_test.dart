/// Compose Flow Integration Tests
/// Tests: Tweet creation, character limit, cancel flow
library;

import 'test_helper.dart';

void main() {
  initIntegrationTests();

  group('Compose Flow Tests', () {
    late FluxStore store;

    setUp(() {
      commonSetUp();
      store = createTestStore();
      setupAuthenticatedState(store);
      setupTimelineFeed(store);
    });

    tearDown(() {
      commonTearDown();
    });

    testWidgets('Scenario 3.1: Successfully compose and post a tweet', (
      WidgetTester tester,
    ) async {
      // Arrange: Start at Home
      await pumpApp(tester, store: store);

      // Tap compose button
      final composeButton = find.byIcon(CupertinoIcons.square_pencil);
      expect(composeButton, findsOneWidget);
      await tester.tap(composeButton);
      await tester.pumpAndSettle();

      // Verify we're on Compose page
      expect(find.text('New Tweet'), findsOneWidget);
      expect(find.text('Cancel'), findsOneWidget);
      expect(find.text('Post'), findsOneWidget);

      // Enter tweet content
      final textField = find.byType(CupertinoTextField);
      expect(textField, findsOneWidget);
      await tester.enterText(textField, 'This is a test tweet from E2E');
      await tester.pumpAndSettle();

      // Verify character count
      expect(find.text('29/280'), findsOneWidget);

      // Tap Post button
      await tester.tap(find.text('Post'));
      await tester.pumpAndSettle();

      // Verify we're back on Home
      expect(find.text('Home'), findsWidgets);
      expect(find.byType(CupertinoPageScaffold), findsWidgets);
    });

    testWidgets('Scenario 3.2: Cannot post empty tweet', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Compose
      await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
      await tester.pumpAndSettle();

      // Verify Post button is disabled when empty
      final postButtonFinder = find.widgetWithText(CupertinoButton, 'Post');
      expect(postButtonFinder, findsOneWidget);

      final postButton = tester.widget<CupertinoButton>(postButtonFinder);
      expect(
        postButton.onPressed,
        isNull,
        reason: 'Post button should be disabled with empty content',
      );
    });

    testWidgets('Scenario 3.3: Character count and limit validation', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Compose
      await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
      await tester.pumpAndSettle();

      final textField = find.byType(CupertinoTextField);

      // Test with 100 characters
      final hundredChars = 'A' * 100;
      await tester.enterText(textField, hundredChars);
      await tester.pumpAndSettle();

      expect(find.text('100/280'), findsOneWidget);

      // Clear and test with 280 characters (limit)
      await tester.enterText(textField, 'B' * 280);
      await tester.pumpAndSettle();

      expect(find.text('280/280'), findsOneWidget);

      // Verify Post button is enabled
      final postButton = tester.widget<CupertinoButton>(
        find.widgetWithText(CupertinoButton, 'Post'),
      );
      expect(postButton.onPressed, isNotNull);

      // Test with 281 characters (over limit)
      await tester.enterText(textField, 'C' * 281);
      await tester.pumpAndSettle();

      expect(find.text('281/280'), findsOneWidget);

      // Verify Post button is now disabled
      final postButtonOverLimit = tester.widget<CupertinoButton>(
        find.widgetWithText(CupertinoButton, 'Post'),
      );
      expect(postButtonOverLimit.onPressed, isNull);
    });

    testWidgets('Scenario 3.4: Cancel compose and return to Home', (
      WidgetTester tester,
    ) async {
      await pumpApp(tester, store: store);

      // Navigate to Compose
      await tester.tap(find.byIcon(CupertinoIcons.square_pencil));
      await tester.pumpAndSettle();

      // Enter some text
      await tester.enterText(
        find.byType(CupertinoTextField),
        'Draft text that will be discarded',
      );
      await tester.pumpAndSettle();

      // Tap Cancel
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      // Verify we're back on Home
      expect(find.text('Home'), findsWidgets);

      // Verify the draft was not posted (not in timeline)
      expect(find.text('Draft text that will be discarded'), findsNothing);
    });
  });
}
