/// ProfileView — user profile page (mirrors Swift ProfileView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';
import 'widgets/tweet_row.dart';

class ProfileView extends StatelessWidget {
  final String userId;
  const ProfileView({super.key, required this.userId});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final profile = store.get<ProfilePage>('profile/$userId');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(middle: Text('@$userId')),
      child: SafeArea(child: _buildBody(context, store, profile)),
    );
  }

  Widget _buildBody(
    BuildContext context,
    FluxStore store,
    ProfilePage? profile,
  ) {
    if (profile == null) {
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
        children: [
          _profileHeader(context, store, profile),
          Container(
            height: 1,
            color: CupertinoColors.separator.resolveFrom(context),
          ),
          if (profile.tweets.isEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 32),
              child: Text(
                store.t('ui/profile/no_tweets'),
                style: const TextStyle(color: CupertinoColors.secondaryLabel),
              ),
            )
          else
            ...profile.tweets.map(
              (item) => Column(
                children: [
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 8,
                    ),
                    child: TweetRow(item: item),
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

  Widget _profileHeader(
    BuildContext context,
    FluxStore store,
    ProfilePage profile,
  ) {
    final user = profile.user;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        children: [
          // Avatar
          Container(
            width: 72,
            height: 72,
            decoration: BoxDecoration(
              color: CupertinoColors.activeBlue.withAlpha(51),
              shape: BoxShape.circle,
            ),
            alignment: Alignment.center,
            child: Text(
              user.displayName.isNotEmpty ? user.displayName[0] : '?',
              style: const TextStyle(
                fontSize: 28,
                color: CupertinoColors.activeBlue,
              ),
            ),
          ),

          const SizedBox(height: 12),

          Text(
            user.displayName,
            style: const TextStyle(fontSize: 22, fontWeight: FontWeight.bold),
          ),

          const SizedBox(height: 4),

          Text(
            '@${user.username}',
            style: const TextStyle(
              fontSize: 15,
              color: CupertinoColors.secondaryLabel,
            ),
          ),

          if (user.bio != null && user.bio!.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text(
              user.bio!,
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 17),
            ),
          ],

          const SizedBox(height: 12),

          // Stats
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              _stat(user.followerCount, store.t('ui/profile/followers')),
              const SizedBox(width: 24),
              _stat(user.followingCount, store.t('ui/profile/following')),
              const SizedBox(width: 24),
              _stat(user.tweetCount, store.t('ui/profile/tweets')),
            ],
          ),

          const SizedBox(height: 12),

          // Follow / Unfollow button
          if (profile.followedByMe)
            CupertinoButton(
              onPressed: () {
                store.emit('user/unfollow', {'userId': userId});
              },
              child: SizedBox(
                width: 120,
                child: Center(child: Text(store.t('ui/profile/unfollow'))),
              ),
            )
          else
            CupertinoButton.filled(
              onPressed: () {
                store.emit('user/follow', {'userId': userId});
              },
              child: SizedBox(
                width: 120,
                child: Center(child: Text(store.t('ui/profile/follow'))),
              ),
            ),
        ],
      ),
    );
  }

  Widget _stat(int count, String label) {
    return Column(
      children: [
        Text(
          '$count',
          style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
        ),
        Text(
          label,
          style: const TextStyle(
            fontSize: 12,
            color: CupertinoColors.secondaryLabel,
          ),
        ),
      ],
    );
  }
}
