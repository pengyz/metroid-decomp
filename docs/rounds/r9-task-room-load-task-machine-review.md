# r9 复核：多帧载房任务机 A-lite（`7bb78969`）

- 被复核件：`7bb78969` feat(core) 多帧载房任务机
- 复核员：独立对抗复核子代理 `ea9fa087`（只读、证伪立场、未跑 run-all/全量测试）
- **VERDICT: pass-with-findings**（1×P1 + 4×P2 + 1×P3）
- 复核后修复：`0703ca26`

## 逐条结论

| # | 检验项 | 结论 | 要点 |
|---|---|---|---|
| 1 | asm 语义（`SetupRoom ; 07:EA2B` / `AttribTableWrite ; 07:EA13` / `EndOfRoom ; 07:EAF4` / `RoomFinished`） | PASS（叙述 P3） | 实现正确：先 `and #$0F` 取旧值查表、再 `inc RoomNumber`；帧边界只可能看到 `F1/F3/FF` 的推演与三房实测自洽，但属**事后拟合**（`F2/F4` 同拍跨过不是独立预测） |
| 2 | 相位闸是否吞掉真实载房 | **FAIL（P1）** | 相位 2 期间 `scroll.rs:40` 仍无条件调 `setup_room` ⇒ 新请求被装载但 `work_cycles`/`dropped` 全丢；注释与实现不符 |
| 3 | 尾段推进鲁棒性 | **FAIL（P2）** | `attrib_table_write` 恒返回 true ⇒ `break` 分支是死代码；`debug_assert` 在 release 被编译掉，游标脱格会走成「看似合法的房号」⇒ 下一帧无端整房重载 |
| 4 | 窗长来源 | PASS-with-findings（P2） | `dropped` 可为 0（`room_draw.rs:1493` 自证 BASE/FRAME==0）而无守卫；尾段被主循环与门步**双重驱动** |
| 5 | L4 数字 | PASS（逐格亲验） | 恰 3 格变动（`case-morphball-probe` 2080/2110/2160 全部 `(diff,8)→(match,0)`），无第 4 格、无 flaky、baseline 与 matrix 零差异 |
| 6 | 快照/回放 | **FAIL（P2）** | `restore_mss` 用 `Self::default()` ⇒ 相位丢失；而本发把窗内 `$5A` 从 `$F0` 改成**房号**，恢复后会**重复整房装载 + 重复掉帧窗**（风险由本发新引入） |
| 7 | 第 4 个洞（`RoomPtr` 未分块） | PASS-with-findings（P2） | 终态与 emu 一致、160 格零回归；但窗中任意时刻的 `RoomPtr/RoomRAM` 进度仍不同 ⇒ 任何窗内采样 `$33/$34` 的 L3 trace 都会分歧，commit 措辞强于证据 |

## 复核员指出的盲区（已并入文档）

- 「帧边界只可能是 `F1/F3/FF`」是**相容性推演**，不是独立预言；若某房间的写点节奏不同，需要新证据。
- 复核未跑 `cargo test --workspace` / `gates/cli.py l4` / `run-all`（按硬约束），故 L4 之外的行为面由实施方负责举证。

## 修复对照（`0703ca26`）

| finding | 修法 |
|---|---|
| P1 请求判据 | `world::room_load::is_new_load_request`（房号 `< $F0` 且（相位 0 或 房号 ≠ 在装房号））；新请求抢占旧任务并照常记账 |
| P2 尾段脱格 | `unported!("bank07:EA13-tail-cursor")` fail-loud |
| P2 窗长 0 | `dropped.max(1)`（Mesen 三房最小冻结 1 帧） |
| P2 快照丢相位 | `restore_mss` 从 `$005A` 重建相位（窗长不可恢复，已声明） |
| P2 双驱动 | 相位 2 期间不走主循环 `setup_room`，尾段由 `room_load_gate_step` 独占 |
| P3 叙述反了 | 文档原地更正为「先 `and #$0F` 再 `inc`」 |

修复后 `cargo test --workspace` **771 passed / 0 failed**、`gates/cli.py check` **PASS=23**、
L4 **20 案/160 格 · match 151 · diff 9 · blocked 0** 与复核前**逐格 0 变动**（行为中性）。

## 第二轮复核（`922ff7e4`）：**reject**（新洞）→ 修复

第一轮 5 条 findings 逐条 PASS 关闭，但复核员用仓库预编译二进制实测出**本轮新引入的真洞**：

