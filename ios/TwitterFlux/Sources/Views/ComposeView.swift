// ComposeView — tweet compose page.

import SwiftUI

struct ComposeView: View {
    @EnvironmentObject var store: FluxStore
    @Environment(\.dismiss) var dismiss
    @State private var content = ""

    private var compose: ComposeState? { store.get("compose/state") }
    private var charCount: Int { content.count }
    private var isOverLimit: Bool { charCount > 280 }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                TextEditor(text: $content)
                    .padding()
                    .onChange(of: content) { _, newValue in
                        store.emit("compose/update-field", json: [
                            "field": "content",
                            "value": newValue,
                        ])
                    }

                Divider()

                HStack {
                    Text(store.t("format/char_count?current=\(charCount)&max=280"))
                        .font(.caption)
                        .foregroundColor(isOverLimit ? .red : .secondary)

                    Spacer()

                    if let error = compose?.error {
                        Text(error)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
                .padding()
            }
            .navigationTitle(store.t("ui/compose/title"))
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(store.t("ui/compose/cancel")) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(store.t("ui/compose/post")) { postTweet() }
                        .bold()
                        .disabled(content.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                  || isOverLimit
                                  || compose?.busy == true)
                }
            }
        }
    }

    private func postTweet() {
        store.emit("tweet/create", json: ["content": content])
        dismiss()
    }
}
