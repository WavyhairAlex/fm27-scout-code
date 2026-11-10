# Moneyball Sprint D（第二部分）：三模式（2026-11-09）

> 状态：新增全局侦察模式 `Full / Fair play / Moneyball`，持久化在本地设置里。
> 模式同时影响 UI 可见性和后端价格模型：非 Full 模式不使用 CA/PA 给球员定价。

## 1. 三种模式

| 模式 | 可见 | 隐藏 |
|---|---|---|
| Full | CA/PA、属性、角色、成长、隐藏属性（需手动开启） | — |
| Fair play | 属性、角色、位置、合同、身价/工资 | CA/PA、成长（含 CA/PA）、隐藏属性 |
| Moneyball | 位置、合同、身价/工资、统计/价格/相似球员 | CA/PA、全部属性、角色、成长、隐藏属性 |

- 工具栏新增模式选择器；选择会写入 `localStorage`（`fm27scout.dataMode.v1`）。
- 切换模式会清空 CA/PA/PA-CA 差值和隐藏属性筛选；Moneyball 还会清空所有属性筛选。
- 当前排序如果是 CA/PA/角色分，会重置为身价排序，避免用隐藏分数排序泄露信息。
- 隐藏属性开关只在 Full 模式可用；菜单快捷键在非 Full 模式会被忽略。

## 2. 各界面行为

- **球员表**：非 Full 隐藏 CA/PA；Moneyball 再隐藏角色列和全部属性列。列菜单里也无法重新打开被模式排除的列。
- **筛选侧栏**：非 Full 隐藏 CA/PA 输入；Moneyball 隐藏属性规则；非 Full 隐藏隐藏属性开关。依赖 PA 的“小妖/高潜”预设只在 Full 出现。
- **球员详情**：非 Full 隐藏 CA/PA 徽章和成长页；Moneyball 再隐藏属性页和角色页；编辑按钮只在 Full 出现。
- **对比**：非 Full 隐藏 CA/PA；Moneyball 隐藏属性对比并给出提示。
- **俱乐部阵容**：非 Full 隐藏平均 CA/PA 和 CA/PA 列；非 Full 的阵容顺序改为按工资，避免用 CA 顺序泄露能力。分析中心的阵容面板同样处理。
- **分析中心**：非 Full 隐藏概览和小妖页（都依赖 CA/PA）；Moneyball 隐藏性价比页（属性驱动）。
- **数据版**：非 Full 隐藏 CA 列和最低 CA 筛选，请求里的 `ca_min` 置空；Moneyball 下价格模型不使用 CA/PA。
- **性价比页**：非 Full 隐藏 CA；Fair play 的质量改为纯角色分（不再混 CA）。Moneyball 页面本身不显示。

## 3. 后端

- `StatsMoneyballRequest.mode`：`full` / `fair_play` / `moneyball`，默认 `full`。
  - 价格模型的特征向量长度不变；非 Full 时 CA、CA²、PA 三个特征置 0，ridge 会自动给出零系数。
  - `StatsMoneyballMeta.mode` 回传实际模式。
  - 内存缓存键加入 `mode`；只有 `full` 模式写入 `player_metrics`，避免不同模式的价格互相覆盖。
- `ValueFinderRequest.mode`：
  - Full：质量 = 0.6 × 角色分百分位 + 0.4 × CA 百分位。
  - Fair play / Moneyball：质量 = 角色分百分位。
  - 缓存键为 `(save_id, mode)`；`ValueFinderMeta.mode` 回传。
- 模型版本不变（v3 / `hedonic-ridge-v2`）；模式只改变输入特征，不改变评分公式本身。

## 4. 明确没有覆盖的地方

1. **导出没有脱敏**：球员/数据版 CSV/JSON 仍包含 CA/PA（数据版的模式价格已按模式计算）。导出对话框在非 Full 模式会显示警告。要彻底脱敏需要给 `ExportWriter` 增加模式参数并改写 CSV/JSON 列，留到后续。
2. **员工工作区**：Staff 的 CA/PA/属性不受模式影响；三种模式针对球员侦察。
3. **俱乐部概览/联赛排名**：俱乐部层级的声望、财务、历史排名仍显示；只有包含球员 CA/PA 的列表被处理。
4. **API 载荷**：模式是 UI 纪律，不是安全边界；Tauri 命令仍返回完整球员对象。

## 5. 测试

- `stats_moneyball::tests::normalizes_the_data_mode`
- `stats_moneyball::tests::moneyball_mode_removes_ca_and_pa_from_the_price_features`
- `moneyball::tests::fair_play_drops_ca_from_the_value_finder_quality`
- 真实存档 `season_stats_real_save` 通过；Rust 全量 126 个核心测试通过。

前端仍无法在本机构建（没有 Node）；三模式的 UI 改动需要在 Node 24 环境跑 `pnpm build && pnpm test`，重点检查：
- 球员表列显隐和列菜单；
- 筛选侧栏 CA/PA/属性/隐藏开关；
- 详情页签切换（模式切换后自动落到位置页）；
- 分析中心页签和阵容表；
- 数据版/性价比页的 CA 列与请求。

## 6. 下一步

- 联赛积分榜/赛程 parser spike（先只读、fail-closed、与 oracle 对数）。
- 阵容需求视图（最弱位置 + 市场升级候选）。
- 多存档 as-of-date 回测。
