// MainTabView (iOS) — system tab bar implementation.

import SwiftUI

#if !targetEnvironment(macCatalyst)
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
                InboxView()
            }
            .tabItem {
                Image(systemName: "tray.fill")
                Text(store.t("ui/tab/inbox"))
            }
            .tag(2)

            NavigationStack {
                MeView()
            }
            .tabItem {
                Image(systemName: "person.fill")
                Text(store.t("ui/tab/me"))
            }
            .tag(3)
        }
    }
}
#endif
