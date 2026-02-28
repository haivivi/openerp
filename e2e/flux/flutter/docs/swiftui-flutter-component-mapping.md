# SwiftUI ↔ Flutter Cupertino 组件 1:1 映射

> 基于 `e2e/flux/swift/` 与 `e2e/flux/flutter/` 的实际代码。

---

## 1. App 结构 & 路由

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `@main struct App : App` | `CupertinoApp` | |
| `WindowGroup { RootView() }` | `CupertinoApp(home: RootView())` | Flutter 无 WindowGroup 概念 |
| `@StateObject var store` | 外部注入 `FluxStore` 实例 | |
| `.environmentObject(store)` | `FluxStoreScope`（自定义 `InheritedNotifier`） | |
| `@EnvironmentObject var store` | `FluxStoreScope.of(context)` | |
| `if !isLoggedIn { LoginView() } else { MainTabView() }` | 三元表达式返回不同 Widget | 完全一致 |

## 2. 导航

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `NavigationStack` | `CupertinoPageScaffold` + `CupertinoNavigationBar` | |
| `.navigationTitle("Home")` | `CupertinoNavigationBar(middle: Text("Home"))` | |
| `.navigationBarTitleDisplayMode(.inline)` | CupertinoNavigationBar 默认 inline | |
| `.navigationBarTitleDisplayMode(.large)` | `CupertinoSliverNavigationBar` | 当前未使用 |
| `NavigationLink(destination: Page()) { content }` | `GestureDetector(onTap: () => Navigator.push(CupertinoPageRoute(...)))` | 列表行 |
| `NavigationLink` 在 List Section 内 | `CupertinoListTile(trailing: CupertinoListTileChevron(), onTap:)` | 设置页 |
| `.toolbar { ToolbarItem(.topBarTrailing) { Button } }` | `CupertinoNavigationBar(trailing: CupertinoButton(...))` | |
| `.toolbar { ToolbarItem(.cancellationAction) { Button } }` | `CupertinoNavigationBar(leading: CupertinoButton(...))` | |
| `.toolbar { ToolbarItem(.confirmationAction) { Button } }` | `CupertinoNavigationBar(trailing: CupertinoButton(...))` | |
| `@Environment(\.dismiss)` + `dismiss()` | `Navigator.of(context).pop()` | |

## 3. Tab 导航

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `TabView(selection: $tab)` | `CupertinoTabScaffold` | |
| `.tabItem { Image(systemName:); Text(...) }` | `CupertinoTabBar` + `BottomNavigationBarItem` | |
| `.tag(0)` | `tabBuilder` 中 `index` 参数 | |
| `@State selectedTab` | CupertinoTabScaffold 内部管理 | |
| Tab 内 `NavigationStack` | `CupertinoTabView(builder:)` | 每 Tab 独立 Navigator |

## 4. 列表 & 表单

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `List { }.listStyle(.plain)` | `ListView.separated` | 时间线/搜索结果 |
| `List { }.listStyle(.insetGrouped)` | `ListView` + `CupertinoListSection.insetGrouped` | Me/设置页 |
| `Form { Section { } }` | `ListView` + `CupertinoListSection.insetGrouped` | 编辑/密码表单 |
| `Section("Header") { ... }` | `CupertinoListSection.insetGrouped(header: Text("Header"), children:)` | |
| `Section { ... }` (无 header) | `CupertinoListSection.insetGrouped(children:)` | |
| `ForEach(items) { item in ... }` | `ListView.builder(itemBuilder:)` 或 `...items.map(...)` | |
| `ScrollView { LazyVStack { } }` | `SingleChildScrollView(child: Column(...))` 或 `ListView` | |
| `.refreshable { }` | `CupertinoSliverRefreshControl` | 当前未实现 |
| `Divider()` | `Container(height: 1, color: CupertinoColors.separator)` | |

## 5. 列表行

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `NavigationLink { Label(text, systemImage:) }` | `CupertinoListTile(leading: Icon, title: Text, trailing: CupertinoListTileChevron(), onTap:)` | |
| `HStack { Label(...) Spacer() Text(info) }` | `CupertinoListTile(title:, additionalInfo: Text(info), trailing:)` | |
| `Button(role: .destructive) { Label(...) }` | `CupertinoListTile(title: Text(style: destructiveRed), leading: Icon(color: destructiveRed), onTap:)` | |
| `Label(text, systemImage:)` | `Row(children: [Icon(CupertinoIcons.xxx), SizedBox(width: 8), Text(...)])` | 非列表中 |

