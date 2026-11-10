# Moneyball 模式可行性分析

> 分析对象：FM27 Scout（Rust 解析核心 + Tauri/React 桌面端）  
> 验证存档：`BB KING - Parma Calcio.fm`，build `27.0.0+2486695`，游戏日期 2026-12-05  
> 参考实现：本地 `.ref-fmsave`（fmsave 0.7.0，experimental FM27）  
> 结论日期：2026-11-02

## 0. 结论摘要

**可行，但必须分阶段，且不要把“身价/CA 比值”包装成 Moneyball。**

- **真实统计版 Moneyball 在数据上可行。** 参考实现已经能从 FM27 存档读取赛季统计。验证存档中有 155,469 行赛季统计，40,713 名球员（84.8%）至少有一项统计，19,176 名球员有 ≥900 分钟的 overall 数据。
- **当前 Rust 解析器还没有读取赛季统计。** 这是主要工程缺口，不是数据缺口。需要移植 season stats reader，并给容器层补上 unlisted span frame 的读取能力。
- **建模比解析更难。** 直接用 xG+xA/90 或“产出/成本”排序，会把低级别联赛、30 岁以上、CA 80–100 的球员推到榜首。必须做位置分组、联赛强度/俱乐部声望分层、样本收缩、成本口径和风险调整。
- **推荐路线：** 2–3 天 Phase 0 验证 → 1–2 周 Phase 1“Value Finder”（只用现有数据）→ 3–5 周 Phase 2 统计版 Moneyball → 2–3 周 Phase 3 校准。单人、熟悉代码库的粗略总工作量 6–10 周。
- **Go 条件：** Phase 0 能确认 8–10 个可靠统计字段，并且前 50 名经人工复核后不出现系统性低级别联赛偏差。否则只做 Value Finder，不宣称 Moneyball。

## 1. 在本项目中如何定义 Moneyball

不是“找高潜小妖”（项目已有 wonderkids 快速筛选），也不是“找 CA/身价最高的球员”。本项目的 Moneyball 应定义为：

> 在给定预算、位置和风险约束下，找到**场上产出高于同层市场预期**、且**总拥有成本低于同层市场水平**的球员。

三个独立维度：

1. **产出（Production）**：当前赛季每 90 分钟的实际/预期产出。
2. **成本（Cost）**：转会价值（或用户输入预算）+ 工资 × 合同/持有年限。
3. **风险与可持续性（Risk）**：样本量、伤病、年龄、合同、联赛强度、数据缺失。

FM 特有的市场低效点可以作为**假设**，但不能直接当结论：

- CA 权重不计入的属性：determination、natural fitness、flair、aggression、隐藏属性；高值不推高 CA，因而可能不推高身价。
- 定位球、长界外球、点球等低 CA 权重技能。
- 合同年、挂牌、租借挂牌、低 reputation 俱乐部/联赛。
- 年龄歧视：30+ 即战力通常折价。
- 位置错配：角色分高但当前位置/CA 不突出。
- 多位置与阵容深度价值。

### 1.1 产品可行性

**目标用户：**

- 低预算/低级别联赛、挑战模式、喜欢数据分析的 FM 玩家。
- 想避免“只看 CA/PA”或“只看身价”的玩家。
- 不是只想找高潜小妖的玩家——那已经有 wonderkids 快速筛选。

**核心场景：**

- 转会窗/租借市场：给定预算、位置和风险偏好，生成 20–50 人候选。
- 解释“为什么便宜且好用”：产出、成本、风险、样本量。
- 对比 2–4 名候选，导出 shortlist，或把筛选条件送回主表。

**差异化：**

- 跨俱乐部/联赛的市场视图，而不是复刻游戏内球探评分。
- 自定义预算和工资上限，而不是依赖只有 122 家俱乐部才有的财务数据。
- 成本拆解、置信度、风险标记和“为什么是他”，避免黑箱排名。
- 与现有角色分、成长快照、对比、导出功能联动。

**成功指标：**

- Moneyball tab 的使用率，以及从候选打开详情/加入对比/导出的转化率。
- 保存视图和重复使用率，而不是只看点击量。
- 用户反馈中“推荐是否可信/可执行”的比例。
- 不追求“榜单看起来刺激”，追求“推荐能解释、能复现、能落地”。

