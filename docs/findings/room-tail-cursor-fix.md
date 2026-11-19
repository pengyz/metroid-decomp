# 载房尾游标补格修复：实测边界与主张更正

日期：2026-11-19
对象提交：`a5ff63d3`（代码 + 1 测试）、`a07eece0`（测试数基线 772→773）
复核：`docs/rounds/r9-task-room-tail-cursor-review.md`（独立复核员，verdict = pass-with-findings）

## 1. asm 事实（A 级，复核员独立读毕三条全成立）

- `CheckUpdateNameTable ; 07:E54A`（`refs/m1disasm/SRC/prg7_engine.asm:7702`）首件 `jsr SetupRoom`，随后 `ldx RoomNumber / inx / bne RTS_X173` ⇒ **只有 `RoomNumber == $FF` 才继续到阈值检查与列 kick**，推进在守卫之前。
- `AttribTableWrite ; 07:EA13`（`:8672`）：`lda RoomNumber / and #$0F / inc RoomNumber / jsr JumpEngine`，表 `.word ExitSub, WritePPUAttribTbl, ExitSub, WritePPUAttribTbl, RoomFinished` ⇒ **一步推进/调用**；`RoomFinished ; 07:EA26` 置 `$FF`。
- `SetupRoom ; 07:EA2B`（`:8693`）：`cmp #$FF / cmp #$FE / cmp #$F0 / bcs AttribTableWrite` ⇒ `>= $F0` 走尾游标分支；`EndOfRoom ; 07:EAF4`（`:8876`）把 `RoomNumber` 置 `$F0`。

补充（复核员核出、影响判据建模）：主循环 `jsr ScrollDoor` **×2/帧**（`:2027-2028`）；`DoorEntryStatus == 5` 时 `ScrollDoor` 不滚（`:7007-7045`）；行走滚动调用点 `Lx180 ; 07:E65E`（`:7884 jsr ScrollLeft`）/ `Lx184 ; 07:E692`（`:7928 jsr ScrollRight`）是**每像素一次**。

## 2. 修复内容（`crates/game-core/src/world/room_load.rs`）

`room_load_gate_step` 相位 2 第一拍，若 `crate::ram_map::DOOR_ENTRY_STATUS`（`$0056`）不为 0，则额外调一次 `advance_room_load_tail`。等价于「载房完成帧还有一次滚动调用没被 Rust 的整拍冻结窗建模」。

## 3. 实测边界（原始数据）

room 0（门入场，`$0056 = 01`，`ScrollX` 每帧 +2）：

| 采集点 | pre | f1 | f3 | f5 | f8 | f16 | f30 | f60 | end |
|---|---|---|---|---|---|---|---|---|---|
| 改前 px | 0 | 0 | 0 | 0 | 440 | 564 | 564 | 564 | 564 |
| 改后 px | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

改前产物 `shots_rust/_tmp_sweep_315944-1795093879_a0_r00.l4report.json`，改后 `shots_rust/_tmp_sweep_453361-1795095361_a0_r00.l4report.json`。窗末（相位 1）补格的中间变体有 f3 = 244px 瞬态实录（`_tmp_sweep_448937-…`），故补偿位置必须在相位 2 第一拍。

`case-morphball-probe`（`$0056 == 0`，Samus 站住、`ScrollX` 恒 1）：改前 21/21、改后 21/21。
`run-all --skip-emu`：160 格 **match 151 / diff 9 / blocked 0**，与改动前逐格一致（复核员用 tracked 的 `data/status/l4_ratchet_baseline.json` 独立比对，0 处不一致）。
测试：`cargo test --workspace` **773 passed**（game-core lib 682）；`gates/cli.py check` PASS=23。

## 4. 主张更正（P1，复核员证伪）

`a5ff63d3` 的 subject 与正文把本修复称作「B 类缺列根因」并写出「Brinstar 47 房扫里 35 房」。**该主张不成立，现予收回。**

改后扫 12 个 B 类房（`3,4,5,6,7,9,10,11,12,13,14,15`，`--captures 60,960`，结果 `/tmp/sweep-b-class.json`）：**ok = 3（room 04/06/09），mismatch = 9**（room 03 = 512px、05 = 801、07 = 148、10 = 184、11 = 256、12 = 3166、13 = 5189、14 = 291、15 = 1302）。

正确表述：本修复消除的是「门入场滚动路径下尾游标晚一格 ⇒ `ScrollX = 8` 的 `idx = 1` kick 窗被守卫挡掉 ⇒ 名表第 1 列永不写入」这一**具体机制**。它解释了 room 0 与抽样中 3 房的全部分歧；其余 9 房的分歧需要另行定性。

room 11 的残余签名已抽样核对（`_tmp_sweep_493308-1795098930_a0_r11_f01860_f960.ppu.hex` vs `shots_rust/` 同名）：chr 0、palette 0、**nt1 = 573 格 / nt2 = 597 格 / nt3 = 36 格**，且差异横跨全部行与列（row 0-31 全有）⇒ **不是缺列签名**，更接近整屏内容/镜头位置差异，属另一类。

## 5. 已知近似（P2/P3，登记不修）

- 判据 `$0056 != 0` 是「完成帧还有剩余滚动调用」的**代理**，不是 asm 真条件。两类偏差：①漏补偿——行走跨屏（`ScrollX == 0` 特殊径 `inc MapPosX + GetRoomNum`）与满速行走 2 次调用/帧在 `$0056 == 0` 时同样存在剩余调用；②误补偿——`$0056 == 5`（DoorDelay）非 0 却不滚。
- 实现补的是**一整帧边界步**（`F3 → F4 → FF`，含两次 `AttribTableWrite`），注释已按「补一整帧边界步」表述，不再说「补一次调用」。
- 尾游标模型仍是「按帧边界 3 步 + `tail == 2` 判据」的近似（`room_load.rs` 的 `advance_room_load_tail` 与 `unported!("bank07:EA13-tail-cursor")` fail-loud 保留）。真正的按调用计数模型属后续卡。

## 6. 后续任务（已登记）

1. 剩余 9 房定性：先按「整屏 nt 差异」与「缺列签名」分流，再逐类定位（room 12/13 量级 3166/5189px 优先级最高）。
2. 行走跨屏 `$0056 == 0` 的 L4 用例（补 P2 的漏补偿面）；`$0056 == 5` 相位用例（补误补偿面）。
3. 47 房全量重扫（改后）并公开逐房结果，替换旧台账里的 pre-fix 结论。
