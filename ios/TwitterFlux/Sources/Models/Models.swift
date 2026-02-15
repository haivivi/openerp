// Twitter state models — Swift mirrors of Rust #[state] types.
// Golden test: hand-written. Production: auto-generated from #[state] definitions.

import Foundation

// MARK: - auth/state

struct AuthState: Codable {
    let phase: AuthPhase
    let user: UserProfile?
    let busy: Bool
    let error: String?
}

enum AuthPhase: String, Codable {
    case unauthenticated
    case authenticated
}

struct UserProfile: Codable {
    let id: String
    let username: String
    let displayName: String
    let bio: String?
    let avatar: String?
    let followerCount: Int
    let followingCount: Int
    let tweetCount: Int
}

// MARK: - timeline/feed

struct TimelineFeed: Codable {
    let items: [FeedItem]
    let loading: Bool
    let hasMore: Bool
    let error: String?
}

struct FeedItem: Codable, Identifiable {
    let tweetId: String
    let author: UserProfile
    let content: String
    let likeCount: Int
    let likedByMe: Bool
    let replyCount: Int
    let replyToId: String?
    let createdAt: String

    var id: String { tweetId }
}

// MARK: - compose/state

struct ComposeState: Codable {
    let content: String
    let replyToId: String?
    let busy: Bool
    let error: String?
}

// MARK: - profile/{id}

struct ProfilePage: Codable {
    let user: UserProfile
    let tweets: [FeedItem]
    let followedByMe: Bool
    let loading: Bool
}

// MARK: - tweet/{id}

struct TweetDetailState: Codable {
    let tweet: FeedItem
    let replies: [FeedItem]
    let loading: Bool
}

// MARK: - search/state

struct SearchState: Codable {
    let query: String
    let users: [UserProfile]
    let tweets: [FeedItem]
    let loading: Bool
    let error: String?
}

// MARK: - settings/state

struct SettingsState: Codable {
    let displayName: String
    let bio: String
    let busy: Bool
    let saved: Bool
    let error: String?
}

// MARK: - settings/password

struct PasswordState: Codable {
    let busy: Bool
    let success: Bool
    let error: String?
}

// MARK: - app/route

struct AppRoute: Codable {
    let path: String

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        path = try container.decode(String.self)
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(path)
    }
}
