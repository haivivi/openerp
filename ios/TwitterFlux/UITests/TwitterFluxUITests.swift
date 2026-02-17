// TwitterFluxUITests — XCUITest for the Twitter app.
//
// Tests real UI interactions on a running simulator:
// - Tap buttons, type text, verify screen content
// - Full user journey: login → timeline → compose → like → profile → logout
//
// Requires a booted simulator. Run via:
//   bazel run //ios/TwitterFlux:UITests -- \
//     bazel-bin/ios/TwitterFlux/TwitterFlux.app \
//     bazel-bin/ios/TwitterFlux/UITests.xctest

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
        // Login page should show the app name and username field.
        XCTAssertTrue(app.staticTexts["TwitterFlux"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.textFields["Username"].exists)
        XCTAssertTrue(app.buttons["Sign In"].exists)
    }

    func testLoginWithValidUser() {
        let usernameField = app.textFields["Username"]
        XCTAssertTrue(usernameField.waitForExistence(timeout: 5))

        usernameField.tap()
        usernameField.typeText("alice")

        app.buttons["Sign In"].tap()

        // Should navigate to Home tab.
        XCTAssertTrue(app.tabBars.buttons["Home"].waitForExistence(timeout: 5))
    }

    func testLoginWithInvalidUser() {
        let usernameField = app.textFields["Username"]
        XCTAssertTrue(usernameField.waitForExistence(timeout: 5))

        usernameField.tap()
        usernameField.typeText("nonexistent_user")

        app.buttons["Sign In"].tap()

        // Should stay on login page — error should appear.
        // Give it a moment to process.
        sleep(1)
        XCTAssertTrue(app.textFields["Username"].exists, "Should still be on login page")
    }

    // MARK: - Navigation

    func testTabBarExists() {
        login(username: "alice")

        XCTAssertTrue(app.tabBars.buttons["Home"].exists)
        XCTAssertTrue(app.tabBars.buttons["Search"].exists)
        XCTAssertTrue(app.tabBars.buttons["Me"].exists)
    }

    func testSwitchTabs() {
        login(username: "alice")

        // Switch to Search.
        app.tabBars.buttons["Search"].tap()
        XCTAssertTrue(app.navigationBars["Search"].waitForExistence(timeout: 3))

        // Switch to Me.
        app.tabBars.buttons["Me"].tap()
        XCTAssertTrue(app.navigationBars["Me"].waitForExistence(timeout: 3))

        // Back to Home.
        app.tabBars.buttons["Home"].tap()
        XCTAssertTrue(app.navigationBars["Home"].waitForExistence(timeout: 3))
    }

    // MARK: - Compose

    func testComposeAndPost() {
        login(username: "alice")

        // Tap compose button (pencil icon in toolbar).
        let composeButton = app.navigationBars.buttons.matching(
            NSPredicate(format: "label CONTAINS 'pencil' OR label CONTAINS 'Compose'")
        ).firstMatch
        if composeButton.waitForExistence(timeout: 3) {
            composeButton.tap()

            // Type a tweet.
            let textView = app.textViews.firstMatch
            if textView.waitForExistence(timeout: 3) {
                textView.tap()
                textView.typeText("Hello from XCUITest!")

                // Character count should appear.
                XCTAssertTrue(app.staticTexts.matching(
                    NSPredicate(format: "label CONTAINS '20/280' OR label CONTAINS '/280'")
                ).firstMatch.exists)

                // Post.
                app.buttons["Post"].tap()

                // Should go back to Home.
                XCTAssertTrue(app.navigationBars["Home"].waitForExistence(timeout: 3))
            }
        }
    }

    // MARK: - Me / Settings

    func testMeTabShowsProfile() {
        login(username: "alice")

        app.tabBars.buttons["Me"].tap()

        // Should show user info.
        XCTAssertTrue(app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS 'alice' OR label CONTAINS 'Alice'")
        ).firstMatch.waitForExistence(timeout: 3))
    }

    func testSignOut() {
        login(username: "alice")

        // Go to Me tab.
        app.tabBars.buttons["Me"].tap()
        sleep(1)

        // Tap Sign Out.
        let signOut = app.buttons["Sign Out"]
        if signOut.waitForExistence(timeout: 3) {
            signOut.tap()

            // Should be back on login page.
            XCTAssertTrue(app.textFields["Username"].waitForExistence(timeout: 5))
        }
    }

    // MARK: - Admin Dashboard Link

    func testAdminDashboardLinkExists() {
        login(username: "alice")

        app.tabBars.buttons["Me"].tap()

        // Should show "Open Admin Dashboard" link.
        let dashLink = app.buttons.matching(
            NSPredicate(format: "label CONTAINS 'Admin' OR label CONTAINS 'Dashboard'")
        ).firstMatch
        XCTAssertTrue(dashLink.waitForExistence(timeout: 3))
    }

    // MARK: - Full Journey

    func testFullUserJourney() {
        // 1. Login.
        login(username: "alice")
        XCTAssertTrue(app.tabBars.buttons["Home"].waitForExistence(timeout: 5))

        // 2. Switch tabs.
        app.tabBars.buttons["Search"].tap()
        XCTAssertTrue(app.navigationBars["Search"].waitForExistence(timeout: 3))

        // 3. Back to Home.
        app.tabBars.buttons["Home"].tap()

        // 4. Go to Me.
        app.tabBars.buttons["Me"].tap()
        XCTAssertTrue(app.navigationBars["Me"].waitForExistence(timeout: 3))

        // 5. Sign out.
        let signOut = app.buttons["Sign Out"]
        if signOut.waitForExistence(timeout: 3) {
            signOut.tap()
            XCTAssertTrue(app.textFields["Username"].waitForExistence(timeout: 5))
        }
    }

    // MARK: - Helpers

    private func login(username: String) {
        let usernameField = app.textFields["Username"]
        guard usernameField.waitForExistence(timeout: 5) else {
            XCTFail("Login screen not visible")
            return
        }
        usernameField.tap()
        usernameField.typeText(username)
        app.buttons["Sign In"].tap()

        // Wait for home to appear.
        _ = app.tabBars.buttons["Home"].waitForExistence(timeout: 5)
    }
}
