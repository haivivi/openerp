// ProfileView — user profile page.

import SwiftUI

struct ProfileView: View {
    @EnvironmentObject var store: FluxStore
    let userId: String

    private var profile: ProfilePage? {
        store.get("profile/\(userId)")
    }

    var body: some View {
        Group {
            if let p = profile {
                ScrollView {
                    VStack(spacing: 16) {
                        profileHeader(p)

                        Divider()

                        if p.tweets.isEmpty {
                            Text(store.t("ui/profile/no_tweets"))
                                .foregroundColor(.secondary)
                                .padding(.top, 32)
                        } else {
                            LazyVStack(spacing: 0) {
                                ForEach(p.tweets, id: \.tweetId) { item in
                                    TweetRow(item: item)
                                        .padding(.horizontal)
                                        .padding(.vertical, 8)
                                    Divider()
                                }
                            }
                        }
                    }
                }
            } else {
                ProgressView(store.t("ui/common/loading"))
            }
        }
        .navigationTitle("@\(userId)")
        .onAppear {
            store.emit("profile/load", json: ["userId": userId])
        }
    }

    @ViewBuilder
    private func profileHeader(_ p: ProfilePage) -> some View {
        VStack(spacing: 12) {
            Circle()
                .fill(Color.blue.opacity(0.2))
                .frame(width: 72, height: 72)
                .overlay(
                    Text(String(p.user.displayName.prefix(1)))
                        .font(.title)
                        .foregroundColor(.blue)
                )

            Text(p.user.displayName)
                .font(.title2.bold())

            Text("@\(p.user.username)")
                .font(.subheadline)
                .foregroundColor(.secondary)

            if let bio = p.user.bio {
                Text(bio)
                    .font(.body)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
            }

            HStack(spacing: 24) {
                stat(count: p.user.followerCount, label: store.t("ui/profile/followers"))
                stat(count: p.user.followingCount, label: store.t("ui/profile/following"))
                stat(count: p.user.tweetCount, label: store.t("ui/profile/tweets"))
            }

            if p.followedByMe {
                Button(action: toggleFollow) {
                    Text(store.t("ui/profile/unfollow"))
                        .frame(width: 120)
                }
                .buttonStyle(.bordered)
            } else {
                Button(action: toggleFollow) {
                    Text(store.t("ui/profile/follow"))
                        .frame(width: 120)
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding()
    }

    private func stat(count: Int, label: String) -> some View {
        VStack {
            Text("\(count)")
                .font(.headline)
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    private func toggleFollow() {
        if profile?.followedByMe == true {
            store.emit("user/unfollow", json: ["userId": userId])
        } else {
            store.emit("user/follow", json: ["userId": userId])
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            store.emit("profile/load", json: ["userId": userId])
        }
    }
}
