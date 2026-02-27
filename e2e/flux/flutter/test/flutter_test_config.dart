/// flutter_test_config.dart — auto-loaded by `flutter test`.
///
/// Loads Roboto font under multiple family names so ALL text renders
/// correctly in golden screenshots — including CupertinoNavigationBar
/// and other widgets that reference `.SF Pro Text` internally.
library;

import 'dart:async';
import 'dart:io';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

const _fontFiles = [
  'fonts/Roboto-Light.ttf',
  'fonts/Roboto-Regular.ttf',
  'fonts/Roboto-Medium.ttf',
  'fonts/Roboto-Bold.ttf',
];

/// Font families that Cupertino widgets may reference internally.
const _families = [
  'Roboto',
  '.SF Pro Text',
  '.SF Pro Display',
  '.SF UI Text',
  '.SF UI Display',
  'CupertinoSystemText',
  'CupertinoSystemDisplay',
];

Future<void> testExecutable(FutureOr<void> Function() testMain) async {
  TestWidgetsFlutterBinding.ensureInitialized();

  // Load each font weight under every family alias.
  final fontBytes = <ByteData>[];
  for (final path in _fontFiles) {
    final file = File(path);
    if (file.existsSync()) {
      final raw = file.readAsBytesSync();
      fontBytes.add(ByteData.view(Uint8List.fromList(raw).buffer));
    }
  }

  for (final family in _families) {
    final loader = FontLoader(family);
    for (final bytes in fontBytes) {
      loader.addFont(Future.value(bytes));
    }
    await loader.load();
  }

  await testMain();
}
