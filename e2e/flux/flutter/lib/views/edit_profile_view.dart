/// EditProfileView — edit display name and bio (mirrors Swift EditProfileView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';

class EditProfileView extends StatefulWidget {
  const EditProfileView({super.key});

  @override
  State<EditProfileView> createState() => _EditProfileViewState();
}

class _EditProfileViewState extends State<EditProfileView> {
  final _displayNameController = TextEditingController();
  final _bioController = TextEditingController();
  bool _loaded = false;

  @override
  void dispose() {
    _displayNameController.dispose();
    _bioController.dispose();
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_loaded) {
      final store = FluxStoreScope.of(context);
      final settings = store.get<SettingsState>('settings/state');
      if (settings != null) {
        _displayNameController.text = settings.displayName;
        _bioController.text = settings.bio;
      }
      _loaded = true;
    }
  }

  void _save() {
    final store = FluxStoreScope.of(context);
    store.emit('settings/save', {
      'displayName': _displayNameController.text,
      'bio': _bioController.text,
    });
  }

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final settings = store.get<SettingsState>('settings/state');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/me/edit_profile')),
      ),
      child: SafeArea(
        child: ListView(
          children: [
            // Display name
            CupertinoListSection.insetGrouped(
              header: Text(store.t('ui/edit/display_name')),
              children: [
                CupertinoTextField(
                  controller: _displayNameController,
                  placeholder: store.t('ui/edit/display_name'),
                  padding: const EdgeInsets.all(12),
                  decoration: const BoxDecoration(),
                  onChanged: (_) => setState(() {}),
                ),
              ],
            ),

            // Bio
            CupertinoListSection.insetGrouped(
              header: Text(store.t('ui/edit/bio')),
              children: [
                SizedBox(
                  height: 100,
                  child: CupertinoTextField(
                    controller: _bioController,
                    maxLines: null,
                    expands: true,
                    textAlignVertical: TextAlignVertical.top,
                    padding: const EdgeInsets.all(12),
                    decoration: const BoxDecoration(),
                    onChanged: (_) => setState(() {}),
                  ),
                ),
              ],
            ),

            // Error
            if (settings?.error != null)
              CupertinoListSection.insetGrouped(
                children: [
                  Padding(
                    padding: const EdgeInsets.all(12),
                    child: Text(
                      settings!.error!,
                      style: const TextStyle(
                        color: CupertinoColors.destructiveRed,
                      ),
                    ),
                  ),
                ],
              ),

            // Saved indicator
            if (settings?.saved == true)
              CupertinoListSection.insetGrouped(
                children: [
                  Padding(
                    padding: const EdgeInsets.all(12),
                    child: Row(
                      children: [
                        const Icon(
                          CupertinoIcons.checkmark_circle_fill,
                          color: CupertinoColors.activeGreen,
                          size: 18,
                        ),
                        const SizedBox(width: 8),
                        Text(
                          store.t('ui/edit/saved'),
                          style: const TextStyle(
                            color: CupertinoColors.activeGreen,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),

            // Save button
            CupertinoListSection.insetGrouped(
              children: [
                CupertinoListTile(
                  title: Center(
                    child: settings?.busy == true
                        ? const CupertinoActivityIndicator()
                        : Text(
                            store.t('ui/edit/save'),
                            style: TextStyle(
                              color:
                                  _displayNameController.text.trim().isEmpty ||
                                      settings?.busy == true
                                  ? CupertinoColors.secondaryLabel
                                  : CupertinoColors.activeBlue,
                            ),
                          ),
                  ),
                  onTap:
                      _displayNameController.text.trim().isEmpty ||
                          settings?.busy == true
                      ? null
                      : _save,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
