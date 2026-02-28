/// LoginView — auth/login page (mirrors Swift LoginView).
library;

import 'package:flutter/cupertino.dart';

import '../models/models.dart';
import '../store/flux_store.dart';

class LoginView extends StatefulWidget {
  const LoginView({super.key});

  @override
  State<LoginView> createState() => _LoginViewState();
}

class _LoginViewState extends State<LoginView> {
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();

  @override
  void dispose() {
    _usernameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  void _login() {
    final store = FluxStoreScope.of(context);
    store.emit('auth/login', {
      'username': _usernameController.text,
      'password': _passwordController.text,
    });
  }

  @override
  Widget build(BuildContext context) {
    final store = FluxStoreScope.of(context);
    final auth = store.get<AuthState>('auth/state');

    final username = _usernameController.text;
    final password = _passwordController.text;
    final canSubmit =
        username.isNotEmpty && password.isNotEmpty && auth?.busy != true;

    return CupertinoPageScaffold(
      child: SafeArea(
        child: Column(
          children: [
            const Spacer(),

            // App icon
            const Icon(
              CupertinoIcons.chat_bubble_2_fill,
              size: 48,
              color: CupertinoColors.activeBlue,
            ),

            const SizedBox(height: 24),

            // Title
            const Text(
              'TwitterFlux',
              style: TextStyle(fontSize: 34, fontWeight: FontWeight.bold),
            ),

            const SizedBox(height: 8),

            // Subtitle
            Text(
              'Powered by Flux State Engine',
              style: TextStyle(
                fontSize: 15,
                color: CupertinoColors.secondaryLabel.resolveFrom(context),
              ),
            ),

            const SizedBox(height: 24),

            // Form fields
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: Column(
                children: [
                  CupertinoTextField(
                    controller: _usernameController,
                    placeholder: store.t('ui/login/username'),
                    autocorrect: false,
                    onChanged: (_) => setState(() {}),
                  ),
                  const SizedBox(height: 16),
                  CupertinoTextField(
                    controller: _passwordController,
                    placeholder: store.t('ui/login/password'),
                    obscureText: true,
                    onChanged: (_) => setState(() {}),
                  ),
                  const SizedBox(height: 16),

                  // Sign In button
                  SizedBox(
                    width: double.infinity,
                    child: CupertinoButton.filled(
                      onPressed: canSubmit ? _login : null,
                      child: auth?.busy == true
                          ? const CupertinoActivityIndicator(
                              color: CupertinoColors.white,
                            )
                          : Text(store.t('ui/login/button')),
                    ),
                  ),

                  // Error message
                  if (auth?.error != null) ...[
                    const SizedBox(height: 12),
                    Text(
                      auth!.error!,
                      style: const TextStyle(
                        fontSize: 12,
                        color: CupertinoColors.destructiveRed,
                      ),
                    ),
                  ],
                ],
              ),
            ),

            const Spacer(),

            // Hint
            Padding(
              padding: const EdgeInsets.only(bottom: 16),
              child: Text(
                store.t('ui/login/hint'),
                style: TextStyle(
                  fontSize: 12,
                  color: CupertinoColors.secondaryLabel.resolveFrom(context),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
