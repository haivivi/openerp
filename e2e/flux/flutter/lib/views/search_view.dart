/// SearchView — search users and tweets (mirrors Swift SearchView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';
import 'profile_view.dart';
import 'tweet_detail_view.dart';
import 'widgets/tweet_row.dart';

class SearchView extends StatefulWidget {
  const SearchView({super.key});

  @override
  State<SearchView> createState() => _SearchViewState();
}

class _SearchViewState extends State<SearchView> {
  final _queryController = TextEditingController();

  @override
  void dispose() {
    _queryController.dispose();
    super.dispose();
  }

  void _search() {
    final query = _queryController.text.trim();
    if (query.isEmpty) return;
    FluxStoreScope.of(context).emit('search/query', {'query': query});
  }

  void _clear() {
    _queryController.clear();
    setState(() {});
    FluxStoreScope.of(context).emit('search/clear');
  }

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final results = store.get<SearchState>('search/state');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/search/title')),
      ),
      child: SafeArea(
        child: Column(
          children: [
            // Search bar
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
              child: CupertinoSearchTextField(
                controller: _queryController,
                placeholder: store.t('ui/search/placeholder'),
                onSubmitted: (_) => _search(),
                onSuffixTap: _clear,
              ),
            ),

            // Results
            Expanded(child: _buildResults(context, store, results)),
          ],
        ),
      ),
    );
  }

  Widget _buildResults(
    BuildContext context,
    FluxStore store,
    SearchState? results,
  ) {
    if (results == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              CupertinoIcons.search,
              size: 36,
              color: CupertinoColors.secondaryLabel,
            ),
            const SizedBox(height: 8),
            Text(
              store.t('ui/search/placeholder'),
              style: const TextStyle(color: CupertinoColors.secondaryLabel),
            ),
          ],
        ),
      );
    }

    if (results.loading) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.only(top: 32),
          child: CupertinoActivityIndicator(),
        ),
      );
    }

    if (results.users.isEmpty &&
        results.tweets.isEmpty &&
        results.query.isNotEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.only(top: 32),
          child: Text(
            store.t('ui/search/no_results'),
            style: const TextStyle(color: CupertinoColors.secondaryLabel),
          ),
        ),
      );
    }

    return ListView(
      children: [
        if (results.users.isNotEmpty) ...[
          _sectionHeader(store.t('ui/search/users')),
          ...results.users.map((user) => _userTile(context, store, user)),
        ],
        if (results.tweets.isNotEmpty) ...[
          _sectionHeader(store.t('ui/search/tweets_section')),
          ...results.tweets.map(
            (item) => GestureDetector(
              onTap: () {
                Navigator.of(context).push(
                  CupertinoPageRoute<void>(
                    builder: (_) => TweetDetailView(tweetId: item.tweetId),
                  ),
                );
              },
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 4,
                ),
                child: TweetRow(item: item),
              ),
            ),
          ),
        ],
      ],
    );
  }

  Widget _sectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
      child: Text(
        title,
        style: const TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w600,
          color: CupertinoColors.secondaryLabel,
        ),
      ),
    );
  }

  Widget _userTile(BuildContext context, FluxStore store, UserProfile user) {
    return GestureDetector(
      onTap: () {
        Navigator.of(context).push(
          CupertinoPageRoute<void>(
            builder: (_) => ProfileView(userId: user.id),
          ),
        );
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: CupertinoColors.activeBlue.withAlpha(51),
                shape: BoxShape.circle,
              ),
              alignment: Alignment.center,
              child: Text(
                user.displayName.isNotEmpty ? user.displayName[0] : '?',
                style: const TextStyle(
                  fontSize: 17,
                  fontWeight: FontWeight.w600,
                  color: CupertinoColors.activeBlue,
                ),
              ),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    user.displayName,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  Text(
                    '@${user.username}',
                    style: const TextStyle(
                      fontSize: 12,
                      color: CupertinoColors.secondaryLabel,
                    ),
                  ),
                ],
              ),
            ),
            Text(
              store.t('format/tweet_count?count=${user.tweetCount}'),
              style: const TextStyle(
                fontSize: 11,
                color: CupertinoColors.secondaryLabel,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