**不做什么：**

- 不自动替用户买人，不做“一键最强阵”。
- 不预测真实足球，只分析游戏内市场和存档数据。
- 不默认开启隐藏属性，不用于线上联赛作弊。
- 不发布没有成本/样本/风险说明的单一黑箱分数。

## 2. 数据可行性

### 2.1 现有数据（无需改解析器）

| 数据 | 当前来源 | 用途 | 主要限制 |
|---|---|---|---|
| CA / PA | `players.ca/pa` | 能力基线、成长空间 | CA 不是实际产出；PA 可能是区间 |
| 52 项属性 | `players.attributes_json` | 角色适配、CA 豁免属性、风险 | 隐藏属性默认不显示 |
| 位置与双足 | `players.positions_json`、左右脚 | 位置分组、角色分 | 位置评分是游戏内 1–20 |
| 年龄 / 身高 | `players.age/height_cm` | 年龄曲线、位置筛选 | 无 |
| 身价 | `players.transfer_value` | 成本代理 | 不是实际转会费；约 18% 为 zero/placeholder/unset |
| 工资 | `contracts.wage` | 成本、工资结构 | 存档单位，不是显示货币 |
| 合同 | `contracts.end_date/start_date` | 合同年、议价能力 | 无签字费/佣金 |
| 挂牌 / 租借 | `players.transfer_listed/loan_listed` | 市场低效信号 | 只有布尔值 |
| 伤病 / 停赛 | `injuries_json/suspensions_json` | 风险、可用性 | 伤病史不完整 |
| 性格 / 隐藏属性 | `personality_json`、hidden attributes | 风险、成长 | 默认关闭；线上联赛需提示 |
| 角色分 | `roles.rs`，29 个角色 | 位置适配、替代产出 | 权重是近似值，不是 FM 内部公式 |
| 俱乐部声望 / 上季排名 | `clubs.reputation/last_league_position` | 联赛层级代理 | 没有 club → competition 映射 |
| 俱乐部财务 | `club_finances` | 购买力背景 | 项目参考存档只有 122 家俱乐部有完整财务 |
| 成长快照 | `player_snapshots` | 跨存档 CA/身价/工资变化 | 需要同一存档的多个时间点 |

现有查询层已经支持年龄、CA/PA、位置、属性、身价、工资、合同到期、挂牌、可用性、隐藏属性、排序和分页（`crates/fm27-core/src/store.rs` 的 `PlayerQuery`）。这为 Value Finder 提供了很好的基础。

### 2.2 缺失数据

| 缺失项 | 影响 | 现状 |
|---|---|---|
| 赛季统计（分钟、xG、xA、传球、抢断、评分等） | 无法做真正的产出型 Moneyball | 参考实现可读；Rust 未解析 |
| 联赛成员关系 / 联赛强度 | 无法做严格的跨联赛水平调整 | 只有俱乐部 nation/reputation/last position |
| 实际转会费 / 要价 | 成本只能用身价代理 | 存档不提供成交价 |
| 签字费、佣金、工资条款 | 总成本低估 | 合同只解析工资/年限/部分条款 |
| 比赛级对手强度、首发/分钟分布 | 无法做赛程强度调整 | 只有赛季汇总 |
| 工作许可、报名规则、球员意愿 | 推荐可能不可执行 | 未解析 |
| 赛季历史（多赛季） | 无法区分单季爆发和稳定产出 | 参考实现有 season history；Rust 未接 |

### 2.3 真实存档验证结果

使用本地参考实现读取 `BB KING - Parma Calcio.fm`。该存档是赛季中段（12 月 5 日），不是赛季末；赛季初的覆盖率和样本量会明显更低，因此下面的数字应视为“赛季中段可用数据”的上界参考。

| 指标 | 数值 |
|---|---|
| 球员总数 | 48,001 |
| 赛季统计行数 | 155,469 |
| 至少有一项统计的球员 | 40,713（84.8%） |
| 有 league 或 overall 统计的球员 | 36,254（75.5%） |
| 有 ≥900 分钟 overall 数据的球员 | 19,176（39.9%） |
| `overall` 行 | 37,314 |
| `league` 行 | 35,706 |
| `cup` 行 | 11,641 |
| `continental` 行 | 5,438 |
| `international` 行 | 4,325 |

