/// Twitter state models — Dart mirrors of Rust #[state] types.
/// Golden test: hand-written. Production: auto-generated from #[state] definitions.
library;

// MARK: - auth/state

enum AuthPhase { unauthenticated, authenticated }

class AuthState {
  final AuthPhase phase;
  final UserProfile? user;
  final bool busy;
  final String? error;

  const AuthState({
    required this.phase,
    this.user,
    this.busy = false,
    this.error,
  });

  AuthState copyWith({
    AuthPhase? phase,
    UserProfile? user,
    bool? busy,
    String? error,
    bool clearError = false,
  }) {
    return AuthState(
      phase: phase ?? this.phase,
      user: user ?? this.user,
      busy: busy ?? this.busy,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

class UserProfile {
  final String id;
  final String username;
  final String displayName;
  final String? bio;
  final String? avatar;
  final int followerCount;
  final int followingCount;
  final int tweetCount;

  const UserProfile({
    required this.id,
    required this.username,
    required this.displayName,
    this.bio,
    this.avatar,
    this.followerCount = 0,
    this.followingCount = 0,
    this.tweetCount = 0,
  });
}

// MARK: - timeline/feed

class TimelineFeed {
  final List<FeedItem> items;
  final bool loading;
  final bool hasMore;
  final String? error;

  const TimelineFeed({
    this.items = const [],
    this.loading = false,
    this.hasMore = false,
    this.error,
  });
}

class FeedItem {
  final String tweetId;
  final UserProfile author;
  final String content;
  final int likeCount;
  final bool likedByMe;
  final int replyCount;
  final String? replyToId;
  final String createdAt;

  const FeedItem({
    required this.tweetId,
    required this.author,
    required this.content,
    this.likeCount = 0,
    this.likedByMe = false,
    this.replyCount = 0,
    this.replyToId,
    this.createdAt = '',
  });
}

// MARK: - compose/state

class ComposeState {
  final String content;
  final String? replyToId;
  final bool busy;
  final String? error;

  const ComposeState({
    this.content = '',
    this.replyToId,
    this.busy = false,
    this.error,
  });
}

// MARK: - profile/{id}

class ProfilePage {
  final UserProfile user;
  final List<FeedItem> tweets;
  final bool followedByMe;
  final bool loading;

  const ProfilePage({
    required this.user,
    this.tweets = const [],
    this.followedByMe = false,
    this.loading = false,
  });
}

// MARK: - tweet/{id}

class TweetDetailState {
  final FeedItem tweet;
  final List<FeedItem> replies;
  final bool loading;

  const TweetDetailState({
    required this.tweet,
    this.replies = const [],
    this.loading = false,
  });
}

// MARK: - search/state

class SearchState {
  final String query;
  final List<UserProfile> users;
  final List<FeedItem> tweets;
  final bool loading;
  final String? error;

  const SearchState({
    this.query = '',
    this.users = const [],
    this.tweets = const [],
    this.loading = false,
    this.error,
  });
}

// MARK: - settings/state

class SettingsState {
  final String displayName;
  final String bio;
  final bool busy;
  final bool saved;
  final String? error;

  const SettingsState({
    this.displayName = '',
    this.bio = '',
    this.busy = false,
    this.saved = false,
    this.error,
  });
}

// MARK: - settings/password

class PasswordState {
  final bool busy;
  final bool success;
  final String? error;

  const PasswordState({this.busy = false, this.success = false, this.error});
}

// MARK: - inbox/state

class InboxState {
  final List<InboxMessage> messages;
  final int unreadCount;
  final bool loading;
  final String? error;

  const InboxState({
    this.messages = const [],
    this.unreadCount = 0,
    this.loading = false,
    this.error,
  });
}

class InboxMessage {
  final String id;
  final String kind;
  final String title;
  final String body;
  final bool read;
  final String createdAt;

  const InboxMessage({
    required this.id,
    required this.kind,
    required this.title,
    required this.body,
    this.read = false,
    this.createdAt = '',
  });
}
