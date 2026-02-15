// TwitterFluxApp — entry point.
// Flux owns all state. SwiftUI only renders.

import SwiftUI

@main
struct TwitterFluxApp: App {
    @StateObject private var store = FluxStore()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(store)
                .onAppear {
                    store.emit("app/initialize")
                }
        }
    }
}

/// Root view — routes based on `app/route` state.
struct RootView: View {
    @EnvironmentObject var store: FluxStore

    private var route: String {
        (store.get("app/route") as AppRoute?)?.path ?? "/login"
    }

    var body: some View {
        Group {
            switch routePrefix {
            case "/login":
                LoginView()
            case "/home":
                HomeView()
            case "/profile":
                if let userId = routeParam {
                    NavigationStack {
                        ProfileView(userId: userId)
                    }
                }
            case "/tweet":
                if let tweetId = routeParam {
                    NavigationStack {
                        TweetDetailView(tweetId: tweetId)
                    }
                }
            default:
                LoginView()
            }
        }
        .animation(.default, value: route)
    }

    /// First path component: "/home", "/login", "/profile", "/tweet"
    private var routePrefix: String {
        let parts = route.split(separator: "/", maxSplits: 2)
        return parts.first.map { "/\($0)" } ?? "/login"
    }

    /// Second path component (for /profile/{id}, /tweet/{id}).
    private var routeParam: String? {
        let parts = route.split(separator: "/", maxSplits: 2)
        return parts.count > 1 ? String(parts[1]) : nil
    }
}
