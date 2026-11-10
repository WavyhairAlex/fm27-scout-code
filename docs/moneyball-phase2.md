# Moneyball Phase 2 进度

> 日期：2026-11-03  
> 后续变更：评分已升级到 `stats-core-v2` / `stats-advanced-v2`（固定 300 分钟评分下限、npxG、经验贝叶斯收缩、不确定性区间），见 [`moneyball-phase4.md`](moneyball-phase4.md)。本文的 `min_minutes = 900` SQL 与 v1 权重仅作历史记录。  
> 分支：`feature/moneyball`  
> 状态：Phase 2 主体完成：容器、parser、schema、core/advanced 评分、Stats tab、持久化 `player_metrics`、位置视图、详情分解和多存档回测均已落地。剩余只有受数据限制的联赛强度、更长周期回测和发布收尾。

## 1. 已完成

### 1.1 容器：unlisted span frame

`crates/fm27-core/src/container.rs` 新增：

- `Region`：已列出的 section frame，或目录未列出的 frame 区间。
- `FrameSpan`：一个 zstd frame 的 offset/size/skippable。
- `SaveFile::regions` / `region()` / `region_names()`。
- `SaveFile::walk_region_frames()`：只读 frame/block header，不解压 payload。
- `SaveFile::region_frame_heads()`：读取每个 frame 的压缩前缀，只解压所需 head。
- `SaveFile::read_region_frame()`：按 `FrameSpan` 解压单个 frame。

FM27 的赛季统计 frame 位于：

```text
unlisted_after_non_pl_hist_ls
```

base 存档中该 region：

```text
start       49,254,734
end         62,405,450
frames      1,136
candidates  1
```

目标 frame：

```text
compressed    6,334,678 bytes
decompressed 29,346,698 bytes
```

### 1.2 Season stats parser

新增 `crates/fm27-core/src/season_stats.rs`：

- `SeasonStatsKind`：non_competitive / league / cup / continental / international / overall / calendar_year_overall / calendar_year_international。
- `RawSeasonRecord` / `SeasonStatsWalk`。
- `SeasonPlayer`：uid、name、team_id、goalkeeper。
- `SeasonStatRow`：所有统计字段 + `advanced_available`。
- `read_season_stats()`：定位 frame、walk、关联球员/俱乐部、解码每一行。
- `walk_season_stats()`：严格 walk，检查：
  - section header；
  - record marker / key 唯一；
  - 8 个 own slots；
  - other team count ≤16；
  - rating item count ≤256；
  - form entry count ≤16；
  - 列表结束符 + 16 字节 footer 精确结束。
- 行解码：
  - `line_marker = 01 07`，`line_bytes = 155`；
  - 门将/普通球员 shared words 按 natural position 选择；
  - `goals_outside_box`、`pressures_*` 写 `None`；
  - `distance_km == 0` → `advanced_available = false`。

### 1.3 接入 parse

`ParsedSave` 新增 `season_stats: Vec<SeasonStatRow>`；`ParseReport` 新增 `season_stats` 计数。

- 每次 `parse_save` 在球员解码后读取赛季统计。
- 解析失败不会让整个存档失败：写入 warning，`season_stats` 为空，现有 Value Finder 仍可用。
- 真实存档验证时 warning 只有原有的 markerless player 提示。

### 1.4 SQLite schema / Store

`shared/schema.sql` 新增 `player_season_stats`：

- 主键 `(save_id, player_uid, kind, team_id)`；未知 team 用 0。
- 保留原始计数、xG/xA/xGP、门将字段、`advanced_available`。
- 索引：`(save_id, player_uid)`、`(save_id, kind, minutes)`。
- `advanced_available = false` 时，高级字段写 `NULL`，不把 0 当真实值。
- `player_metrics`：按 `(save_id, player_uid, model, min_minutes)` 保存完整评分行；`model` 使用版本化值 `stats-core-v1` / `stats-advanced-v1`。所有评分、成本、球员/俱乐部/统计展示字段都是普通 SQL 列，`flags_json` / `drivers_json` 只保存变长列表。索引：`(save_id, model, min_minutes, score DESC)` 和 `(save_id, model, position_group, score DESC)`。`saves` 删除时级联清理。