**按俱乐部声望分层的 ≥900 分钟覆盖率：**

| 声望层级（从低到高） | 有 ≥900 分钟 overall 的球员 / 该层球员 | 覆盖率 |
|---|---:|---:|
| 1（最低） | 256 / 278 | 92.1% |
| 2 | 689 / 852 | 80.9% |
| 3 | 1,574 / 1,752 | 89.8% |
| 4 | 4,421 / 6,803 | 65.0% |
| 5（最高） | 12,203 / 33,983 | 35.9% |

未解析/声望为 0 的 4,333 名球员未列入上表，其中 33 人有 ≥900 分钟。最高声望层覆盖率低，主要是因为豪门阵容庞大、很多球员不足 900 分钟，不是低级别联赛缺数据。这反而说明：**必须把“样本不足”做成显式状态，而不是把 0 分钟球员排进榜单。**

**字段可用性（overall，≥900 分钟）：**

| 字段 | 非空 | 说明 |
|---|---:|---|
| `expected_goals` | 100% | 9.5% 为 0 |
| `expected_assists` | 100% | 17.7% 为 0 |
| `average_rating` | 100% | 可用性较好 |
| `passes_attempted` | 100% | 可用 |
| `tackles_attempted` | 100% | 可用 |
| `distance_km` | 100% | 非空，但约 37% 为 0；0 代表高级统计整块缺失，见 Phase 0 |
| `expected_goals_prevented` | 100% | 全表中 94.4% 为 0（非门将没有该值）；门将行中约 35.8% 为 0 |
| `save_percent` | 1,063 / 1,657 门将行 | 门将字段覆盖率较低 |
| `pressures_attempted` | 0% | FM27 不支持 |
| `goals_outside_box` | 0% | FM27 不支持 |

> Phase 0 更正：非空不等于可用。`distance_km == 0` 的行中，关键传球、渐进传球、拦截、争顶、解围、封堵、跑动、扑救、xGP 全部被置为 0；这些 0 必须按缺失处理。高级数据覆盖率随俱乐部声望从 9.8% 升到 98.2%，详见 [`docs/moneyball-phase0.md`](moneyball-phase0.md)。

**身价状态（48,001 名球员）：**

| 状态 | 数量 |
|---|---:|
| `ok` | 39,246 |
| `zero` | 6,773 |
| `placeholder` | 1,527 |
| `unset` | 455 |

约 18% 的身价不能直接当成本使用。成本模型必须显式处理“未知”，不能把 `unset/placeholder` 当成免费。

### 2.4 参考实现与移植成本

参考实现 `fmsave 0.7.0` 的 `player_season_stats()` 已经实现 experimental FM27 读取：

- 赛季统计不是顶层 section，而是 span region 中一个未列入目录的 frame。
- FM27 布局：`line_marker = b"\x01\x07"`，`line_bytes = 155`，要求 player key 唯一。
- 不支持字段：`goals_outside_box`、`pressures_attempted`、`pressures_completed`。
- 参考 reader 约 490 行，数据模型约 611 行；Rust 移植预估 700–1,000 行（含测试）。
- 参考实现说明：完整存档约 400,000 行，读取需要几秒和数百 MB 内存；本验证存档 155,469 行，规模更小。

当前 Rust 容器层（`crates/fm27-core/src/container.rs`）只从顶层目录读取 `.dat/.cmt` section；`SaveFile::read_section()` 依赖 `sections` 映射，没有遍历 span region / unlisted frame 的 API。需要补：

1. 枚举目录未列出的 frame（至少 span region 的 frame head）。
2. 按声明的压缩/解压大小安全解压候选 frame。
3. 用 section header、footer 和完整 walk 做 fail-closed 校验。
4. 把 `player_season_stats` 写入 SQLite，并与球员/俱乐部 join。

### 2.5 原型验证得到的三个结论

在验证存档上做了简单的 Python 原型（不是最终模型）：