## 6. 输入控件

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `TextField("placeholder", text: $binding)` | `CupertinoTextField(placeholder:, controller:)` | |
| `.textFieldStyle(.roundedBorder)` | CupertinoTextField 默认圆角 | |
| `SecureField("placeholder", text: $binding)` | `CupertinoTextField(obscureText: true, placeholder:, controller:)` | |
| `TextEditor(text: $binding)` | `CupertinoTextField(maxLines: null, expands: true, textAlignVertical: TextAlignVertical.top)` | |
| `.autocapitalization(.none)` | `autocorrect: false` | |
| 自定义 `HStack { Image TextField Button }` 搜索栏 | `CupertinoSearchTextField` | Flutter 内建 |

## 7. 按钮

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `Button { } label: { Text(...) }.buttonStyle(.borderedProminent)` | `CupertinoButton.filled(onPressed:, child:)` | 主操作 |
| `Button { } label: { Text(...) }.buttonStyle(.bordered)` | `CupertinoButton(onPressed:, child:)` | 次要操作 |
| `Button { } label: { ... }.buttonStyle(.plain)` | `GestureDetector(onTap:, child:)` | 无样式点击 |
| `Button(role: .destructive)` | 用红色文字的 `CupertinoListTile(onTap:)` | |
| `.disabled(condition)` | `onPressed: condition ? handler : null` | null 禁用 |
| `.bold()` | `TextStyle(fontWeight: FontWeight.bold)` | |

## 8. 加载指示

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `ProgressView()` | `CupertinoActivityIndicator()` | |
| `ProgressView("Loading...")` | `Column(children: [CupertinoActivityIndicator(), Text("Loading...")])` | |

## 9. 布局容器

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `VStack(spacing: N)` | `Column` + `SizedBox(height: N)` | |
| `VStack(alignment: .leading)` | `Column(crossAxisAlignment: CrossAxisAlignment.start)` | |
| `HStack(spacing: N)` | `Row` + `SizedBox(width: N)` | |
| `Spacer()` | `Spacer()` | 完全一致 |
| `.padding(.horizontal, 16)` | `Padding(padding: EdgeInsets.symmetric(horizontal: 16))` | |
| `.padding(.vertical, 8)` | `Padding(padding: EdgeInsets.symmetric(vertical: 8))` | |
| `.padding(16)` | `Padding(padding: EdgeInsets.all(16))` | |
| `.frame(width: W, height: H)` | `SizedBox(width: W, height: H)` | |
| `.frame(maxWidth: .infinity)` | `SizedBox(width: double.infinity)` | |
| `Group { if/else }` | 三元表达式或 if/else Widget | |

## 10. 文字样式

| SwiftUI | Flutter TextStyle | 备注 |
|---------|-------------------|------|
| `.font(.largeTitle.bold())` | `fontSize: 34, fontWeight: FontWeight.bold` | |
| `.font(.title)` | `fontSize: 28, fontWeight: FontWeight.bold` | |
| `.font(.title2.bold())` | `fontSize: 22, fontWeight: FontWeight.bold` | |
| `.font(.title3)` | `fontSize: 20` | |
| `.font(.headline)` | `fontSize: 17, fontWeight: FontWeight.w600` | |
| `.font(.subheadline)` | `fontSize: 15` | |
| `.font(.subheadline.bold())` | `fontSize: 15, fontWeight: FontWeight.bold` | |
| `.font(.body)` | `fontSize: 17` | |
| `.font(.caption)` | `fontSize: 12` | |
| `.font(.caption2)` | `fontSize: 11` | |
| `.font(.system(size: N))` | `fontSize: N` | |
| `.foregroundColor(.primary)` | `color: CupertinoColors.label` | |
| `.foregroundColor(.secondary)` | `color: CupertinoColors.secondaryLabel` | |
| `.foregroundColor(.blue)` | `color: CupertinoColors.activeBlue` | |
| `.foregroundColor(.red)` | `color: CupertinoColors.destructiveRed` | |
| `.foregroundColor(.green)` | `color: CupertinoColors.activeGreen` | |
| `.foregroundColor(.orange)` | `color: CupertinoColors.activeOrange` | |
| `.foregroundColor(.white)` | `color: CupertinoColors.white` | |
| `.multilineTextAlignment(.center)` | `textAlign: TextAlign.center` | |
| `.lineLimit(N)` | `maxLines: N, overflow: TextOverflow.ellipsis` | |
| `.bold()` | `fontWeight: FontWeight.bold` | |

