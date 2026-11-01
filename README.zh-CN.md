[English](README.md) | 中文

# FM27 Scout

macOS 上的 Football Manager 27 球探工具，桌面 App 只读：Tauri 2 + Rust 解析核心 + React/TypeScript UI。CLI 另带一个实验性存档编辑器，只输出新存档。

> 非官方工具，与 Sports Interactive / SEGA 无关。桌面 App 和读取命令不修改存档；实验性 `edit` CLI 只写新文件，不碰源存档。不联网，不上传任何数据。

## 界面截图

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-dark.webp">
  <img src="docs/screenshots/main-light.webp" alt="主界面：筛选、可排序球员列表与球员档案面板">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/analysis-dark.webp">
  <img src="docs/screenshots/analysis-light.webp" alt="分析中心：年龄与 CA 分布、俱乐部实力与合同到期">
</picture>

## 当前状态

0.2.1 可运行版本已经完成：

- 直接读取 FM27 `27.0.0+2486695` 存档，容器 / zstd / section 目录全部由 Rust 解析。
- 支持 Finder 双击 / 拖入 `.fm` 文件；解析前先复制只读快照，避免游戏保存中读到半截文件。
- 球员：46,273 条记录，含属性、隐藏属性、CA/PA、位置、性格、习惯、合同、身价、状态。
- 俱乐部：26,214 个，球队映射、声望、上赛季排名。
- 赛事：从存档阶段表解析 1,724 项赛事和 3,459 条阶段记录，其中 1,586 项带编辑器 database_id，可与用户赛事名称表联动。
- 合同：44,341 份，含工资、起止日期、租借、条款。
- 与 Python `fmsave` oracle 的共有字段全量逐字段对比：**0 差异**；名称、角色、伤病等派生字段由回归测试覆盖。
- 解析耗时约 **1.3 秒**（release，M 系列 Mac），含赛事索引。
- SQLite 缓存 + 服务端分页查询；CLI 与桌面 App 共用 `fm27-core`。
- React UI：筛选、多列排序、球员详情、2–4 人对比、CSV/JSON 导出、隐藏属性开关、浅色/深色主题。
- 多语言界面：英语 / 简体中文，默认跟随系统语言，不支持的语言回退英语；可在设置中切换，原生菜单同步切换。
- 分析中心：年龄/CA 分布、俱乐部实力榜、阵容深度、小妖雷达（年龄 + PA + CA 差距）。
- 快捷筛选：小妖、合同年、高潜、伤病 / 伤病史 / 停赛 / 租借 / 自由球员 / 可出战。
- 角色评分：29 个 FM 风格角色按关键属性加权并叠加位置熟练度，详情页显示全部角色与最佳角色。
- 成长曲线：每次解析按存档名 + 游戏日期写入快照，详情页显示 CA / PA / 身价历史。
- 性别与名称：解析俱乐部性别（男子 / 女子），内置 228 个国家/地区中文名称表；赛事名称表支持用户 CSV。
- 筛选联动：俱乐部、国籍、性别、状态计数随当前筛选实时更新。
- 发布配置：CI 发布 Universal Binary DMG；可选本地 Developer ID 签名/公证。
- 应用内更新：Tauri Updater 从 GitHub Releases 清单检查更新，验证更新包签名后安装并重启；签名密钥通过 `scripts/setup-updater-signing.sh` 一次配置。

真实存档未提交到仓库；开发时通过环境变量指定。

## 目录

```text
apps/desktop/          Tauri 2 应用 + React UI
crates/fm27-core/      Rust 解析、SQLite、查询、导出
crates/fm27-cli/       fm27scout CLI
shared/schema.sql      SQLite schema
scripts/               Python oracle 导出与 Rust/oracle 对比
docs/                  布局、解析与验证说明
```

## 构建产物

已完成本地验证的构建：

```text
target/universal-apple-darwin/release/bundle/dmg/FM27 Scout_0.2.1_universal.dmg
target/universal-apple-darwin/release/bundle/macos/FM27 Scout.app
```

本地开发构建也可以只生成当前架构的 `target/release/bundle`。

Universal Binary / 签名 / 公证使用 `scripts/release-macos.sh`，详见
[docs/release.md](docs/release.md)。配置更新签名 secrets 后，CI 在 `v*` tag 上发布 GitHub Release，包含未签名 Universal DMG、带签名的更新包和 SHA-256 校验和。CI 构建未经公证，首次打开需前往「系统设置 → 隐私与安全性 → 仍要打开」。

## 开发环境

- macOS 14+
- Rust stable（`rustup`）
- Node 20+ / pnpm
- Python 3.12 + `fmsave`（仅用于 oracle 对比）

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 前端依赖
cd apps/desktop
pnpm install

