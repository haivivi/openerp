// HomeView — timeline feed with navigation.

import SwiftUI

struct HomeView: View {
    @EnvironmentObject var store: FluxStore

    private var auth: AuthState? { store.get("auth/state") }
    private var feed: TimelineFeed? { store.get("timeline/feed") }

    var body: some View {
        Group {
            if let feed = feed {
                if feed.items.isEmpty && !feed.loading {
                    VStack(spacing: 12) {
                        Image(systemName: "bubble.left")
                            .font(.system(size: 48))
                            .foregroundColor(.secondary)
                        Text(store.t("ui/home/empty"))
                            .font(.headline)
                        Text(store.t("ui/home/empty_hint"))
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                } else {
                    List(feed.items) { item in
                        NavigationLink(destination: TweetDetailView(tweetId: item.tweetId)) {
                            TweetRow(item: item)
                        }
                        .listRowSeparator(.visible)
                    }
                    .listStyle(.plain)
                    .refreshable {
                        store.emit("timeline/load")
                    }
                }
            } else {
                ProgressView(store.t("ui/common/loading"))
            }
        }
        .navigationTitle(store.t("ui/home/title"))
        .toolbar {
            #if os(iOS)
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink(destination: ComposeView()) {
                    Image(systemName: "square.and.pencil")
                }
            }
            #else
            ToolbarItem {
                NavigationLink(destination: ComposeView()) {
                    Image(systemName: "square.and.pencil")
                }
            }
            #endif
        }
    }
}

// MARK: - Tweet Row

struct TweetRow: View {
    @EnvironmentObject var store: FluxStore
    let item: FeedItem

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            NavigationLink(destination: ProfileView(userId: item.author.id)) {
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
                            .foregroundColor(.primary)
                        Text("@\(item.author.username)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    Spacer()
                }
            }
            .buttonStyle(.plain)

            Text(item.content)
                .font(.body)

            if item.replyToId != nil {
                Label(store.t("ui/tweet/reply"), systemImage: "arrowshape.turn.up.left")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            HStack(spacing: 24) {
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
