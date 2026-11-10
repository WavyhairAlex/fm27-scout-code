# Moneyball Sprint A：评分层修正（2026-11-09）

> 状态：已完成 `stats-core-v2` / `stats-advanced-v2` 的评分层修正；Value Finder 升级为 `value-finder-v2`，只同步了位置分组和伤病窗口（它的置信度仍是数据完整度，不含样本量）。
> 本阶段不涉及解析器改动，不 bump `PARSER_VERSION`；模型名升级负责让旧缓存失效。
> 后续变更：Sprint B 加入公平价格模型（`moneyball-phase5.md`），Sprint C 再加入子位置、统计字段、时间窗口并升级到 v3（`moneyball-phase6.md`）；本文的 v2 名称和“尚未做”列表是当时的记录。

## 0. Product decisions taken

本轮先拍板了四个方向，Sprint A 只落地前两个的直接结果：

1. **质量与价格分开**：`score` 将来只表示质量，价格单独成列（公平价格模型在 Sprint B）。
2. **xG → npxG**：本阶段已完成。
3. **解析联赛积分榜/赛程做对手强度**：留到后续阶段（工程量大）。
4. **Full / Fair-play / Moneyball 三模式**：留到后续阶段（隐藏属性/CA/PA 的展示层切换）。

## 1. 为什么要改

