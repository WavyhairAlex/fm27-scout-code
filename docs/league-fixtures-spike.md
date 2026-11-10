# 联赛积分榜/赛程 Parser Spike（2026-11-09）

> 目标：用赛程/积分榜做对手强度和联赛层级，替代俱乐部声望代理。
> 结论：**No-Go（暂不上线）**。记录能扫出来，但球队 id 没有可信的俱乐部映射，无法安全地构建积分榜或对手强度。

## 1. 参考实现的现状

- `fmsave` 的 FM27 布局只注册了 `MATCH_RECORDS`、赛季统计、合同、阶段表等；**没有注册 `FixtureCalendarLayout` / `LeagueTableLayout`**。这两个布局只存在于 FM26。
- FM27 有一个 `experimental_span()`，但源码明确写着：只解码“原始 fixture core 和 counted rules”，**不是语义赛程/积分榜**，不链接规则到赛事，状态未知时为 `None`，不做游戏内验证。
- `save.experimental_span()` 默认帧上限 8 MiB，遇到赛季统计帧（约 29 MB）会直接失败；用自定义 limits 才能读出整段 span。

## 2. 实测数据（`FM27 Scout Editor Test - Simone.fm`，2486695，2026-07-25）

| 项 | 结果 |
|---|---:|
| span 解压字节 | 32,598,789 |
| frames | 1,136 |
| fixture cores | 27,116 |
| strict anchors | 26,952 |
| 日期范围 | 2025-09-04 – 2028-05-26 |
| 有 stage id | 27,116 / 27,116 |
| 有 stadium ordinal | 27,041 / 27,116 |
| played=true / false / unknown | 1,458 / 25,492 / 166 |
| stage id 能 join 阶段表 | 27,116 / 27,116 |

阶段映射是好的；问题在球队 id。

## 3. 阻塞点：fixture team id 无法映射到俱乐部

fixture 的 `home_team_id` / `away_team_id` 在 66,xxx 量级，而：

| 映射目标 | 命中 |
|---|---:|
| `club.teams[].team_id`（俱乐部球队 id，约 51） | **0 / 27,116** |
| `club.uid`（2,662 个偶然重叠） | 2,662 / 27,116 |
| `club.unique_id`（2,702 个偶然重叠） | 2,702 / 27,116 |

也就是说，fixture 记录里的球队 id 属于另一个实体空间，参考实现也没有解码这张映射表。没有映射就无法把比赛归到俱乐部，也就没有积分榜、对手强度或赛程强度。

另一个风险：`AP King - Milan (v03).fm` 是 build 2493426，参考实现的 experimental FM27 布局只覆盖 2486695；即使映射解决，也还要在第二个 build 上重新验证。

## 4. 判定

- **No-Go**：不在当前解析器里加入赛程/积分榜驱动的对手强度。
- 这不是扫描器的问题，而是**缺少比赛球队 → 俱乐部的映射表**；继续猜 offset 或按 id 范围硬匹配会制造看似合理但错误的数据，违反项目“不猜偏移”的约束。

## 5. 解除阻塞需要什么

1. 找到并解码 fixture team id 的映射表（可能在 `game_db` 的 competition/team 入口或俱乐部记录的另一个数组里），用多份存档验证一对多/一对零的情况。
2. 在 2486695 和 2493426 两个 build 上对同一场比赛做游戏内对照（日期、主客队、比分）。
3. 通过后，再按现有季节统计的方式 fail-closed 接入：fixture 行数、stage join、日期窗口、主客队 join 率都要有下限检查。

## 6. 本轮的替代交付

不依赖新解析的**阵容需求视图**：用现有 v3 统计评分，按位置组找出本队最弱位置，并列出同组、费用已知、质量更高的市场候选（质量提升、公平价、价差、性价比）。详见 [`moneyball-phase9.md`](moneyball-phase9.md)。
