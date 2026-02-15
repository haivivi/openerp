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
                // Text input
                TextEditor(text: $content)
                    .padding()
                    .onChange(of: content) { _, newValue in
                        store.emit("compose/update-field", json: [
                            "field": "content",
                            "value": newValue,
                        ])
                    }

                Divider()

                // Bottom bar
                HStack {
                    // Character count
                    Text("\(charCount)/280")
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
            .navigationTitle("Compose")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Post") { postTweet() }
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
