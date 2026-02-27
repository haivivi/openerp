/// HomeView — timeline feed with navigation (mirrors Swift HomeView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';
import 'compose_view.dart';
import 'tweet_detail_view.dart';
import 'widgets/tweet_row.dart';

class HomeView extends StatelessWidget {
  const HomeView({super.key});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final feed = store.get<TimelineFeed>('timeline/feed');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/home/title')),
        trailing: CupertinoButton(
          padding: EdgeInsets.zero,
          child: const Icon(CupertinoIcons.square_pencil),
          onPressed: () {
            Navigator.of(context).push(
              CupertinoPageRoute<void>(builder: (_) => const ComposeView()),
            );
          },
        ),
      ),
      child: SafeArea(child: _buildBody(context, store, feed)),
    );
  }

  Widget _buildBody(BuildContext context, FluxStore store, TimelineFeed? feed) {
    if (feed == null) {
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

    if (feed.items.isEmpty && !feed.loading) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              CupertinoIcons.chat_bubble,
              size: 48,
              color: CupertinoColors.secondaryLabel,
            ),
            const SizedBox(height: 12),
            Text(
              store.t('ui/home/empty'),
              style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 4),
            Text(
              store.t('ui/home/empty_hint'),
              style: const TextStyle(
                fontSize: 15,
                color: CupertinoColors.secondaryLabel,
              ),
            ),
          ],
        ),
      );
    }

    return ListView.separated(
      itemCount: feed.items.length,
      separatorBuilder: (context, index) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final item = feed.items[index];
        return GestureDetector(
          onTap: () {
            Navigator.of(context).push(
              CupertinoPageRoute<void>(
                builder: (_) => TweetDetailView(tweetId: item.tweetId),
              ),
            );
          },
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: TweetRow(item: item),
          ),
        );
      },
    );
  }
}

/// Thin Material-like divider for Cupertino lists.
class Divider extends StatelessWidget {
  final double height;
  const Divider({super.key, this.height = 1});

  @override
  Widget build(BuildContext context) {
    return Container(
      height: height,
      color: CupertinoColors.separator.resolveFrom(context),
    );
  }
}
