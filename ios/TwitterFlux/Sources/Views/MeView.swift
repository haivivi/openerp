// MeView — current user's profile + settings.

import SwiftUI

struct MeView: View {
    @EnvironmentObject var store: FluxStore

    private var auth: AuthState? { store.get("auth/state") }

    var body: some View {
        List {
            // Profile header
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
                                Text("Followers")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            VStack {
                                Text("\(user.followingCount)")
                                    .font(.headline)
                                Text("Following")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                            VStack {
                                Text("\(user.tweetCount)")
                                    .font(.headline)
                                Text("Tweets")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
            }

            // Settings
            Section("Settings") {
                NavigationLink(destination: EditProfileView()) {
                    Label("Edit Profile", systemImage: "person.crop.circle")
                }
                NavigationLink(destination: ChangePasswordView()) {
                    Label("Change Password", systemImage: "lock")
                }
            }

            // Account
            Section {
                Button(role: .destructive) {
                    store.emit("auth/logout")
                } label: {
                    Label("Sign Out", systemImage: "rectangle.portrait.and.arrow.right")
                        .foregroundColor(.red)
                }
            }
        }
        #if os(iOS)
        .listStyle(.insetGrouped)
        #endif
        .navigationTitle("Me")
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
            Section("Display Name") {
                TextField("Display name", text: $displayName)
            }
            Section("Bio") {
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
                    Label("Saved!", systemImage: "checkmark.circle.fill")
                        .foregroundColor(.green)
                }
            }

            Section {
                Button(action: save) {
                    if settings?.busy == true {
                        ProgressView()
                    } else {
                        Text("Save Changes")
                    }
                }
                .disabled(displayName.trimmingCharacters(in: .whitespaces).isEmpty
                          || settings?.busy == true)
            }
        }
        .navigationTitle("Edit Profile")
        .onAppear {
            if !loaded {
                store.emit("settings/load")
                loaded = true
            }
            // Load current values from settings state.
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
            Section("Current Password") {
                SecureField("Current password", text: $oldPassword)
            }
            Section("New Password") {
                SecureField("New password", text: $newPassword)
                SecureField("Confirm new password", text: $confirmPassword)
            }

            if let error = pwState?.error {
                Section {
                    Text(error).foregroundColor(.red)
                }
            }

            if pwState?.success == true {
                Section {
                    Label("Password changed!", systemImage: "checkmark.circle.fill")
                        .foregroundColor(.green)
                }
            }

            Section {
                Button("Change Password") {
                    changePassword()
                }
                .disabled(oldPassword.isEmpty || newPassword.isEmpty
                          || newPassword != confirmPassword)
            }
        }
        .navigationTitle("Change Password")
    }

    private func changePassword() {
        store.emit("settings/change-password", json: [
            "oldPassword": oldPassword,
            "newPassword": newPassword,
        ])
    }
}
