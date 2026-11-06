# Rust 性能审查与优化建议

> 审查日期：2026-11-03；实施更新：2026-11-04
> 范围：`crates/fm27-core`、`crates/fm27-cli`、`apps/desktop/src-tauri`，以及 `shared/schema.sql`。
>
> 本文最初是**静态代码审查**，随后已实施第一批优化；完成项和验证结果见第 11 节。工作区内没有真实 `.fm` 存档，因此仍未运行 Criterion、峰值 RSS 或完整解析基准。除特别说明外，文中的收益均为“预期收益”，具体幅度必须用真实存档验证。
>
> 当前工作区包含未提交修改；行号以审查时的文件内容为准。

## 1. 结论摘要

当前 release 配置已经使用 Thin LTO 和单 codegen unit，解析主流程也不是明显的算法级瓶颈。最值得优化的地方集中在三类：

1. **解析热循环中的确定性浪费**：角色评分构造了几乎所有最终会被丢弃的对象，并对属性/位置做大量字符串线性查找。
2. **交互请求重复全量计算**：Value Finder 每次翻页都重新读取并评分全部球员；Stats Moneyball 的分钟阈值会生成无限缓存和持久化物化行。
3. **桌面端阻塞与锁粒度过大**：重查询、评分、导出以同步 Tauri 命令执行，并共用一把全局 `Store` 锁；导出甚至持锁写文件。

### 优先级总览

| ID | 优先级 | 热点 | 主要证据 | 预期收益 | 风险 |
|---|---|---|---|---|---|
| R01 | P0 | `best_role` 构造全部 29 个角色后丢弃 | `roles.rs:537-583` | 高：CPU、分配、导入时间 | 低-中 |
| R02 | P0 | Value Finder 每次请求全量读取和评分 | `store.rs:1504-1550` | 高：翻页/筛选/排序延迟 | 中 |
| R03 | P0 | Stats Moneyball 阈值缓存无界、重复扫描 | `store.rs:43-53`、`stats_moneyball.rs:697-805` | 高：首次评分与连续输入 | 中 |
| R04 | P0 | 同步 Tauri 命令 + 全局 Store 锁 | `lib.rs:261-323`、`lib.rs:651-660` | 高：UI 卡顿、请求排队 | 中 |
| R05 | P0 | 未知大小 zstd 帧按 64 MiB / 1 GiB 上限预分配 | `container.rs:174-179`、`container.rs:320-349` | 中-高：峰值内存/容量 | 中 |
| R06 | P1 | 导出用分页 API 反复 COUNT + OFFSET 并整批入内存 | `store.rs:941-1058`、`export.rs` | 中-高：大导出时间/内存 | 中 |
| R07 | P1 | 属性解码重复字符串查表；JSON 反序列化先建 owned-key BTreeMap | `parse.rs:308-317`、`model.rs:36-46` | 中-高：解析与分页分配 | 低 |
| R08 | P1 | 俱乐部列表每次执行两遍相同聚合 CTE | `store.rs:22-41`、`store.rs:1888-1927` | 中：俱乐部页延迟 | 低-中 |
| R09 | P1 | 存档写出同时保留完整源文件和完整输出 | `container.rs:408-446` | 中：编辑峰值内存 | 中 |
| R10 | P1 | 多个模块重复朴素字节搜索，并重复扫描相同 selector | `parse.rs:650-697`、`contracts.rs:240-275` | 中：解析常数开销 | 中 |
| R11 | P1 | CLI `index` 缓存命中前仍完整解析 | `fm27-cli/src/main.rs:213-225` | 中：重复索引耗时 | 低 |
| R12 | P2 | 赛季统计固定槽位使用多个小 `Vec`，合同选择创建临时 `pool` | `season_stats.rs:258-263`、`contracts.rs:612-625` | 低-中：分配次数 | 低 |
| R13 | P2 | 两个 SQLite 索引与主键重复/左前缀重复 | `shared/schema.sql:157-162`、`324-329` | 低：写放大和存储 | 低 |
| R14 | P2 | 批量俱乐部/球场编辑存在 O(K×N)、O(E×S) 查找 | `facilities.rs:123-150`、`stadiums.rs:20-22` | 低-中：批量编辑 | 低 |

## 2. 历史基线（不是本次实测）

项目文档记录的参考存档：

| 指标 | 历史记录 | 来源 |
|---|---:|---|
| 存档大小 | 49,042,131 bytes | `docs/validation.md` |
| 解压后 `game_db` | 约 137 MB | `docs/validation.md` |
| 球员数 | 46,273 | `docs/validation.md` |
| 合同数 | 44,341 | `docs/validation.md` |
| 球场数 | 27,042 | `docs/validation.md` |
| 解析（release） | 约 1.3 s | `docs/validation.md` |
| 解析 + SQLite 写入 | 约 4.5 s | `docs/validation.md` |
| 后续俱乐部扩展存档解析 | 2.10 s | `docs/validation.md` |

这些数字来自不同时间点和不同功能范围，不能直接作为当前回归基线。尤其“解析 1.3 s、解析 + 写入 4.5 s”说明写入/序列化路径曾占总时间的大部分，值得优先建立分阶段基准。

## 3. 高优先级优化

### R01. 重写 `best_role` 热路径，避免构造后丢弃 28/29 个角色

**位置**

- `crates/fm27-core/src/parse.rs:425`
- `crates/fm27-core/src/roles.rs:537-583`
- `crates/fm27-core/src/model.rs:20-25`、`model.rs:64-70`

**现状**

每名球员导入时调用 `best_role`，而 `best_role` 先调用 `score_roles`：

