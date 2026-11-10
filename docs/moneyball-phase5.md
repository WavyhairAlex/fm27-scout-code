# Moneyball Sprint B：公平价格模型（2026-11-09）

> 状态：`stats-core-v2` / `stats-advanced-v2` 现在带 `fair_price`、`value_gap_pct` 和独立的 `bargain_score`；`score` 只表示质量。
> 后续变更：Sprint C 已升级为 `stats-core-v3` / `stats-advanced-v3`、`hedonic-ridge-v2`，位置哑变量从 6 个增到 8 个，并调整了 `bargain_score` 权重；见 [`moneyball-phase6.md`](moneyball-phase6.md)。
> 价格模型是 `hedonic-ridge-v1`，训练数据来自同一存档，不联网、不引入新依赖。

## 0. 本轮拍板的结果

- 质量与价格分开：`score = production_tier`（纯质量），价格进入 `fair_price` / `value_gap_pct`。
- “性价比”不再藏在 `score` 里，而是单独的 `bargain_score` 复合列。
- 价格模型必须做折外预测：任何球员的公平价都不能用他自己的身价拟合。

## 1. 为什么这样做

外部做法指向同一个结构：

- [Moneyball Scout](https://moneyballscoutml.com/methodology) 用分位数价格模型预测市场价，再用“价格差 + 质量 + 合同 + 分钟份额 + 状态”构成 bargain index；模型强调“从没见过该球员的价格”。
- [ScoutSheet](https://scoutsheet.fm/) 用**同一存档里的相似球员**给出公平价，再显示“低于公平价 59%”这类折扣。
- 转会费文献（[McHale & Holmes 2023](https://ideas.repec.org/a/eee/ejores/v306y2023i1p389-399.html)、[Poli et al. 2024](https://www.mdpi.com/2227-7072/12/3/93)）都表明年龄、表现、合同和联赛层级能显著改善费用预测。

v2 之前的 `cost_pct` 只是“成本在同类里的名次”，无法回答“他应该值多少”。`surplus = production − cost_pct` 也把两个百分位相减，不是货币意义上的便宜。

## 2. 模型

### 2.1 形式

```text
log(1 + fee) ≈ β0
             + β1 · quality + β2 · quality²
             + β3 · CA + β4 · CA² + β5 · PA
             + β6 · age' + β7 · age'²          (age' = (age − 26) / 10)
             + β8 · log(1 + minutes/90)
             + β9 · log(1 + reputation/1000)
             + β10 · reputation_known
             + β11 · contract_years/5 + β12 · contract_known
             + β13 · league_known
             + Σ β · position dummies (GK/DC/FB/MID/AM/ST)
```

平方项用来拟合顶级球员的凸性；没有它们时，线性外推会把 3500 万的前锋估到 1 亿以上。

- 目标：`log(1 + transfer_value)`，仅使用 `transfer_value_state = "ok"` 且费用大于 0 的球员。
- 特征标准化后做 ridge（λ = 1，截距不惩罚），高斯消元求解，无外部依赖。
- 5 折按 uid 的乘法哈希分配，避免 uid 按俱乐部连号时折间不均衡。
- **折外价格**：每个球员的 `fair_price` 来自没有见过他的那一折模型。
- **中位数价格**：预测在 `log(1+fee)` 空间完成，`fair_price` 是条件中位数。真实存档的折外 σ 约为 1，若改用 `σ²/2` 的均值修正会把所有价格抬高约 70%、把半个市场标成“便宜”，因此不用均值修正。
- **价格上限**：公平价被限制在该位置组已观测费用的第 99 百分位，避免线性外推给出市场从未出现过的价格。
- 未知身价/自由身球员：用 5 折模型的对数均值（几何平均价）给 `fair_price`，但没有 `value_gap_pct`。
- 保护条件：训练行 < 200、折内训练行 < 10、折外 R² < 0.05 时整体关闭价格模型，行上写 `no_price_baseline`，UI 显示“无价格基准”。

### 2.2 输出

| 字段 | 含义 |
|---|---|
| `fair_price` | 模型给出的市场价（存档单位） |
| `value_gap_pct` | `(fair_price − fee) / ((fair_price + fee) / 2) × 100`；对称百分比，正数表示比同类便宜，范围约 ±200 |
| `bargain_score` | `0.55 × 质量下限 + 0.45 × 组内价差百分位`；没有价格差时为 `None`。用下限避免小样本爆发上榜 |
| `score` | 质量百分位，等于 `production_tier` |
| `score_low` | 质量下限，等于 `production_low` |

`bargain_score` 是明确的复合列，不替代 `score`；UI 默认按它排序，点“产出”表头即可回到纯质量榜。

`surplus`（质量百分位 − 成本百分位）仍保留为兼容字段，但 UI 不再展示；判断便宜与否应看 `value_gap_pct`。

### 2.3 为什么不用“相似球员 + 均值”

同存档的相似球员匹配需要再定义距离、位置和合同权重，容易在 48k 人里找到不相似的“相似球员”。ridge 是同一思路的参数化版本：所有球员参与，但按特征加权；折外预测保证不能复制自己的价格。等 Sprint C 做相似球员功能时，两者可以互补。

## 3. 代码与数据

- 新模块 `crates/fm27-core/src/price_model.rs`：
  - `PriceInput`、`PriceModel::fit`、`predict_oof`、`predict_market`；
  - `contract_years()`：用存档 `game_date` 计算合同剩余年限；
  - 20 维特征，`FEATURE_COUNT` 有测试锁定。
- `stats_moneyball.rs`：
  - `price_features()`、`attach_fair_prices()`；
  - 价格差百分位在**位置组内**计算，且在所有请求过滤之前；
  - `score` 改为质量，价格字段独立；
  - 排序键新增 `bargain`、`value_gap`、`fair_price`。
- `StatsMoneyballMeta` 新增 `price_model` 和 `priced`；`priced` 是拿到公平价的已评分行数。
- `StatsMoneyballCandidate` 新增 `contract_years`，由 store 用 `game_date + contract_end` 计算。
- `player_metrics` 新增 `fair_price`、`value_gap_pct`、`bargain_score` 三列；schema 与 `migrate()` 同步。
- 物化行仍写 `min_minutes = 0`；模型版本不变（`stats-core-v2` 尚未发布，Sprint A/B 一起发布）。

## 4. UI

- 数据版默认按 `bargain` 排序；表头新增“价差”和“性价比”，移除重复的“评分”列（质量仍由“产出”列和详情展示）。
- 价差列正数绿色、负数红色；详情面板新增“公平价”和“价差”。
- 新增文案：`analysis.stats.table.bargain`、`analysis.stats.table.valueGap`、`analysis.stats.detail.fairPrice`、`analysis.stats.detail.valueGap`、`analysis.stats.priceHint`、`analysis.stats.flag.no_price_baseline`。
- 预设和重置都会把排序设为 `bargain`；“保守排序”仍按质量下限。

## 5. 真实存档验证（AP King - Milan，2027-10-15）

`target/editor-test/AP King - Milan (v03).fm`：

| 指标 | 值 |
|---|---:|
| 球员 | 53,092 |
| 300+ 分钟参与评分 | 21,106 |
| 有身价、可算价差 | 18,799 |
| 价差 p5 / p25 / p50 / p75 / p95 | −127.7 / −57.5 / **−2.8** / +51.8 / +131.3 |
| log(身价) vs log(公平价) | r = 0.736，slope = 0.758 |

默认筛选（CA ≥ 120、年龄 ≤ 30、≥ 900 分钟、不含门将）下的 1,099 人价差中位数为 −40%。这是选择偏差：筛选只保留高 CA 的尾部，而该尾部的实际身价低于模型的条件中位数；全市场的中位数已经接近 0。榜单排序使用的是位置组内价差百分位，不受这个水平偏移影响。

验证中发现并修掉的三处：

1. 最初用 `σ²/2` 把对数预测还原成条件均值，真实 σ ≈ 1 时把所有公平价抬高约 70%，中位价差变成 +48% → 改为条件中位数。
2. 线性模型对顶级球员外推过猛（3470 万的前锋得到 1.26 亿公平价）→ 加入 quality² / CA²，并把公平价限制在位置组费用的第 99 百分位（同一球员现在为 5410 万）。
3. `bargain_score` 用质量点估计时，300–500 分钟的小样本会靠运气进前 20 → 改用质量下限。

## 6. 测试

- `price_model::tests::fits_a_linear_log_price_surface_out_of_sample`：线性市场下折外 R² > 0.8。
- `price_model::tests::out_of_fold_prediction_ignores_the_players_own_fee`：只改目标球员的身价，他的折外价格不变（防泄漏）。
- `price_model::tests::free_and_unknown_fees_use_the_fold_average`：未知身价走 5 折几何均价。
- `refuses_to_price_a_market_that_is_too_small`、`contract_years_reads_dates_and_ignores_missing_values`。
- `stats_moneyball::tests::fair_price_and_value_gap_are_separate_from_quality`：`score == production_tier`，低价高质球员得到正价差。
- `stats_moneyball::tests::free_and_unknown_fees_get_a_price_but_no_gap`。
- `stats_moneyball::tests::a_small_market_leaves_the_price_model_off`。
- store 端到端测试补了三列物化读回。

命令：

```bash
cargo test -p fm27-core
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## 7. 已知限制

1. **`transfer_value` 是市场估值，不是实际要价。** 公平价是“同类型球员的市场价”，不是谈判报价；签字费、经纪人佣金和条款仍未解析。
2. **价格模型使用 CA/PA。** 这能显著提高拟合度，但也意味着 `value_gap` 部分反映“身价相对 CA/PA 是否偏低”。Sprint C/D 的 Fair-play / Moneyball 模式需要单独的隐藏 CA/PA 版本。
3. **没有分位数区间。** 目前只有点估计；折外残差标准差已保留在 `PriceModel` 上，后续可以输出 10/50/90 分位价格。
4. **价格上限会压平极端值。** 位置组第 99 百分位以外的价格会被截断，这是有意的保守处理，但也意味着“模型认为他值更多”在顶级球员上会被低估。
5. **自由身没有价差。** 费用为 0 时只给公平价，不计算百分比差，避免“100% 便宜”的假信号。
6. **位置只是 6 组。** 子位置分组和联赛强度仍在 Sprint C。
7. **联赛层级只通过俱乐部声望和 `league_known` 进入模型。** 还没有联赛系数或积分榜/赛程。

## 8. 下一步

- Sprint C：子位置分组、未使用字段（`possession_won/lost`、`key_tackles`、`clear_cut_chances_created`、`mistakes_leading_to_goal`、`distance_km`、`fouls/cards`）、赛事分离与 12 个月窗口、年龄/合同进入质量评分。
- Sprint D：联赛积分榜/赛程做对手强度、三模式、相似球员、阵容需求视图、导出、as-of-date 回测。