```sql
SELECT player_uid, name, position_group, score, surplus, cost
FROM player_metrics
WHERE save_id = 1 AND model = 'stats-core-v1' AND min_minutes = 900
ORDER BY score DESC
LIMIT 50;
```

`Store::insert_parsed()` 会写入全部统计行；新增 `Store::season_stats_count(save_id)` 用于验证。

### 1.5 统计版评分

新增 `crates/fm27-core/src/stats_moneyball.rs`：

- `StatsMoneyballRequest/Row/Page/Meta`。
- 两种模型：
  - `stats-core-v1`：xG/xA、射正、转化率、传球成功率、传球完成/90、抢断、评分和门将失球/零封；所有球员可评分。
  - `stats-advanced-v1`：加入关键传球、渐进传球、拦截、争顶、解围、封堵、盘带、传中、扑救率、xGP/90；只评分 `advanced_available=true` 的球员。
- 位置组 × 声望层级内计算百分位；K=900 分钟收缩。
- 成本 = `transfer_value + wage × 52 × 3`；未知费用/工资按 Value Finder 的规则处理。
- `production_tier`、`cost_pct_tier`、`surplus`、`score`、`confidence`、`flags`、`drivers`。
- 筛选在评分后应用，保证用户改条件时百分位不漂移。

`Store::stats_moneyball()` 读取 `player_season_stats`，按当前俱乐部/最大分钟去重，然后调用纯函数评分。

评分拆成两步：

- `score_stats_candidates()`：只做百分位/成本/评分，结果与请求过滤器无关。
- `apply_stats_request()`：只做筛选、排序、分页。

`Store` 在内存中按 `(save_id, model, min_minutes)` 缓存评分结果，翻页和改筛选条件不会重新计算百分位；内存未命中时再从 SQLite `player_metrics` 读取，重启后也能复用。`insert_parsed` / `delete_save` 会清空内存缓存，删除存档时 SQLite 行级联清理；`clear_stats_cache()` 用于测试和重新读取 SQLite 行。`player_metrics_count()` / `player_metrics_rows()` 可直接读取物化结果，供 SQL 查询和后续导出使用。

### 1.6 UI / Tauri

- Tauri command：`stats_moneyball`。
- 前端：`useStatsMoneyball()`、`queryStatsMoneyball()`、`StatsMoneyball*` 类型、mock 数据。
- `AnalysisDialog` 新增第五个 tab：**Stats moneyball / 数据版**。
- 控件：core/advanced、位置快捷 tab、联赛、最低 CA、最大年龄、最少分钟、最大费用/周薪、未知费用、排序、分页。
- 预设：Win-now / Resale / Budget；自定义视图保存在本地 `localStorage`。
- 表格：球员/俱乐部/联赛、位置、年龄、CA/PA、分钟、产出、成本、评分、surplus、置信度、flags；点击打开球员详情。
- 每行可展开 “Why?”：显示样本分钟、高级数据是否记录、产出/成本百分位、drivers、flags 和成本拆解。
- 默认只列外场球员；位置选择 `GK` 时进入门将榜，避免门将和普通位置混排。
- 属性表 `drivers` 以 title 显示前 3 个贡献指标。

## 2. 真实存档验证

使用 `BB KING - Parma Calcio.fm`（build `27.0.0+2486695`）：

```text
players        48,000
clubs          26,138
season_stats   156,272
stored rows    156,272
```

与 Phase 0 参考实现的行数完全一致。

所有 `overall` 行中：37,426 行，`advanced_available = true` 24,079 行，缺失高级块 13,347 行（包含低分钟行；≥900 分钟的缺失比例与 Phase 0 的约 37% 一致）。

抽查 Giovanni Fabbian 的 `overall` 行：

