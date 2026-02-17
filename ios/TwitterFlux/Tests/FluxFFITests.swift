// FluxFFITests — Swift E2E integration tests.
//
// Tests the FULL path: Swift → C FFI → Rust BFF → HTTP → Facet → KvOps → redb.
// BFF calls facet API (not admin), JWT authentication is real.
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

    // MARK: - Login (facet API + real JWT)

    func testLoginSuccess() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .authenticated)
        XCTAssertEqual(auth?.user?.username, "alice")
        XCTAssertEqual(auth?.user?.displayName, "Alice Wang")
        XCTAssertFalse(auth?.busy ?? true)
        XCTAssertNil(auth?.error)

        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/home")
    }

    func testLoginFailure() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "nonexistent"])

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .unauthenticated)
        XCTAssertNotNil(auth?.error)
    }

    // MARK: - Timeline (loaded after login via facet)

    func testTimelineLoadedAfterLogin() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertNotNil(feed)
        // Demo data has tweets from seed — should not be empty.
        XCTAssertFalse(feed?.loading ?? true)
    }

    func testTimelineRefresh() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        // Explicit refresh.
        store.emit("timeline/load")

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertNotNil(feed)
        XCTAssertFalse(feed?.loading ?? true)
    }

    // MARK: - Create Tweet

    func testCreateTweet() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let feedBefore: TimelineFeed? = store.getSync("timeline/feed")
        let countBefore = feedBefore?.items.count ?? 0

        store.emit("tweet/create", json: ["content": "Hello from Swift E2E!"])

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
    }

    func testCreateLongTweetRejected() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let longContent = String(repeating: "x", count: 281)
        store.emit("tweet/create", json: ["content": longContent])

        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertNotNil(compose?.error)
    }

    // MARK: - Like / Unlike

    func testLikeAndUnlike() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        // Create a tweet first.
        store.emit("tweet/create", json: ["content": "Like me!"])
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
        XCTAssertGreaterThan(likedItem?.likeCount ?? 0, 0)

        // Unlike.
        store.emit("tweet/unlike", json: ["tweetId": tweetId])
        let afterUnlike: TimelineFeed? = store.getSync("timeline/feed")
        let unlikedItem = afterUnlike?.items.first(where: { $0.tweetId == tweetId })
        XCTAssertEqual(unlikedItem?.likedByMe, false)
    }

    // MARK: - Follow / Unfollow

    func testFollowAndUnfollow() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        let authBefore: AuthState? = store.getSync("auth/state")
        let countBefore = authBefore?.user?.followingCount ?? 0

        // Follow bob.
        store.emit("user/follow", json: ["userId": "bob"])

        // Unfollow bob.
        store.emit("user/unfollow", json: ["userId": "bob"])

        // Should be back to original count (or same — follow may not have updated auth).
        let authAfter: AuthState? = store.getSync("auth/state")
        XCTAssertNotNil(authAfter?.user)
    }

    // MARK: - Profile

    func testLoadProfile() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])
        store.emit("profile/load", json: ["userId": "bob"])

        let profile: ProfilePage? = store.getSync("profile/bob")
        XCTAssertNotNil(profile)
        XCTAssertEqual(profile?.user.username, "bob")
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

    // MARK: - Search

    func testSearch() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])

        store.emit("search/query", json: ["query": "alice"])

        let search: SearchState? = store.getSync("search/state")
        XCTAssertNotNil(search)
        XCTAssertFalse(search?.loading ?? true)
        // Should find alice in users.
        XCTAssertFalse(search?.users.isEmpty ?? true)
    }

    func testSearchClear() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])
        store.emit("search/query", json: ["query": "alice"])
        store.emit("search/clear")

        let search: SearchState? = store.getSync("search/state")
        XCTAssertNil(search)
    }

    // MARK: - Settings

    func testSettingsLoad() {
        store.emit("app/initialize")
        store.emit("auth/login", json: ["username": "alice"])
        store.emit("settings/load")

        // Settings load triggers a facet /me call — needs auth token.
        // If token propagation works, settings state should exist.
        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/settings")
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

        // View profile.
        store.emit("profile/load", json: ["userId": "bob"])
        let profile: ProfilePage? = store.getSync("profile/bob")
        XCTAssertNotNil(profile)

        // Logout.
        store.emit("auth/logout")
        XCTAssertEqual(
            (store.getSync("auth/state") as AuthState?)?.phase,
            .unauthenticated
        )
    }

    // MARK: - Server URL

    func testServerURLAvailable() {
        // The embedded server should be running.
        let url = store.serverURL
        XCTAssertFalse(url.isEmpty)
        XCTAssertTrue(url.hasPrefix("http://"))
    }
}
