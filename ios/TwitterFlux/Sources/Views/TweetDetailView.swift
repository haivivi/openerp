// TweetDetailView — tweet detail + replies page.

import SwiftUI

struct TweetDetailView: View {
    @EnvironmentObject var store: FluxStore
    let tweetId: String

    private var detail: TweetDetailState? {
        store.get("tweet/\(tweetId)")
    }

    var body: some View {
        Group {
            if let d = detail {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        // Main tweet
                        VStack(alignment: .leading, spacing: 12) {
                            HStack(spacing: 8) {
                                Circle()
                                    .fill(Color.blue.opacity(0.2))
                                    .frame(width: 44, height: 44)
                                    .overlay(
                                        Text(String(d.tweet.author.displayName.prefix(1)))
                                            .font(.headline)
                                            .foregroundColor(.blue)
                                    )

                                VStack(alignment: .leading) {
                                    Text(d.tweet.author.displayName)
                                        .font(.headline)
                                    Text("@\(d.tweet.author.username)")
                                        .font(.subheadline)
                                        .foregroundColor(.secondary)
                                }
                            }

                            Text(d.tweet.content)
                                .font(.title3)

                            // Stats
                            HStack(spacing: 16) {
                                Label("\(d.tweet.likeCount)", systemImage: d.tweet.likedByMe ? "heart.fill" : "heart")
                                    .foregroundColor(d.tweet.likedByMe ? .red : .secondary)
                                Label("\(d.tweet.replyCount)", systemImage: "bubble.right")
                                    .foregroundColor(.secondary)
                            }
                            .font(.subheadline)
                        }
                        .padding()

                        Divider()

                        // Replies
                        if d.replies.isEmpty {
                            Text("No replies yet")
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
            } else {
                ProgressView("Loading...")
            }
        }
        .navigationTitle("Tweet")
        .onAppear {
            store.emit("tweet/load", json: ["tweetId": tweetId])
        }
    }
}