1. **原始产出榜不是简单的“低级别联赛榜”。** 按 xG+xA+关键传球等指标排序的前 200 名，俱乐部声望中位数约 5,400（中上层），最低两个声望层级（约后 50%）占比只有 0.5%。
2. **但“产出/成本”榜会明显偏向低级别联赛老将。** 加入成本后，榜首出现大量 30 岁以上、CA 80–100、低级别联赛球员。这是典型的“便宜但未必能升级”的假阳性。
3. **指标定义会决定榜单。** 第一次“相对 CA/年龄/层级的生产剩余”原型中，门将指标设计不当，导致榜单被门将占满；这说明**指标设计和验证比数据可得性更关键**。

另外，产出与俱乐部声望的相关系数约为 **0.309**：层级能解释一部分产出差异，但不能解释全部。因此需要“层级调整 + 置信度”，而不是简单乘以一个联赛系数。

## 3. 技术方案

### 3.1 解析与存储

数据流：

```text
save.fm
  -> SaveFile（新增 unlisted span frame 读取）
  -> season_stats walk + player/team join
  -> SQLite player_season_stats
  -> moneyball::compute（位置分组、同层百分位、成本、风险）
  -> SQLite player_metrics
  -> PlayerQuery / AnalysisDialog
```

新增 `crates/fm27-core/src/season_stats.rs`：

- 按参考布局 walk section，逐条记录读取 own_lines / other_teams / form list。
- 用 `pindex + 1` 关联球员，用 team id 关联俱乐部。
- 门将和普通球员共用部分字段，按 natural positions 中的 GK 决定解码方式。
- 不支持的字段写 `NULL`，不猜测。
- 任一条记录结构不合法就整体 fail-closed，返回警告并回退到 Value Finder。

建议的 schema：

```sql
CREATE TABLE IF NOT EXISTS player_season_stats (
  save_id       INTEGER NOT NULL,
  player_uid    INTEGER NOT NULL,
  kind          TEXT NOT NULL,          -- overall / league / cup / ...
  team_id       INTEGER NOT NULL DEFAULT 0, -- 0 = 未解析/未知球队
  club_uid      INTEGER,
  minutes       INTEGER NOT NULL,
  starts        INTEGER NOT NULL,
  sub_apps      INTEGER NOT NULL,
  average_rating REAL,
  goals         INTEGER NOT NULL,
  assists       INTEGER NOT NULL,
  expected_goals REAL,
  expected_assists REAL,
  shots         INTEGER NOT NULL,
  shots_on_target INTEGER NOT NULL,
  passes_attempted INTEGER NOT NULL,
  passes_completed INTEGER NOT NULL,
  progressive_passes INTEGER NOT NULL,
  key_passes    INTEGER NOT NULL,
  tackles_attempted INTEGER NOT NULL,
  tackles_completed INTEGER NOT NULL,
  interceptions INTEGER NOT NULL,
  headers_won   INTEGER,
  aerial_challenges INTEGER,
  clearances    INTEGER,
  blocks        INTEGER,
  distance_km   REAL NOT NULL,
  high_intensity_sprints INTEGER NOT NULL,
  saves_held    INTEGER,
  saves_parried INTEGER,
  saves_tipped  INTEGER,
  shots_on_target_faced INTEGER,
  expected_goals_prevented REAL,
  PRIMARY KEY (save_id, player_uid, kind, team_id)
);

CREATE INDEX player_season_stats_player
  ON player_season_stats(save_id, player_uid);
CREATE INDEX player_season_stats_minutes
  ON player_season_stats(save_id, kind, minutes);
```

再增加一个物化评分表，避免每次查询都重算：

```sql
CREATE TABLE IF NOT EXISTS player_metrics (
  save_id        INTEGER NOT NULL,
  player_uid     INTEGER NOT NULL,
  model_version  TEXT NOT NULL,
  position_group TEXT NOT NULL,
  production_pct REAL,
  cost_total     REAL,
  cost_pct       REAL,
  surplus        REAL,
  confidence     REAL,
  risk_flags     TEXT NOT NULL DEFAULT '[]',
  drivers_json   TEXT NOT NULL DEFAULT '[]',
  computed_at    TEXT NOT NULL,
  PRIMARY KEY (save_id, player_uid, model_version)
);
```

评分在 `parse` / `edit` 完成后重算；模型权重变更时 bump `model_version`。原始统计表保留，便于导出和解释。

