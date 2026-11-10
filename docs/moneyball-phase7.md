# Moneyball Sprint D（第一部分）：导出与相似球员（2026-11-09）

> 状态：数据版新增 CSV/JSON 导出和“相似球员”检索；评分模型仍是 `stats-core-v3` / `stats-advanced-v3`，价格模型 `hedonic-ridge-v2`。
> 本阶段不改变评分，只新增两个使用现有评分结果的能力。

## 1. 数据版 CSV/JSON 导出

- 后端新增 `StatsExportWriter`（`crates/fm27-core/src/export.rs`）：
  - CSV 约 57 列：身份、位置、分钟、CA/PA、质量/下限、费用/成本/公平价/价差/性价比、置信度、原始统计、挂牌/伤病/费用状态、flags 和 drivers。
  - JSON 直接序列化 `StatsMoneyballRow`；内部用的 `metric_percentiles` 不进入导出。
- `Store::stats_moneyball_export()` 返回**所有**过滤后的行，不受分页 500 条限制。
- 新 Tauri 命令 `export_stats`；前端数据版提供 CSV/JSON 选择器和导出按钮，复用 `pickExportFile` 保存对话框。
- 浏览器 mock 模式会生成同样格式的 Blob 下载。

真实存档（AP King，900+ 分钟）导出 8,577 行；CSV 表头与每条记录列数一致，JSON 可解析。

## 2. 相似球员

- 每行保存一份 `metric_percentiles`：位置模板内各指标的百分位。物化表新增 `percentiles_json`，重启后不需要重算。
- 距离：按位置模板权重加权的欧氏距离，只在**同一位置组**、共享权重 ≥60% 的球员之间计算。
- 旧 v3 物化行如果没有百分位向量，会在第一次查询时重新评分并写回。
- 新 Tauri 命令 `similar_stats_players`；API `similarStatsPlayers`；React hook `useSimilarStatsPlayers`。
- UI：展开“为什么？”后，底部显示同组相似球员，包含位置、年龄、质量、价差和风格距离；点击直接打开该球员。
- 相似球员的候选集遵循当前视图的筛选条件（分钟、联赛、年龄、市场等），避免在 300 分钟样本里找“相似”。

真实存档检查：AP King 的顶级 ST 返回的相似球员全部是 ST，距离约 1–4（百分位尺度）。

## 3. 数据与兼容性

- `player_metrics` 新增 `percentiles_json TEXT NOT NULL DEFAULT '{}'`；`shared/schema.sql` 与 `migrate()` 同步。
- `StatsMoneyballRow.metric_percentiles` 标记为 `serde(skip_serializing)`，不改变 Tauri 响应和前端类型。
- 导出命令和相似命令都走可写 store 的 `run_store_command`，与 `stats_moneyball` 相同。

## 4. 测试

- `stats_moneyball::tests::exports_scored_rows_as_csv_and_json`：CSV 表头/记录列数、JSON 数组和字段。
- `stats_moneyball::tests::similar_players_follow_the_metric_profile_within_the_group`：同组、近邻优先、跨位置排除。
- store 端到端：物化行读回 `metric_percentiles`，并调用相似查询。
- 真实存档 `season_stats_real_save` 继续通过；导出与相似球员在 AP King 上手工跑通。

## 5. 本阶段没有做的（及原因）

1. **联赛积分榜/赛程解析**：需要先做布局逆向并用 oracle 校验，直接猜 offset 会违反“不要猜偏移”的项目约束；建议作为独立的 parser spike。
2. **全局 Full / Fair-play / Moneyball 三模式**：需要同时隐藏 PlayerTable、Inspector、ClubInspector、Squad 面板和导出里的 CA/PA/属性；这是一次跨 UI 的改动，等前端能在本机 `pnpm build/test` 时再做。
3. **阵容需求视图**：现有 `squad_analysis` 只统计人数/年龄/CA，不是“最弱位置 + 市场升级候选”；需要先确定“我的俱乐部”来源或俱乐部选择器。
4. **as-of-date 回测**：当前可用的独立存档只有 AP King 一个时间点；`target/editor-test/results` 是编辑器测试存档，不能作为真实回测样本。脚本可以写，但没有可解释的结果。

## 6. 下一步

- 三模式（跨 UI）与阵容需求视图。
- 联赛表格/赛程的 parser spike（先只读、fail-closed、与 oracle 对数）。
- 多存档回测：需要同一正常生涯的 3–4 个时间点。
