# Moneyball Phase 0 验证记录

> 日期：2026-11-03  
> 分支：`feature/moneyball`  
> 参考实现：`.ref-fmsave`（fmsave 0.7.0，experimental FM27）  
> 验证存档：`BB KING - Parma Calcio.fm`、`(v02)`、`(v03)`  
> Build：`27.0.0+2486695`  
> 复现脚本：`scripts/moneyball_phase0.py`

## 0. 结论

**Go：自动验证和 16 名 Parma 球员的游戏内对照均已通过，Phase 0 完成；可以进入 Phase 2 的 parser/schema 工作。**

已确认：

- 三个存档、约 15.5 万行赛季统计，结构校验、汇总一致性、每 90 分钟换算全部通过。
- 有 8–10 个字段可以在所有球员上稳定使用：分钟、首发/替补、进球、助攻、xG、xA、射门、射正、传球、抢断、评分。
- 高级字段（关键传球、渐进传球、拦截、争顶、解围、封堵、跑动、扑救、xGP）**只覆盖约 62% 的球员**，且覆盖率随俱乐部声望从 9.8% 升到 98.2%。不能把缺失值当 0。
- 无约束的“产出/成本”榜会被 30 岁以上、CA 80–110 的低级别联赛球员占据；加上 `CA ≥ 120`、`年龄 ≤ 30`、有效费用/工资后，榜单变得可解释且跨存档稳定。
- 三个存档 × 两个模型（core / advanced）的约束版 top 50 中，有 **30 名球员在全部 6 个榜单中都出现**，可作为第一版人工复核名单。
- 16 名 Parma 球员的游戏内对照全部通过（16/16），核心字段误差均在容差内。

**No-Go 的部分：**

- 不要发布无约束的单一综合分榜单。
- 不要把高级字段的 0 当成真实 0。
- 不要把 CA/身价比或“便宜老将”包装成统计版 Moneyball。
- 不要在未完成游戏内抽样对照前对外宣称“已验证”。

## 1. 验证方法

### 1.1 存档和样本

| 存档 | 游戏内时间点 | 球员数 | 赛季统计行数 | 有统计的球员 | overall 行 | overall ≥900 分钟 | 去重后可评分 |
|---|---|---:|---:|---:|---:|---:|---:|
| base | 2026-12-12 | 48,000 | 156,272 | 40,749 | 37,426 | 19,592 | 19,582 |
| v02 | 2026-12-05 | 48,001 | 155,469 | 40,713 | 37,314 | 19,186 | 19,176 |
| v03 | 2026-11-28 | 48,002 | 154,128 | 40,648 | 37,181 | 18,804 | 18,794 |

三个存档是同一个生涯的不同时间点，适合做稳定性检查，不是三个独立生涯。v02 的统计数字与 `docs/moneyball-feasibility.md` 中早先的验证存档完全一致（155,469 行、40,713 人、19,176 人 ≥900 分钟），说明分析口径可复现。

### 1.2 数据口径

- 只使用 `kind == overall`（俱乐部赛季总计），不使用 league/cup/international 等分项，避免重复计算。
- 40,749 名有统计的球员中，36,327 人有 `overall` 行；4,422 人只有分项统计（如杯赛/国际比赛），不进入主榜。
- 已验证 `overall = league + cup + continental`（不含 international）；37,258 / 37,146 / 37,111 组检查全部零误差。
- 同一球员可能有多条 `overall` 行（一线队 + B/卫星队）：1,072 / 1,036 / 973 人。评分保留“当前俱乐部、分钟最多”的一条；只有 10 人在两条行都 ≥900 分钟，因此对榜单影响很小。
- 主榜门槛：≥900 分钟；<450 分钟在 UI 中只显示“样本不足”（本次原型未纳入评分）。
- 百分位在位置组内计算；同时计算位置组 × 俱乐部声望层级两个版本。
- 收缩：`adjusted = (metric × minutes + peer_mean × K) / (minutes + K)`，K=900。
- 成本：`transfer_value + wage × 52 × 3`；只使用 `transfer_value_state == ok` 且费用 >0、工资 >0 的行，未知费用/工资不参与成本榜。base 存档有 16,840 / 19,582（86.0%）行同时具备可用费用和工资。

### 1.3 两个模型

**Core（推荐作为 v1 主榜）**：只用所有球员都有的字段。

