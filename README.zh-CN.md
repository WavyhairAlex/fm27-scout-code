[English](README.md) | 中文

# FM27 Scout

macOS 上的 Football Manager 27 球探工作台，带可选的实验性存档编辑器。基于 Tauri 2、Rust 解析核心和 React/TypeScript UI。

> 非官方工具，与 Sports Interactive / SEGA 无关。读取只读；编辑只写入独立的新存档，绝不修改源文件。不上传存档或球员数据。

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-dark.webp">
  <img src="docs/screenshots/main-light.webp" alt="主界面：筛选、可排序球员列表与球员档案面板">
</picture>

## 功能

- 直接读取 FM27 `27.0.0+2486695` 存档；SQLite 缓存 + 服务端分页，4.6 万+ 球员仍保持流畅。
- 筛选、多列排序、球员档案、2–4 人对比、CSV/JSON 导出、角色评分、成长曲线和分析中心。
- 俱乐部工作区：可搜索的俱乐部列表、阵容摘要、月度财政、拥有的球场和公司设施。财政行只覆盖存档跟踪的联赛国家。
- 可选存档编辑器：CA/PA、属性、隐藏属性、Determination、性格和习惯。修改按球员暂存，最后统一写入一个新存档。
- 低风险俱乐部编辑：俱乐部声望、最新月份财政快照（余额、转会预算、周薪预算）、公司设施和自有球场容量。源存档永远不会被修改。
- CA 预算助手：按社区 FM 位置权重实时估算，并提示超预算/低预算。
- 英语 / 简体中文界面，默认跟随系统语言。
- CI 发布 Universal macOS 版本；可选 Developer ID 签名/公证。

完整功能、解析覆盖和验证结果见 [docs/features.md](docs/features.md)。

## 安装

从 [GitHub Releases](https://github.com/shlk20/fm27-scout-code/releases) 下载最新 Universal DMG。需要 macOS 14+。

CI 构建未公证。首次打开请前往 **系统设置 → 隐私与安全性 → 仍要打开**。

## 开发

```bash
cd apps/desktop
pnpm install
pnpm tauri dev
```

目录结构、CLI、构建产物和验证命令见 [docs/development.md](docs/development.md)。

## 文档

- [功能与解析覆盖](docs/features.md)
- [存档编辑器](docs/editor.md)
- [开发、CLI 与验证](docs/development.md)
- [安全边界、已知限制与后续计划](docs/limitations.md)
- [发布流程](docs/release.md)
- [FM27 存档布局](docs/fm27-layout.md)
- [名称表格式](docs/name-tables.md)

## 安全

- 源存档永远不会被修改。编辑只写新文件，拒绝覆盖已有输出，并会重新解析结果做校验。
- 不读取游戏内存、不注入进程、不启用 shell 或任意文件系统插件。
- 除可选的更新检查外不发起网络请求；存档和球员数据不会离开本机。
- 隐藏属性默认关闭。

详见 [docs/limitations.md](docs/limitations.md)。

## 许可证

MIT。解析逻辑参考并移植自 [fmsave](https://github.com/rhiever/fmsave)。详见 [THIRD_PARTY.md](THIRD_PARTY.md)。
