// TwitterFluxApp — entry point.
// Flux owns all state. SwiftUI only renders.

import SwiftUI

#if !TESTING
@main
#endif
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

    private var isLoggedIn: Bool {
        (store.get("auth/state") as AuthState?)?.phase == .authenticated
    }

    var body: some View {
        Group {
            if !isLoggedIn {
                LoginView()
            } else {
                MainTabView()
            }
        }
    }
}

/// Main tab view — shown after login.
struct MainTabView: View {
    @EnvironmentObject var store: FluxStore
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            NavigationStack {
                HomeView()
            }
            .tabItem {
                Image(systemName: "house.fill")
                Text(store.t("ui/tab/home"))
            }
            .tag(0)

            NavigationStack {
                SearchView()
            }
            .tabItem {
                Image(systemName: "magnifyingglass")
                Text(store.t("ui/tab/search"))
            }
            .tag(1)

            NavigationStack {
                MeView()
            }
            .tabItem {
                Image(systemName: "person.fill")
                Text(store.t("ui/tab/me"))
            }
            .tag(2)
        }
    }
}