- 遍历全部 29 个角色；
- 每个角色构造完整 `RoleScore`；
- 每个 `RoleScore` 分配 `id`、`name`、`group`、`position` 四个 `String`；
- 最后 `max_by` 只保留一个，其余立即丢弃。

同时，`score_role` 的 271 个属性权重项通过 `Attributes::get` 查找，后者每次线性扫描 52 个属性键；位置查找也线性扫描 15 个键。按 46,273 名球员计算，仅角色结果字符串就有约 537 万次分配，其中约 518 万次属于最终丢弃的对象。

**建议**

1. 为 `best_role` 增加轻量折叠路径：只计算 `(RoleSpec, score, position_rating)`，最后仅为获胜角色构造一次 `RoleScore`。
2. 保留现有 `score_roles` 给角色详情面板，因为它确实需要全部结果。
3. 把角色权重预编译为 `(attribute_index, weight)`，角色位置预编译为 `position_index`；静态初始化一次，之后热循环不再做字符串查找。
4. 预计算每个角色的权重总和。
5. 属性查找改为下标访问；`Positions::get` 同理。

**必须保持的语义**

- 缺失属性仍按 1 处理，缺失位置仍按 1 处理。
- 浮点累加顺序尽量保持不变，避免临界分数变化。
- Rust 的 `Iterator::max_by` 在同分时返回最后一个最大值；重写折叠时必须保留该选择规则。
- 增加测试覆盖全部 52 个属性键、15 个位置键以及同分角色。

**验证**

- 对 `best_role` 单独做微基准，并统计分配次数。
- 对比优化前后全部 46,273 名球员的 `best_role_id/name/score/position`，要求逐项一致。
- 再测完整解析总时间，避免只优化微基准却没有改善端到端。

### R02. 为 Value Finder 建立请求无关的评分基线缓存

**位置**

- `crates/fm27-core/src/store.rs:1504-1550`
- `crates/fm27-core/src/moneyball.rs:329-606`
- `apps/desktop/src/lib/query.ts:153-160`
- `apps/desktop/src/components/AnalysisDialog.tsx:1398-1434`

**现状**

`Store::value_finder` 的 SQL 只有 `WHERE p.save_id = ?1`，没有请求筛选或分页。每次翻页、改筛选或改排序都会：

1. 读取该存档全部候选球员及其合同、俱乐部；
2. 重建位置组和联赛集合；
3. 为每个联赛收集 CA 并排序求中位数；
4. 对全部位置组重新计算角色、CA、成本百分位；
5. 对完整过滤结果排序；
6. 只返回当前 40 行。

页面大小是 40，但每次请求都重新做全量工作。

**建议**

采用与 Stats Moneyball 相同的“评分”和“请求过滤”分离结构：

- `score_value_candidates(save_id)`：只依赖存档，计算百分位和基础评分；
- `apply_value_request(&scored, request)`：执行筛选、排序、分页；
- 以 `save_id` 为键缓存 `Arc<Vec<ValueFinderRow>>`；
- 缓存联赛列表和联赛 CA 中位数等派生元数据；
- 存档删除、重新索引或编辑后使缓存失效。

需要跨进程冷启动加速时，再把基线物化到 SQLite；不必第一步就改 schema。

**必须保持的语义**

- 百分位必须在用户筛选之前计算，不能先筛选再算百分位。
- 缓存基线应保留未知费用球员，由 `include_unknown_fee` 在请求阶段决定是否排除。
- `meta.scored`、`meta.filtered` 和 `data.total` 的现有含义不能因拆分而改变。
- 缓存需要有界或按存档淘汰，避免长时间运行持续增长。

### R03. 收紧 Stats Moneyball 的缓存维度与重复扫描

**位置**

- `crates/fm27-core/src/store.rs:43-53`
- `crates/fm27-core/src/store.rs:1364-1498`
- `crates/fm27-core/src/stats_moneyball.rs:697-805`
- `apps/desktop/src/components/AnalysisDialog.tsx:892-898`、`1935-1957`

**现状 A：阈值形成无限缓存维度**

`StatsCacheKey` 和 `player_metrics` 的主键都包含精确 `min_minutes`。分钟输入框每次 `onChange` 都会请求一个新阈值；每个新阈值可能：

- 全量读取赛季统计；
- 重新评分；
- `DELETE/INSERT` 一整套 `player_metrics`；
- 新增一个没有上限的内存 `Arc<Vec<_>>` 缓存条目。

**现状 B：均值阶段重复全表扫描**

`score_stats_candidates` 对 6 个位置组循环，每组调用两次 `peer_means`（global 和 tier）。每次 `peer_means` 都遍历完整候选集合并重新分桶，因此最多产生 12 次全候选扫描。候选的位置组已经在前一步计算过，却没有被复用。

`position_group()` 还会在热循环中拆分字符串、收集 `Vec<&str>`，并可能分配大写字符串；`production()` 构造的全局 drivers 在 `stats_moneyball.rs:905` 被直接丢弃。

**建议**

1. 前端对分钟输入防抖，或在失焦/确认后再请求；不要每敲一个数字就重评分。
2. 将常用阈值限制为预设值或明确量化步长。
3. 内存缓存改为有界 LRU；`player_metrics` 增加每存档/模型的保留策略。
4. 非标准临时阈值可以只返回内存结果，不永久写回数据库。
5. 每名候选只计算一次紧凑的 `position_group` 和 reputation tier。
6. 在一次分桶遍历中计算需要的均值，或只计算当前组；不要为每个组重扫全表。
7. 为全局 production 提供不构造 drivers 的轻量路径。