> 实际实现（Phase 2）把球员/俱乐部/统计展示字段、成本、评分、置信度、flags 和 drivers 都落成 `player_metrics` 普通列，并额外保存 `min_minutes`；不再是 JSON 缓存。详见 [`docs/moneyball-phase2.md`](moneyball-phase2.md)。

### 3.2 评分模型

**位置分组：** GK、DC、FB/WB、DM/CM、AM/W、ST。不要跨位置比较。

**同层比较：** 位置组 × 联赛/声望层级 × 分钟段。当前没有 club → competition 映射，第一版用俱乐部 reputation 分层作为代理；后续再用参考实现的 league tables/fixtures 补联赛成员和对手强度。

**每 90 分钟指标（按位置）：**

| 位置组 | 主要指标 |
|---|---|
| ST / AM | xG/90、xA/90、关键传球/90、射正/90、转化率 |
| DM / CM | xA/90、关键传球/90、渐进传球/90、传球成功率、抢断/90、拦截/90 |
| DC / FB / WB | 抢断/90、拦截/90、争顶成功率、解围/90、封堵/90、传球成功率、关键传球/90 |
| GK | 扑救率、xGP/90（验证后）、每 90 失球、零封；第一版可只用扑救率 + 失球 |

**样本收缩：**

```text
adjusted = (metric * minutes + peer_mean * K) / (minutes + K)
```

K 建议从 900 分钟起步；少于 450 分钟只显示“样本不足”，不参与主榜。

**成本：**

```text
cost_total = fee + wage * 52 * horizon
```

- 默认 `horizon = 3` 年，可由用户调整。
- `fee` 用 `transfer_value` 代理；`zero/placeholder/unset` 不参与费用排序。
- 缺失身价时提供“仅工资”模式，并明确标注“费用未知”。
- 默认显示“存档单位”；用户可在设置中选择显示货币，按近似汇率换算并明确标注。

**属性边际（Attribute edge）：**

- 没有统计或样本不足时，用 CA 豁免/低权重属性 + 角色分做辅助信号：determination、natural fitness、flair、aggression、定位球、长界外球等。
- 计算 `role_surplus = 角色分 − 同 CA 同位置的预期角色分`，作为“低 CA 但角色适配好”的代理。
- 只作为 tiebreaker 或 Value Finder 的主信号，不替代统计产出；隐藏属性默认关闭。

**三个镜头，而不是一个黑箱分数：**

1. **Now**：当前产出 / 总成本，适合 Win-now。
2. **Growth**：PA−CA、年龄曲线、成长快照，适合转售/养成。
3. **Market**：实际成本 vs 同层同画像的预期成本，寻找市场折价。

最后可以给一个 `Moneyball rating`，但必须显示三项分解、置信度和风险。权重放在一个版本化配置里，提供三个预设：

- **Win-now**：当前产出权重高，年龄惩罚低。
- **Resale**：成长/年龄权重高，要求年龄 ≤ 23。
- **Budget**：成本权重高，适合低预算俱乐部。

**风险与置信度：**

- 分钟数、字段覆盖、身价状态 → confidence 0–1。
- injury proneness、consistency、important matches、adaptability、professionalism → risk flags（隐藏属性默认关闭）。
- 年龄、合同剩余、租借/挂牌、停赛 → 可用性/可执行性标记。
- 对低样本、低级别联赛、缺失身价的球员，不进入主榜或加显著标记。

### 3.3 UI 方案

在现有 `AnalysisDialog` 中增加第四个 tab：**Moneyball**（不建议单独做一套页面）。

控制区：

- 预算：最大转会费、最大周薪。
- 位置/角色：位置组 + 具体角色（复用 `roles.rs`）。
- 年龄、合同到期、最少分钟、风险容忍度。
- 预设：Win-now / Resale / Budget。
- “仅看可执行目标”：挂牌、合同年、租借、自由身。

表格列：

- 球员、俱乐部、年龄、位置、分钟。
- 核心每 90 指标（按位置显示 3–5 个）。
- 产出百分位、成本、成本百分位、surplus、confidence、风险标记。

详情面板：

