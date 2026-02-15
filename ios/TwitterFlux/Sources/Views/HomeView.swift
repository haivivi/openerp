// HomeView — timeline/feed page.

import SwiftUI

struct HomeView: View {
    @EnvironmentObject var store: FluxStore

    private var auth: AuthState? { store.get("auth/state") }
    private var feed: TimelineFeed? { store.get("timeline/feed") }

    var body: some View {
        NavigationStack {
            Group {
                if let feed = feed {
                    if feed.items.isEmpty && !feed.loading {
                        ContentUnavailableView(
                            "No tweets yet",
                            systemImage: "bubble.left",
                            description: Text("Be the first to tweet!")
                        )
                    } else {
                        List(feed.items) { item in
                            TweetRow(item: item)
                                .listRowSeparator(.visible)
                        }
                        .listStyle(.plain)
                        .refreshable {
                            store.emit("timeline/load")
                        }
                    }
                } else {
                    ProgressView("Loading...")
                }
            }
            .navigationTitle("Home")
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    if let user = auth?.user {
                        Text("@\(user.username)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    HStack(spacing: 12) {
                        NavigationLink(destination: ComposeView()) {
                            Image(systemName: "square.and.pencil")
                        }
                        Button("Logout") {
                            store.emit("auth/logout")
                        }
                        .font(.caption)
                    }
                }
            }
        }
    }
}

// MARK: - Tweet Row

struct TweetRow: View {
    @EnvironmentObject var store: FluxStore
    let item: FeedItem

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Author
            HStack(spacing: 8) {
                Circle()
                    .fill(Color.blue.opacity(0.2))
                    .frame(width: 36, height: 36)
                    .overlay(
                        Text(String(item.author.displayName.prefix(1)))
                            .font(.headline)
                            .foregroundColor(.blue)
                    )

                VStack(alignment: .leading) {
                    Text(item.author.displayName)
                        .font(.subheadline.bold())
                    Text("@\(item.author.username)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()
            }

            // Content
            Text(item.content)
                .font(.body)

            // Actions
            HStack(spacing: 24) {
                // Like
                Button(action: toggleLike) {
                    HStack(spacing: 4) {
                        Image(systemName: item.likedByMe ? "heart.fill" : "heart")
                            .foregroundColor(item.likedByMe ? .red : .secondary)
                        Text("\(item.likeCount)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .buttonStyle(.plain)

                // Reply count
                HStack(spacing: 4) {
                    Image(systemName: "bubble.right")
                        .foregroundColor(.secondary)
                    Text("\(item.replyCount)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()
            }
            .padding(.top, 4)
        }
        .padding(.vertical, 4)
    }

    private func toggleLike() {
        if item.likedByMe {
            store.emit("tweet/unlike", json: ["tweetId": item.tweetId])
        } else {
            store.emit("tweet/like", json: ["tweetId": item.tweetId])
        }
    }
}