**不要做的简化**

不能简单改成“始终按 900 分钟评分，再按其他分钟数过滤”。当前阈值会改变候选集合、均值和百分位，直接过滤会改变排名语义。任何缓存或量化方案都必须保持同一阈值下的结果一致。

### R04. 将重命令移出主线程，缩短全局 Store 锁持有时间

**位置**

- `apps/desktop/src-tauri/src/lib.rs:261-323`
- `apps/desktop/src-tauri/src/lib.rs:579-660`
- `apps/desktop/src-tauri/src/lib.rs:182-250`

**现状**

`query_players`、`value_finder`、`stats_moneyball`、`query_clubs`、`export_players` 等是同步 Tauri 命令。重计算或文件 I/O 会占用命令执行线程；整个应用又只有一个 `Arc<Mutex<Store>>`，所有数据库工作被串行化。

`export_players` 在 `lib.rs:658-660` 获取 `store` 后，直到 `write_export` 完成都没有显式释放；CSV/JSON 序列化和磁盘写入都占着数据库锁。

`open_save` 虽然把实际解析放进了 `spawn_blocking`，但 `inspect_save`、快照复制和 `insert_parsed` 仍有部分在阻塞池之外，需要单独检查。

**建议**

1. 将数据库查询、评分和导出移入 `spawn_blocking`，或使用明确的异步命令。
2. 在阻塞闭包内短期获取 `Store` 锁；不要跨 `.await` 持有 `std::sync::MutexGuard`。
3. 至少先 `drop(store)` 再写导出文件，立即消除“写磁盘占用数据库锁”。
4. 如果查询并发确实重要，使用多个只读连接或连接池；仅把同一把锁移到后台线程只能改善 UI 响应，不能提高吞吐量。
5. 将 `inspect_save`、快照复制、`insert_parsed` 一并纳入统一的阻塞任务边界。

**风险**

多连接会引入缓存一致性、写事务协调和 WAL 并发问题。建议先做最小改动：后台执行 + 缩短锁范围，再评估连接池。

### R05. 避免未知大小 zstd 帧按 64 MiB / 1 GiB 上限预分配

**位置**

- `crates/fm27-core/src/container.rs:23-26`
- `crates/fm27-core/src/container.rs:174-179`
- `crates/fm27-core/src/container.rs:320-349`
- `crates/fm27-core/src/container.rs:1115-1134`

**现状**

`decompress_frame` 对没有 `expected_size` 的帧把 cap 直接传给 `zstd::bulk::decompress`：

- 目录帧的 cap 是 64 MiB；
- 赛季统计等区域帧的 cap 是 1 GiB。

当前锁定的 zstd 版本在没有 frame content size 时，会按这个上限为输出 `Vec` 预留容量。赛季统计的 `body` 在 walk/decode 期间一直持有该容量。这是明确的**容量请求**，不等于等量常驻物理内存，因为分配页可能按需触碰；但大存档下的峰值 RSS 仍需实测。

**建议**

1. 先读取 zstd frame header 的 content size；若存在且不超过 cap，按声明大小分配，解压后仍校验实际长度。
2. 对没有 content size 的帧使用有上限的流式解压，让输出按实际大小增长，并读取到帧尾以保留校验和/截断检查。
3. `read_region_frame` 可评估从 `File::take(compressed_size)` 直接流式解压，避免同时持有完整压缩缓冲区和完整解压结果。
4. 保留现有 frame cap、总 cap、校验和与提前 EOF 行为。

**实测结果（2026-11-04）**

在参考存档上，未知大小帧使用流式解码会让峰值 RSS 增加约 30 MiB，原因是 decoder window。最终实现只对已知 content size 使用精确分配；未知大小继续使用有 cap 的 bulk 解压。这样保留了低 RSS，同时仍避免已知帧的过度容量请求。

**验证**

分别测量 `SaveFile::open`、目录解压、赛季统计帧读取的：

- `Vec::capacity`；
- 峰值 RSS；
- 解压耗时；
- 损坏帧/截断帧的错误路径。

### R06. 为导出提供专用流式游标

**位置**

- `crates/fm27-core/src/store.rs:941-1058`
- `crates/fm27-core/src/export.rs:17-123`
- `apps/desktop/src-tauri/src/lib.rs:651-660`

**现状**

`query_all` 通过分页 API 实现导出：

- 每 500 行调用一次 `query_players`；
- 每页都执行一次 `COUNT(*)`，即使导出不需要总数；
- 使用递增 `OFFSET`；
- 所有 `PlayerRow` 先收集到 `Vec`，然后才写 CSV/JSON；
- 桌面路径还在持有数据库锁时完成文件写入。

CSV/JSON 写出本身已经使用流式 writer，问题主要在前面的分页和整批物化。

**建议**

1. 提供专用导出查询：一次查询、逐行读取、立即写入 writer。
2. 若必须复用分页，使用以 `p.uid` 为唯一 tie-breaker 的 keyset/cursor 分页，避免深 `OFFSET`。
3. 文件 I/O 前释放数据库锁；若流式读取需要长期持有连接，改用独立只读连接。
4. 失败时写入临时文件，成功后原子替换，避免留下半份导出。

**验证**

- 46k 球员 CSV/JSON 的峰值内存、耗时和输出逐字节一致性。
- 默认排序、多列排序、空结果和导出中断路径。

## 4. 中优先级优化

### R07. 去掉解析与分页中的重复字符串查表和中间 map

**位置**

