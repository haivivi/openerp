// SearchView — search users and tweets.

import SwiftUI

struct SearchView: View {
    @EnvironmentObject var store: FluxStore
    @State private var query = ""

    private var results: SearchState? { store.get("search/state") }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField(store.t("ui/search/placeholder"), text: $query)
                    .onSubmit { search() }
                if !query.isEmpty {
                    Button(action: clear) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(10)
            #if os(iOS)
            .background(Color(.systemGray6))
            #else
            .background(Color.gray.opacity(0.1))
            #endif
            .cornerRadius(10)
            .padding(.horizontal)
            .padding(.top, 8)

            if let results = results {
                if results.loading {
                    ProgressView()
                        .padding(.top, 32)
                } else {
                    List {
                        if !results.users.isEmpty {
                            Section(store.t("ui/search/users")) {
                                ForEach(results.users, id: \.id) { user in
                                    NavigationLink(destination: ProfileView(userId: user.id)) {
                                        HStack(spacing: 10) {
                                            Circle()
                                                .fill(Color.blue.opacity(0.2))
                                                .frame(width: 40, height: 40)
                                                .overlay(
                                                    Text(String(user.displayName.prefix(1)))
                                                        .font(.headline)
                                                        .foregroundColor(.blue)
                                                )
                                            VStack(alignment: .leading) {
                                                Text(user.displayName)
                                                    .font(.subheadline.bold())
                                                Text("@\(user.username)")
                                                    .font(.caption)
                                                    .foregroundColor(.secondary)
                                            }
                                            Spacer()
                                            Text(store.t("format/tweet_count?count=\(user.tweetCount)"))
                                                .font(.caption2)
                                                .foregroundColor(.secondary)
                                        }
                                    }
                                }
                            }
                        }

                        if !results.tweets.isEmpty {
                            Section(store.t("ui/search/tweets_section")) {
                                ForEach(results.tweets) { item in
                                    NavigationLink(destination: TweetDetailView(tweetId: item.tweetId)) {
                                        TweetRow(item: item)
                                    }
                                }
                            }
                        }

                        if results.users.isEmpty && results.tweets.isEmpty && !results.query.isEmpty {
                            Text(store.t("ui/search/no_results"))
                                .foregroundColor(.secondary)
                                .frame(maxWidth: .infinity)
                                .padding(.top, 32)
                        }
                    }
                    #if os(iOS)
                    .listStyle(.insetGrouped)
                    #endif
                }
            } else {
                VStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 36))
                        .foregroundColor(.secondary)
                    Text(store.t("ui/search/placeholder"))
                        .foregroundColor(.secondary)
                }
                .padding(.top, 60)
            }

            Spacer()
        }
        .navigationTitle(store.t("ui/search/title"))
    }

    private func search() {
        guard !query.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        store.emit("search/query", json: ["query": query])
    }

    private func clear() {
        query = ""
        store.emit("search/clear")
    }
}