对照外部做法（[Moneyball Scout 方法论](https://moneyballscoutml.com/methodology)、[StatsBomb 雷达](https://www.hudl.com/blog/new-statsbomb-radars-2023-update)、[ScoutSheet](https://scoutsheet.fm/)）和转会费模型文献，v1 有六处会直接改变结论的问题：

| 问题 | v1 行为 | 影响 |
|---|---|---|
| 百分位基准随 `min_minutes` 变化 | 先按用户阈值过滤，再算均值/百分位 | 改一次滑块，所有人的排名都会动，和文档承诺相反 |
| 声望档没有最小样本量 | 6 位置组 × 6 声望档直接算百分位 | 某个档只有几个人时会出现 0/50/100 分位 |
| 所有指标按分钟收缩 | 传球成功率、抢断成功率、争顶率也用分钟 | 比率应该按“尝试次数”收缩，分钟不是样本量 |
| core 门将没有扑救数据 | 失球/90 + 零封/90 + 评分 | core 门将榜实际是球队防守榜 |
| ST 权重含转化率 | `goals/shots` 占 20% | 含点球且不稳定；npxG 已解析却没用上 |
| 置信度是加法清单 | 0.55 起加项，满分恰好 1.0 | 不是不确定性度量，`min_confidence` 筛选几乎不起作用 |

## 2. 本阶段改动

### 2.1 固定评分下限

- 新增 `SCORE_FLOOR_MINUTES = 300`（`stats_moneyball.rs`）。
- 所有 300 分钟以上的球员只评分一次；`min_minutes` 改为**展示过滤**（`apply_stats_request`）。
- 缓存键从 `(save_id, model, min_minutes)` 改成 `(save_id, model)`。
- `player_metrics.min_minutes = 0` 成为固定下限评分的哨兵值。旧的多阈值行仍留在旧模型名下，查询不会命中。
- 对外 API `player_metrics_count(save_id, model)`、`player_metrics_rows(save_id, model)` 不再接收 `min_minutes`。

结果：拖动“最少分钟”只改变显示范围，不改百分位。

### 2.2 对等组最小样本量

- `MIN_PEER_SAMPLE = 30`：声望档不足 30 人时，质量与成本百分位都回退到位置组。
- 新增行字段 `peer_count`：实际使用的对等组人数；“为什么是他”面板会显示 `同层共 N 人`。
- 成本百分位同样支持回退，避免薄档直接丢行。

### 2.3 按指标类型的经验贝叶斯收缩

`Metric` 现在区分两类：

- **比率**（传球成功率、抢断成功率、争顶成功率、扑救率）：Beta-Binomial 先验，样本量 = 尝试次数。
- **计数/每 90 指标**（npxG/90、xA/90、射正、传球、抢断、解围、失球、零封、xGP 等）：Gamma-Poisson 先验，样本量 = 分钟。

先验强度用矩估计按“位置组 × 指标”拟合：

```text
between = Var(观察值) − E[within]
within  = 比率: p(1−p)/attempts      计数: 90 × mean / minutes
K       = 比率: p(1−p)/between       计数: 90 × mean / between
```

- K 的上下限：比率 `[10, 1000]` 次尝试，计数 `[90, 3600]` 分钟（上限约一个完整赛季的证据量）。
- 组内少于 30 人或方差退化时回退到固定先验：比率 200 次尝试，计数 900 分钟。
- 收缩目标仍是“位置组 × 声望档”的均值；先验强度用整个位置组拟合，避免薄档自己定先验。

### 2.4 npxG 与去转化率

- `Metric::Xg90` → `Metric::Npxg90`，读取 `player_season_stats.non_penalty_expected_goals`，标签 `npxG/90`。
- 删除 `Conversion`（`goals/shots`），进球/期望的过度表现不进入评分。
- 新权重（总和为 1）：

| 位置 | core | advanced |
|---|---|---|
| ST | npxG 0.45 · xA 0.25 · 射正 0.30 | npxG 0.35 · xA 0.20 · 关键传球 0.20 · 射正 0.25 |
| AM | npxG 0.25 · xA 0.40 · 射正 0.15 · 传球成功率 0.20 | npxG 0.20 · xA 0.30 · 关键传球 0.20 · 射正 0.10 · 盘带 0.20 |
| MID / DC / FB | 不变 | 不变 |
| GK | 评分 0.55 · 失球/90 0.25 · 零封/90 0.20 | 扑救率 0.40 · xGP/90 0.35 · 评分 0.25 |

门将说明：扑救字段只存在于高级数据块（`advanced_available`），所以 core 门将不再让“失球/零封”占 80% 权重，改用比赛评分承担主要信号；高级模型则去掉了队伍相关的失球项。

### 2.5 不确定性与保守排序

每个指标用自身证据量估计百分位的抽样误差：

```text
sd_metric    = sqrt(p(1−p) / evidence) × 100
sd_production = sqrt(Σ (weight × sd_metric)²) / Σ available_weight
band          = 1.2816 × sd_production        (80% 区间)
```

- 新增字段：`production_low`、`production_high`、`score_low`。
- `confidence = clamp(1 − 2×band/100, 0, 1)`：300 分钟的前锋大约 0.6，2700 分钟的前锋大约 0.87。
- 新增排序键 `score_low` / `production_low`；UI 增加“保守排序”开关。
- “为什么是他”面板显示 80% 区间和对等组人数。
- UI 的置信度阈值改为 60% / 75% / 85%，颜色分档改为 0.85 / 0.65。

### 2.6 伤病窗口与数据归属

- 新增 `injuries::recent_within_window(date, game_date, months)`；`HISTORY_MONTHS = 24`。
- `players.recent_injury` 保存的是生涯最近一次伤病日期；现在用存档 `game_date` 计算 24 个月窗口，得到 `injury_recent`。
- `exclude_injury_history` 只排除窗口内的伤病；`injury_history` 标记同步。
- Value Finder 使用同一规则和同一常数。
- 新增 `stats_from_other_club`：赛季统计行属于球员已离开的俱乐部时（例如一月转会），标记并进入 flags 列表。

### 2.7 统一位置分组

- 新增共享函数 `moneyball::position_group_for(text, fallback)`。
- 两个模型都以存档的 `positions_text`（自然位置，按熟悉度排序）优先；Stats 没有备选，Value Finder 用最佳角色位置兜底。
- 修复了 v1 中“Value Finder 按最佳角色位置分组、Stats 按自然位置分组”的错位。
- Value Finder 的模型名同步升到 `value-finder-v2`（分组变化会改变分数）。

## 3. 数据与兼容性

- 模型名：`stats-core-v2` / `stats-advanced-v2`，Value Finder 为 `value-finder-v2`（旧 `v1` 行不会再被读取）。
- `player_metrics` 新增列（全部可空或有默认值）：`non_penalty_expected_goals`、`production_low`、`production_high`、`score_low`、`peer_count`、`stats_from_other_club`、`injury_recent`。
- `shared/schema.sql` 与 `migrate()` 同步；已有缓存走加列迁移。
- 解析器未改，`layout::PARSER_VERSION` 不变。
- 物化行统一写 `min_minutes = 0`；直接写 SQL 时要按这个值查：

```sql
SELECT player_uid, name, position_group, score, production_low, peer_count
FROM player_metrics
WHERE save_id = 1 AND model = 'stats-core-v2' AND min_minutes = 0
ORDER BY score DESC
LIMIT 50;
```

## 4. 测试

新增或更新：

- `scoring_floor_keeps_percentiles_stable_when_min_minutes_changes`
- `thin_reputation_tiers_fall_back_to_the_position_group`
- `npxg_drives_the_striker_model`
- `confidence_reflects_the_evidence_behind_a_percentile`
- `prior_strength_grows_when_players_cluster`
- `shrink_uses_the_metric_exposure_and_prior`
- `injury_filters_exclude_unavailable_players`（新增窗口内/窗口外两个用例）
- `injuries::tests::keeps_only_an_injury_inside_the_history_window`
- `moneyball::tests::shared_group_prefers_natural_positions_over_the_role_position`
- `store::tests::stats_cache_is_bounded`（缓存键去掉阈值）

验证命令：

```bash
cargo test -p fm27-core
cargo test --workspace
cargo clippy -p fm27-core -p fm27-cli --all-targets -- -D warnings
cargo fmt --all -- --check
```

## 5. 尚未做的（Sprint B 起）

1. ~~**公平价格模型**~~：已完成，见 [`moneyball-phase5.md`](moneyball-phase5.md)；`score` 现在只表示质量。
2. **子位置分组**：DM/MC/边路、AMC/AML/AMR、CB/FB/WB。
3. **未使用字段**：`possession_won/lost`、`key_tackles`、`key_headers`、`clear_cut_chances_created`、`mistakes_leading_to_goal`、`distance_km`、`fouls/cards` 等。
4. **赛事分离与 12 个月窗口**：`league` / `continental` / `calendar_year_overall` 行已在库里，评分仍只读 `overall`。
5. **年龄/合同进评分**：Resale 预设目前仍只是过滤年龄。
6. **联赛积分榜/赛程**：对手强度和联赛层级系数。
7. **并行运行的三模式**（Full / Fair-play / Moneyball）。
8. **相似球员、阵容需求视图、导出**。
