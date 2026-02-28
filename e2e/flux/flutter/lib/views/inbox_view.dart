/// InboxView — in-app messages / 站内信 (mirrors Swift InboxView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';

class InboxView extends StatelessWidget {
  const InboxView({super.key});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final inbox = store.get<InboxState>('inbox/state');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/inbox/title')),
      ),
      child: SafeArea(child: _buildBody(context, store, inbox)),
    );
  }

  Widget _buildBody(BuildContext context, FluxStore store, InboxState? inbox) {
    if (inbox == null || inbox.loading) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CupertinoActivityIndicator(),
            const SizedBox(height: 8),
            Text(
              store.t('ui/common/loading'),
              style: const TextStyle(color: CupertinoColors.secondaryLabel),
            ),
          ],
        ),
      );
    }

    if (inbox.messages.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              CupertinoIcons.tray,
              size: 48,
              color: CupertinoColors.secondaryLabel,
            ),
            const SizedBox(height: 12),
            Text(
              store.t('ui/inbox/empty'),
              style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
            ),
          ],
        ),
      );
    }

    return ListView.separated(
      itemCount: inbox.messages.length,
      separatorBuilder: (context, index) => Container(
        height: 1,
        color: CupertinoColors.separator.resolveFrom(context),
      ),
      itemBuilder: (_, index) {
        final msg = inbox.messages[index];
        return _MessageRow(message: msg);
      },
    );
  }
}

// ---------------------------------------------------------------------------

class _MessageRow extends StatelessWidget {
  final InboxMessage message;
  const _MessageRow({required this.message});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Kind badge + unread indicator
          Row(
            children: [
              _kindBadge(store),
              const Spacer(),
              if (!message.read)
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: CupertinoColors.destructiveRed,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    store.t('ui/inbox/unread'),
                    style: const TextStyle(
                      fontSize: 11,
                      color: CupertinoColors.white,
                    ),
                  ),
                ),
            ],
          ),

          const SizedBox(height: 8),

          // Title
          Text(
            message.title,
            style: TextStyle(
              fontSize: 17,
              fontWeight: FontWeight.w600,
              color: message.read
                  ? CupertinoColors.secondaryLabel
                  : CupertinoColors.label,
            ),
          ),

          const SizedBox(height: 4),

          // Body
          Text(
            message.body,
            maxLines: 3,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              fontSize: 15,
              color: CupertinoColors.secondaryLabel,
            ),
          ),

          // Mark as read button
          if (!message.read) ...[
            const SizedBox(height: 8),
            CupertinoButton(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
              minimumSize: Size.zero,
              child: Text(
                store.t('ui/inbox/mark_read'),
                style: const TextStyle(fontSize: 12),
              ),
              onPressed: () {
                store.emit('inbox/mark-read', {'messageId': message.id});
              },
            ),
          ],
        ],
      ),
    );
  }

  Widget _kindBadge(FluxStore store) {
    final (IconData icon, Color color) = switch (message.kind) {
      'broadcast' => (CupertinoIcons.speaker_2, CupertinoColors.activeOrange),
      'system' => (CupertinoIcons.gear, CupertinoColors.activeBlue),
      'personal' => (CupertinoIcons.person, CupertinoColors.activeGreen),
      _ => (CupertinoIcons.envelope, CupertinoColors.systemGrey),
    };

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 14, color: color),
        const SizedBox(width: 4),
        Text(
          _capitalize(message.kind),
          style: TextStyle(fontSize: 12, color: color),
        ),
      ],
    );
  }

  static String _capitalize(String s) =>
      s.isEmpty ? s : '${s[0].toUpperCase()}${s.substring(1)}';
}
