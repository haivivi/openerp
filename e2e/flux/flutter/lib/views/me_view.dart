/// MeView — current user's profile + settings (mirrors Swift MeView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';
import 'change_password_view.dart';
import 'edit_profile_view.dart';
import 'language_picker_view.dart';

class MeView extends StatelessWidget {
  const MeView({super.key});

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final auth = store.get<AuthState>('auth/state');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/me/title')),
      ),
      child: SafeArea(
        child: ListView(
          children: [
            // User info section
            if (auth?.user != null)
              _userInfoSection(context, store, auth!.user!),

            // Settings section
            CupertinoListSection.insetGrouped(
              header: Text(store.t('ui/me/settings')),
              children: [
                CupertinoListTile(
                  leading: const Icon(CupertinoIcons.person_crop_circle),
                  title: Text(store.t('ui/me/edit_profile')),
                  trailing: const CupertinoListTileChevron(),
                  onTap: () {
                    Navigator.of(context).push(
                      CupertinoPageRoute<void>(
                        builder: (_) => const EditProfileView(),
                      ),
                    );
                  },
                ),
                CupertinoListTile(
                  leading: const Icon(CupertinoIcons.lock),
                  title: Text(store.t('ui/me/change_password')),
                  trailing: const CupertinoListTileChevron(),
                  onTap: () {
                    Navigator.of(context).push(
                      CupertinoPageRoute<void>(
                        builder: (_) => const ChangePasswordView(),
                      ),
                    );
                  },
                ),
                CupertinoListTile(
                  leading: const Icon(CupertinoIcons.globe),
                  title: Text(store.t('ui/me/language')),
                  additionalInfo: Text(store.t('ui/lang/current')),
                  trailing: const CupertinoListTileChevron(),
                  onTap: () {
                    Navigator.of(context).push(
                      CupertinoPageRoute<void>(
                        builder: (_) => const LanguagePickerView(),
                      ),
                    );
                  },
                ),
              ],
            ),

            // Developer section
            CupertinoListSection.insetGrouped(
              header: Text(store.t('ui/me/developer')),
              children: [
                CupertinoListTile(
                  leading: const Icon(CupertinoIcons.globe),
                  title: Text(store.t('ui/me/admin_dashboard')),
                ),
                CupertinoListTile(
                  title: Text(
                    store.serverURL,
                    style: const TextStyle(
                      fontSize: 12,
                      color: CupertinoColors.secondaryLabel,
                    ),
                  ),
                ),
              ],
            ),

            // Sign out section
            CupertinoListSection.insetGrouped(
              children: [
                CupertinoListTile(
                  leading: const Icon(
                    CupertinoIcons.square_arrow_right,
                    color: CupertinoColors.destructiveRed,
                  ),
                  title: Text(
                    store.t('ui/me/sign_out'),
                    style: const TextStyle(
                      color: CupertinoColors.destructiveRed,
                    ),
                  ),
                  onTap: () {
                    store.emit('auth/logout');
                  },
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _userInfoSection(
    BuildContext context,
    FluxStore store,
    UserProfile user,
  ) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 16),
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
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: Text(
                user.bio!,
                textAlign: TextAlign.center,
                style: const TextStyle(fontSize: 17),
              ),
            ),
          ],
          const SizedBox(height: 12),
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
