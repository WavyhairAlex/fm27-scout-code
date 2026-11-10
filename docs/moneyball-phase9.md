# Moneyball Sprint E：阵容需求视图（2026-11-09）

> 状态：分析中心“阵容”页新增最弱位置 + 同层升级候选。
> 依据：[`league-fixtures-spike.md`](league-fixtures-spike.md) 的 No-Go 结论——赛程球队 id 还无法映射到俱乐部，因此本轮用现有 v3 统计评分，不引入新解析。

## 1. 规则

对选定俱乐部，按位置组取**本队当前球员**的统计质量（`score`，组内百分位）：

- 只看 300+ 分钟（评分下限），所以赛季初也能覆盖大部分一线队。
- 排除 `stats_from_other_club`：租借在外的球员数据不属于母队的当前水平。
- 每个位置组取质量最高的球员作为“当前水平”。
- 位置组按当前水平从低到高排序，UI 显示最弱的 4 组。

升级候选：

- 同一位置组、**同一俱乐部声望层级**、其他俱乐部；
- 300+ 分钟、费用已知、`score` 高于本队当前水平；
- 按 `bargain_score` 降序（没有价格基准时按质量降序），每组取 3 人；
- 输出质量提升、费用、公平价、价差和性价比。

**为什么限制同层**：位置百分位是在“位置组 × 声望层级”内计算的，跨层直接比质量会把低级别联赛的高分球员误当成升级目标。真实存档上未加限制时，皇马的位置需求会列出秘鲁/哥伦比亚低级别球员；限制同层后恢复为同级别俱乐部。

## 2. 实现

- 核心纯函数：`stats_moneyball::squad_needs_from_rows`。
- Store：`Store::squad_needs(SquadNeedsRequest)`，复用 `stats_scored_rows` 缓存；默认 `core` / `overall` / 300 分钟。
- Tauri 命令：`squad_needs`。
- 前端：`useSquadNeeds(clubUid)`；分析中心新增独立“需求”页签（带俱乐部选择器），俱乐部检查器的阵容页也显示同一区块；点击候选打开球员详情。
- 模式：三种侦察模式都可使用；不使用 CA/PA，只看统计质量与价格。

## 3. 真实存档示例（AP King - Milan，2027-10）

A.C. Milan（8 个位置组，按当前水平升序）：

| 位置 | 当前最好 | 质量 | 示例升级 |
|---|---|---:|---|
| ST | Francesco Camarda | 44.1 | Franjo Ivanović（Benfica，质量 +24.1，费用 5.2M，价差 +88%） |
| DM | Christian Comotto | 49.8 | Giacomo Faticanti（Juventus，+17.0，费用 49k，价差 +181%） |
| MC | Adrien Rabiot | 51.5 | Pelayo（Real Madrid，+16.4，费用 123k，价差 +123%） |
| DC | David Odogu | 63.5 | David Heindl（Bayern，+6.4，费用 16k，价差 +186%） |
| FB | Davide Bartesaghi | 63.9 | Maximilian Hennig（Bayern，+11.9，费用 443k） |
| WM | Samuel Chukwueze | 64.6 | Amad（Arsenal，+16.2，费用 61.7M） |
| AMC | Nico Paz | 69.9 | Giacomo De Pieri（Inter，+12.6，费用 5.1M） |
| GK | Mike Maignan | 92.6 | Diego Kochen（Barcelona，+5.8，费用 2.6M） |

这些是**候选**，不是推荐买入：没有预算、工资结构、报名规则和球员意愿；费用是市场估值，不是要价。

## 4. 测试

- `squad_needs_finds_the_weakest_group_and_affordable_upgrades`：同队最好、同组候选、未知费用排除、弱于当前水平排除。
- `squad_needs_ignore_other_tiers_and_loan_stats`：跨声望层排除、租借数据排除。
- 真实存档手工检查：Milan 8 组，弱点排序和同层候选符合预期。

## 5. 已知限制

1. **早期样本噪声**：300 分钟的质量百分位波动大；UI 只在阵容页作为概览，不应替代数据版的保守排序。
2. **没有预算约束**：只要求费用已知；候选可能远超俱乐部预算。
3. **没有联赛强度**：同层限制只能防止跨层误比，不能修正同层内的联赛差异。
4. **租借在外球员不参与母队水平**：这是有意的；租借在内的球员正常计入。
5. **候选只按性价比排序**：没有考虑位置适配、角色、年龄曲线或合同剩余。

## 6. 下一步

- 赛程 parser 需要先解决 fixture team id → 俱乐部的映射（见 spike 文档）。
- 多存档 as-of-date 回测。
- 预算/工资上限进入阵容需求视图。
