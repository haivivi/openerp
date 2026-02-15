// FluxFFITests — Swift integration tests for the Flux FFI bridge.
//
// Tests the FULL path: Swift → C FFI → Rust BFF → state → C FFI → Swift.
// Runs on macOS host (no simulator needed).

import XCTest
@testable import TwitterFlux

final class FluxFFITests: XCTestCase {

    var store: FluxStore!

    override func setUp() {
        super.setUp()
        store = FluxStore()
    }

    override func tearDown() {
        store = nil
        super.tearDown()
    }

    // MARK: - Lifecycle

    func testInitialize() {
        store.emit("app/initialize")

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertNotNil(auth)
        XCTAssertEqual(auth?.phase, .unauthenticated)
        XCTAssertNil(auth?.user)
        XCTAssertFalse(auth?.busy ?? true)

        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/login")
    }

    // MARK: - Login

    func testLoginSuccess() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .authenticated)
        XCTAssertEqual(auth?.user?.username, "alice")
        XCTAssertEqual(auth?.user?.displayName, "Alice Wang")
        XCTAssertFalse(auth?.busy ?? true)

        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/home")
    }

    func testLoginFailure() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "nonexistent"])

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .unauthenticated)
        XCTAssertNotNil(auth?.error)
        XCTAssertTrue(auth?.error?.contains("not found") ?? false)
    }

    // MARK: - Timeline

    func testTimelineLoadedAfterLogin() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertNotNil(feed)
        XCTAssertFalse(feed?.items.isEmpty ?? true)
        XCTAssertFalse(feed?.loading ?? true)
    }

    func testTimelineItemsHaveAuthorInfo() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        guard let items = feed?.items, !items.isEmpty else {
            XCTFail("Timeline should have items")
            return
        }

        for item in items {
            XCTAssertFalse(item.author.username.isEmpty)
            XCTAssertFalse(item.tweetId.isEmpty)
            XCTAssertFalse(item.content.isEmpty)
        }
    }

    // MARK: - Tweet Create

    func testCreateTweet() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let feedBefore: TimelineFeed? = store.getSync("timeline/feed")
        let countBefore = feedBefore?.items.count ?? 0

        store.emit("tweet/create", json: ["content": "Hello from Swift tests!"])

        let feedAfter: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertEqual(feedAfter?.items.count, countBefore + 1)

        // Compose should be cleared.
        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertEqual(compose?.content, "")
        XCTAssertNil(compose?.error)
        XCTAssertFalse(compose?.busy ?? true)
    }

    func testCreateEmptyTweetRejected() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])
        store.emit("tweet/create", json: ["content": "   "])

        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertNotNil(compose?.error)
        XCTAssertTrue(compose?.error?.contains("empty") ?? false)
    }

    func testCreateLongTweetRejected() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let longContent = String(repeating: "x", count: 281)
        store.emit("tweet/create", json: ["content": longContent])

        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertNotNil(compose?.error)
        XCTAssertTrue(compose?.error?.contains("280") ?? false)
    }

    // MARK: - Like

    func testLikeAndUnlike() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        guard let tweetId = feed?.items.first?.tweetId else {
            XCTFail("Need at least one tweet")
            return
        }

        // Like.
        store.emit("tweet/like", json: ["tweetId": tweetId])
        let afterLike: TimelineFeed? = store.getSync("timeline/feed")
        let likedItem = afterLike?.items.first(where: { $0.tweetId == tweetId })
        XCTAssertEqual(likedItem?.likedByMe, true)
        XCTAssertEqual(likedItem?.likeCount, 1)

        // Unlike.
        store.emit("tweet/unlike", json: ["tweetId": tweetId])
        let afterUnlike: TimelineFeed? = store.getSync("timeline/feed")
        let unlikedItem = afterUnlike?.items.first(where: { $0.tweetId == tweetId })
        XCTAssertEqual(unlikedItem?.likedByMe, false)
        XCTAssertEqual(unlikedItem?.likeCount, 0)
    }

    // MARK: - Follow

    func testFollowAndUnfollow() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        // Follow bob.
        store.emit("user/follow", json: ["userId": "bob"])

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.user?.followingCount, 1)

        // Unfollow bob.
        store.emit("user/unfollow", json: ["userId": "bob"])

        let authAfter: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(authAfter?.user?.followingCount, 0)
    }

    // MARK: - Profile

    func testLoadProfile() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])
        store.emit("profile/load", json: ["userId": "bob"])

        let profile: ProfilePage? = store.getSync("profile/bob")
        XCTAssertNotNil(profile)
        XCTAssertEqual(profile?.user.username, "bob")
        XCTAssertEqual(profile?.user.displayName, "Bob Li")

        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/profile/bob")
    }

    // MARK: - Logout

    func testLogout() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])
        XCTAssertEqual(
            (store.getSync("auth/state") as AuthState?)?.phase,
            .authenticated
        )

        store.emit("auth/logout")

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .unauthenticated)

        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/login")

        // Timeline cleared.
        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertNil(feed)
    }

    // MARK: - Full Flow

    func testFullFlow() {
        // Initialize.
        store.emit("app/initialize")
        XCTAssertEqual(
            (store.getSync("auth/state") as AuthState?)?.phase,
            .unauthenticated
        )

        // Login.
        store.emit("auth/login", json: ["username": "alice"])
        XCTAssertEqual(
            (store.getSync("auth/state") as AuthState?)?.phase,
            .authenticated
        )
        XCTAssertEqual(
            (store.getSync("app/route") as AppRoute?)?.path,
            "/home"
        )

        // Post tweet.
        store.emit("tweet/create", json: ["content": "Full flow test!"])
        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertTrue(feed?.items.contains(where: { $0.content == "Full flow test!" }) ?? false)

        // Like.
        if let id = feed?.items.first(where: { $0.content == "Full flow test!" })?.tweetId {
            store.emit("tweet/like", json: ["tweetId": id])
            let afterLike: TimelineFeed? = store.getSync("timeline/feed")
            XCTAssertEqual(
                afterLike?.items.first(where: { $0.tweetId == id })?.likedByMe,
                true
            )
        }

        // Follow.
        store.emit("user/follow", json: ["userId": "bob"])
        XCTAssertEqual(
            (store.getSync("auth/state") as AuthState?)?.user?.followingCount,
            1
        )

        // View profile.
        store.emit("profile/load", json: ["userId": "bob"])
        let profile: ProfilePage? = store.getSync("profile/bob")
        XCTAssertEqual(profile?.followedByMe, true)

        // Logout.
        store.emit("auth/logout")
        XCTAssertEqual(
            (store.getSync("auth/state") as AuthState?)?.phase,
            .unauthenticated
        )
    }
}
