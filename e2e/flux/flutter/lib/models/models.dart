/// Twitter state models — Dart mirrors of Rust #[state] types.
/// Golden test: hand-written. Production: auto-generated from #[state] definitions.
///
/// Each model has a `fromJson` factory matching Swift's Codable decoding,
/// so FluxStore can deserialize JSON from the Rust FFI engine.
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

  factory AuthState.fromJson(Map<String, dynamic> json) => AuthState(
    phase: json['phase'] == 'authenticated'
        ? AuthPhase.authenticated
        : AuthPhase.unauthenticated,
    user: json['user'] != null
        ? UserProfile.fromJson(json['user'] as Map<String, dynamic>)
        : null,
    busy: json['busy'] as bool? ?? false,
    error: json['error'] as String?,
  );

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

  factory UserProfile.fromJson(Map<String, dynamic> json) => UserProfile(
    id: json['id'] as String? ?? '',
    username: json['username'] as String? ?? '',
    displayName: json['displayName'] as String? ?? '',
    bio: json['bio'] as String?,
    avatar: json['avatar'] as String?,
    followerCount: json['followerCount'] as int? ?? 0,
    followingCount: json['followingCount'] as int? ?? 0,
    tweetCount: json['tweetCount'] as int? ?? 0,
  );
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

  factory TimelineFeed.fromJson(Map<String, dynamic> json) => TimelineFeed(
    items:
        (json['items'] as List<dynamic>?)
            ?.map((e) => FeedItem.fromJson(e as Map<String, dynamic>))
            .toList() ??
        const [],
    loading: json['loading'] as bool? ?? false,
    hasMore: json['hasMore'] as bool? ?? false,
    error: json['error'] as String?,
  );
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

  factory FeedItem.fromJson(Map<String, dynamic> json) => FeedItem(
    tweetId: json['tweetId'] as String? ?? '',
    author: UserProfile.fromJson(json['author'] as Map<String, dynamic>),
    content: json['content'] as String? ?? '',
    likeCount: json['likeCount'] as int? ?? 0,
    likedByMe: json['likedByMe'] as bool? ?? false,
    replyCount: json['replyCount'] as int? ?? 0,
    replyToId: json['replyToId'] as String?,
    createdAt: json['createdAt'] as String? ?? '',
  );
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

  factory ComposeState.fromJson(Map<String, dynamic> json) => ComposeState(
    content: json['content'] as String? ?? '',
    replyToId: json['replyToId'] as String?,
    busy: json['busy'] as bool? ?? false,
    error: json['error'] as String?,
  );
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

  factory ProfilePage.fromJson(Map<String, dynamic> json) => ProfilePage(
    user: UserProfile.fromJson(json['user'] as Map<String, dynamic>),
    tweets:
        (json['tweets'] as List<dynamic>?)
            ?.map((e) => FeedItem.fromJson(e as Map<String, dynamic>))
            .toList() ??
        const [],
    followedByMe: json['followedByMe'] as bool? ?? false,
    loading: json['loading'] as bool? ?? false,
  );
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

  factory TweetDetailState.fromJson(Map<String, dynamic> json) =>
      TweetDetailState(
        tweet: FeedItem.fromJson(json['tweet'] as Map<String, dynamic>),
        replies:
            (json['replies'] as List<dynamic>?)
                ?.map((e) => FeedItem.fromJson(e as Map<String, dynamic>))
                .toList() ??
            const [],
        loading: json['loading'] as bool? ?? false,
      );
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

  factory SearchState.fromJson(Map<String, dynamic> json) => SearchState(
    query: json['query'] as String? ?? '',
    users:
        (json['users'] as List<dynamic>?)
            ?.map((e) => UserProfile.fromJson(e as Map<String, dynamic>))
            .toList() ??
        const [],
    tweets:
        (json['tweets'] as List<dynamic>?)
            ?.map((e) => FeedItem.fromJson(e as Map<String, dynamic>))
            .toList() ??
        const [],
    loading: json['loading'] as bool? ?? false,
    error: json['error'] as String?,
  );
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

  factory SettingsState.fromJson(Map<String, dynamic> json) => SettingsState(
    displayName: json['displayName'] as String? ?? '',
    bio: json['bio'] as String? ?? '',
    busy: json['busy'] as bool? ?? false,
    saved: json['saved'] as bool? ?? false,
    error: json['error'] as String?,
  );
}

// MARK: - settings/password

class PasswordState {
  final bool busy;
  final bool success;
  final String? error;

  const PasswordState({this.busy = false, this.success = false, this.error});

  factory PasswordState.fromJson(Map<String, dynamic> json) => PasswordState(
    busy: json['busy'] as bool? ?? false,
    success: json['success'] as bool? ?? false,
    error: json['error'] as String?,
  );
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

  factory InboxState.fromJson(Map<String, dynamic> json) => InboxState(
    messages:
        (json['messages'] as List<dynamic>?)
            ?.map((e) => InboxMessage.fromJson(e as Map<String, dynamic>))
            .toList() ??
        const [],
    unreadCount: json['unreadCount'] as int? ?? 0,
    loading: json['loading'] as bool? ?? false,
    error: json['error'] as String?,
  );
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

  factory InboxMessage.fromJson(Map<String, dynamic> json) => InboxMessage(
    id: json['id'] as String? ?? '',
    kind: json['kind'] as String? ?? '',
    title: json['title'] as String? ?? '',
    body: json['body'] as String? ?? '',
    read: json['read'] as bool? ?? false,
    createdAt: json['createdAt'] as String? ?? '',
  );
}
