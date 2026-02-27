/// TweetRow — reusable tweet cell (mirrors Swift TweetRow).
library;

import 'package:flutter/cupertino.dart';

import '../../models/models.dart';
import '../../store/flux_store.dart';
import '../profile_view.dart';

class TweetRow extends StatelessWidget {
  final FeedItem item;

  const TweetRow({super.key, required this.item});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Author header
          GestureDetector(
            onTap: () {
              Navigator.of(context).push(
                CupertinoPageRoute<void>(
                  builder: (_) => ProfileView(userId: item.author.id),
                ),
              );
            },
            child: Row(
              children: [
                // Avatar circle
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: CupertinoColors.activeBlue.withAlpha(51),
                    shape: BoxShape.circle,
                  ),
                  alignment: Alignment.center,
                  child: Text(
                    item.author.displayName.isNotEmpty
                        ? item.author.displayName[0]
                        : '?',
                    style: const TextStyle(
                      fontSize: 17,
                      fontWeight: FontWeight.w600,
                      color: CupertinoColors.activeBlue,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        item.author.displayName,
                        style: const TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      Text(
                        '@${item.author.username}',
                        style: const TextStyle(
                          fontSize: 12,
                          color: CupertinoColors.secondaryLabel,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          const SizedBox(height: 8),

          // Content
          Text(item.content, style: const TextStyle(fontSize: 17)),

          // Reply indicator
          if (item.replyToId != null) ...[
            const SizedBox(height: 4),
            Row(
              children: [
                const Icon(
                  CupertinoIcons.reply,
                  size: 12,
                  color: CupertinoColors.secondaryLabel,
                ),
                const SizedBox(width: 4),
                Text(
                  store.t('ui/tweet/reply'),
                  style: const TextStyle(
                    fontSize: 11,
                    color: CupertinoColors.secondaryLabel,
                  ),
                ),
              ],
            ),
          ],

          const SizedBox(height: 8),

          // Action bar
          Row(
            children: [
              GestureDetector(
                onTap: () {
                  if (item.likedByMe) {
                    store.emit('tweet/unlike', {'tweetId': item.tweetId});
                  } else {
                    store.emit('tweet/like', {'tweetId': item.tweetId});
                  }
                },
                child: Row(
                  children: [
                    Icon(
                      item.likedByMe
                          ? CupertinoIcons.heart_fill
                          : CupertinoIcons.heart,
                      size: 16,
                      color: item.likedByMe
                          ? CupertinoColors.destructiveRed
                          : CupertinoColors.secondaryLabel,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '${item.likeCount}',
                      style: const TextStyle(
                        fontSize: 12,
                        color: CupertinoColors.secondaryLabel,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 24),
              Row(
                children: [
                  const Icon(
                    CupertinoIcons.chat_bubble,
                    size: 16,
                    color: CupertinoColors.secondaryLabel,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    '${item.replyCount}',
                    style: const TextStyle(
                      fontSize: 12,
                      color: CupertinoColors.secondaryLabel,
                    ),
                  ),
                ],
              ),
              const Spacer(),
            ],
          ),
        ],
      ),
    );
  }
}