- `crates/fm27-core/src/parse.rs:308-317`
- `crates/fm27-core/src/layout.rs:544-556`
- `crates/fm27-core/src/model.rs:36-46`、`model.rs:73-83`
- `crates/fm27-core/src/store.rs:977-1031`

**现状 A：解析属性时重复求下标**

循环本身已经拥有 `index`，却又调用 `stored_attribute_index(key)` 线性扫描 `ATTRIBUTE_KEYS`，然后 `unwrap_or(index)`。按 46,273 名球员计算，这里约产生 6,376 万次键比较。

建议在热点中直接计算：

```rust
let stored_index = if index < PlayerLayout::LEFT_FOOT_INDEX {
    index
} else {
    index + 2
};
```

保留 `stored_attribute_index` 给编辑器处理外部动态键。用 `layout_tests` 覆盖全部 52 项和两个脚部字节。

**现状 B：JSON 反序列化先建 owned-key BTreeMap**

`Attributes` 和 `Positions` 的 `Deserialize` 都先构造 `BTreeMap<String, _>`，再复制到定长数组。默认每页 150 行时，仅这两个字段就产生约 10,050 个临时键字符串和 BTree 节点。

建议实现流式 `Visitor`：

- 使用 `MapAccess` 逐字段读取；
- 键映射为固定下标后直接写数组；
- 未知字段用 `IgnoredAny` 保持忽略；
- 缺失字段默认 0；
- 重复已知字段保持“后值覆盖前值”。

**现状 C：默认隐藏模式仍解析 personality**

`query_players` 无论 `include_hidden` 与否都会先解析 `personality_json`，随后默认路径又把 `row.personality = None`。建议在读取时根据 `include_hidden` 跳过解析。

**验证**

- JSON 兼容性测试：缺字段、未知字段、重复字段、格式错误。
- 150 行和 500 行分页的分配次数与 p95 延迟。
- 默认隐藏和 `include_hidden=true` 两种输出完全一致。

### R08. 合并或按需构建俱乐部列表的聚合 CTE

**位置**

- `crates/fm27-core/src/store.rs:22-41`
- `crates/fm27-core/src/store.rs:1888-1927`

**现状**

`query_clubs` 的计数查询和取页查询各带一次相同 `CLUB_CTE`。该 CTE 会扫描当前存档全部球员、合同和球场，按俱乐部聚合人数、平均年龄/CA、总身价、总工资、球场数和容量。即使俱乐部搜索只命中少数球队，也会先构造完整聚合；即使筛选条件完全不涉及聚合字段，计数查询仍然带上这些 CTE。

**建议**

1. 短期：当筛选条件不引用 stats/stads 时，计数只查 `clubs`。
2. 或者使用 `COUNT(*) OVER()` 在一次查询中得到总数和当前页；注意空页时窗口函数不返回行，需要回退计数。
3. 按筛选条件决定是否构建对应 CTE，而不是始终构建两个。
4. 长期：存档导入后数据不可变，可在导入时物化每俱乐部统计列并建立索引。

**验证**

- 默认列表、俱乐部名搜索、按球员人数/平均 CA/球场容量筛选的 `EXPLAIN QUERY PLAN`。
- 深页、空页和总数一致性。
- 物化统计必须保持自由球员、合同和球场聚合口径不变。

### R09. 存档写出改为流式复制未替换帧

**位置**

- `crates/fm27-core/src/container.rs:408-446`
- `crates/fm27-core/src/container.rs:451-541`
- 调用方：`crates/fm27-core/src/editor.rs:403-416`、`club_editor.rs:557-570`

**现状**

`write_with_replacements_and_name`：

- 用 `std::fs::read` 把完整源存档读入 `source`；
- 再按 `source.len()` 分配完整 `output_bytes`；
- 所有 gap、未替换帧和 tail 都复制到输出缓冲；
- 两者一直共存到统一 `write_all`。

对约 49 MB 存档，仅这两个缓冲区就接近 98 MB，还未计入 137 MB 解压 `game_db` 和压缩后的替换帧。

**建议**

第一阶段保留不可变 `source` 的一致性语义，但把输出改为 `BufWriter<NamedTempFile>` 分段写入：

- header；
- gap；
- 替换帧或原帧；
- tail；
- 新 trailer。

单独维护 `new_offset` 和最终文件长度。这样可消除整个 `output_bytes` 和一次全文件复制。继续保留临时文件、`sync_all`、`persist` 的原子替换语义。

若要进一步去掉 `source`，必须从稳定快照流式读取；不能在源文件可能被游戏改写时直接边读边写。编辑后的完整重解析验证也不应为了性能删除。

**验证**

- 空替换、多个替换、unlisted gap、尾部区域。
- 输出逐字节/哈希对比。
- 大存档峰值 RSS、写耗时和失败清理。

### R10. 统一字节搜索并减少重复 selector 扫描

**位置**

- `crates/fm27-core/src/parse.rs:267-292`、`parse.rs:650-697`
- `crates/fm27-core/src/contracts.rs:240-275`、`contracts.rs:736-746`
- `crates/fm27-core/src/player_scan.rs:77-99`、`player_scan.rs:222-229`
- `crates/fm27-core/src/clubs.rs:1019-1029`
- `crates/fm27-core/src/facilities.rs:153-160`
- `crates/fm27-core/src/stadiums.rs:183-193`
- `crates/fm27-core/src/nations.rs:176-183`

**现状**

多个模块各自实现相同的 `find_bytes`，均使用 `windows(needle.len()).position(...)` 逐窗口比较。每名球员还会：