| 位置组 | 指标 |
|---|---|
| ST | xG/90、xA/90、射正/90、转化率 |
| AM | xG/90、xA/90、射正/90、传球成功率 |
| MID | xA/90、传球成功率、传球完成/90、抢断成功/90 |
| DC | 抢断成功/90、抢断成功率、传球成功率 |
| FB | 抢断成功/90、抢断成功率、传球成功率、xA/90 |
| GK | 每 90 失球（反向）、每 90 零封、平均评分 |

**Advanced（只用于有高级数据的子集）**：使用更完整、但覆盖率更低的高级指标集（关键传球、渐进传球、拦截、争顶、解围、封堵、盘带、传中、扑救率、xGP/90），要求指标覆盖率 ≥99%，因此会排除约 37% 的球员；base 存档实际评分 12,273 / 19,582（62.7%）。不能和 core 榜直接混排。

## 2. 结构完整性

| 检查 | base | v02 | v03 |
|---|---:|---:|---:|
| 重复 `(player, kind, team)` key | 0 | 0 | 0 |
| 未知球员行 | 0 | 0 | 0 |
| 未知俱乐部行 | 0 | 0 | 0 |
| 未知球队行 | 0 | 0 | 0 |
| `overall` 汇总不一致 | 0 / 37,258 | 0 / 37,146 | 0 / 37,111 |
| 每 90 分钟换算最大误差 | 0 | 0 | 0 |
| 分钟/射门/传球/抢断/争顶/负数/评分越界 | 0 | 0 | 0 |
| 扑救总数 > 被射正数 | 12 | 12 | 11 |

说明：

- 每 90 分钟字段与 `count × 90 / minutes` 完全一致。
- 12 条扑救异常全部来自低分钟杯赛/非正式比赛，均不足 900 分钟；`save_percent > 100` 已在评分中按缺失处理。
- `overall` 不含国际比赛；如果需要全赛事口径，需要额外加 `international` 行。

## 3. 字段可信度

### 3.1 全量可用字段

以下字段在三个存档的 ≥900 分钟 overall 行中覆盖率 100%，且范围/汇总检查通过：

- `minutes`、`starts`、`substitute_appearances`
- `goals`、`assists`、`player_of_the_match`
- `expected_goals`、`non_penalty_expected_goals`、`expected_assists`
- `shots`、`shots_on_target`
- `passes_attempted`、`passes_completed`、`pass_completion_percent`
- `tackles_attempted`、`tackles_completed`
- `average_rating`（≥900 分钟时 100%）
- 黄牌/红牌/犯规

`tackle_completion_percent` 在 ≥900 分钟行中约 95.7% 可用，其余是 0 次抢断导致的除零；按缺失处理。

### 3.2 关键发现：高级统计存在整块缺失

约 **37%** 的 ≥900 分钟球员，`distance_km == 0`，同时以下字段全部为 0：

`key_passes`、`open_play_key_passes`、`progressive_passes`、`clear_cut_chances_created`、`crosses_*`、`dribbles`、`offsides`、`distance_km`、`high_intensity_sprints`、`aerial_challenges_attempted`、`headers_won`、`key_headers`、`interceptions`、`possession_won/lost`、`blocks`、`shots_blocked`、`clearances`、`saves_*`、`shots_on_target_faced`、`expected_goals_prevented`。

但同一批行的 xG、xA、射门、传球、抢断、评分仍然正常。也就是说：

- 这些 0 不是“球员真的没有做到”，而是该俱乐部/联赛没有记录高级统计。
- `distance_km == 0` 是一个几乎完美的哨兵：约 7,300 行中只有 1–8 行出现某个高级字段非 0。
- 写入数据库时必须把这些 0 转成 `NULL`，并记录 `advanced_available = false`。

高级数据可用率：

| 俱乐部声望层级 | 高级数据可用 / 总数 | 覆盖率 |
|---|---:|---:|
| 1（<2k） | 148 / 1,515 | 9.8% |
| 2（2–4k） | 2,447 / 5,515 | 44.4% |
| 3（4–6k） | 6,045 / 8,505 | 71.1% |
| 4（6–8k） | 3,251 / 3,629 | 89.6% |
| 5（8k+） | 378 / 385 | 98.2% |

