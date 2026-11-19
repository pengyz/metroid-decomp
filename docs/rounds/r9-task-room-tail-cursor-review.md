# r9 复核：载房尾游标补格修复（`a5ff63d3` + `a07eece0`）

- 复核员：独立复核员（verifier 角色，非实施方），立场=证伪
- 对象：`crates/game-core/src/world/room_load.rs` 的 `room_load_gate_step` 相位 2 新增 4 行判据 + `mod tests_room_load_task` 的 `door_scroll_compensates_one_tail_step`
- 结论：**pass-with-findings**（代码改动真实增益且零回归；中心主张被改后房扫证伪，须收回）

## 逐项判定

| 项 | 判定 | 要点 |
|---|---|---|
| A asm 事实核对 | PASS | `:7702` CheckUpdateNameTable 首件 `jsr SetupRoom` 且推进在守卫前；`:8672` AttribTableWrite 一步/调用；`:8693` SetupRoom `>= $F0` 分支——三条独立读毕成立 |
| B 判据合理性 | FAIL | `$0056 != 0` 是代理不是真条件：行走跨屏（`ScrollX==0` 径）与满速行走在 `$0056==0` 时同样有剩余调用（漏补偿）；`$0056==5` DoorDelay 非 0 却不滚（误补偿） |
| C 零回归声明 | PASS | 用 tracked 基线逐采集点独立比对 20 案 160 格：match 151 / diff 9 / blocked 0，0 处不一致 |
| D 测试有效性 | PASS | 变异 `!=0 → ==0` 后该测 FAILED（`room_load.rs:476: left 243 / right 255`），已复原 |
| E 越界声明 | FAIL | subject 与正文「B 类缺列根因 / 35 房」超出证据；改后 12 房扫 3 ok / 9 mismatch 直接反证；「补一次调用」与实现（补一整帧边界步）不符；测试数 682 与 773 口径混写 |

## findings 处置

| # | sev | 处置 |
|---|---|---|
| 1 | P1 | **已收回主张**并落 `docs/findings/room-tail-cursor-fix.md` §4；改后 12 房扫数据入库 `data/status/room_sweep_b_class_postfix.json`；剩余 9 房登记为后续任务 |
| 2 | P2 | 登记为已知近似（同文档 §5），后续卡补「按剩余滚动调用数推尾」+ 行走跨屏 `$0056==0` 与 `$0056==5` 两个 L4 用例 |
| 3 | P3 | 注释与文档改为「补一整帧边界步（含两次 AttribTableWrite）」 |
| 4 | P3 | room0 逐点实测值写入文档 §3 表格（改前 f8=440 / f16..end=564） |
| 5 | P3 | 文档分别写 workspace 773 / game-core lib 682 |
| 6 | P3 | 文档 §5 登记「按帧边界 3 步 + `tail==2`」的模型近似边界，fail-loud 保留 |

## 复核员补充核出的事实（已入库文档 §1）

- 主循环 `jsr ScrollDoor` ×2/帧（`prg7_engine.asm:2027-2028`）
- `DoorEntryStatus == 5` 时 `ScrollDoor` 不滚（`:7007-7045`）
- 行走滚动每像素一次（`:7884 jsr ScrollLeft` / `:7928 jsr ScrollRight`）