1. `closing_person_header` 搜索 `(pindex + 1)`；
2. `PersonDecoder` 扫描人员块；
3. `ContractDecoder::find_chain_records` 再次搜索相同 selector。

closing 和 contract 的窗口边界略有差异，但高度重叠，needle 完全相同。

另外，`clubs.rs:950-1016` 的状态表发现逻辑在首个命中前把搜索范围设为整个 `game_db`，最多允许 16 次前导全库未命中；异常或稀疏输入下可能出现扫描放大。

**建议**

1. 先抽出一个共享搜索封装，保持“返回首个命中”的语义，内部评估 `memchr::memmem`（多字节）和 `memchr`（单字节）。这是低风险常数优化，不改变算法结构。
2. 若 profile 仍显示热点，让每个球员窗口的一次 selector 搜索同时产出 closing candidates 和 chain candidates，再由两个解码器分别验证。
3. 状态表优化应改为“一次扫描建立候选索引，再按现有规则验证”，不要简单缩小窗口或首次 miss 就退出，否则会改变合法稀疏存档的解析结果。

**注意**

`memmem` 只降低搜索常数，不解决设施批量编辑的 O(K×N)，也不能替代减少扫描次数。短模式和频繁候选验证可能压缩收益，必须用真实存档基准决定是否值得引入依赖。

### R11. CLI `index` 先查缓存，再决定是否完整解析

**位置**

- `crates/fm27-cli/src/main.rs:213-225`
- 可复用模式：`apps/desktop/src-tauri/src/lib.rs:182-199`

**现状**

`Command::Index` 先执行 `parse_save`，再打开 Store 并 `find_cached`。缓存命中时仍然支付了完整解析成本。

**建议**

改为：

1. `inspect_save` 获取 fingerprint；
2. 打开 Store 并 `find_cached`；
3. 缓存未命中或 `--force` 时才 `parse_save` 并 `insert_parsed`。

必须保持与当前相同的 parser version 和 fingerprint 口径，以及 `--force` 的语义。

## 5. 低优先级与批量路径

### R12. 减少固定槽位和临时集合分配

- `season_stats.rs:258-263`：每条赛季记录为固定 8 个 `own_lines` 创建一个 `Vec`，每个其他球队又为固定 6 个槽位创建 `Vec`。可评估改为 `[Option<usize>; OWN_SLOTS]` 和 `[Option<usize>; OTHER_TEAM_SLOTS]`。槽位数量是编译期常量，分配次数下降是确定的；数组会增大内联结构尺寸，需同时测峰值内存。
- `season_stats.rs:476-500`：每行统计都克隆 `player_name`、`club_name`、`club_short_name`。若这些名称只用于写入和展示，可考虑借用视图、ID 或共享字符串；这会扩大改动范围。
- `contracts.rs:612-625`：`select_in_effect` 先把所有 `has_terms` 记录收集到临时 `Vec`，随后只消费一次。可以直接在后续循环中使用 `iter().enumerate().filter(...)`。必须保持 `at_own_club`、`latest`、`earliest_running` 的选择优先级。
- `player_scan.rs:33-45`：marker 扫描和 completeness 扫描各自按 offset 有序，当前拼接后又做 `O(P log P)` 排序。可改为线性 merge + 去重，降为 `O(P)`；需保留 485 条 markerless 记录和最终顺序。

### R13. 清理重复 SQLite 索引

**位置**

- `shared/schema.sql:157`：主键 `(save_id, player_uid, kind, team_id)`
- `shared/schema.sql:161-162`：`player_season_stats_player(save_id, player_uid)`
- `shared/schema.sql:324`：主键 `(save_id, club_uid, month)`
- `shared/schema.sql:328-329`：`club_finances_club(save_id, club_uid, month)`

**结论**

- `club_finances_club` 与主键列和顺序完全相同，是明确重复。
- `player_season_stats_player` 是主键左前缀，查询上可由主键支持。

在内存 SQLite 中执行 `EXPLAIN QUERY PLAN` 验证：

```text
season stats by player
  SEARCH player_season_stats USING INDEX player_season_stats_player (save_id=? AND player_uid=?)
club finance history
  SEARCH club_finances USING INDEX club_finances_club (save_id=? AND club_uid=?)
```

删除两个二级索引后，相同查询改用：

```text
sqlite_autoindex_player_season_stats_1
sqlite_autoindex_club_finances_1
```

**迁移注意**

1. 只从新库 schema 删除 `CREATE INDEX` 不够，旧库索引不会自动消失；需要显式迁移：
   ```sql
   DROP INDEX IF EXISTS club_finances_club;
   DROP INDEX IF EXISTS player_season_stats_player;
   ```
2. 检查所有 SQL 是否存在 `INDEXED BY club_finances_club` 或 `INDEXED BY player_season_stats_player`；有则删索引后会报错。
3. `player_season_stats_player` 更窄，某些只按球员读取的索引扫描可能有 I/O 优势；应先基准再删。
4. 删除索引不保证数据库文件立即缩小；空间通常先进入空闲页，不应仅为观察文件缩小而执行高成本 `VACUUM`。
5. 对批量插入和代表查询重新做 `EXPLAIN QUERY PLAN` 与 p95 测试。

### R14. 批量化俱乐部/球场编辑查找

**位置**

- `crates/fm27-core/src/facilities.rs:123-150`
- `crates/fm27-core/src/club_editor.rs:383-405`
- `crates/fm27-core/src/stadiums.rs:20-22`
- `crates/fm27-core/src/club_editor.rs:237-247`、`482-497`
- `crates/fm27-core/src/club_editor.rs:293-321`

