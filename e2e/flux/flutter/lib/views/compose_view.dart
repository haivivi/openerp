/// ComposeView — tweet compose page (mirrors Swift ComposeView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';

class ComposeView extends StatefulWidget {
  const ComposeView({super.key});

  @override
  State<ComposeView> createState() => _ComposeViewState();
}

class _ComposeViewState extends State<ComposeView> {
  final _contentController = TextEditingController();

  int get _charCount => _contentController.text.length;
  bool get _isOverLimit => _charCount > 280;

  @override
  void dispose() {
    _contentController.dispose();
    super.dispose();
  }

  void _post() {
    final store = FluxStoreScope.of(context);
    store.emit('tweet/create', {'content': _contentController.text});
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final compose = store.get<ComposeState>('compose/state');

    final canPost =
        _contentController.text.trim().isNotEmpty &&
        !_isOverLimit &&
        compose?.busy != true;

    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text(store.t('ui/compose/title')),
        leading: CupertinoButton(
          padding: EdgeInsets.zero,
          child: Text(store.t('ui/compose/cancel')),
          onPressed: () => Navigator.of(context).pop(),
        ),
        trailing: CupertinoButton(
          padding: EdgeInsets.zero,
          onPressed: canPost ? _post : null,
          child: Text(
            store.t('ui/compose/post'),
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
        ),
      ),
      child: SafeArea(
        child: Column(
          children: [
            // Text editor area
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: CupertinoTextField(
                  controller: _contentController,
                  maxLines: null,
                  expands: true,
                  textAlignVertical: TextAlignVertical.top,
                  decoration: const BoxDecoration(),
                  onChanged: (_) => setState(() {}),
                ),
              ),
            ),

            // Divider
            Container(
              height: 1,
              color: CupertinoColors.separator.resolveFrom(context),
            ),

            // Bottom bar: char count + error
            Padding(
              padding: const EdgeInsets.all(16),
              child: Row(
                children: [
                  Text(
                    store.t('format/char_count?current=$_charCount&max=280'),
                    style: TextStyle(
                      fontSize: 12,
                      color: _isOverLimit
                          ? CupertinoColors.destructiveRed
                          : CupertinoColors.secondaryLabel,
                    ),
                  ),
                  const Spacer(),
                  if (compose?.error != null)
                    Text(
                      compose!.error!,
                      style: const TextStyle(
                        fontSize: 12,
                        color: CupertinoColors.destructiveRed,
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
