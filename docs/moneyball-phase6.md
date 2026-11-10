# Moneyball Sprint C：子位置、统计字段与时间窗口（2026-11-09）

> 状态：模型升级为 `stats-core-v3` / `stats-advanced-v3`，价格模型为 `hedonic-ridge-v2`，Value Finder 为 `value-finder-v3`。
> 本阶段把中场拆成三个位置组、接入已解析但未使用的统计字段、增加统计窗口选择，并把年龄/合同/分钟负荷放进 `bargain_score`。

## 1. 子位置分组

原来是 6 组，`MID` 同时包含 DM/MC/ML/MR，`AM` 同时包含 AMC/AML/AMR；这两组的指标模板差异太大。现在是 8 组：

| 组 | 自然位置 |
|---|---|
| GK | GK |
| DC | DC, SW |
| FB | DL, DR, WBL, WBR |
| DM | DM |
| MC | MC |
| WM | ML, MR, AML, AMR |
| AMC | AMC |
| ST | STC, ST, SC |

- 分组函数仍然是 `moneyball::position_group_for`；两个模型共用。
- 旧筛选值 `MID` / `AM` 继续可用，分别匹配 `{DM,MC,WM}` 和 `{WM,AMC}`，本地保存的视图不会失效。
- 声望档 × 位置组最多 8 × 6 个桶；不足 30 人的桶仍回退到位置组。

## 2. 指标模板

核心变化：

| 组 | core | advanced（新增/变化） |
|---|---|---|
| DM | 抢断、抢断成功率、传球成功率、传球完成/90 | + 拦截、夺回球权、渐进传球、争顶 |
| MC | xA、传球成功率、传球完成、抢断 | + 关键传球、渐进传球、拦截、夺回球权 |
| WM | xA、射正、传球成功率 | + 关键传球、传中、盘带、绝佳机会、距离/90 |
| AMC | npxG、xA、射正、传球成功率 | + 关键传球、绝佳机会、盘带 |
| DC | 抢断、抢断成功率、传球成功率 | + 关键抢断、关键争顶 |
| FB | 抢断、抢断成功率、传球成功率、xA | + 距离/90（跑动负荷） |
| ST | npxG、xA、射正 | + 关键传球、绝佳机会 |

新增指标（全部来自 `player_season_stats` 已有字段）：

- `KeyTackles90`：`key_tackles`
- `PossessionWon90`：`possession_won`
- `ClearCutChances90`：`clear_cut_chances_created`
- `KeyHeaders90`：`key_headers`
- `Distance90`：`distance_km` × 90 / 分钟

新增风险标记：

- `mistakes`：`mistakes_leading_to_goal > 0`
- `discipline`：红牌、≥5 黄牌，或犯规 ≥40

这些字段属于高级统计块；`advanced_available = false` 时仍按缺失处理，不写 0。

`fouls_made`、黄/红牌和失误致丢球不进入评分权重，只做风险标记；`distance_km` 只用于 FB 的 advanced 模板，避免把跑动当成通用质量。

## 3. 统计窗口

`StatsMoneyballRequest` 新增 `window`：

| 值 | `player_season_stats.kind` | 说明 |
|---|---|---|
| `overall`（默认） | `overall` | 全部赛事汇总 |
| `league` | `league` | 只算联赛，避免杯赛/国家队刷数据 |
| `calendar` | `calendar_year_overall` | 近 12 个月窗口 |

- 只有 `overall` 会写入 `player_metrics`；`league` / `calendar` 在内存里缓存，重启后重算一次。
- 缓存键从 `(save_id, model)` 变成 `(save_id, model, window)`。
- UI 数据版新增“统计窗口”选择；保存视图会记住窗口。
- 价格模型在每个窗口上单独训练，因此同一球员在联赛窗口和全赛事窗口的公平价可能不同。

## 4. `bargain_score` 更新

```text
bargain = 0.45 × 质量下限
        + 0.30 × 位置组内价差百分位
        + 0.10 × 年龄加成        (≤20 满、27+ 为 0)
        + 0.10 × 合同杠杆        (剩余 0 年满、5 年为 0)
        + 0.05 × 分钟负荷        (2700 分钟封顶)
```

- 年龄和合同同时出现在价格模型和 bargain 里，这是有意的：前者影响市场价，后者表达转售/议价偏好（与外部 bargain index 一致）。
- `score` 仍然是纯质量，不含年龄/合同。

## 5. 真实存档验证（AP King - Milan，2027-10-15）

| 项 | 结果 |
|---|---:|
| 300+ 分钟参与评分 | 21,106 |
| 分组人数 | DC 3,928 / FB 3,187 / DM 2,058 / MC 2,361 / WM 3,536 / AMC 1,523 / ST 2,849 / GK 1,664 |
| overall 900+ 分钟 | 9,273 |
| league 900+ 分钟 | 7,240 |
| calendar 900+ 分钟 | 19,341 |
| 有公平价 | 21,106 |
| 有 bargain | 18,799 |
| `mistakes` 标记 | 284 |
| `discipline` 标记 | 4,023 |

`season_stats_real_save` 全链路测试在 AP King 上通过。窗口切换、旧 `MID` 筛选兼容和分组模板权重和都有单元测试。

## 6. 测试

新增：

- `legacy_position_filters_cover_the_split_groups`
- `legacy_mid_filter_still_matches_split_groups`
- `every_position_template_sums_to_one`
- `advanced_metrics_use_the_new_possession_fields`
- `normalizes_the_statistics_window`
- `bargain_parts_reward_youth_short_contracts_and_minutes`
- store 端到端追加 league/calendar 行，验证窗口读取且不会写入物化表。

```bash
cargo test -p fm27-core
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
FM27_TEST_SAVE="target/editor-test/AP King - Milan (v03).fm" \
  cargo test -p fm27-core --release --test season_stats_real_save -- --ignored
```

## 7. 已知限制

1. **权重仍是手工设定**，不是从“对净胜球的影响”拟合出来的；每个位置的 ±20% 敏感性需要重新跑。
2. **`distance_km` 只用于 FB**；跑动/冲刺的战术含义因体系而异。
3. **`calendar` 窗口会混入上赛季**，赛季初样本会突然变大，榜单和价格模型都会随窗口变化。
4. **`league` 窗口依赖联赛名称解析**；未解析联赛的球员只能看 `overall`。
5. **风险标记没有进入排序**，只作为过滤/提示。
6. **子位置只有 8 组**，没有把 WB 与 FB、DM 与 CM 再细分。
7. 价格模型的 CA/PA 依赖和 p99 上限仍在（见 phase5）。

## 8. 下一步

Sprint D：联赛积分榜/赛程做对手强度、Full / Fair-play / Moneyball 三模式、相似球员、阵容需求视图、CSV/JSON 导出、as-of-date 回测。