- **P1（新洞）**：`restore_mss` 按 `$005A` 重建相位的启发式把「上电/标题/密码屏 `$5A=$00`」
  （`analysis/bundles/title-recon.json`、`sc-password-entry.json`、`sc-password-entry3.json`、
  `sc-title-menu3.json` 四件真夹具都是这个值）误判成「正在装 0 号房」⇒ 开局**幻影尾段**
  （`$5A` 被推成 `00→F1→F3→FF`，与 oracle 恒 `00` 不符），并在 `room_ptr=$0000` 上让
  `attrib_table_write` 的 `read_wram` 下溢 **panic（f=2）**。
- **P2（覆盖盲区）**：20 案矩阵的 `snapshot` 全为 `none` ⇒ 快照恢复路径零门禁覆盖。
- **P3**：`dropped.max(1)` 的理由不可达（`LOAD_INIT+LOAD_SHELL=10562 > 780`），行为无害但注释失真。

修复（提交见下）：
- 相位重建加**正向鉴别**：`$5A < $F0` **且** `$33/$34 ∈ [$6000,$8000)` 才算窗内（上电 `$0000` ⇒ 空闲）。
- `attrib_table_write` 前置校验 `room_ptr` 域，越界 `unported!("bank07:EA13-attrib-ptr")`——D7 fail-loud，
  不再 panic/静默写脏属性表。
- 新增回归测 `power_on_restore_leaves_task_idle_and_gate_step_is_noop`（上电面恢复 ⇒ 相位空闲、
  门步不动 `$5A`）；`restore_mss_rebuilds_window_phase_from_room_number` 补 `$5A=$00`+`$33/$34=$00` 边界。
- 实测（仓库二进制）：`./target/debug/rust_screenshot --bundle analysis/bundles/title-recon` 8 帧
  `$5A` 恒 `0`、无 panic（修复前 f=2 崩）。
- 诚实说明：**没有**把 bundle 起跑做成 L4 像素用例——`restore_mss` 只回灌 RAM/WRAM/regs，
  bundle v1 不含 PPU 面，像素比较必然 61440/61440 全差（试过、已删）；该路径的守卫落在上面两条单测。

## 第三轮复核（`922ff7e4`）：**pass-with-findings**（3×P3，无 P1/P2）

- 三条 `$5A=$00` 现成 bundle（`title-recon` / `sc-password-entry3` / `sc-title-menu3`）实测 8 帧
  `$5A` 恒 `00`、无 panic；阴性对照 `mid-move`（`$5A=$FF`、ptr `$6000`）零回归。
- `attrib_table_write` 的 `room_ptr` 域校验不误炸：合法域由 `select_room_ram`（`room_draw.rs:801-803`）
  钉死为 `$6000/$6400`，且它在 `draw_room`/尾段之前执行；`$FE` 短路径在 attrib 之前就 `unported!`。
- 回归测真打穿（变异 M1「删 ptr 鉴别」⇒ 两条测试红；M2「删整块重建」⇒ 旧测红）：
  两条测试互补覆盖「误报/漏重建」两向。
- 49 个 bundle 实测分布：`$5A` = `{FF:44, 00:4, F1:1}`、`$39/$3A` = `{WRAM:45, 0000:4}`；
  `$5A<$F0` 的 4 件全部 `ptr=$0000`（上电件）⇒ 新判据与真实数据一致，`sc-room-scroll`（`$5A=$F1`、ptr `$6400`）未被误判。
- L4 逐格仍 20 案/160 格 match 151 · diff 9 · blocked 0，两侧零差异。

### 第三轮遗留 P3（已处置/登记）

| # | 项 | 处置 |
|---|---|---|
| P3-1 | `.max(1)` 注释理由曾是「小房间会算出 0」（不可达） | **已改**：注明按现有周期常量不可达（`INIT+SHELL=10562 > 780`），保留只为把「窗同拍关闭」挡在定义域外 |
| P3-2 | 49 个 bundle 里没有 `$5A<$F0 ∧ ptr∈WRAM` 的真实「载房中段快照」 | **登记为 gap**：窗内重建只由单测构造值钉住；真实样本缺失，不 silent 假定 |
| P3-3 | bundle 起跑仍在差分门禁之外（L4 全 `snapshot none`；bundle v1 无 PPU 面 ⇒ 像素比必全差） | **登记为 gap** + 采纳建议方向：改用**非像素判据**逐帧比 `$5A/$39/$3A` 与 `analysis/traces/title-recon.frames.jsonl`(f795-812) |