三个存档的总可用率分别是 62.8%、62.1%、61.1%；顶级五大联赛接近 95–100%。

**影响：**

- Advanced 模型只能用于“有高级数据”的子集，不能和缺失行混排。
- Core 模型必须作为全量主榜，否则会系统性排除低级别联赛球员。
- 高级字段只能作为附加列、置信度或单独榜单，不能默认参与跨层排名。

### 3.3 门将字段

- 1,669 条 ≥900 分钟门将行中，`saves_held` 等字段在所有行都有值，但 591 行属于上面的缺失块（值为 0），实际可用扑救数据 1,078 行（64.6%）。
- `save_percent` 非空 1,073 行；`expected_goals_prevented` 非零 1,073 行，二者基本对齐。
- `goals_allowed` 和 `clean_sheets` 在所有门将行可用，可作为 core 门将模型。
- 这两个指标受球队整体防守影响很大；有 `save_percent`/xGP 时优先用 advanced 门将模型。
- v1 建议门将单独榜单或单独 tab；不要和 outfield 混排。

### 3.4 明确不可用/不推荐字段

| 字段 | 问题 | 处理 |
|---|---|---|
| `pressures_attempted/completed` | FM27 覆盖率 0% | 不使用 |
| `goals_outside_box` | FM27 覆盖率 0% | 不使用 |
| `high_intensity_sprints` | 即使有高级数据，仍有 1,038 行是 0 | 不使用 |
| `distance_km` | 只作为高级块哨兵，不直接评分 | 只做缺失检测 |
| `shots_outside_box`、`free_kick_shots` | 缺失块中为 0；语义未完全确认 | v1 不用 |
| `average_rating` | 缺失块球员均值 7.10，高级数据球员均值 6.83 | 只能层级内比较，不能跨层直接用 |
| `expected_goals_prevented` | 非门将 94.5% 为 0 | 只用于门将 |

## 4. 原型结果

### 4.1 无约束榜单不可用

Core 模型、无 CA/年龄约束的 `surplus_tier` 榜单会被以下类型占据：

| 球员 | 位置 | 年龄 | CA | 俱乐部声望 | 产出 | 成本百分位 | surplus |
|---|---|---:|---:|---:|---:|---:|---:|
| Nicolás Iachetti | AM | 26 | 104 | 5,450 | 88.8 | 0.6 | 88.1 |
| Fabricio Rojas | AM | 26 | 109 | 4,800 | 86.9 | 2.8 | 84.1 |
| Jonathan Bolingi | ST | 32 | 115 | 1,450 | 84.3 | 0.9 | 83.3 |
| Mattia Mustacchio | AM | 37 | 82 | 4,250 | 81.9 | 1.1 | 80.8 |
| Paolo Guerrero | ST | 42 | 112 | 6,103 | 74.5 | 6.3 | 68.2 |

这不是可用的引援建议：便宜、低 CA、年龄大，且很可能无法升级到更高级别联赛。

### 4.2 约束后榜单可解释

约束：`CA ≥ 120`、`年龄 ≤ 30`、有效费用和工资、≥900 分钟。Core 模型 base 存档 top 10：

| # | 球员 | 俱乐部 | 位置 | 年龄 | CA | 分钟 | 产出 | 成本百分位 | surplus |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | Iván Leguizamón | Club Olimpia | AM | 24 | 123 | 1,717 | 86.5 | 19.5 | 67.0 |
| 2 | Alexander Alvarado | LDU Quito | AM | 27 | 120 | 2,000 | 80.5 | 20.3 | 60.2 |
| 3 | Marcelo Allende | Mamelodi Sundowns | AM | 27 | 128 | 1,563 | 70.0 | 11.2 | 58.8 |
| 4 | Johan Martínez | Deportivo Cali | AM | 24 | 126 | 1,164 | 80.7 | 24.1 | 56.6 |
| 5 | David Martínez | Independiente | MID | 22 | 123 | 1,252 | 76.6 | 20.3 | 56.3 |
| 6 | Ronald Briones | Independiente del Valle | MID | 26 | 120 | 1,786 | 75.9 | 21.8 | 54.1 |
| 7 | Halam Loboa | El Equipo del Pueblo | MID | 20 | 123 | 2,926 | 78.4 | 24.8 | 53.6 |
| 8 | Gabriel Villamil | LDU Quito | MID | 25 | 125 | 2,141 | 73.9 | 20.4 | 53.5 |
| 9 | Santiago González | Sporting Cristal | AM | 27 | 122 | 1,076 | 68.0 | 15.5 | 52.6 |
| 10 | Fernando Cornejo | LDU Quito | MID | 30 | 122 | 1,790 | 61.4 | 10.0 | 51.4 |

