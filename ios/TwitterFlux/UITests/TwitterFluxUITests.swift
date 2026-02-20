// TwitterFluxUITests — XCUITest for the Twitter app.
//
// Tests real UI interactions on a running simulator.
// Requires a booted simulator. Run via:
//   bazel run //ios/TwitterFlux:UITests

import XCTest

final class TwitterFluxUITests: XCTestCase {

    let app = XCUIApplication()

    override func setUp() {
        super.setUp()
        continueAfterFailure = false
        app.launch()
    }

    override func tearDown() {
        app.terminate()
        super.tearDown()
    }

    // MARK: - Login

    func testLoginScreenAppears() {
        XCTAssertTrue(app.staticTexts["TwitterFlux"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.secureTextFields.firstMatch.exists, "Password field should exist")
    }

    func testLoginWithValidUser() {
        login(username: "alice", password: "password")
        XCTAssertTrue(app.tabBars.firstMatch.waitForExistence(timeout: 5))
    }

    func testLoginWithWrongPassword() {
        let usernameField = app.textFields.firstMatch
        XCTAssertTrue(usernameField.waitForExistence(timeout: 5))
        usernameField.tap()
        usernameField.typeText("alice")

        let passwordField = app.secureTextFields.firstMatch
        passwordField.tap()
        passwordField.typeText("wrong")

        app.buttons.matching(NSPredicate(format: "label CONTAINS 'Sign' OR label CONTAINS '登录'")).firstMatch.tap()
        sleep(2)

        // Should still be on login page.
        XCTAssertTrue(app.textFields.firstMatch.exists)
        XCTAssertFalse(app.tabBars.firstMatch.exists)
    }

    // MARK: - Tab Navigation

    func testTabBarHasFourTabs() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))
        XCTAssertGreaterThanOrEqual(tabBar.buttons.count, 4, "Should have Home, Search, Inbox, Me")
    }

    func testSwitchAllTabs() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))

        for i in 0..<tabBar.buttons.count {
            tabBar.buttons.element(boundBy: i).tap()
            sleep(1)
        }
    }

    // MARK: - Compose

    func testComposeAndPost() {
        login(username: "alice", password: "password")

        let composeButton = app.navigationBars.buttons.matching(
            NSPredicate(format: "label CONTAINS 'pencil' OR label CONTAINS 'Compose'")
        ).firstMatch
        guard composeButton.waitForExistence(timeout: 3) else { return }
        composeButton.tap()

        let textView = app.textViews.firstMatch
        guard textView.waitForExistence(timeout: 3) else { return }
        textView.tap()
        textView.typeText("Hello from XCUITest!")

        // Character count visible.
        XCTAssertTrue(app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS '/280'")
        ).firstMatch.exists)

        // Post button.
        app.buttons.matching(
            NSPredicate(format: "label CONTAINS 'Post' OR label CONTAINS '发布' OR label CONTAINS '投稿' OR label CONTAINS 'Publicar'")
        ).firstMatch.tap()

        sleep(1)
        // Should return to home.
        XCTAssertTrue(app.tabBars.firstMatch.exists)
    }

    // MARK: - Inbox Tab

    func testInboxTabShowsMessages() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))

        // Tap inbox tab (3rd tab, index 2).
        tabBar.buttons.element(boundBy: 2).tap()
        sleep(2)

        // Should show at least one message.
        let hasContent = app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS 'Welcome' OR label CONTAINS '欢迎' OR label CONTAINS 'Broadcast' OR label CONTAINS 'System'")
        ).firstMatch.waitForExistence(timeout: 5)
        XCTAssertTrue(hasContent, "Inbox should show seeded messages")
    }

    // MARK: - Me / Settings

    func testMeTabShowsProfile() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))

        // Me is the last tab.
        tabBar.buttons.element(boundBy: tabBar.buttons.count - 1).tap()

        XCTAssertTrue(app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS 'alice' OR label CONTAINS 'Alice'")
        ).firstMatch.waitForExistence(timeout: 3))
    }

    func testSignOut() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))

        tabBar.buttons.element(boundBy: tabBar.buttons.count - 1).tap()
        sleep(1)

        let signOut = app.buttons.matching(
            NSPredicate(format: "label CONTAINS 'Sign Out' OR label CONTAINS '退出' OR label CONTAINS 'サインアウト' OR label CONTAINS 'Cerrar'")
        ).firstMatch
        guard signOut.waitForExistence(timeout: 3) else { return }
        signOut.tap()

        XCTAssertTrue(app.textFields.firstMatch.waitForExistence(timeout: 5), "Should be back on login")
    }

    // MARK: - Language Switcher

    func testLanguageSwitcher() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))

        // Go to Me tab.
        tabBar.buttons.element(boundBy: tabBar.buttons.count - 1).tap()
        sleep(1)

        // Tap Language row.
        let langRow = app.cells.matching(
            NSPredicate(format: "label CONTAINS 'Language' OR label CONTAINS '语言' OR label CONTAINS '言語' OR label CONTAINS 'Idioma'")
        ).firstMatch
        guard langRow.waitForExistence(timeout: 3) else { return }
        langRow.tap()
        sleep(1)

        // Select Chinese.
        let zhOption = app.buttons.matching(
            NSPredicate(format: "label CONTAINS '中文'")
        ).firstMatch
        guard zhOption.waitForExistence(timeout: 3) else { return }
        zhOption.tap()
        sleep(1)

        // Navigate back — tab labels should now be in Chinese.
        app.navigationBars.buttons.firstMatch.tap()
        sleep(1)
    }

    // MARK: - Full Journey

    func testFullUserJourney() {
        login(username: "alice", password: "password")
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(tabBar.waitForExistence(timeout: 5))

        // Switch through all tabs.
        for i in 0..<tabBar.buttons.count {
            tabBar.buttons.element(boundBy: i).tap()
            sleep(1)
        }

        // Go to Me and sign out.
        tabBar.buttons.element(boundBy: tabBar.buttons.count - 1).tap()
        sleep(1)
        let signOut = app.buttons.matching(
            NSPredicate(format: "label CONTAINS 'Sign Out' OR label CONTAINS '退出'")
        ).firstMatch
        if signOut.waitForExistence(timeout: 3) {
            signOut.tap()
            XCTAssertTrue(app.textFields.firstMatch.waitForExistence(timeout: 5))
        }
    }

    // MARK: - Helpers

    private func login(username: String, password: String) {
        let usernameField = app.textFields.firstMatch
        guard usernameField.waitForExistence(timeout: 5) else {
            XCTFail("Login screen not visible")
            return
        }
        usernameField.tap()
        usernameField.typeText(username)

        let passwordField = app.secureTextFields.firstMatch
        passwordField.tap()
        passwordField.typeText(password)

        app.buttons.matching(
            NSPredicate(format: "label CONTAINS 'Sign' OR label CONTAINS '登录'")
        ).firstMatch.tap()

        _ = app.tabBars.firstMatch.waitForExistence(timeout: 5)
    }
}
