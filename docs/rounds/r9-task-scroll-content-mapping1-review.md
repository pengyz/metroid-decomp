# R9 对抗复核 verdict——commit 8cb29338（draw_room 0xFF 直终径径汇 EndOfRoom，$4A 双写面）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-28
- 对象：`8cb29338` fix(core),docs: 流送内容映射①——draw_room 0xFF 直终径径汇 EndOfRoom
- **verdict：needs_revision**（核修正确、证据成立；**仅测试有效性一点 FAIL**）

## 逐条复核

### 1. asm 核 + Rust 两径等价 —— PASS

- asm `refs/m1disasm/SRC/prg7_engine.asm`：
  - `EndOfRoom: ; 07:EAF4` 在 **:8727**（commit 写 :8725，偏 2 行，无实质影响），
    :8728-8732 实录：`ldx #$F0 / stx RoomNumber` + `lda ScrollDir / sta TempScrollDir`
    —— **双写面（$5A=$F0 + $4A=ScrollDir）确认**。
  - DrawRoom 0xFF 直终径：`DrawRoom: ; 07:EAAA`（:8666），`cmp #$FF / beq EndOfRoom`
    在 **:8670**（commit 写 :8672，偏 2 行，无实质影响）。
  - EnemyLoop 尾径：`cmp #$FF / beq EndOfRoom`（:8713）——双径同汇确认。
- Rust `crates/game-core/src/frame/room.rs`：
  - draw_room 0xFF 臂（:62-68）与 enemy_loop 0xFF 臂（:442-444）均调 `end_of_room(mem)`；
  - `end_of_room`（:480-495）写 `0x005A=0xF0` + `0x004A=SCROLL_DIR`——与 asm 双写面同形。
  - 已知已记账分歧：asm EndOfRoom 落入 EndOfRoomHorizontal/Vertical（kick）；Rust 侧
    kick 留滚动径（commit 前既定决策，函数注释在案）——非本次回归。
- **结论：修复方向与实现等价，A 级证据成立。**

### 2. L4 声明核 —— PASS（实测复现）

复跑 `python3 tools/l4_diff_run.py run-all`（fresh/fresh、blocked 0）：

- 矩阵：**match 23 / diff 16** —— 与声称一致。
- case-002：f1000 neighbor-room = **11267**、f1150 end = **11256**、f850 scrolling = 400
  （声称 14668→11267 / 14663→11256 的现值端复现；-23% 比较基线为修前值，本次未重采修前面）。
- case-001：f950 combo-action = **1134**（持平）、f1200 end = **16030** —— 与声称一致。
- 门链五点（case-002 f420/620/650/700/790）全 match；落地三点（case-001 f715/720/725/735
  —— 含 720）全 match（diff_pixels 0/61440）。

### 3. 测试合规 —— **FAIL（needs_revision 唯一项）**

- `tests_stream_content_mapping::draw_room_ff_path_writes_temp_scroll_dir`
  （room.rs:1063-1073）**未驱动 draw_room 的 0xFF 臂**——直接调
  `crate::frame::room::end_of_room(&mut m)`。该测试对修复**零回归保护**：
  即便把 0xFF 臂回退为旧内联 `write_ram(0x005A,0xF0)`，测试仍通过。
- porter 自注「静态表不可注径（直接构造不可行）」**不成立**（可证伪）：
  `crates/game-data/src/area/graphics.rs:46-48` `ROOM_STREAMS[1] =
  [0x02,0x07,0x02, 0x02,0x87,0x02, 0xFF]` —— palette + 2 对象 + **直终 0xFF** 的现成房流；
  `draw_room(&mut m, 1)` 即走被修路径（i=1 对象×2 → i=7 命中 0xFF 臂）。
- 处方：测试改为 `draw_room(&mut m, 1)`（前置 `SCROLL_DIR=3`、`PPUCTRL_ZP` 任意），
  断言 `$5A=$F0 && $4A==3`——一行改动即转强测。

### 4. 必跑验证 —— PASS

- `cargo test --workspace`：**437 passed / 0 failed**（game-core 403 + 3 + 27 + 4；
  = 修前基线 436 + 本 commit 新增 1 测，任务书"应 436"为修前口径，现树含新测故 437）。
  注：commit message 写"全套件 403 passed"实为 game-core 单 crate 数，口径误记（不影响裁定）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0 SKIP=1 TOLERATED=2**，CHECK: PASS。
  - 复核过程记录：首次跑 check 与 run-all 并发，gate 读到半写 matrix 报 l4
    "新增 blocked"——run 完成后干净复跑即 PASS=13 FAIL=0；系复核操作时序 artifact，非产品分歧。
  - 遗留 WARN（非本 commit 引入）：`ratchet:rust_test_count 基线 440/现值 441 PENDING`、
    `naming_ledger 631/738 PENDING`——需 baseline update 另行清账。

## 裁定

| 点 | 结论 |
|---|---|
| 1 asm 核/两径等价 | pass |
| 2 L4 声明 | pass（现值端全复现） |
| 3 测试有效性 | **fail**（弱测 + 不可行论证被证伪） |
| 4 必跑验证 | pass（437/0；gates PASS=13 FAIL=0） |

**verdict = needs_revision**：修复本体与证据无需返工；仅需将 room.rs:1063 测试改为真驱动
`draw_room(m,1)` 直终径（或补一条等价强测）后复审即 pass。

Evidence: prg7_engine.asm :8666/:8670/:8713/:8727-8732；room.rs :62-68/:442-444/:480-495/:1063-1073；
graphics.rs :46-48（ROOM_STREAMS[1]）；shots_rust/matrix.md（2026-10-28 run-all 实测）；
gates/cli.py check 复跑输出（PASS=13 FAIL=0）。