## 11. 图形 & 装饰

| SwiftUI | Flutter Cupertino | 备注 |
|---------|-------------------|------|
| `Circle().fill(Color.blue.opacity(0.2)).frame(W,H).overlay(Text)` | `Container(width: W, height: H, decoration: BoxDecoration(color: activeBlue.withAlpha(51), shape: BoxShape.circle), child: Text)` | 头像 |
| `.background(Color.red).cornerRadius(4)` | `Container(decoration: BoxDecoration(color: destructiveRed, borderRadius: BorderRadius.circular(4)))` | Badge |
| `Color(.systemGray6)` | `CupertinoColors.systemGrey6` | |

## 12. SF Symbols → CupertinoIcons

| SF Symbol | CupertinoIcons | 出现位置 |
|-----------|---------------|---------|
| `house.fill` | `.house_fill` | Tab |
| `magnifyingglass` | `.search` | Tab, Search |
| `tray.fill` | `.tray_fill` | Tab |
| `tray` | `.tray` | Inbox 空态 |
| `person.fill` | `.person_fill` | Tab |
| `bubble.left.and.bubble.right.fill` | `.chat_bubble_2_fill` | Login icon |
| `bubble.left` | `.chat_bubble` | Home 空态 |
| `bubble.right` | `.chat_bubble` | TweetRow 回复数 |
| `square.and.pencil` | `.square_pencil` | Home toolbar |
| `arrowshape.turn.up.left` | `.reply` | TweetRow 回复标签 |
| `heart` | `.heart` | TweetRow 未赞 |
| `heart.fill` | `.heart_fill` | TweetRow 已赞 |
| `megaphone` | `.speaker_2` | Inbox broadcast |
| `gear` | `.gear` | Inbox system |
| `person` | `.person` | Inbox personal |
| `envelope` | `.envelope` | Inbox default |
| `person.crop.circle` | `.person_crop_circle` | Me 编辑资料 |
| `lock` | `.lock` | Me 改密码 |
| `globe` | `.globe` | Me 语言/Dashboard |
| `rectangle.portrait.and.arrow.right` | `.square_arrow_right` | Me 退出 |
| `checkmark.circle.fill` | `.checkmark_circle_fill` | 保存成功 |
| `checkmark` | `.checkmark` | 语言选中 |
| `xmark.circle.fill` | (CupertinoSearchTextField 内建) | 搜索清除 |

## 13. 状态管理

| SwiftUI | Flutter | 备注 |
|---------|---------|------|
| `@State var x` | `StatefulWidget` + `setState()` | 本地 UI 状态 |
| `@StateObject var store` | 外部注入 `FluxStore` | App 级别 |
| `@EnvironmentObject var store` | `FluxStoreScope.of(context)` | 子树访问 |
| `@Environment(\.dismiss)` | `Navigator.of(context).pop()` | 关闭页面 |
| `store.get("path") as T?` | `store.get<T>("path")` | 读状态 |
| `store.emit("path", json:)` | `store.emit("path", {json})` | 发 action |
| `store.t("key")` | `store.t("key")` | 翻译 |

## 14. 已知差异

| 方面 | SwiftUI | Flutter Cupertino | 影响 |
|------|---------|-------------------|------|
| Large Title | `.navigationBarTitleDisplayMode(.large)` 自动 | 需手动用 `CupertinoSliverNavigationBar` | 当前用 inline |
| Pull-to-refresh | `.refreshable { }` 原生 | 需 `CupertinoSliverRefreshControl` | 未实现 |
| 文字选择 | `.textSelection(.enabled)` | 需 `SelectableText` | 未实现 |
| URL 打开 | `Link(destination: url)` | 需 `url_launcher` 包 | 未实现 |
| 字体 | SF Pro (系统) | Roboto (bundled for tests) | Golden 差异 |
| List 动画 | SwiftUI 自动 transition | 需手动 `AnimatedList` | 未实现 |