- 三项分解（Now/Growth/Market）。
- 成本拆解：fee + wage × 年限。
- “为什么是他”：贡献最大的 3 个指标。
- 样本量、缺失字段、联赛层级。
- 打开球员详情、加入对比、导出。

与主表联动：

- 新增排序键：`moneyball_score`、`moneyball_confidence`、`moneyball_cost`。
- 或者像 wonderkids 一样，“Apply filters”把条件送回主表。
- 导出复用现有 CSV/JSON 导出，增加字段。

i18n：所有新文案加到 `apps/desktop/src/lib/i18n.ts`，中英文都要有；隐藏属性相关提示复用现有警告。

### 3.4 性能

- 48k 球员、155k 统计行对 SQLite 很小。
- 物化 `player_metrics`，查询只做过滤/排序/分页。
- 索引：`(save_id, model_version, production_pct DESC)`、`(save_id, position_group, surplus DESC)`。
- 解析目标：新增赛季统计解析 < 2 秒；评分物化 < 1 秒；热查询 < 500ms。
- 原始统计和评分分开存储，导出时不依赖 UI 计算。

## 4. 验证与校准

### 4.1 解析验证

- 与参考实现对比行数：误差 < 0.1%。
- 抽样 20 名球员，把 xG、xA、分钟、传球、抢断与游戏内球员资料页对比；计数误差 ±1，xG/xA 误差 ±0.05。
- 分别验证门将和普通球员行，确认 shared words 解码正确。
- 缺 section、截断、重复 key、footer 不匹配时必须 fail-closed，不能产出半份数据。

### 4.2 模型验证

- 前 50 名人工复核（至少 2 名 FM 玩家）：是否出现明显不可用球员、低级别联赛偏差、门将异常。
- 约束：主榜所有球员 ≥900 分钟；身价状态合法；同一俱乐部/联赛占比不超过 30%（除非用户主动筛选）。
- 稳定性：权重 ±20% 时，前 50 名重叠率应保持合理；否则模型太脆弱。
- 回测：用同一生涯的多个游戏日期，检查高 Moneyball 分的球员下一阶段是否获得更多分钟、CA 增长或身价上涨；`player_snapshots` 已支持跨存档快照。
- 解释性：每条推荐必须能说清“哪 3 个指标贡献最大、成本怎么算、风险是什么”。

### 4.3 验收标准

- 解析：参考存档行数匹配；抽样字段通过；解析失败时 UI 明确提示并回退。
- 查询：48k 球员下，Moneyball 首页 < 500ms（热缓存）。
- 模型：前 50 名全部满足分钟/身价约束；人工复核通过；无单一联赛/俱乐部垄断。
- UX：预算、位置、年龄、合同、分钟、风险过滤可用；中英文完整；可导出。
- 安全：不修改源存档；不联网；隐藏属性默认关闭并提示。

## 5. 风险与限制

| 风险 | 影响 | 缓解 |
|---|---|---|
| FM27 布局只验证了一个 build | 解析失败或字段错位 | 特性开关；严格校验；失败回退 Value Finder；只展示已验证字段 |
| 赛季统计字段部分未确认 | 指标不可信 | Phase 0 逐字段对照游戏 UI；v1 只用高置信字段；门将指标单独验证 |
| 样本量小/赛季初 | 榜单噪声大 | ≥900 分钟主榜；收缩；confidence；样本不足单列 |
| 联赛强度无法严格调整 | 低级别联赛假阳性 | 先用俱乐部 reputation 分层；后续接 league tables/fixtures；明确标注 |
| 身价 ≠ 转会费，18% 不可用 | 成本失真 | 用户输入预算；仅工资模式；排除 invalid；显示“估算” |
| 没有签字费/佣金/工资条款 | 总成本低估 | 标注“不含签字费/佣金”；后续解析合同条款 |
| 门将统计覆盖率低 | GK 榜单不可靠 | v1 可先不做 GK，或只用扑救率/失球并单独标注 |
| 单一综合分黑箱化 | 用户不信任/被误导 | 三镜头 + 分解 + “为什么是他”；不隐藏成本/样本 |
| 范围蔓延到“Moneyball XI”优化器 | 工期失控 | v1 不做自动阵容优化；Phase 3 再评估 |
| 线上联赛公平性 | 隐藏属性争议 | 沿用现有单人提示；默认关闭隐藏属性 |

