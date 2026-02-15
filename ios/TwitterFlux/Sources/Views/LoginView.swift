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

            // Logo
            Image(systemName: "bubble.left.and.bubble.right.fill")
                .font(.system(size: 48))
                .foregroundColor(.blue)

            Text("TwitterFlux")
                .font(.largeTitle.bold())

            Text("Powered by Flux State Engine")
                .font(.subheadline)
                .foregroundColor(.secondary)

            // Form
            VStack(spacing: 16) {
                TextField("Username", text: $username)
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
                        Text("Sign In")
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

            // Demo hint
            Text("Try: alice, bob, or carol")
                .font(.caption)
                .foregroundColor(.secondary)
                .padding(.bottom, 16)
        }
    }

    private func login() {
        store.emit("auth/login", json: ["username": username])
    }
}