# 开发模式
pnpm tauri dev
```

## CLI

```bash
cargo build --release -p fm27-cli

# 容器信息（不解压 game_db）
./target/release/fm27scout inspect "/path/to/save.fm"

# 完整解析报告
./target/release/fm27scout validate "/path/to/save.fm"

# 前 1000 名球员
./target/release/fm27scout players "/path/to/save.fm" --limit 1000 --format json

# 导出 CSV
./target/release/fm27scout players "/path/to/save.fm" --limit 50000 --format csv --output players.csv

# 解析并写入 SQLite 缓存
./target/release/fm27scout index "/path/to/save.fm" --db fm27scout.db

# 赛事索引与编辑器 database_id
./target/release/fm27scout competitions "/path/to/save.fm" --output competitions.json

# 实验功能：把单个球员的修改写入新存档（原存档不会被修改）
./target/release/fm27scout edit "/path/to/save.fm" --uid 228 \
  --ca 150 --pa 199 --attr crossing=100 --attr consistency=100 \
  --personality professionalism=20 --output edited.fm
```

`edit` 子命令的字段和安全说明见 [docs/editor.md](docs/editor.md)。它只支持
build `27.0.0+2486695`，并且始终输出独立的新存档。CA 是属性预算：修改 CA 后 FM 会在
大约一个月内重新分配参与 CA 计算的可见属性；隐藏属性、Determination、性格和习惯
保持稳定。只改可见属性会被压回，推荐 CA + 少量关键属性的工作流。

## 验证

```bash
# Rust 单元测试
cargo test -p fm27-core

# 真实存档回归测试（需要存档）
FM27_TEST_SAVE="$HOME/Library/Application Support/Sports Interactive/Football Manager 27/games/BB KING - Casciavit.fm" \
  cargo test -p fm27-core --release -- --ignored

# Python oracle 导出 + 对比
python3 -m venv .venv
.venv/bin/pip install -e .ref-fmsave    # 开发期本地克隆
.venv/bin/python scripts/oracle_export.py --output /tmp/oracle.json
.venv/bin/python scripts/compare_outputs.py /tmp/oracle.json /tmp/rust.json

# 赛事索引对比
.venv/bin/python scripts/oracle_competitions.py --output /tmp/oracle_competitions.json
./target/release/fm27scout competitions "/path/to/save.fm" --output /tmp/rust_competitions.json
.venv/bin/python scripts/compare_competitions.py /tmp/oracle_competitions.json /tmp/rust_competitions.json
```

参考结果见 [docs/validation.md](docs/validation.md)。名称表格式见
[docs/name-tables.md](docs/name-tables.md)，发布流程见 [docs/release.md](docs/release.md)。

## 安全边界

- 桌面 App 和所有读取命令只读；可选的 `edit` CLI 子命令只写入**新存档**，拒绝覆盖源文件，并会重新解析输出文件做校验。
- 不读取游戏内存，不注入进程。
- 不启用 Tauri `shell` / 任意文件系统插件；只使用用户主动选择的文件。
- 除用户主动触发的更新检查外不发起网络请求；更新只访问配置的 GitHub Releases 地址并下载签名更新包，不上传存档或球员数据。
- 隐藏属性默认关闭；开启时提示仅限单机使用。
- 数据库、日志、快照只写入 `~/Library/Application Support/com.fm27scout.desktop/`；快照解析后删除。

## 已知限制

- 当前只针对 build `27.0.0+2486695` 做了完整验证；其他 FM27 build 会降级并提示，不猜测字段。
- 国家名称已内置；赛事名称不存于存档，需按编辑器 `database_id` 提供 `shared/competition-names.csv`（赛事/阶段索引本身已解析并缓存）。
- 伤病数据来自 `injury_manager`：FM27 把当前伤病的预计复出窗口存在两个 stride-14 数组里，log 数组则是滚动伤病历史。“伤病中”由预计复出窗口推导，不再依赖 typed 数组是否有数据。
- 合同条款、租借和合同链已解析；少数未观测的 fallback 结构会显式留空。
- Windows/Linux 暂不构建。
- CI 发布的是未签名 DMG；首次打开需前往「系统设置 → 隐私与安全性 → 仍要打开」。Developer ID 签名/公证为可选项，CI 不使用，且需要付费 Apple Developer 账号。
- 只有把更新密钥对配置为 GitHub Actions Secrets 后，CI 才会发布签名更新包和 `latest.json`；未配置时应用内更新检查不可用。

## 后续

1. 无障碍细节、Intel 实机回归、更多真实存档边界测试。

## 许可证与归属

本项目代码使用 MIT。解析逻辑参考并移植自 [fmsave](https://github.com/rhiever/fmsave)（MIT）。详见 [THIRD_PARTY.md](THIRD_PARTY.md)。