三个存档的约束榜组成高度一致：

| 存档 | 声望层级 | 年龄中位数 | CA 中位数 | 最大俱乐部占比 |
|---|---|---:|---:|---:|
| base | 4:47, 5:3 | 26 | 122.5 | 6% |
| v02 | 3:1, 4:46, 5:3 | 26 | 123.0 | 6% |
| v03 | 3:1, 4:46, 5:3 | 26 | 122.5 | 8% |

没有单一俱乐部或联赛超过 30%。这一版榜单可以解释为“二级联赛/非五大联赛中，CA 120–130、年龄 20–30、成本低的即战力”。

成本仍是 `transfer_value` 代理，不是实际要价；3 年工资也不含签字费、佣金和工资条款。个别候选的估值极低（例如 25k），可能是合同/估值口径或数据异常，必须在游戏内确认要价。榜单只能作为候选池，不能当成成交价预测。

### 4.3 稳定性

约束版 top 50 的重叠：

| 模型 | base–v02 | base–v03 | v02–v03 |
|---|---:|---:|---:|
| core constrained surplus | 46/50 | 43/50 | 46/50 |
| advanced constrained surplus | 48/50 | 43/50 | 45/50 |

Core 与 advanced 在 base 存档的约束榜重叠为 34/50。核心候选（Leguizamón、Alvarado、Cornejo、Johan Martínez、Villamil、David Martínez 等）在两个模型中都稳定出现。

### 4.4 6 个榜单的共识名单

3 个存档 × 2 个模型，共 6 个约束版 top 50；其中 **30 名球员在全部 6 个榜单中出现**。完整名单和数值在 `docs/moneyball-phase0-consensus.csv`。

如果只看 core 模型，3 个存档的 3/3 共识有 43 人（包含一些没有高级数据的俱乐部），见 `docs/moneyball-phase0-consensus-core.csv`；6/6 的 30 人名单更严格，适合作为第一轮人工复核。

前 10 名（按 base core surplus 排序）：

| 球员 | 俱乐部 | 位置 | 年龄 | CA | 共识次数 |
|---|---|---|---:|---:|---:|
| Iván Leguizamón | Club Olimpia | AM | 24 | 123 | 6 |
| Alexander Alvarado | LDU Quito | AM | 27 | 120 | 6 |
| Marcelo Allende | Mamelodi Sundowns | AM | 27 | 128 | 6 |
| Johan Martínez | Deportivo Cali | AM | 24 | 126 | 6 |
| David Martínez | Independiente | MID | 22 | 123 | 6 |
| Ronald Briones | Independiente del Valle | MID | 26 | 120 | 6 |
| Halam Loboa | El Equipo del Pueblo | MID | 20 | 123 | 6 |
| Gabriel Villamil | LDU Quito | MID | 25 | 125 | 6 |
| Santiago González | Sporting Cristal | AM | 27 | 122 | 6 |
| Fernando Cornejo | LDU Quito | MID | 30 | 122 | 6 |

### 4.5 位置偏差

- Core `production_global` top 50：AM 34、ST 16，没有 DC/FB/MID。AM/ST 的指标更相关，组合百分位更容易冲高。
- 约束 `production` top 50：AM 27、ST 17、MID 6，仍没有 DC/FB。
- 约束 `surplus` top 50：AM 15、DC 7、FB 7、MID 14、ST 7，成本项把位置分布拉回平衡。

结论：

- 不要做一个跨位置的单一“产出榜”；至少按位置组分开，或给每个位置组固定配额。
- DC/FB 的 core 模型指标有限（只有抢断、抢断成功率、传球成功率），可信度低于 ST/AM/MID；应在 UI 里标注。
- GK 单独榜单。

### 4.6 各位置约束榜（core，base，top 3）

