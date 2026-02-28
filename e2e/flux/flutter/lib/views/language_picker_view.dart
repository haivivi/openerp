/// LanguagePickerView — language selection (mirrors Swift LanguagePickerView).
library;

import 'package:flutter/cupertino.dart';

import '../store/flux_store.dart';

class LanguagePickerView extends StatelessWidget {
  const LanguagePickerView({super.key});

  static const _languages = [
    (code: 'en', flag: '🇺🇸', name: 'English'),
    (code: 'zh-CN', flag: '🇨🇳', name: '简体中文'),
    (code: 'ja', flag: '🇯🇵', name: '日本語'),
    (code: 'es', flag: '🇪🇸', name: 'Español'),
  ];

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final currentCode = store.t('ui/lang/code');

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/me/language')),
      ),
      child: SafeArea(
        child: ListView.builder(
          itemCount: _languages.length,
          itemBuilder: (context, index) {
            final lang = _languages[index];
            final isSelected = currentCode == lang.code;

            return GestureDetector(
              onTap: () {
                store.setLocale(lang.code);
              },
              child: Container(
                color: CupertinoColors.systemBackground.resolveFrom(context),
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 14,
                ),
                child: Row(
                  children: [
                    Text(lang.flag, style: const TextStyle(fontSize: 22)),
                    const SizedBox(width: 12),
                    Text(lang.name, style: const TextStyle(fontSize: 17)),
                    const Spacer(),
                    if (isSelected)
                      const Icon(
                        CupertinoIcons.checkmark,
                        color: CupertinoColors.activeBlue,
                        size: 20,
                      ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}