| 字段 | Rust parser | Phase 0 reference |
|---|---:|---:|
| minutes | 1,292 | 1,292 |
| starts | 14 | 14 |
| substitute appearances | 1 | 1 |
| goals | 4 | 4 |
| assists | 3 | 3 |
| xG | 4.21 | 4.21 |
| xA | 5.20 | 5.20 |
| passes attempted / completed | 507 / 406 | 507 / 406 |
| progressive passes | 26 | 26 |
| key passes | 29 | 29 |
| interceptions | 23 | 23 |
| distance km | 189.0 | 189.0 |
| average rating | 7.06 | 7.06 |

### 2.1 统计版榜单抽查

使用 `CA ≥ 120`、`年龄 ≤ 30`、`≥900 分钟`：

| 模型 | 可评分人数 | 说明 |
|---|---:|---|
| core | 2,603 | 全量外场球员（GK 默认单独查看） |
| advanced | 2,399 | 仅有高级数据的外场球员 |

Core top 5（默认不混排门将）：

| 球员 | 位置 | 年龄 | CA | 分钟 | 产出 | 成本百分位 | surplus |
|---|---|---:|---:|---:|---:|---:|---:|
| Iván Leguizamón | AM | 24 | 123 | 1,717 | 86.6 | 20.2 | 66.4 |
| Alexander Alvarado | AM | 27 | 120 | 2,000 | 80.6 | 21.0 | 59.5 |
| Marcelo Allende | AM | 27 | 128 | 1,563 | 70.1 | 11.8 | 58.3 |
| Johan Martínez | AM | 24 | 126 | 1,164 | 80.8 | 25.0 | 55.8 |
| David Martínez | MID | 22 | 123 | 1,252 | 76.6 | 21.5 | 55.1 |

门将不参与跨位置混排；在 UI 选择 `GK` 后才进入门将榜。

### 2.2 多存档回测

对 base / v02 / v03 使用 `CA ≥ 120`、`年龄 ≤ 30`、`≥900 分钟`：

| 模型 | base–v02 | base–v03 | v02–v03 | 6/6 共识人数 |
|---|---:|---:|---:|---:|
| core | 44/50 | 44/50 | 47/50 | 28 |
| advanced | 48/50 | 44/50 | 44/50 | 28 |

- Core/advanced 在同一存档的 top 50 重叠约 31–33/50。
- 28 名球员在 6 个榜单中全部出现（Leguizamón、Alvarado、Allende、Johan Martínez、David Martínez、Villamil、Cornejo 等）。
- 外场位置组成（base core）：AM 17、MID 14、ST 7、DC 7、FB 5；最大俱乐部占比 6%，没有单一俱乐部垄断。

### 2.3 结果回测（v03 → v02）

用 v03 的 core top 50 观察下一时间点（v02）：

- 50/50 在 v02 仍有 ≥900 分钟。
- CA：4 人上升，46 人不变，0 人下降。
- 身价：6 人上升，24 人下降，20 人不变；中位变化 0，范围约 -17 万到 +23 万（存档单位）。
- 50/50 都能在 v02 的 materialized metrics 中重新找到评分。

这说明短期（约 1 周）榜单没有出现“球员消失/样本断崖”；但时间跨度太短，不能证明长期预测能力。后续需要更长的生涯快照。

## 3. 测试

新增：

- `container::tests::finds_and_reads_a_frame_in_an_unlisted_region`
- `container::tests::rejects_corrupt_bytes_in_an_unlisted_region`
- `season_stats::tests::walks_and_decodes_a_record`
- `season_stats::tests::rejects_a_repeated_player_key`
- `season_stats::tests::rejects_a_bad_footer_length`
- `stats_moneyball::tests::cheap_production_beats_expensive_production`
- `stats_moneyball::tests::advanced_model_excludes_rows_without_the_advanced_block`
- `stats_moneyball::tests::goalkeepers_are_separate_by_default`
- `stats_moneyball::tests::filters_are_applied_after_scoring`
- `tests/season_stats_real_save.rs`（ignored，需要 `FM27_TEST_SAVE`，同时验证 store 和两种评分模型）