| 位置 | 球员 | 俱乐部 | 年龄 | CA | surplus |
|---|---|---|---:|---:|---:|
| GK | Alejandro Rodríguez | Deportivo Cali | 25 | 121 | 61.6 |
| GK | Axel Werner | Rosario Central | 30 | 120 | 49.8 |
| GK | Gonzalo Valle | LDU Quito | 30 | 128 | 48.1 |
| DC | Renzo Garcés | Alianza Lima | 30 | 126 | 42.9 |
| DC | Álvaro Carrillo | APOEL Nicosia | 24 | 120 | 39.8 |
| DC | Matteo Gabbia | A.C. Milan | 27 | 146 | 38.5 |
| FB | Adrián Marín | Orlando City | 29 | 125 | 37.3 |
| FB | Jamerson | Vitória | 28 | 122 | 37.2 |
| FB | Nahuel Banegas | Independiente | 30 | 122 | 31.8 |
| MID | David Martínez | Independiente | 22 | 123 | 56.3 |
| MID | Ronald Briones | Independiente del Valle | 26 | 120 | 54.1 |
| MID | Halam Loboa | El Equipo del Pueblo | 20 | 123 | 53.6 |
| AM | Iván Leguizamón | Club Olimpia | 24 | 123 | 67.0 |
| AM | Alexander Alvarado | LDU Quito | 27 | 120 | 60.2 |
| AM | Marcelo Allende | Mamelodi Sundowns | 27 | 128 | 58.8 |
| ST | Sebastián Ferreira | Club Olimpia | 28 | 120 | 50.3 |
| ST | Ercan Kara | Rapid Wien | 30 | 121 | 48.0 |
| ST | Mourad | Tijuana | 28 | 120 | 45.1 |

## 5. 游戏内人工对照

### 5.1 待验证样本

`docs/moneyball-phase0-sample.csv` 包含 base 存档 Parma 一线队 16 名 ≥450 分钟球员，最容易在游戏内找到。以下 8 名 ≥900 分钟，建议优先核对：

| 球员 | 位置 | 分钟 | 进球 | 助攻 | xG | xA | 传球成功率 | 抢断成功 | 拦截 | 评分 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Giovanni Fabbian | MID | 1,292 | 4 | 3 | 4.21 | 5.20 | 80.08% | 5 | 23 | 7.06 |
| Adrián Bernabé | MID | 1,229 | 1 | 5 | 3.50 | 3.99 | 87.22% | 11 | 32 | 6.93 |
| Simone Lontani | ST | 1,227 | 5 | 2 | 4.07 | 1.93 | 81.77% | 6 | 29 | 6.84 |
| Enrico Delprato | FB | 1,146 | 0 | 3 | 0.13 | 2.39 | 87.38% | 19 | 71 | 7.19 |
| El Bilal Touré | ST | 1,047 | 3 | 2 | 2.00 | 2.44 | 78.47% | 10 | 23 | 6.84 |
| Emanuele Valeri | FB | 1,030 | 0 | 0 | 0.29 | 0.76 | 84.86% | 11 | 44 | 6.86 |
| Lautaro Valenti | DC | 985 | 0 | 0 | 0.00 | 0.00 | 90.36% | 7 | 41 | 6.83 |
| Diego Carlos | DC | 900 | 0 | 0 | 0.00 | 0.00 | 91.33% | 5 | 50 | 6.88 |

CSV 里还有 2 名 <900 分钟的门将（Edoardo Corvi、Rui Araki）和 6 名轮换球员，可用于检查样本不足时字段是否仍然正确。

### 5.2 对照标准

- 计数类（分钟、进球、助攻、射门、传球、抢断、拦截）：±1。
- xG / xA：±0.05。
- 百分比：±0.1 个百分点。
- 评分：±0.01。
- 记录到 CSV 的 `manual_verified` / `notes` 列。

**验证结果：2026-11-03 已完成 16 名 Parma 球员的游戏内对照，16/16 全部通过。** 计数、xG/xA、百分比和评分均在容差内。CSV 的 `manual_verified` 列已标记为 `yes`。

## 6. Go / No-Go

### Go（自动验证 + 人工对照均已满足）

- 8–10 个核心字段在三个存档上通过结构、范围、汇总和每 90 分钟检查。
- 约束版榜单跨 3 个时间点、2 个模型稳定：43–48/50 重叠。
- 约束版榜单没有单一俱乐部/联赛垄断，CA/年龄/成本都可解释。
- 16 名 Parma 球员的游戏内对照全部通过。
- 30 名共识球员可直接进入后续回测和人工复核。

