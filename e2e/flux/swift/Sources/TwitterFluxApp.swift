// TwitterFluxApp — entry point.
// Flux owns all state. SwiftUI only renders.

import SwiftUI
#if targetEnvironment(macCatalyst)
import UIKit
#endif

#if !TESTING
@main
#endif
struct TwitterFluxApp: App {
    @StateObject private var store = FluxStore()

    private let iphoneCanvasWidth: CGFloat = 390
    private let iphoneCanvasHeight: CGFloat = 844

    @MainActor
    private func configureCatalystForIOSLikePresentation() {
#if targetEnvironment(macCatalyst)
        let targetSize = CGSize(width: iphoneCanvasWidth, height: iphoneCanvasHeight)
        for scene in UIApplication.shared.connectedScenes {
            guard let windowScene = scene as? UIWindowScene else { continue }

            if let restrictions = windowScene.sizeRestrictions {
                restrictions.minimumSize = targetSize
                restrictions.maximumSize = targetSize
            }

            if let titlebar = windowScene.titlebar {
                titlebar.titleVisibility = .hidden
                titlebar.toolbar = nil
            }

            if #available(iOS 17.0, *) {
                windowScene.traitOverrides.horizontalSizeClass = .compact
                windowScene.traitOverrides.verticalSizeClass = .regular
            }

            for window in windowScene.windows {
                if #available(iOS 17.0, *) {
                    window.traitOverrides.horizontalSizeClass = .compact
                    window.traitOverrides.verticalSizeClass = .regular
                }
            }
        }
#endif
    }

    var body: some Scene {
#if os(macOS) || targetEnvironment(macCatalyst)
        WindowGroup {
            RootView()
                .frame(width: iphoneCanvasWidth, height: iphoneCanvasHeight)
                .environmentObject(store)
                .onAppear {
                    configureCatalystForIOSLikePresentation()
                    store.emit("app/initialize")
                }
        }
        .defaultSize(width: iphoneCanvasWidth, height: iphoneCanvasHeight)
        .windowResizability(.contentSize)
#else
        WindowGroup {
            RootView()
                .environmentObject(store)
                .onAppear {
                    store.emit("app/initialize")
                }
        }
#endif
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
