/// Golden test harness — fixed environment for deterministic screenshots.
///
/// Wraps any widget in a consistent rendering context:
/// - Device size: 375 × 812 (iPhone X logical points)
/// - DPR: 3.0
/// - Locale: en_US
/// - textScaleFactor: 1.0
/// - Theme: Cupertino light
/// - Platform: iOS
/// - Animations: disabled (pumped to settle)
library;

import 'package:flutter/cupertino.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:twitter_flux/models/models.dart';
import 'package:twitter_flux/store/flux_store.dart';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const Size kTestDeviceSize = Size(375, 812);
const double kTestDevicePixelRatio = 3.0;

// ---------------------------------------------------------------------------
// Test fixtures — canonical data for all 11 pages
// ---------------------------------------------------------------------------

const _alice = UserProfile(
  id: 'alice',
  username: 'alice',
  displayName: 'Alice',
  bio: 'Flutter & Rust enthusiast',
  followerCount: 42,
  followingCount: 18,
  tweetCount: 7,
);

const _bob = UserProfile(
  id: 'bob',
  username: 'bob',
  displayName: 'Bob',
  bio: 'Loves coffee ☕',
  followerCount: 100,
  followingCount: 50,
  tweetCount: 23,
);

const _tweet1 = FeedItem(
  tweetId: 't1',
  author: _alice,
  content: 'Hello from TwitterFlux! This is a test tweet.',
  likeCount: 5,
  likedByMe: false,
  replyCount: 2,
  createdAt: '2026-02-27T10:00:00Z',
);

const _tweet2 = FeedItem(
  tweetId: 't2',
  author: _bob,
  content: 'Great day for coding! #flutter #rust',
  likeCount: 12,
  likedByMe: true,
  replyCount: 0,
  createdAt: '2026-02-27T09:30:00Z',
);

const _reply1 = FeedItem(
  tweetId: 'r1',
  author: _bob,
  content: 'Nice tweet Alice!',
  likeCount: 1,
  replyCount: 0,
  replyToId: 't1',
  createdAt: '2026-02-27T10:05:00Z',
);

const _message1 = InboxMessage(
  id: 'm1',
  kind: 'system',
  title: 'Welcome to Flux!',
  body: 'You have successfully joined TwitterFlux. Start exploring!',
  read: false,
  createdAt: '2026-02-27T08:00:00Z',
);

const _message2 = InboxMessage(
  id: 'm2',
  kind: 'broadcast',
  title: 'New Feature Available',
  body: 'We just shipped language switching. Try it in Settings!',
  read: true,
  createdAt: '2026-02-26T14:00:00Z',
);

const _message3 = InboxMessage(
  id: 'm3',
  kind: 'personal',
  title: '@bob liked your tweet',
  body: 'Bob liked "Hello from TwitterFlux!"',
  read: false,
  createdAt: '2026-02-27T09:45:00Z',
);

// ---------------------------------------------------------------------------
// Store factory — pre-populated FluxStore for Golden tests
// ---------------------------------------------------------------------------

/// Creates a [FluxStore] pre-populated with fixture data for the given [page].
///
/// Each page name corresponds to one of the 11 target Golden pages:
/// login, home, search, inbox, compose, tweet_detail, profile, me,
/// language_picker, edit_profile, change_password.
FluxStore goldenStore(String page) {
  final store = FluxStore();

  // Most pages need an authenticated user.
  if (page != 'login') {
    store.setState(
      'auth/state',
      const AuthState(phase: AuthPhase.authenticated, user: _alice),
    );
  }

  switch (page) {
    case 'login':
      store.setState(
        'auth/state',
        const AuthState(phase: AuthPhase.unauthenticated),
      );

    case 'home':
      store.setState(
        'timeline/feed',
        const TimelineFeed(items: [_tweet1, _tweet2]),
      );

    case 'search':
      store.setState(
        'search/state',
        const SearchState(query: 'flutter', users: [_bob], tweets: [_tweet2]),
      );

    case 'inbox':
      store.setState(
        'inbox/state',
        const InboxState(
          messages: [_message1, _message2, _message3],
          unreadCount: 2,
        ),
      );

    case 'compose':
      store.setState('compose/state', const ComposeState());

    case 'tweet_detail':
      store.setState(
        'tweet/t1',
        const TweetDetailState(tweet: _tweet1, replies: [_reply1]),
      );

    case 'profile':
      store.setState(
        'profile/bob',
        const ProfilePage(user: _bob, tweets: [_tweet2], followedByMe: false),
      );

    case 'me':
      // auth/state already set above.
      break;

    case 'language_picker':
      // Only needs auth — locale is in the store.
      break;

    case 'edit_profile':
      store.setState(
        'settings/state',
        const SettingsState(
          displayName: 'Alice',
          bio: 'Flutter & Rust enthusiast',
        ),
      );

    case 'change_password':
      store.setState('settings/password', const PasswordState());
  }

  return store;
}

// ---------------------------------------------------------------------------
// GoldenHarness widget — wraps a page widget for screenshot
// ---------------------------------------------------------------------------

/// Wraps [child] in a deterministic Cupertino environment for Golden tests.
class GoldenHarness extends StatelessWidget {
  final FluxStore store;
  final Widget child;

  const GoldenHarness({super.key, required this.store, required this.child});

  @override
  Widget build(BuildContext context) {
    return FluxStoreScope(
      store: store,
      child: CupertinoApp(
        debugShowCheckedModeBanner: false,
        theme: const CupertinoThemeData(brightness: Brightness.light),
        home: child,
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Configures [tester] surface to the fixed Golden device size.
Future<void> setGoldenDeviceSize(WidgetTester tester) async {
  tester.view.physicalSize = kTestDeviceSize * kTestDevicePixelRatio;
  tester.view.devicePixelRatio = kTestDevicePixelRatio;
  addTearDown(() {
    tester.view.resetPhysicalSize();
    tester.view.resetDevicePixelRatio();
  });
}