**现状**

- `locate_facility_row` 每次从偏移 0 搜索俱乐部键。对 K 个不同俱乐部做设施编辑，最坏 O(K × game_db 大小)。
- `StadiumIndex::stadium` 对 27,042 个球场做线性 `find`；应用和验证各存在 O(E×S) 路径。
- `ClubEditor::open` 无条件解码完整球场表，即使请求只修改声望或财务。

**建议**

1. 为编辑器建立一次性的 `club_uid -> FacilityRowOffsets` 索引；复用 `read_club_facilities` 的单遍扫描和验证逻辑。
2. `StadiumIndex` 构建时保存 `uid -> position`，将 `stadium()` 改为 O(1)；验证循环前建立一次 `HashMap<u32, &Stadium>`。
3. 球场索引改为首次需要时 lazy 初始化，改善非球场编辑。
4. 保留“首个有效行”、重复行和 `used_rows` 的冲突语义。

## 6. 其他可选项（先测再做）

| 项目 | 位置 | 建议 | 前置条件 |
|---|---|---|---|
| 搜索和 JSON 筛选索引 | `store.rs:2750-3004`、`schema.sql:75-87` | 仅为热点位置/属性和 `pa-ca` 增加生成列/表达式索引；搜索评估 FTS5 | 必须先用 `EXPLAIN QUERY PLAN` 确认扫描与临时 B-tree |
| Analysis Overview | `store.rs:1649-1788` | 循环外格式化截止日期；移除未使用 `pa`；中位数用 `select_nth_unstable` | 保持空年龄、空俱乐部、CA=0 口径 |
| 球员详情角色面板 | `lib.rs:579-594`、`store.rs:1553-1575` | 只读取 `attributes_json` 和 `positions_json`，避免重复解析完整 `data_json` | 保持 `include_hidden=true` 语义 |
| 筛选选项请求 | `store.rs:2245-2469`、`apps/desktop/src/lib/query.ts` | 从 React Query key 去掉排序等无关字段；缓存基础 facet | facet 各自清除不同维度，不能简单合并 |
| 解析并行化 | `parse.rs:210-496` | 在角色、扫描、分配优化后评估 Rayon indexed parallel iterator | 保持球员顺序、进度和确定性；先测内存带宽与分配器竞争 |
| WAL checkpoint | `store.rs:819-824` | 比较 `TRUNCATE`、`PASSIVE` 和自动 checkpoint | 记录导入尾延迟、WAL 峰值和磁盘双份占用 |

## 7. 建议实施顺序

### 阶段 0：建立可信基线

1. 增加 Criterion 基准：容器解压、球员扫描、人员块解码、`best_role`、赛季统计 walk/decode、SQLite 写入。
2. 为完整解析增加分阶段计时：打开容器、解压 `game_db`、俱乐部索引、球员扫描、人员/合同、赛季统计、角色、序列化、SQLite。
3. 记录峰值 RSS 和分配次数，不只记录 wall time。
4. 对默认球员页、Value Finder、Stats、俱乐部页、导出和模糊搜索做 p50/p95 查询基准。
5. 固定真实存档、release 配置和机器环境。

### 阶段 1：低风险高收益

1. R01 `best_role` 轻量路径和预编译索引。
2. R07 属性下标直算、跳过未使用 JSON、流式 Visitor。
3. R11 CLI 先查缓存。
4. R13 删除确定重复索引并迁移旧库。
5. R04 先缩短导出锁范围，再把重命令移入 `spawn_blocking`。

### 阶段 2：交互与查询架构

1. R02 Value Finder 基线缓存。
2. R03 Stats 有界缓存、防抖、均值单次分桶。
3. R06 专用流式导出。（已完成）
4. R08 俱乐部 CTE 合并/物化。
5. R05 zstd content size 检测；流式路径实测会提高 RSS，最终保留 bulk 路径。（已完成）

### 阶段 3：内存与批量路径

1. R09 存档写出流式化。（已完成第一阶段）
2. R10 共享搜索与 selector 扫描合并。
3. R12 固定槽位、合同临时集合、线性 merge。
4. R14 批量编辑索引。（已完成）
5. 最后再评估解析并行化、FTS 和更多生成列索引。

## 8. 基准与回归验收

### 必须保持的结果

- 46,273 名球员、485 条 markerless 记录、44,341 份合同等现有计数不变。
- Python oracle 对比仍为 0 差异。
- 所有现有 `--ignored` 真实存档测试通过。
- 编辑输出重新解析后字段一致，源存档不被修改。
- CSV/JSON 导出字段、排序和内容一致。
- 缓存命中/失效、未知费用、隐藏属性、同分角色等边界语义不变。

### 性能验收建议

不要直接照搬历史目标。先测当前基线，再为每项优化设定相对目标：

- 解析总时间不得回退；核心热点应有可测改善。
- Value Finder 翻页在缓存命中后不应再全量读取和评分。
- Stats 连续修改分钟阈值不应无限增长内存和数据库行数。
- 重查询和导出期间 UI 主线程不应出现长时间阻塞。
- 大存档编辑峰值 RSS 应显著低于“源文件 + 输出文件 + 解压 game_db”同时驻留。
- 查询优化必须附 `EXPLAIN QUERY PLAN` 和冷/热缓存数据。

## 9. 不建议的优化

