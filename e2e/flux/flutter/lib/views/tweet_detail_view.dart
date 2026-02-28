/// TweetDetailView — tweet detail + replies + compose reply
/// (mirrors Swift TweetDetailView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';
import 'profile_view.dart';
import 'widgets/tweet_row.dart';

class TweetDetailView extends StatefulWidget {
  final String tweetId;
  const TweetDetailView({super.key, required this.tweetId});

  @override
  State<TweetDetailView> createState() => _TweetDetailViewState();
}

class _TweetDetailViewState extends State<TweetDetailView> {
  final _replyController = TextEditingController();

  @override
  void dispose() {
    _replyController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final detail = store.get<TweetDetailState>('tweet/${widget.tweetId}');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/profile/tweets')),
      ),
      child: SafeArea(child: _buildBody(context, store, detail)),
    );
  }

  Widget _buildBody(
    BuildContext context,
    FluxStore store,
    TweetDetailState? detail,
  ) {
    if (detail == null) {
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

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _mainTweet(context, store, detail.tweet),
          Container(
            height: 1,
            color: CupertinoColors.separator.resolveFrom(context),
          ),
          _replyCompose(context, store),
          Container(
            height: 1,
            color: CupertinoColors.separator.resolveFrom(context),
          ),
          if (detail.replies.isEmpty)
            Padding(
              padding: const EdgeInsets.all(16),
              child: Text(
                store.t('ui/tweet/no_replies'),
                style: const TextStyle(color: CupertinoColors.secondaryLabel),
              ),
            )
          else
            ...detail.replies.map(
              (reply) => Column(
                children: [
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 8,
                    ),
                    child: TweetRow(item: reply),
                  ),
                  Container(
                    height: 1,
                    color: CupertinoColors.separator.resolveFrom(context),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }

  Widget _mainTweet(BuildContext context, FluxStore store, FeedItem tweet) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Author header
          GestureDetector(
            onTap: () {
              Navigator.of(context).push(
                CupertinoPageRoute<void>(
                  builder: (_) => ProfileView(userId: tweet.author.id),
                ),
              );
            },
            child: Row(
              children: [
                Container(
                  width: 48,
                  height: 48,
                  decoration: BoxDecoration(
                    color: CupertinoColors.activeBlue.withAlpha(51),
                    shape: BoxShape.circle,
                  ),
                  alignment: Alignment.center,
                  child: Text(
                    tweet.author.displayName.isNotEmpty
                        ? tweet.author.displayName[0]
                        : '?',
                    style: const TextStyle(
                      fontSize: 20,
                      color: CupertinoColors.activeBlue,
                    ),
                  ),
                ),
                const SizedBox(width: 10),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      tweet.author.displayName,
                      style: const TextStyle(
                        fontSize: 17,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    Text(
                      '@${tweet.author.username}',
                      style: const TextStyle(
                        fontSize: 15,
                        color: CupertinoColors.secondaryLabel,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),

          const SizedBox(height: 12),

          // Content
          Text(tweet.content, style: const TextStyle(fontSize: 20)),

          const SizedBox(height: 12),

          // Actions
          Row(
            children: [
              GestureDetector(
                onTap: () {
                  final store = FluxStoreScope.of(context);
                  if (tweet.likedByMe) {
                    store.emit('tweet/unlike', {'tweetId': tweet.tweetId});
                  } else {
                    store.emit('tweet/like', {'tweetId': tweet.tweetId});
                  }
                },
                child: Row(
                  children: [
                    Icon(
                      tweet.likedByMe
                          ? CupertinoIcons.heart_fill
                          : CupertinoIcons.heart,
                      size: 18,
                      color: tweet.likedByMe
                          ? CupertinoColors.destructiveRed
                          : CupertinoColors.secondaryLabel,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '${tweet.likeCount}',
                      style: const TextStyle(
                        fontSize: 15,
                        color: CupertinoColors.secondaryLabel,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 16),
              Row(
                children: [
                  const Icon(
                    CupertinoIcons.chat_bubble,
                    size: 18,
                    color: CupertinoColors.secondaryLabel,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    '${tweet.replyCount}',
                    style: const TextStyle(
                      fontSize: 15,
                      color: CupertinoColors.secondaryLabel,
                    ),
                  ),
                ],
              ),
            ],
          ),

          const SizedBox(height: 8),

          // Date
          Text(
            _formatDate(tweet.createdAt),
            style: const TextStyle(
              fontSize: 12,
              color: CupertinoColors.secondaryLabel,
            ),
          ),
        ],
      ),
    );
  }

  Widget _replyCompose(BuildContext context, FluxStore store) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Row(
        children: [
          Expanded(
            child: CupertinoTextField(
              controller: _replyController,
              placeholder: store.t('ui/compose/reply_placeholder'),
              onChanged: (_) => setState(() {}),
            ),
          ),
          const SizedBox(width: 8),
          CupertinoButton.filled(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            onPressed: _replyController.text.trim().isEmpty
                ? null
                : () {
                    store.emit('tweet/create', {
                      'content': _replyController.text,
                      'replyToId': widget.tweetId,
                    });
                    _replyController.clear();
                    setState(() {});
                  },
            child: Text(
              store.t('ui/tweet/reply'),
              style: const TextStyle(fontSize: 14),
            ),
          ),
        ],
      ),
    );
  }

  static String _formatDate(String dateStr) {
    final idx = dateStr.indexOf('T');
    if (idx > 0) return dateStr.substring(0, idx);
    return dateStr;
  }
}
