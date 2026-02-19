// MeView — current user's profile + settings.

import SwiftUI

struct MeView: View {
    @EnvironmentObject var store: FluxStore

    private var auth: AuthState? { store.get("auth/state") }

    var body: some View {
        List {
            if let user = auth?.user {
                Section {
                    VStack(spacing: 12) {
                        Circle()
                            .fill(Color.blue.opacity(0.2))
                            .frame(width: 72, height: 72)
                            .overlay(
                                Text(String(user.displayName.prefix(1)))
                                    .font(.title)
                                    .foregroundColor(.blue)
                            )

                        Text(user.displayName)
                            .font(.title2.bold())

                        Text("@\(user.username)")
                            .font(.subheadline)
                            .foregroundColor(.secondary)

                        if let bio = user.bio {
                            Text(bio)
                                .font(.body)
                                .multilineTextAlignment(.center)
                        }

                        HStack(spacing: 24) {
                            VStack {
                                Text("\(user.followerCount)")
                                    .font(.headline)
                                Text(store.t("ui/profile/followers"))
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            VStack {
                                Text("\(user.followingCount)")
                                    .font(.headline)
                                Text(store.t("ui/profile/following"))
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            VStack {
                                Text("\(user.tweetCount)")
                                    .font(.headline)
                                Text(store.t("ui/profile/tweets"))
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
            }

            Section(store.t("ui/me/settings")) {
                NavigationLink(destination: EditProfileView()) {
                    Label(store.t("ui/me/edit_profile"), systemImage: "person.crop.circle")
                }
                NavigationLink(destination: ChangePasswordView()) {
                    Label(store.t("ui/me/change_password"), systemImage: "lock")
                }
            }

            Section(store.t("ui/me/developer")) {
                if let url = store.dashboardURL {
                    Link(destination: url) {
                        Label(store.t("ui/me/admin_dashboard"), systemImage: "globe")
                    }
                }
                Text(store.serverURL)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .textSelection(.enabled)
            }

            Section {
                Button(role: .destructive) {
                    store.emit("auth/logout")
                } label: {
                    Label(store.t("ui/me/sign_out"), systemImage: "rectangle.portrait.and.arrow.right")
                        .foregroundColor(.red)
                }
            }
        }
        #if os(iOS)
        .listStyle(.insetGrouped)
        #endif
        .navigationTitle(store.t("ui/me/title"))
    }
}

// MARK: - Edit Profile

struct EditProfileView: View {
    @EnvironmentObject var store: FluxStore
    @State private var displayName = ""
    @State private var bio = ""
    @State private var loaded = false

    private var settings: SettingsState? { store.get("settings/state") }

    var body: some View {
        Form {
            Section(store.t("ui/edit/display_name")) {
                TextField(store.t("ui/edit/display_name"), text: $displayName)
            }
            Section(store.t("ui/edit/bio")) {
                TextEditor(text: $bio)
                    .frame(minHeight: 80)
            }

            if let error = settings?.error {
                Section {
                    Text(error).foregroundColor(.red)
                }
            }

            if settings?.saved == true {
                Section {
                    Label(store.t("ui/edit/saved"), systemImage: "checkmark.circle.fill")
                        .foregroundColor(.green)
                }
            }

            Section {
                Button(action: save) {
                    if settings?.busy == true {
                        ProgressView()
                    } else {
                        Text(store.t("ui/edit/save"))
                    }
                }
                .disabled(displayName.trimmingCharacters(in: .whitespaces).isEmpty
                          || settings?.busy == true)
            }
        }
        .navigationTitle(store.t("ui/me/edit_profile"))
        .onAppear {
            if !loaded {
                store.emit("settings/load")
                loaded = true
            }
            if let s = settings {
                displayName = s.displayName
                bio = s.bio
            }
        }
        .onChange(of: settings?.displayName) { _, newVal in
            if let v = newVal, !loaded { displayName = v }
        }
    }

    private func save() {
        store.emit("settings/save", json: [
            "displayName": displayName,
            "bio": bio,
        ])
    }
}

// MARK: - Change Password

struct ChangePasswordView: View {
    @EnvironmentObject var store: FluxStore
    @State private var oldPassword = ""
    @State private var newPassword = ""
    @State private var confirmPassword = ""

    private var pwState: PasswordState? { store.get("settings/password") }

    var body: some View {
        Form {
            Section(store.t("ui/password/current")) {
                SecureField(store.t("ui/password/current"), text: $oldPassword)
            }
            Section(store.t("ui/password/new")) {
                SecureField(store.t("ui/password/new"), text: $newPassword)
                SecureField(store.t("ui/password/confirm"), text: $confirmPassword)
            }

            if let error = pwState?.error {
                Section {
                    Text(error).foregroundColor(.red)
                }
            }

            if pwState?.success == true {
                Section {
                    Label(store.t("ui/password/changed"), systemImage: "checkmark.circle.fill")
                        .foregroundColor(.green)
                }
            }

            Section {
                Button(store.t("ui/password/change")) {
                    changePassword()
                }
                .disabled(oldPassword.isEmpty || newPassword.isEmpty
                          || newPassword != confirmPassword)
            }
        }
        .navigationTitle(store.t("ui/me/change_password"))
    }

    private func changePassword() {
        store.emit("settings/change-password", json: [
            "oldPassword": oldPassword,
            "newPassword": newPassword,
        ])
    }
}
