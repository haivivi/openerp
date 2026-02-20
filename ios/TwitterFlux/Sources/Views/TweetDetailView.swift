// TweetDetailView — tweet detail + replies + compose reply.

import SwiftUI

struct TweetDetailView: View {
    @EnvironmentObject var store: FluxStore
    let tweetId: String
    @State private var replyText = ""

    private var detail: TweetDetailState? {
        store.get("tweet/\(tweetId)")
    }

    var body: some View {
        Group {
            if let d = detail {
                VStack(spacing: 0) {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 0) {
                            mainTweet(d.tweet)

                            Divider()

                            replyCompose

                            Divider()

                            if d.replies.isEmpty {
                                Text(store.t("ui/tweet/no_replies"))
                                    .foregroundColor(.secondary)
                                    .padding()
                            } else {
                                LazyVStack(spacing: 0) {
                                    ForEach(d.replies, id: \.tweetId) { reply in
                                        TweetRow(item: reply)
                                            .padding(.horizontal)
                                            .padding(.vertical, 8)
                                        Divider()
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                ProgressView(store.t("ui/common/loading"))
            }
        }
        .navigationTitle(store.t("ui/profile/tweets"))
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
        .onAppear {
            store.emit("tweet/load", json: ["tweetId": tweetId])
        }
    }

    @ViewBuilder
    private func mainTweet(_ tweet: FeedItem) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            NavigationLink(destination: ProfileView(userId: tweet.author.id)) {
                HStack(spacing: 10) {
                    Circle()
                        .fill(Color.blue.opacity(0.2))
                        .frame(width: 48, height: 48)
                        .overlay(
                            Text(String(tweet.author.displayName.prefix(1)))
                                .font(.title3)
                                .foregroundColor(.blue)
                        )
                    VStack(alignment: .leading) {
                        Text(tweet.author.displayName)
                            .font(.headline)
                            .foregroundColor(.primary)
                        Text("@\(tweet.author.username)")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .buttonStyle(.plain)

            Text(tweet.content)
                .font(.title3)

            HStack(spacing: 16) {
                Button(action: {
                    if tweet.likedByMe {
                        store.emit("tweet/unlike", json: ["tweetId": tweet.tweetId])
                    } else {
                        store.emit("tweet/like", json: ["tweetId": tweet.tweetId])
                    }
                    store.emit("tweet/load", json: ["tweetId": tweetId])
                }) {
                    Label("\(tweet.likeCount)", systemImage: tweet.likedByMe ? "heart.fill" : "heart")
                        .foregroundColor(tweet.likedByMe ? .red : .secondary)
                }
                .buttonStyle(.plain)

                Label("\(tweet.replyCount)", systemImage: "bubble.right")
                    .foregroundColor(.secondary)
            }
            .font(.subheadline)

            Text(formatDate(tweet.createdAt))
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
    }

    private var replyCompose: some View {
        HStack(spacing: 8) {
            TextField(store.t("ui/compose/reply_placeholder"), text: $replyText)
                .textFieldStyle(.roundedBorder)

            Button(store.t("ui/tweet/reply")) {
                guard !replyText.trimmingCharacters(in: .whitespaces).isEmpty else { return }
                store.emit("tweet/create", json: [
                    "content": replyText,
                    "replyToId": tweetId,
                ])
                replyText = ""
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                    store.emit("tweet/load", json: ["tweetId": tweetId])
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(replyText.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding()
    }

    private func formatDate(_ dateStr: String) -> String {
        if let idx = dateStr.firstIndex(of: "T") {
            return String(dateStr[..<idx])
        }
        return dateStr
    }
}
