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
