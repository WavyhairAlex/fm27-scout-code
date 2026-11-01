# macOS 发布（Universal DMG）

FM27 Scout 通过 GitHub Actions 发布**未签名、未公证**的 Universal DMG。
这样不需要 Apple Developer Program 年费；代价是用户首次打开时需要手动绕过 Gatekeeper。

## CI 发布

`.github/workflows/release.yml` 不读取任何 Apple Secrets。

推送 `v*` tag 会构建并发布 GitHub Release。手动触发 workflow 时，可以在 `tag` 输入框填写
一个已存在的 tag（例如 `v1.0.0`）来发布。留空时使用当前选择的 ref：从 tag 触发会发布
Release，从分支触发则只构建并上传 CI artifact。

发布前必须保证 tag 版本与 `apps/desktop/src-tauri/tauri.conf.json` 的 `version` 一致
（例如 tag `v1.0.0` 对应 `1.0.0`），否则 workflow 会在构建前失败。

Release 附件包括：

- `fm27-scout-<version>-universal.dmg`：未签名的 Universal DMG。
- `fm27-scout-<version>-universal.app.tar.gz`：应用内更新包（配置了更新签名时）。
- `fm27-scout-<version>-universal.app.tar.gz.sig`：更新包签名。
- `latest.json`：Tauri Updater 清单，`darwin-aarch64` / `darwin-x86_64` 均指向 Universal 包。
- `SHA256SUMS.txt`：所有发布附件的 SHA-256。

GitHub 平台还会自动附带 `Source code (zip)` / `Source code (tar.gz)`；这是平台行为，
workflow 不额外生成源码压缩包。

构建会运行 Rust/前端测试，检查双架构，并用 `hdiutil verify` 检查 DMG。

## 首次打开未签名版本

1. 打开 DMG，把 **FM27 Scout** 拖进「应用程序」。
2. 先尝试打开一次 App。
3. 打开「系统设置 → 隐私与安全性」，向下滚动，点击「仍要打开」。
4. 确认打开。

也可以在终端移除隔离标记：

```bash
xattr -dr com.apple.quarantine "/Applications/FM27 Scout.app"
open "/Applications/FM27 Scout.app"
```

未签名版本没有经过 Apple 公证，只应从本仓库的 Release 页面下载。

## 应用内更新（Tauri Updater）

应用从 GitHub Releases 的 `latest.json` 检查更新。更新包使用 Tauri 的 minisign
签名验证；公钥在构建时通过 `FM27_UPDATER_PUBKEY` 编译进应用，私钥只存在于 CI Secrets。

首次配置：

```bash
scripts/setup-updater-signing.sh
```

脚本会生成私钥到 `~/.tauri/fm27-scout-updater.key`（不会写入仓库），把启用签名所需的
Tauri 配置写到 `~/.tauri/fm27-scout-updater.tauri.json`，并通过 `gh` 设置 GitHub Actions
Secrets：

- `FM27_UPDATER_PUBKEY`：公钥内容，CI 构建时合并进 Tauri 配置。
- `TAURI_SIGNING_PRIVATE_KEY`：私钥内容，CI 签名更新包时使用。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码（生成时未设置密码则不存在）。

也可以手动配置：

```bash
pnpm --dir apps/desktop tauri signer generate -w ~/.tauri/fm27-scout-updater.key
gh secret set FM27_UPDATER_PUBKEY --body "$(cat ~/.tauri/fm27-scout-updater.key.pub)"
gh secret set TAURI_SIGNING_PRIVATE_KEY --body "$(cat ~/.tauri/fm27-scout-updater.key)"

jq -n --arg pubkey "$(cat ~/.tauri/fm27-scout-updater.key.pub)" \
  '{bundle:{createUpdaterArtifacts:true},plugins:{updater:{pubkey:$pubkey}}}' \
  > ~/.tauri/fm27-scout-updater.tauri.json
```

未配置 secrets 时，release workflow 会记录一条 warning 并继续构建普通 DMG，但不会生成
`latest.json`，应用内更新也不会通过签名校验。配置 secrets 后，workflow 会把公钥合并进
Tauri 配置、启用 `createUpdaterArtifacts`，签名更新包并上传清单。

本地验证签名构建（`setup-updater-signing.sh` 已生成配置文件）：

```bash
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/fm27-scout-updater.key"
cd apps/desktop
pnpm tauri build --target universal-apple-darwin --bundles app,dmg \
  --config "$HOME/.tauri/fm27-scout-updater.tauri.json"
```

应用内更新与 Apple 公证相互独立：更新包仍会替换为未公证的 `.app`。如果将来恢复
Developer ID 签名与公证，更新包也会沿用同一签名流程。

## 可选：本地签名与公证

需要付费 Apple Developer Program 账号。`scripts/release-macos.sh` 仍保留完整的签名、
公证和验证流程：

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"

scripts/release-macos.sh
```

脚本会：

1. 安装 `aarch64-apple-darwin` 与 `x86_64-apple-darwin` target。
2. 构建 Universal `.app` + `.dmg`。
3. 用 `lipo` 检查双架构。
4. 用 `codesign --verify --deep --strict` 检查签名。
5. 用 `spctl --assess` 与 `xcrun stapler validate` 检查 Gatekeeper 与公证票据。
6. 生成 `SHA256SUMS.txt`。

产物：

```text
target/universal-apple-darwin/release/bundle/macos/FM27 Scout.app
target/universal-apple-darwin/release/bundle/dmg/FM27 Scout_1.0.0_universal.dmg
```

如果以后要恢复 CI 签名发布，需要在 workflow 中重新加入 Apple Secrets：
`APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、
`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`。

## 配置

- `apps/desktop/src-tauri/tauri.conf.json`：`hardenedRuntime: true`，
  `minimumSystemVersion: 14.0`，entitlements 指向 `entitlements.plist`。
- `apps/desktop/src-tauri/entitlements.plist`：WKWebView 在 hardened runtime 下需要的
  JIT 权限。
- 不启用 App Sandbox；当前发布方式为未签名 DMG。
- 没有签名环境时，`pnpm tauri build --target universal-apple-darwin --bundles app,dmg`
  会生成 ad-hoc 签名的可执行文件；它能在本机运行，但未通过 Gatekeeper。

## 本地验证

```bash
hdiutil verify "target/universal-apple-darwin/release/bundle/dmg/FM27 Scout_1.0.0_universal.dmg"
hdiutil attach "target/universal-apple-darwin/release/bundle/dmg/FM27 Scout_1.0.0_universal.dmg"
cp -R "/Volumes/FM27 Scout/FM27 Scout.app" /tmp/
lipo -info /tmp/"FM27 Scout.app"/Contents/MacOS/*
```

未签名构建在本地运行时，如果被 Gatekeeper 拦截，使用上面的「仍要打开」或
`xattr -dr com.apple.quarantine`。