- **不要删除编辑后的完整重解析验证**来换取速度；这是防止写坏存档的安全边界。
- **不要为了缓存而改变百分位语义**；百分位必须在请求筛选前计算。
- **不要使用 `target-cpu=native`** 作为默认发布配置；项目发布 Universal macOS 版本。
- **不要仅为本次问题改成 Fat LTO**。现有 Thin LTO + 单 codegen unit 已合理，除非基准证明有稳定收益。
- **不要把所有小型 `Vec` 换成 `HashSet`**。例如俱乐部内 team id 数量很小，线性 `Vec` 可能更快。
- **不要用 `VACUUM` 代替索引治理**；删除索引后的空间回收与查询性能是两件事。

## 10. 审查限制

- 审查阶段未运行基准、未使用 profiler。
- 未读取真实存档，因此所有绝对耗时和内存数字均来自项目历史文档。
- SQLite 索引结论基于内存数据库的查询计划验证，不是实际大库的耗时对比。
- 行号和调用路径可能随未提交修改变化，实施前应重新定位。

## 11. 实施进展（2026-11-04）

已完成：

- **R01 角色评分**：`best_role` 改为轻量折叠，只为获胜角色构造一次 `RoleScore`；角色权重和位置预编译为下标。新增编译路径与旧字符串路径、同分选择的等价测试。
- **R02 Value Finder 缓存**：拆分 `score_value_baseline` 与 `apply_value_request`，按 `save_id` 缓存评分基线；翻页/筛选/排序不再重新读取和评分全部球员。插入、删除存档时清空缓存。
- **R03 Stats Moneyball**：位置组只计算一次；global/tier 均值改为在组内一次分桶，不再为每组重复全表扫描；全局 production 不再构造后丢弃 drivers；内存缓存限制为 16 个条目；分钟输入增加 350 ms 防抖；筛选选项请求不再依赖排序。`player_metrics` 按 `(save_id, model)` 只保留最近使用的 8 个分钟阈值；被淘汰的阈值下次请求时重算，结果与缓存命中一致。
- **R04 桌面阻塞与只读并发**：球员、俱乐部、Moneyball、分析、筛选选项等重查询改为 `spawn_blocking`；导出改为在 blocking task 中流式读取和写出。新增 4 个独立只读 SQLite 连接组成的读池，普通查询不再与导出/写入串行；导出本身使用专用只读连接，不再持有全局 Store 锁。新增 `Store::open_read_only` 及只读写入拒绝测试。
- **Tauri 命令采样**：前端 `timedInvoke` 记录每个 IPC 命令的耗时分位数；`__fm27Perf.commands()` 输出 p50/p95/max，`__fm27Perf.mainThread()` 输出事件循环延迟 p50/p95/max。采样器为每个命令保留最近 512 条样本，并保留最近 256 条主线程延迟样本，避免长期运行无限增长。
- **R05 zstd 内存**：读取 frame content size；已知大小按实际大小分配。真实存档基准发现流式解压未知大小帧会因 decoder window 增加约 30 MiB RSS，因此保留有 cap 的 bulk 解压路径。新增未知 content size 和 cap 拒绝测试。
- **R06 流式导出**：新增 `Store::stream_players` 和 `ExportWriter`；CSV/JSON 逐行从 SQLite 读取并写出，不再重复 `COUNT(*)`、深 `OFFSET` 或整批物化；`query_all` 复用同一流式路径。新增 JSON/CSV 流式导出测试。
- **R09 存档写出**：输出改为 `BufWriter<NamedTempFile>` 分段写入，不再分配完整 `output_bytes`；源存档改为单个只读文件句柄分块读取，不再把完整源文件读入内存。开始和结束都会校验文件大小、mtime 和 header/trailer 摘要；写入期间源文件原地变化或原子替换会失败关闭。真实存档编辑输出与基线逐字节一致。
- **R14 批量编辑与设施扫描**：`StadiumIndex` 增加 UID 位置索引；俱乐部设施行按需一次性建立 `uid -> offsets` 映射；俱乐部编辑验证预建球场 map。解析路径的设施表扫描新增 16 位键前缀位图，在哈希完整 u32 前过滤掉绝大多数字节偏移；候选验证和最长有序序列语义保持不变。真实存档上仅此一项又降低约 11% 解析耗时，俱乐部输出与基线逐字节一致。
- **R07 属性路径**：解析循环直接计算存储下标；`Attributes`/`Positions` 改为流式 JSON Visitor，不再创建中间 `BTreeMap<String, _>`；默认隐藏模式跳过 personality 解析。
- **R08 俱乐部计数**：无聚合筛选时，计数查询不再构建完整球员/球场 CTE；有聚合筛选时保持原查询语义。
- **R10 字节搜索**：新增共享 `find_bytes`，统一使用 `memchr::memmem`。真实存档采样显示 `ContractDecoder::decode` 只占约 3% 样本，closing/contract selector 融合预计收益低于解析语义风险，因此不做；保留现有边界和验证逻辑。
- **R11 CLI 缓存**：`index` 先 `inspect_save` + `find_cached`，未命中才完整解析。
- **R12 分配（部分）**：赛季统计固定槽位改为栈数组；合同选择移除临时 `Vec`。
- **R13 SQLite 索引**：移除 `player_season_stats_player` 和 `club_finances_club`，并在 `migrate` 中显式删除旧库索引。

验证结果：

```text
cargo test --workspace
  fm27-core: 76 passed, 7 ignored（需要真实存档）
  CLI:       9 passed
  container: 2 passed
  desktop:   1 passed, 2 ignored（需要真实存档）

pnpm test
  5 files, 40 tests passed

pnpm build
  tsc + vite build passed
```

真实存档回归：