### v1 发布前仍需遵守的设计约束

1. **v1 只用 core 模型做主榜**；advanced 作为“有高级数据”标签和附加列，不参与跨层混排。
2. **默认约束**：`CA ≥ 120`、`年龄 ≤ 30`、有效费用/工资；用户可调整，但不能默认展示无约束榜。
3. **按位置出榜**；不要把 AM/ST 和 DC/FB 放进同一个无配额榜单。
4. **门将单独**；只使用 `save_percent`/xGP 可用的子集，或先用 core 门将指标。
5. **缺失高级块写 NULL**，并带 `advanced_available` / confidence；绝不能写 0。

### No-Go

- 无约束的“便宜老将”榜。
- 把 CA/身价比或单一黑箱分数叫 Moneyball。
- 把高级字段的 0 当真实值。
- 把本轮单 build、单生涯的验证结果外推到所有存档/build。

### 本轮验证的限制

- 只验证了 FM27 build `27.0.0+2486695` 一个版本；parser 必须对未知 build/layout 失败关闭。
- 三个存档是同一生涯的不同时间点，不是三个独立生涯；独立存档验证仍是后续工作。
- 没有联赛强度/对手强度数据；目前用俱乐部声望层级做代理，不能严格比较“英冠 vs 阿甲 vs 日职”的水平。
- 成本使用 `transfer_value`，不是实际要价；不含签字费、佣金、工资条款。
- 工作许可、联赛报名规则、球员意愿和实际要价尚未解析；榜单不等于可执行交易。
- DC/FB 的 core 指标较弱；门将数据覆盖不完整。这两类位置的产品结论应比 ST/AM/MID 更保守。

## 7. 对 Phase 2 的直接影响

1. **schema 需要 `advanced_available` 布尔列**（或 `NULL` 语义 + confidence），不能只有原始统计列。
2. **core 和 advanced 分开存储/评分**；`player_metrics` 增加 `model` 或 `model_version`，例如 `stats-core-v1`、`stats-advanced-v1`。
3. **parser 必须保留原始 0 和缺失的区别**：Rust 侧根据 `distance_km > 0` 判断高级块是否存在，写入时转 NULL。
4. **门将统计单独校验**；`save_percent` 覆盖 64.6%，且 11–12 条低分钟异常需要过滤。
5. **overall 汇总验证可以作为 parser 的 fail-closed 检查**：`overall == league + cup + continental`，三个存档均零误差。
6. **同一球员多条 overall 行**：按当前俱乐部/最大分钟去重，或保留 `team_id` 并在评分时聚合。
7. **评分默认带约束**：min CA、max age、有效成本；否则 UI 会重现“低级别老将霸榜”。
8. **位置分组和置信度**：DC/FB 的 core 指标弱，UI 要标注；GK 单独。
9. **ST/AM 的 xG 可以再测 npxG**：`non_penalty_expected_goals` 同样 100% 可用，能避免点球手把 xG 抬高；Phase 2 可做 A/B 对比。

## 8. 复现

```bash
# core 模型（推荐 v1）
.venv/bin/python scripts/moneyball_phase0.py --model core --out /tmp/moneyball-phase0-core

# advanced 模型（只用于有高级数据的子集）
.venv/bin/python scripts/moneyball_phase0.py --model advanced --out /tmp/moneyball-phase0-advanced
```

输出：

- `report.md`：覆盖、完整性、榜单组成、稳定性、人工抽样说明。
- `summary.json`：完整结构化结果。
- `top_*.csv`：各榜单。
- `sample_20.csv`：按位置分层的 20 人抽样。
- `sample_managed.csv`：Parma 一线队 ≥450 分钟球员。
- `field_coverage.csv`：所有字段的原始覆盖率。

## 9. 产物

- `docs/moneyball-phase0-consensus.csv`：30 名 6/6 共识球员。
- `docs/moneyball-phase0-consensus-core.csv`：43 名 core 模型 3/3 共识球员。
- `docs/moneyball-phase0-sample.csv`：Parma 一线队 16 人游戏内对照表，已标记 16/16 通过。
- `scripts/moneyball_phase0.py`：本次验证脚本。
- `docs/moneyball-feasibility.md`：可行性分析（已追加 Phase 0 结论链接）。
