// LoginView — auth/login page.

import SwiftUI

struct LoginView: View {
    @EnvironmentObject var store: FluxStore
    @State private var username = ""

    private var auth: AuthState? {
        store.get("auth/state") as AuthState?
    }

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "bubble.left.and.bubble.right.fill")
                .font(.system(size: 48))
                .foregroundColor(.blue)

            Text("TwitterFlux")
                .font(.largeTitle.bold())

            Text("Powered by Flux State Engine")
                .font(.subheadline)
                .foregroundColor(.secondary)

            VStack(spacing: 16) {
                TextField(store.t("ui/login/username"), text: $username)
                    .textFieldStyle(.roundedBorder)
                    #if os(iOS)
                    .autocapitalization(.none)
                    #endif
                    .disableAutocorrection(true)

                Button(action: login) {
                    if auth?.busy == true {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Text(store.t("ui/login/button"))
                            .frame(maxWidth: .infinity)
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(username.isEmpty || auth?.busy == true)

                if let error = auth?.error {
                    Text(error)
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
            .padding(.horizontal, 32)

            Spacer()

            Text(store.t("ui/login/hint"))
                .font(.caption)
                .foregroundColor(.secondary)
                .padding(.bottom, 16)
        }
    }

    private func login() {
        store.emit("auth/login", json: ["username": username])
    }
}
