import 'package:flutter/cupertino.dart';

import 'app.dart';
import 'store/flux_store.dart';

void main() {
  final store = FluxStore();
  store.emit('app/initialize');
  runApp(TwitterFluxApp(store: store));
}
