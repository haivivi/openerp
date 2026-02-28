/// Unit tests for model JSON deserialization.
///
/// Verifies that every model's fromJson correctly parses the JSON format
/// produced by the Rust Flux engine (matching Swift's Codable decoding).
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:twitter_flux/models/models.dart';

void main() {
  group('AuthState.fromJson', () {
    test('unauthenticated with no user', () {
      final state = AuthState.fromJson({
        'phase': 'unauthenticated',
        'user': null,
        'busy': false,
        'error': null,
      });
      expect(state.phase, AuthPhase.unauthenticated);
      expect(state.user, isNull);
      expect(state.busy, isFalse);
      expect(state.error, isNull);
    });

    test('authenticated with user', () {
      final state = AuthState.fromJson({
        'phase': 'authenticated',
        'user': {
          'id': 'alice',
          'username': 'alice',
          'displayName': 'Alice Wang',
          'bio': 'Rust dev',
          'avatar': null,
          'followerCount': 42,
          'followingCount': 18,
          'tweetCount': 7,
        },
        'busy': false,
        'error': null,
      });
      expect(state.phase, AuthPhase.authenticated);
      expect(state.user, isNotNull);
      expect(state.user!.username, 'alice');
      expect(state.user!.displayName, 'Alice Wang');
      expect(state.user!.followerCount, 42);
    });

    test('busy with error', () {
      final state = AuthState.fromJson({
        'phase': 'unauthenticated',
        'busy': true,
        'error': 'Invalid credentials',
      });
      expect(state.busy, isTrue);
      expect(state.error, 'Invalid credentials');
    });
  });

  group('UserProfile.fromJson', () {
    test('full profile', () {
      final user = UserProfile.fromJson({
        'id': 'bob',
        'username': 'bob',
        'displayName': 'Bob Li',
        'bio': 'Designer',
        'avatar': 'https://example.com/avatar.png',
        'followerCount': 100,
        'followingCount': 50,
        'tweetCount': 23,
      });
      expect(user.id, 'bob');
      expect(user.displayName, 'Bob Li');
      expect(user.bio, 'Designer');
      expect(user.avatar, isNotNull);
      expect(user.tweetCount, 23);
    });

    test('minimal profile with defaults', () {
      final user = UserProfile.fromJson({
        'id': 'x',
        'username': 'x',
        'displayName': 'X',
      });
      expect(user.followerCount, 0);
      expect(user.bio, isNull);
    });
  });

  group('TimelineFeed.fromJson', () {
    test('with items', () {
      final feed = TimelineFeed.fromJson({
        'items': [
          {
            'tweetId': 't1',
            'author': {
              'id': 'alice',
              'username': 'alice',
              'displayName': 'Alice',
            },
            'content': 'Hello!',
            'likeCount': 5,
            'likedByMe': true,
            'replyCount': 0,
            'createdAt': '2026-01-01T00:00:00Z',
          },
        ],
        'loading': false,
        'hasMore': true,
      });
      expect(feed.items.length, 1);
      expect(feed.items[0].tweetId, 't1');
      expect(feed.items[0].likedByMe, isTrue);
      expect(feed.hasMore, isTrue);
    });

    test('empty feed', () {
      final feed = TimelineFeed.fromJson({
        'items': <dynamic>[],
        'loading': true,
        'hasMore': false,
      });
      expect(feed.items, isEmpty);
      expect(feed.loading, isTrue);
    });
  });

  group('FeedItem.fromJson', () {
    test('reply item', () {
      final item = FeedItem.fromJson({
        'tweetId': 'r1',
        'author': {'id': 'bob', 'username': 'bob', 'displayName': 'Bob'},
        'content': 'Nice!',
        'likeCount': 1,
        'likedByMe': false,
        'replyCount': 0,
        'replyToId': 't1',
        'createdAt': '2026-01-01T00:05:00Z',
      });
      expect(item.replyToId, 't1');
      expect(item.author.username, 'bob');
    });
  });

  group('ComposeState.fromJson', () {
    test('default state', () {
      final state = ComposeState.fromJson({
        'content': '',
        'replyToId': null,
        'busy': false,
        'error': null,
      });
      expect(state.content, '');
      expect(state.busy, isFalse);
    });
  });

  group('ProfilePage.fromJson', () {
    test('with tweets and follow state', () {
      final page = ProfilePage.fromJson({
        'user': {
          'id': 'bob',
          'username': 'bob',
          'displayName': 'Bob',
          'followerCount': 10,
          'followingCount': 5,
          'tweetCount': 3,
        },
        'tweets': [
          {
            'tweetId': 't1',
            'author': {'id': 'bob', 'username': 'bob', 'displayName': 'Bob'},
            'content': 'Test',
          },
        ],
        'followedByMe': true,
        'loading': false,
      });
      expect(page.user.username, 'bob');
      expect(page.tweets.length, 1);
      expect(page.followedByMe, isTrue);
    });
  });

  group('TweetDetailState.fromJson', () {
    test('tweet with replies', () {
      final detail = TweetDetailState.fromJson({
        'tweet': {
          'tweetId': 't1',
          'author': {
            'id': 'alice',
            'username': 'alice',
            'displayName': 'Alice',
          },
          'content': 'Original tweet',
          'likeCount': 5,
          'replyCount': 1,
        },
        'replies': [
          {
            'tweetId': 'r1',
            'author': {'id': 'bob', 'username': 'bob', 'displayName': 'Bob'},
            'content': 'Reply!',
            'replyToId': 't1',
          },
        ],
        'loading': false,
      });
      expect(detail.tweet.content, 'Original tweet');
      expect(detail.replies.length, 1);
      expect(detail.replies[0].replyToId, 't1');
    });
  });

  group('SearchState.fromJson', () {
    test('with users and tweets', () {
      final state = SearchState.fromJson({
        'query': 'flutter',
        'users': [
          {'id': 'bob', 'username': 'bob', 'displayName': 'Bob'},
        ],
        'tweets': <dynamic>[],
        'loading': false,
      });
      expect(state.query, 'flutter');
      expect(state.users.length, 1);
      expect(state.tweets, isEmpty);
    });
  });

  group('SettingsState.fromJson', () {
    test('saved state', () {
      final state = SettingsState.fromJson({
        'displayName': 'Alice',
        'bio': 'Dev',
        'busy': false,
        'saved': true,
      });
      expect(state.displayName, 'Alice');
      expect(state.saved, isTrue);
    });
  });

  group('PasswordState.fromJson', () {
    test('success', () {
      final state = PasswordState.fromJson({
        'busy': false,
        'success': true,
        'error': null,
      });
      expect(state.success, isTrue);
      expect(state.error, isNull);
    });

    test('error', () {
      final state = PasswordState.fromJson({
        'busy': false,
        'success': false,
        'error': 'Wrong password',
      });
      expect(state.error, 'Wrong password');
    });
  });

  group('InboxState.fromJson', () {
    test('with messages', () {
      final state = InboxState.fromJson({
        'messages': [
          {
            'id': 'm1',
            'kind': 'system',
            'title': 'Welcome!',
            'body': 'Hello world',
            'read': false,
            'createdAt': '2026-01-01T00:00:00Z',
          },
          {
            'id': 'm2',
            'kind': 'broadcast',
            'title': 'Update',
            'body': 'New feature',
            'read': true,
            'createdAt': '2026-01-02T00:00:00Z',
          },
        ],
        'unreadCount': 1,
        'loading': false,
      });
      expect(state.messages.length, 2);
      expect(state.unreadCount, 1);
      expect(state.messages[0].kind, 'system');
      expect(state.messages[0].read, isFalse);
      expect(state.messages[1].read, isTrue);
    });
  });
}