```text
AP King - Milan.fm
  ability_real_save        1 passed
  season_stats_real_save   1 passed
  editor_real_save         1 passed
  desktop ignored tests    2 passed

BB KING - Parma Calcio.fm
  club_real_save           2 passed
```

剩余限制：

- `player_metrics` 淘汰阈值后首次请求会重新评分；这是用有界存储换一次可预期的重算，不改变结果。
- 只读连接池只服务不依赖 Store 内存缓存的查询；Value Finder 和 Stats Moneyball 仍走可写 Store 以共享评分基线缓存。
- R10 的 selector 融合经真实存档采样评估后不实施，理由见上。
- 前端 p95 和主线程延迟采样是开发/诊断入口，不自动上报；需要从开发者工具调用 `__fm27Perf`。

真实存档性能对比见第 12 节。

## 12. 真实存档性能结果（2026-11-04）

### 环境与数据

- 机器：Apple M1 Max，32 GiB，macOS 27.2。
- 存档：`AP King - Milan.fm`，77,994,006 bytes。
- Build：`27.0.0+2493426`，游戏日期 2026-09-18。
- 解析结果：49,196 名球员、26,127 家俱乐部、139,511 行赛季统计、49,177 份合同。
- 解压总量：243,398,595 bytes。
- 基线：改动前提交 `fa3708d`。
- Release 配置：Thin LTO、单 codegen unit。

测试方法：

- `validate` 和 `index --force` 各先预热 1 次，再交替运行基线/当前各 4 次，取中位数。
- 每次 `index` 使用全新的 SQLite 文件。
- 使用 `/usr/bin/time -l` 记录峰值 RSS。
- 查询基准使用内存 SQLite，预热后交替运行 3 次取中位数。
- 查询基准使用独立临时 crate 通过 path 依赖分别链接两个版本；其依赖解析与工作区 lockfile 不完全相同，因此只比较相对变化，不比较绝对跨项目数字。

### CLI：解析与索引

| 场景 | 基线 | 当前 | 变化 |
|---|---:|---:|---:|
| `validate` 中位耗时 | 2.227 s | 1.448 s | **-35.01%** |
| `validate` 峰值 RSS | 410.42 MiB | 402.36 MiB | **-1.96%** |
| `index --force` 中位耗时 | 5.691 s | 4.788 s | **-15.87%** |
| `index --force` 峰值 RSS | 419.98 MiB | 415.98 MiB | **-0.95%** |
| SQLite 文件大小 | 416.59 MiB | 414.48 MiB | **-2.11 MiB（约 -0.51%）** |

### 查询路径

| 场景 | 基线 | 当前 | 变化 |
|---|---:|---:|---:|
| 解析 | 2176.6 ms | 1429.1 ms | **-34.34%** |
| 解析 + 内存 SQLite 写入 | 1690.8 ms | 1641.9 ms | **-2.89%** |
| 默认球员页 | 45.90 ms | 46.69 ms | +1.74% |
| Value Finder 首次请求 | 136.96 ms | 117.05 ms | **-14.54%** |
| Value Finder 第二页（缓存命中） | 137.52 ms | 5.87 ms | **-95.73%** |
| Stats Moneyball 首次请求 | 74.31 ms | 65.05 ms | **-12.46%** |
| Stats Moneyball 第二页（缓存命中） | 0.527 ms | 0.534 ms | +1.33% |
| `query_all`（49,196 行） | 60,193.6 ms | 1,200.7 ms | **-98.01%** |

`query_all` 的绝对值主要来自基线每 500 行重复执行 `COUNT(*)` 和递增 `OFFSET`；当前改为单次流式查询。该结果验证了 R06 的核心路径，但不等于桌面端 CSV/JSON 磁盘写入总耗时也下降 98%。默认球员页和 Stats 缓存页的 1% 左右变化在噪声范围内。

### 存档编辑

同一存档、同一 UID（228），修改 CA/PA，每次写入新副本：

| 场景 | 基线 | 当前 | 变化 |
|---|---:|---:|---:|
| 编辑耗时 | 3.223 s | 2.398 s | **-25.6%** |
| 编辑峰值 RSS | 790.12 MiB | 632.69 MiB | **-20.0%** |
| 输出文件大小 | 76,825,738 B | 76,825,738 B | 0 |
| 输出 SHA-256 | `a5579432…8b31eb` | `a5579432…8b31eb` | 完全一致 |

编辑基准同样预热后交替运行 3 次取中位数。运行结束后源存档 SHA-256 未变化；`validate` 计数和 `clubs` 输出与基线逐字节一致。

### 结论与限制

- 解析、索引、Value Finder 翻页、全量读取和存档编辑均有可测改善。
- 当前版本解析与索引峰值 RSS 低于基线，编辑峰值 RSS 降低约 157 MiB，没有出现优化换内存的情况。
- 解析耗时从早期实现的 1.606 s 进一步降到 1.448 s，主要来自设施表扫描的键前缀过滤；该优化不改变候选验证，俱乐部输出与基线逐字节一致。
- 只读连接池没有单独做吞吐基准；它解决的是导出/重查询期间全局锁串行的问题，正确性由只读连接测试覆盖。
- zstd 未知大小帧的实验性流式解码实测增加约 30 MiB RSS，已撤销；最终保留 bulk 解压路径。
- 这些数字来自一个 78 MB 存档、一台 M1 Max，不能直接外推到所有存档和机器。
- 工作区中 `real_save.rs` 的固定计数与 Modrić 字段断言绑定参考存档 `Casciavit`；当前只有 `AP King - Milan`，因此该测试未强行运行。实际存档计数通过 `validate` 和通用真实存档测试验证。