额外抽查：

- 门将 Edoardo Corvi（720 分钟）：saves_held 11、saves_parried 11、saves_tipped 5、
  shots_on_target_faced 36、save% 75.0；与 Phase 0 样本一致。
- 缺失高级块：Jari Litmanen（1,802 分钟）`distance_km = 0`，key passes / progressive
  passes / interceptions 均为 0，`headers_won = Some(0)`；Rust 侧 `advanced_available = false`，
  与 Phase 0 的哨兵规则一致。
- 物化表：真实测试会在清空内存缓存后从 `player_metrics` 读回同一行，比较 CA、分钟、
  score、production、surplus 和 drivers；`player_metrics_rows()` 的行数与 `COUNT(*)` 一致。

验证命令：

```bash
~/.cargo/bin/cargo test -p fm27-core
~/.cargo/bin/cargo check --workspace
~/.cargo/bin/cargo fmt --all -- --check

FM27_TEST_SAVE="/path/to/save.fm" \
  ~/.cargo/bin/cargo test --release -p fm27-core \
  --test season_stats_real_save -- --ignored --nocapture

# 多存档 top 50 回测（输出 JSONL）
~/.cargo/bin/cargo run --release -p fm27-core --example stats_backtest -- \
  "/path/base.fm" "/path/v02.fm" "/path/v03.fm"

# 结果回测：上一时间点 top 50 在下一时间点的分钟/CA/身价
~/.cargo/bin/cargo run --release -p fm27-core --example stats_result_backtest -- \
  "/path/previous.fm" "/path/next.fm"
```

当前结果：

- `cargo test -p fm27-core`：57 个测试通过。
- 真实存档 ignored test：通过（parser + store + core/advanced 评分）。
- 前端 `pnpm build && pnpm test`：通过（38 个测试）。
- Playwright mock smoke：AnalysisDialog 的 Stats tab 能渲染并返回球员行。
- `ParseReport` TS 类型已同步新增 `season_stats`。

## 4. 下一步

Phase 2 功能已基本收尾；Phase 3 的筛选、独立生涯验证、npxG 和权重敏感性实验见 [`moneyball-phase3.md`](moneyball-phase3.md)。剩余工作主要是数据条件限制：

1. **联赛强度**：FM27 存档只给 5 个顶级联赛名，其他联赛无法可靠映射；当前以俱乐部声望层级做 peer group，避免假精度。要接 league table/fixtures 需要额外解析器或外部联赛系数。
2. **更长周期结果回测**：v03→v02 只有约一周，已确认没有样本断崖；需要在更长生涯间隔上验证评分与未来分钟/CA/身价变化。
3. **发布收尾**：`docs/release-notes.md` 已新增；正式发版时再 bump 版本号、打 tag 并更新 `docs/release.md`。

### Phase 2 完成清单

已完成：容器 unlisted frame、season stats parser、`player_season_stats`、core/advanced 评分、Tauri/UI Stats tab、位置快捷 tab、门将分离、Why 详情、Win-now/Resale/Budget 预设、本地自定义视图、合同/市场/置信度/伤病筛选、内存 + `player_metrics` 持久化、SQL 读取方法、多存档 top 50 回测、短期结果回测、release notes。

按用户要求不做：CSV/JSON 导出。

受数据限制：直接联赛强度系数、更长生涯间隔回测。

## 5. 注意事项

- 当前 `read_season_stats` 固定查找 `unlisted_after_non_pl_hist_ls`；未知 build/layout 会返回错误，parse 会写 warning 并回退，不会产出半份统计。
- `goals_outside_box`、`pressures_attempted`、`pressures_completed` 在 FM27 不支持，数据库写 `NULL`。
- `advanced_available` 来自 `distance_km > 0`；不要用高级字段的 0 参与评分。
- `player_season_stats` 的 `team_id = 0` 表示未知球队，不是合法球队 id。
- 同球员可能有多条 `overall` 行（一线队 + B 队）；评分时需要按当前俱乐部/最大分钟去重。