## 6. 路线图与工作量

以下为**单人、熟悉本代码库**的粗略估计，不是承诺：

| 阶段 | 工作 | 交付物 | 估计 |
|---|---|---|---|
| Phase 0 | 用参考实现跑 2–3 个存档；确定 8–10 个可靠指标；定义位置组/层级；人工复核 top 50 | 指标规格 + 验证记录 + Go/No-Go | 2–3 天 |
| Phase 1 | 只用现有数据做 Value Finder：CA/PA、角色分、身价、工资、合同、年龄；透明成本与风险 | 可用的实验性 tab（不宣称统计版 Moneyball） | 1–2 周 |
| Phase 2 | 移植 season stats parser；容器 span frame；schema；评分物化；Moneyball UI | 统计版 Moneyball beta（feature flag） | 3–5 周 |
| Phase 3 | 联赛强度、回测、预设调参、导出/保存视图、门将指标 | v1 | 2–3 周 |

**总计：6–10 周。** 关键路径是 Phase 2 的 parser/container 移植，不是 UI。

如果只有 2 周：做 Phase 0 + Phase 1，产品名用 **Value Finder**，不要叫 Moneyball。它仍然有价值：快速找“低成本、角色适配、合同/挂牌有利”的球员，但不要声称是统计型市场低效。

## 7. 最终建议

**建议 Go，但分阶段。**

1. **先做 Phase 0，不要先写 Rust parser。** 用参考实现验证字段、覆盖率、指标和榜单，确认“统计版 Moneyball”不是数据幻觉。
2. **Phase 1 先交付 Value Finder。** 它能快速验证 UI、预算过滤、风险提示和用户需求，同时不依赖新 parser。
3. **Phase 2 再做统计版。** 数据已经证明存在，parser 是确定性工程；但必须 feature flag + 失败回退。
4. **不要发布单一黑箱分数。** 至少展示 Production / Cost / Risk 三项和置信度；否则用户无法判断推荐是否可信。
5. **不要把 CA/身价比叫 Moneyball。** 它会系统性地偏向“低价高 CA”，而不是“高于市场预期的产出”。

一句话：**数据足够，工程可控，真正的风险在建模和验证；建议用 2–3 天 spike 决定是否进入 Phase 2。**

## 8. 当前实现进度（feature/moneyball）

已在 `feature/moneyball` 分支落地 Phase 1 的第一个增量：

- `crates/fm27-core/src/moneyball.rs`：`ValueFinderRequest` / `ValueFinderRow` / 评分与百分位函数，含单元测试。
- `Store::value_finder()`：从 SQLite 读取轻量球员行，在 Rust 内按位置组计算质量/成本百分位；筛选在评分后执行，避免百分位随筛选漂移。
- Tauri 命令 `value_finder`：桌面端直接调用。
- `AnalysisDialog` 新增 **Value finder / 性价比** tab：位置、联赛、最低 CA、最低角色分、最大年龄、最大转会费/周薪、市场状态、未知转会费开关、排序与分页。
- 中英文 i18n 文案。
- 默认排除未知转会费和未知工资的球员，避免把“费用未知”误当成零成本；免费球员只有在工资已知时才参与排名。

当前模型是 **value-finder-v1**，明确不是统计版 Moneyball：

- 质量 = 角色分百分位 × 0.6 + CA 百分位 × 0.4（位置组内）。
- 成本 = 转会价值 + 3 年工资；未知转会费默认排除。
- 评分 = 0.55 × 质量百分位 + 0.45 × (1 − 成本百分位)。
- 联赛名称只对存档跟踪的联赛可用；其余球员显示 `unknown_league`，仍可通过 CA/年龄/角色分筛选。
- 尚未接入赛季统计、联赛强度、伤病风险和真正的产出指标。

下一步按本文路线图执行：Phase 0 字段/榜单验证 → Phase 2 season stats parser → 模型校准。

## 9. Phase 0 验证结果（2026-11-03）

Phase 0 已完成，详细记录见 [`docs/moneyball-phase0.md`](moneyball-phase0.md)。

关键结论：

