// InboxView — in-app messages (站内信).

import SwiftUI

struct InboxView: View {
    @EnvironmentObject var store: FluxStore

    private var inbox: InboxState? { store.get("inbox/state") }

    var body: some View {
        Group {
            if let inbox = inbox {
                if inbox.loading {
                    ProgressView(store.t("ui/common/loading"))
                } else if inbox.messages.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "tray")
                            .font(.system(size: 48))
                            .foregroundColor(.secondary)
                        Text(store.t("ui/inbox/empty"))
                            .font(.headline)
                    }
                } else {
                    List(inbox.messages) { msg in
                        MessageRow(message: msg)
                    }
                    .listStyle(.plain)
                    .refreshable {
                        store.emit("inbox/load")
                    }
                }
            } else {
                ProgressView(store.t("ui/common/loading"))
            }
        }
        .navigationTitle(store.t("ui/inbox/title"))
        .onAppear {
            store.emit("inbox/load")
        }
    }
}

struct MessageRow: View {
    @EnvironmentObject var store: FluxStore
    let message: InboxMessage

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                kindBadge
                Spacer()
                if !message.read {
                    Text(store.t("ui/inbox/unread"))
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.red)
                        .foregroundColor(.white)
                        .cornerRadius(4)
                }
            }

            Text(message.title)
                .font(.headline)
                .foregroundColor(message.read ? .secondary : .primary)

            Text(message.body)
                .font(.subheadline)
                .foregroundColor(.secondary)
                .lineLimit(3)

            if !message.read {
                Button(store.t("ui/inbox/mark_read")) {
                    store.emit("inbox/mark-read", json: ["messageId": message.id])
                }
                .font(.caption)
                .buttonStyle(.bordered)
            }
        }
        .padding(.vertical, 4)
    }

    private var kindBadge: some View {
        let (icon, color): (String, Color) = {
            switch message.kind {
            case "broadcast": return ("megaphone", .orange)
            case "system": return ("gear", .blue)
            case "personal": return ("person", .green)
            default: return ("envelope", .gray)
            }
        }()
        return Label(message.kind.capitalized, systemImage: icon)
            .font(.caption)
            .foregroundColor(color)
    }
}
