/// ChangePasswordView — change password form (mirrors Swift ChangePasswordView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';

class ChangePasswordView extends StatefulWidget {
  const ChangePasswordView({super.key});

  @override
  State<ChangePasswordView> createState() => _ChangePasswordViewState();
}

class _ChangePasswordViewState extends State<ChangePasswordView> {
  final _oldPasswordController = TextEditingController();
  final _newPasswordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();

  @override
  void dispose() {
    _oldPasswordController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  void _changePassword() {
    final store = FluxStoreScope.of(context);
    store.emit('settings/change-password', {
      'oldPassword': _oldPasswordController.text,
      'newPassword': _newPasswordController.text,
    });
  }

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final pwState = store.get<PasswordState>('settings/password');

    final canSubmit =
        _oldPasswordController.text.isNotEmpty &&
        _newPasswordController.text.isNotEmpty &&
        _newPasswordController.text == _confirmPasswordController.text;

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/me/change_password')),
      ),
      child: SafeArea(
        child: ListView(
          children: [
            // Current password
            CupertinoListSection.insetGrouped(
              header: Text(store.t('ui/password/current')),
              children: [
                CupertinoTextField(
                  controller: _oldPasswordController,
                  placeholder: store.t('ui/password/current'),
                  obscureText: true,
                  padding: const EdgeInsets.all(12),
                  decoration: const BoxDecoration(),
                  onChanged: (_) => setState(() {}),
                ),
              ],
            ),

            // New password
            CupertinoListSection.insetGrouped(
              header: Text(store.t('ui/password/new')),
              children: [
                CupertinoTextField(
                  controller: _newPasswordController,
                  placeholder: store.t('ui/password/new'),
                  obscureText: true,
                  padding: const EdgeInsets.all(12),
                  decoration: const BoxDecoration(),
                  onChanged: (_) => setState(() {}),
                ),
                CupertinoTextField(
                  controller: _confirmPasswordController,
                  placeholder: store.t('ui/password/confirm'),
                  obscureText: true,
                  padding: const EdgeInsets.all(12),
                  decoration: const BoxDecoration(),
                  onChanged: (_) => setState(() {}),
                ),
              ],
            ),

            // Error
            if (pwState?.error != null)
              CupertinoListSection.insetGrouped(
                children: [
                  Padding(
                    padding: const EdgeInsets.all(12),
                    child: Text(
                      pwState!.error!,
                      style: const TextStyle(
                        color: CupertinoColors.destructiveRed,
                      ),
                    ),
                  ),
                ],
              ),

            // Success
            if (pwState?.success == true)
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
                          store.t('ui/password/changed'),
                          style: const TextStyle(
                            color: CupertinoColors.activeGreen,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),

            // Submit button
            CupertinoListSection.insetGrouped(
              children: [
                CupertinoListTile(
                  title: Center(
                    child: Text(
                      store.t('ui/password/change'),
                      style: TextStyle(
                        color: canSubmit
                            ? CupertinoColors.activeBlue
                            : CupertinoColors.secondaryLabel,
                      ),
                    ),
                  ),
                  onTap: canSubmit ? _changePassword : null,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