- 三个 Parma 存档（base / v02 / v03）共约 15.5 万行赛季统计；结构、`overall` 汇总和每 90 分钟换算全部通过。
- 约 37% 的 ≥900 分钟球员缺少高级统计块：`distance_km == 0` 时，关键传球、渐进传球、拦截、争顶、解围、封堵、跑动、扑救、xGP 全部为 0，但 xG/xA/传球/抢断仍然正常。必须写 `NULL`，不能当 0。
- 高级数据覆盖率随俱乐部声望从 9.8% 升到 98.2%；因此 v1 主榜应使用只含全量字段的 **core** 模型，advanced 只用于有数据的子集。
- 无约束“产出/成本”榜会被 30+ 岁、CA 80–110 的低级别球员占据；加上 `CA ≥ 120`、`年龄 ≤ 30`、有效费用/工资后，榜单跨 3 个存档 × 2 个模型稳定，43–48/50 重叠，且有 30 名球员在全部 6 个榜单中出现。
- 门将扑救数据实际覆盖 64.6%，建议单独榜单；`pressures_*` 和 `goals_outside_box` 在 FM27 中不可用。
- 当前状态为 **Go**：自动验证与 Parma 16 人游戏内对照均已通过；可以进入 Phase 2 parser/schema 工作（`docs/moneyball-phase0.md`）。
- Phase 2 已落地：容器 unlisted span frame、`season_stats.rs`、`player_season_stats` schema、core/advanced 评分、Tauri `stats_moneyball` 命令和 AnalysisDialog “数据版” tab。真实存档 156,272 行与参考实现一致。见 [`docs/moneyball-phase2.md`](moneyball-phase2.md)。
- Sprint A 已落地：固定评分下限、对等组最小样本、经验贝叶斯收缩、npxG、不确定性区间、伤病窗口和统一位置分组。模型升级为 `stats-core-v2` / `stats-advanced-v2`，见 [`docs/moneyball-phase4.md`](moneyball-phase4.md)。
- Sprint B 已落地：同存档折外公平价格模型（`hedonic-ridge-v1`）、`value_gap_pct` 与独立 `bargain_score`；`score` 只表示质量。见 [`docs/moneyball-phase5.md`](moneyball-phase5.md)。
- Sprint C 已落地：8 个位置组、新统计字段、overall/league/calendar 窗口和年龄/合同进入 bargain；模型升级为 v3，价格模型为 `hedonic-ridge-v2`。见 [`docs/moneyball-phase6.md`](moneyball-phase6.md)。
- Sprint D（第一部分）已落地：数据版 CSV/JSON 导出和同位置组相似球员检索。见 [`docs/moneyball-phase7.md`](moneyball-phase7.md)。
- Sprint D（第二部分）已落地：Full / Fair play / Moneyball 三模式，覆盖球员表、筛选、详情、对比、俱乐部阵容、分析中心和数据版；非 Full 模式的价格模型不使用 CA/PA。见 [`docs/moneyball-phase8.md`](moneyball-phase8.md)。
- Sprint E 已落地：阵容需求视图（最弱位置 + 同层升级候选）。赛程/积分榜 parser spike 结论为 No-Go，阻塞点是 fixture team id 无法映射到俱乐部，见 [`docs/league-fixtures-spike.md`](league-fixtures-spike.md)；阵容需求见 [`docs/moneyball-phase9.md`](moneyball-phase9.md)。

## 附录：代码与数据参考

- 球员模型：`crates/fm27-core/src/model.rs`
- 查询/过滤/排序：`crates/fm27-core/src/store.rs`
- 角色分：`crates/fm27-core/src/roles.rs`
- CA 估算与 CA 豁免属性：`crates/fm27-core/src/ability.rs`
- SQLite schema：`shared/schema.sql`
- 现有功能与限制：`docs/features.md`、`docs/limitations.md`
- 参考实现的赛季统计 reader：`.ref-fmsave/src/fmsave/readers/season_stats.py`
- 参考实现的赛季统计模型：`.ref-fmsave/src/fmsave/models/season_stats.py`
- FM27 布局：`.ref-fmsave/src/fmsave/_layouts/fm27_27_0_0.py`
- 验证存档：`BB KING - Parma Calcio.fm`（build `27.0.0+2486695`）
