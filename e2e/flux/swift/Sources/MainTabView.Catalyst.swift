// MainTabView (Catalyst) — iPhone-only bottom tab scaffold.

import SwiftUI

#if targetEnvironment(macCatalyst)
struct MainTabView: View {
    @EnvironmentObject var store: FluxStore
    @State private var selectedTab = 0

    var body: some View {
        CatalystIPhoneTabScaffold(selectedTab: $selectedTab)
            .environmentObject(store)
    }
}

private struct CatalystIPhoneTabScaffold: View {
    @EnvironmentObject var store: FluxStore
    @Binding var selectedTab: Int

    var body: some View {
        VStack(spacing: 0) {
            Group {
                switch selectedTab {
                case 0:
                    NavigationStack { HomeView() }
                case 1:
                    NavigationStack { SearchView() }
                case 2:
                    NavigationStack { InboxView() }
                case 3:
                    NavigationStack { MeView() }
                default:
                    NavigationStack { HomeView() }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider()

            HStack {
                tabButton(index: 0, icon: "house.fill", title: store.t("ui/tab/home"))
                tabButton(index: 1, icon: "magnifyingglass", title: store.t("ui/tab/search"))
                tabButton(index: 2, icon: "tray.fill", title: store.t("ui/tab/inbox"))
                tabButton(index: 3, icon: "person.fill", title: store.t("ui/tab/me"))
            }
            .padding(.horizontal, 6)
            .padding(.top, 8)
            .padding(.bottom, 10)
            .background(.ultraThinMaterial)
        }
    }

    @ViewBuilder
    private func tabButton(index: Int, icon: String, title: String) -> some View {
        let isSelected = selectedTab == index
        Button {
            selectedTab = index
        } label: {
            VStack(spacing: 4) {
                Image(systemName: icon)
                    .font(.system(size: 17, weight: isSelected ? .semibold : .regular))
                Text(title)
                    .font(.caption2)
                    .lineLimit(1)
            }
            .frame(maxWidth: .infinity)
            .foregroundColor(isSelected ? .accentColor : .secondary)
            .padding(.vertical, 4)
        }
        .buttonStyle(.plain)
    }
}
#endif
