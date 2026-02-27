/// Golden tests for all 11 target pages.
///
/// Run:   flutter test test/flux_pages_golden_test.dart
/// Update: flutter test --update-goldens test/flux_pages_golden_test.dart
library;

import 'package:flutter_test/flutter_test.dart';

import 'package:twitter_flux/views/change_password_view.dart';
import 'package:twitter_flux/views/compose_view.dart';
import 'package:twitter_flux/views/edit_profile_view.dart';
import 'package:twitter_flux/views/home_view.dart';
import 'package:twitter_flux/views/inbox_view.dart';
import 'package:twitter_flux/views/language_picker_view.dart';
import 'package:twitter_flux/views/login_view.dart';
import 'package:twitter_flux/views/me_view.dart';
import 'package:twitter_flux/views/profile_view.dart';
import 'package:twitter_flux/views/search_view.dart';
import 'package:twitter_flux/views/tweet_detail_view.dart';

import 'golden_harness.dart';

void main() {
  group('Golden — all pages', () {
    testWidgets('Login page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('login');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const LoginView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(LoginView),
        matchesGoldenFile('golden/login.png'),
      );
    });

    testWidgets('Home page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('home');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const HomeView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(HomeView),
        matchesGoldenFile('golden/home.png'),
      );
    });

    testWidgets('Search page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('search');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const SearchView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(SearchView),
        matchesGoldenFile('golden/search.png'),
      );
    });

    testWidgets('Inbox page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('inbox');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const InboxView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(InboxView),
        matchesGoldenFile('golden/inbox.png'),
      );
    });

    testWidgets('Compose page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('compose');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const ComposeView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(ComposeView),
        matchesGoldenFile('golden/compose.png'),
      );
    });

    testWidgets('TweetDetail page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('tweet_detail');

      await tester.pumpWidget(
        GoldenHarness(
          store: store,
          child: const TweetDetailView(tweetId: 't1'),
        ),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(TweetDetailView),
        matchesGoldenFile('golden/tweet_detail.png'),
      );
    });

    testWidgets('Profile page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('profile');

      await tester.pumpWidget(
        GoldenHarness(
          store: store,
          child: const ProfileView(userId: 'bob'),
        ),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(ProfileView),
        matchesGoldenFile('golden/profile.png'),
      );
    });

    testWidgets('Me page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('me');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const MeView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(MeView),
        matchesGoldenFile('golden/me.png'),
      );
    });

    testWidgets('LanguagePicker page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('language_picker');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const LanguagePickerView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(LanguagePickerView),
        matchesGoldenFile('golden/language_picker.png'),
      );
    });

    testWidgets('EditProfile page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('edit_profile');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const EditProfileView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(EditProfileView),
        matchesGoldenFile('golden/edit_profile.png'),
      );
    });

    testWidgets('ChangePassword page', (tester) async {
      await setGoldenDeviceSize(tester);
      final store = goldenStore('change_password');

      await tester.pumpWidget(
        GoldenHarness(store: store, child: const ChangePasswordView()),
      );
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(ChangePasswordView),
        matchesGoldenFile('golden/change_password.png'),
      );
    });
  });
}
