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
        store.emit("app/initialize")
    }

    override func tearDown() {
        store = nil
        super.tearDown()
    }

    private func login(username: String = "alice", password: String = "password") {
        store.emit("auth/login", json: ["username": username, "password": password])
    }

    // MARK: - Lifecycle

    func testInitialize() {
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
        login()

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .authenticated)
        XCTAssertEqual(auth?.user?.username, "alice")
        XCTAssertFalse(auth?.busy ?? true)
        XCTAssertNil(auth?.error)

        let route: AppRoute? = store.getSync("app/route")
        XCTAssertEqual(route?.path, "/home")
    }

    func testLoginUnknownUser() {
        login(username: "nonexistent")

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .unauthenticated)
        XCTAssertNotNil(auth?.error)
    }

    func testLoginWrongPassword() {
        login(username: "alice", password: "wrong")

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .unauthenticated)
        XCTAssertNotNil(auth?.error)
    }

    func testLoginWithAllUsers() {
        for user in ["alice", "bob", "carol"] {
            let s = FluxStore()
            s.emit("app/initialize")
            s.emit("auth/login", json: ["username": user, "password": "password"])
            let auth: AuthState? = s.getSync("auth/state")
            XCTAssertEqual(auth?.phase, .authenticated, "\(user) should login successfully")
            XCTAssertEqual(auth?.user?.username, user)
        }
    }

    // MARK: - Timeline

    func testTimelineLoadedAfterLogin() {
        login()

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertNotNil(feed)
        XCTAssertFalse(feed?.loading ?? true)
        XCTAssertGreaterThan(feed?.items.count ?? 0, 0, "Seed data should produce tweets")
    }

    func testTimelineRefresh() {
        login()
        store.emit("timeline/load")

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertNotNil(feed)
        XCTAssertFalse(feed?.loading ?? true)
    }

    func testTimelineExcludesReplies() {
        login()

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        let hasReply = feed?.items.contains(where: { $0.replyToId != nil }) ?? false
        XCTAssertFalse(hasReply, "Timeline should only show top-level tweets, not replies")
    }

    // MARK: - Create Tweet

    func testCreateTweet() {
        login()

        let feedBefore: TimelineFeed? = store.getSync("timeline/feed")
        let countBefore = feedBefore?.items.count ?? 0

        store.emit("tweet/create", json: ["content": "Hello from Swift E2E!"])

        let feedAfter: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertEqual(feedAfter?.items.count, countBefore + 1)

        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertEqual(compose?.content, "")
        XCTAssertNil(compose?.error)
    }

    func testCreateEmptyTweetRejected() {
        login()
        store.emit("tweet/create", json: ["content": "   "])

        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertNotNil(compose?.error)
    }

    func testCreateLongTweetRejected() {
        login()
        store.emit("tweet/create", json: ["content": String(repeating: "x", count: 281)])

        let compose: ComposeState? = store.getSync("compose/state")
        XCTAssertNotNil(compose?.error)
    }

    func testCreateReply() {
        login()
        store.emit("tweet/create", json: ["content": "Parent tweet"])

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        guard let parentId = feed?.items.first(where: { $0.content == "Parent tweet" })?.tweetId else {
            XCTFail("Parent tweet not found")
            return
        }

        store.emit("tweet/create", json: ["content": "Reply!", "replyToId": parentId])

        store.emit("tweet/load", json: ["tweetId": parentId])
        let detail: TweetDetailState? = store.getSync("tweet/\(parentId)")
        XCTAssertNotNil(detail)
        XCTAssertGreaterThan(detail?.replies.count ?? 0, 0)
    }

    // MARK: - Like / Unlike

    func testLikeAndUnlike() {
        login()
        store.emit("tweet/create", json: ["content": "Like me!"])

        let feed: TimelineFeed? = store.getSync("timeline/feed")
        guard let tweetId = feed?.items.first(where: { $0.content == "Like me!" })?.tweetId else {
            XCTFail("Tweet not found")
            return
        }

        store.emit("tweet/like", json: ["tweetId": tweetId])
        let afterLike: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertEqual(afterLike?.items.first(where: { $0.tweetId == tweetId })?.likedByMe, true)

        store.emit("tweet/unlike", json: ["tweetId": tweetId])
        let afterUnlike: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertEqual(afterUnlike?.items.first(where: { $0.tweetId == tweetId })?.likedByMe, false)
    }

    // MARK: - Follow / Unfollow

    func testFollowAndUnfollow() {
        login()

        store.emit("user/follow", json: ["userId": "bob"])
        store.emit("profile/load", json: ["userId": "bob"])
        let profileAfterFollow: ProfilePage? = store.getSync("profile/bob")
        XCTAssertEqual(profileAfterFollow?.followedByMe, true)

        store.emit("user/unfollow", json: ["userId": "bob"])
        store.emit("profile/load", json: ["userId": "bob"])
        let profileAfterUnfollow: ProfilePage? = store.getSync("profile/bob")
        XCTAssertEqual(profileAfterUnfollow?.followedByMe, false)
    }

    // MARK: - Profile

    func testLoadProfile() {
        login()
        store.emit("profile/load", json: ["userId": "bob"])

        let profile: ProfilePage? = store.getSync("profile/bob")
        XCTAssertNotNil(profile)
        XCTAssertEqual(profile?.user.username, "bob")
    }

    func testLoadOwnProfile() {
        login()
        store.emit("profile/load", json: ["userId": "alice"])

        let profile: ProfilePage? = store.getSync("profile/alice")
        XCTAssertNotNil(profile)
        XCTAssertEqual(profile?.user.username, "alice")
    }

    // MARK: - Logout

    func testLogout() {
        login()
        XCTAssertEqual((store.getSync("auth/state") as AuthState?)?.phase, .authenticated)

        store.emit("auth/logout")

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.phase, .unauthenticated)
        XCTAssertEqual((store.getSync("app/route") as AppRoute?)?.path, "/login")
        XCTAssertNil(store.getSync("timeline/feed") as TimelineFeed?)
    }

    // MARK: - Search

    func testSearchUsers() {
        login()
        store.emit("search/query", json: ["query": "alice"])

        let search: SearchState? = store.getSync("search/state")
        XCTAssertNotNil(search)
        XCTAssertFalse(search?.loading ?? true)
        XCTAssertFalse(search?.users.isEmpty ?? true)
    }

    func testSearchTweets() {
        login()
        store.emit("tweet/create", json: ["content": "unique_search_term_xyz"])
        store.emit("search/query", json: ["query": "unique_search_term_xyz"])

        let search: SearchState? = store.getSync("search/state")
        XCTAssertFalse(search?.tweets.isEmpty ?? true)
    }

    func testSearchClear() {
        login()
        store.emit("search/query", json: ["query": "alice"])
        store.emit("search/clear")

        XCTAssertNil(store.getSync("search/state") as SearchState?)
    }

    // MARK: - Settings

    func testSettingsLoad() {
        login()
        store.emit("settings/load")

        let settings: SettingsState? = store.getSync("settings/state")
        XCTAssertNotNil(settings)
        XCTAssertFalse(settings?.displayName.isEmpty ?? true)
    }

    func testSettingsSaveProfile() {
        login()
        store.emit("settings/load")
        store.emit("settings/save", json: ["displayName": "Alice Updated", "bio": "New bio"])

        let settings: SettingsState? = store.getSync("settings/state")
        XCTAssertEqual(settings?.saved, true)
        XCTAssertNil(settings?.error)

        let auth: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(auth?.user?.displayName, "Alice Updated")
    }

    func testSettingsSaveEmptyNameRejected() {
        login()
        store.emit("settings/save", json: ["displayName": "  ", "bio": ""])

        let settings: SettingsState? = store.getSync("settings/state")
        XCTAssertNotNil(settings?.error)
        XCTAssertEqual(settings?.saved, false)
    }

    // MARK: - Change Password

    func testChangePassword() {
        login()
        store.emit("settings/change-password", json: [
            "oldPassword": "password",
            "newPassword": "newpass123",
        ])

        let pw: PasswordState? = store.getSync("settings/password")
        XCTAssertEqual(pw?.success, true)
        XCTAssertNil(pw?.error)
    }

    func testChangePasswordWrongOld() {
        login()
        store.emit("settings/change-password", json: [
            "oldPassword": "wrong",
            "newPassword": "newpass123",
        ])

        let pw: PasswordState? = store.getSync("settings/password")
        XCTAssertEqual(pw?.success, false)
        XCTAssertNotNil(pw?.error)
    }

    func testChangePasswordThenRelogin() {
        login()
        store.emit("settings/change-password", json: [
            "oldPassword": "password",
            "newPassword": "changed1",
        ])
        store.emit("auth/logout")

        // Old password fails.
        login(username: "alice", password: "password")
        let authFail: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(authFail?.phase, .unauthenticated)

        // New password works.
        login(username: "alice", password: "changed1")
        let authOk: AuthState? = store.getSync("auth/state")
        XCTAssertEqual(authOk?.phase, .authenticated)
    }

    // MARK: - Inbox (站内信)

    func testInboxLoad() {
        login()
        store.emit("inbox/load")

        let inbox: InboxState? = store.getSync("inbox/state")
        XCTAssertNotNil(inbox)
        XCTAssertFalse(inbox?.loading ?? true)
        XCTAssertGreaterThan(inbox?.messages.count ?? 0, 0, "Seed data should have messages")
        XCTAssertGreaterThan(inbox?.unreadCount ?? 0, 0)
    }

    func testInboxMarkRead() {
        login()
        store.emit("inbox/load")

        let inbox: InboxState? = store.getSync("inbox/state")
        guard let msgId = inbox?.messages.first(where: { !$0.read })?.id else {
            XCTFail("No unread messages")
            return
        }
        let unreadBefore = inbox?.unreadCount ?? 0

        store.emit("inbox/mark-read", json: ["messageId": msgId])

        let after: InboxState? = store.getSync("inbox/state")
        XCTAssertEqual(after?.unreadCount, unreadBefore - 1)
        XCTAssertTrue(after?.messages.first(where: { $0.id == msgId })?.read ?? false)
    }

    func testInboxAliceSeesPersonalMessage() {
        login(username: "alice")
        store.emit("inbox/load")

        let inbox: InboxState? = store.getSync("inbox/state")
        let hasPersonal = inbox?.messages.contains(where: { $0.kind == "personal" }) ?? false
        XCTAssertTrue(hasPersonal, "Alice should see personal message")
    }

    func testInboxBobSeesOnlyBroadcast() {
        login(username: "bob")
        store.emit("inbox/load")

        let inbox: InboxState? = store.getSync("inbox/state")
        let hasPersonal = inbox?.messages.contains(where: { $0.kind == "personal" }) ?? false
        XCTAssertFalse(hasPersonal, "Bob should not see alice's personal message")
    }

    // MARK: - I18n

    func testI18nEnglishDefault() {
        XCTAssertEqual(store.t("ui/login/button"), "Sign In")
        XCTAssertEqual(store.t("ui/tab/home"), "Home")
        XCTAssertEqual(store.t("ui/me/sign_out"), "Sign Out")
    }

    func testI18nChinese() {
        store.setLocale("zh-CN")
        XCTAssertEqual(store.t("ui/login/button"), "登录")
        XCTAssertEqual(store.t("ui/tab/home"), "首页")
        XCTAssertEqual(store.t("ui/compose/placeholder"), "有什么新鲜事？")
    }

    func testI18nJapanese() {
        store.setLocale("ja")
        XCTAssertEqual(store.t("ui/login/button"), "サインイン")
        XCTAssertEqual(store.t("ui/compose/placeholder"), "いまどうしてる？")
    }

    func testI18nSpanish() {
        store.setLocale("es")
        XCTAssertEqual(store.t("ui/login/button"), "Iniciar sesión")
        XCTAssertEqual(store.t("ui/tab/search"), "Buscar")
    }

    func testI18nFormatWithParams() {
        XCTAssertEqual(store.t("format/like_count?count=42"), "42 likes")
        store.setLocale("zh-CN")
        XCTAssertEqual(store.t("format/like_count?count=42"), "42 人赞了")
    }

    func testI18nUnknownKeyReturnsPath() {
        XCTAssertEqual(store.t("ui/nonexistent/key"), "ui/nonexistent/key")
    }

    func testI18nLocaleSwitchUpdatesAll() {
        XCTAssertEqual(store.t("ui/me/sign_out"), "Sign Out")
        store.setLocale("zh-CN")
        XCTAssertEqual(store.t("ui/me/sign_out"), "退出登录")
        store.setLocale("ja")
        XCTAssertEqual(store.t("ui/me/sign_out"), "サインアウト")
        store.setLocale("es")
        XCTAssertEqual(store.t("ui/me/sign_out"), "Cerrar sesión")
        store.setLocale("en")
        XCTAssertEqual(store.t("ui/me/sign_out"), "Sign Out")
    }

    // MARK: - Server URL

    func testServerURLAvailable() {
        let url = store.serverURL
        XCTAssertFalse(url.isEmpty)
        XCTAssertTrue(url.hasPrefix("http://"))
    }

    // MARK: - Full Flow

    func testFullFlow() {
        // Login.
        login()
        XCTAssertEqual((store.getSync("auth/state") as AuthState?)?.phase, .authenticated)

        // Post tweet.
        store.emit("tweet/create", json: ["content": "Full flow test!"])
        let feed: TimelineFeed? = store.getSync("timeline/feed")
        XCTAssertTrue(feed?.items.contains(where: { $0.content == "Full flow test!" }) ?? false)

        // Like.
        if let id = feed?.items.first(where: { $0.content == "Full flow test!" })?.tweetId {
            store.emit("tweet/like", json: ["tweetId": id])
        }

        // Follow.
        store.emit("user/follow", json: ["userId": "bob"])

        // Profile.
        store.emit("profile/load", json: ["userId": "bob"])
        XCTAssertNotNil(store.getSync("profile/bob") as ProfilePage?)

        // Inbox.
        store.emit("inbox/load")
        XCTAssertNotNil(store.getSync("inbox/state") as InboxState?)

        // Search.
        store.emit("search/query", json: ["query": "alice"])
        XCTAssertNotNil(store.getSync("search/state") as SearchState?)

        // Logout.
        store.emit("auth/logout")
        XCTAssertEqual((store.getSync("auth/state") as AuthState?)?.phase, .unauthenticated)
    }
}
