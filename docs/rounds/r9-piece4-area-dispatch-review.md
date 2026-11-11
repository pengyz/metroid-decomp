# r9 · piece④ per-area 图形表分发 — 独立对抗复核档

> verifier（独立复核员，非实施方）· 2026-11-11 · 只读复核，未修改任何 tracked 文件，未 commit。

## 1. 复核对象

- **commit**：`cde9ba9c`（复核时 HEAD）「feat(core): piece④ per-area 图形表分发——五区 AreaPointers + 房流/结构/元tile 按 InArea 选 bank」
- 改动面：8 文件 684+/24-（mainloop.rs / room.rs / area/mod.rs / norfair_graphics.rs 新 / area_dispatch_anchor.rs 新 / baselines 三件套）
- 任务卡（m00001）给定的验收基准：L4 `tools/l4_diff_run.py` 14 用例 110 格 ratchet。

## 2. 证伪要点逐条结论

### A. asm/ROM 逐行对读 — ✓ 成立

- `refs/m1disasm/SRC/prg7_engine.asm` `CopyAreaPointers: ; 07:C8B0`（:1936-1941）：`ldx #$0D / @loop: lda AreaPointers_ROM,x / sta AreaPointers_RAM,x / dex / bpl @loop / rts`，14B 拷 $3B-$48。**AreaPointers_ROM 在各 prg1-5 各有一份（bank 内 $959A），读的是当前映射 bank**——bank 由 InArea 经 `SwitchBank: ; 07:CA23`（:2258）`and #$0F/tay/lda BankTable,y` 换。**故本 commit 的「按 InArea 五区分发」方向正确**（asm 非固定 bank）。调用点两处：prg7:1891（MoreInit 环）、prg7:5037（电梯换区，前置 `ora #$10/jsr IsEngineRunning` 换 bank）。
- `SetupRoom: ; 07:EA2B`（:8578-8600）：`lda RoomNumber / cmp #$FF…` → `asl/tay / lda (AreaPointers_RAM.RoomPtrTable),y`（$3B/$3C 间接）→ rust 直查烘焙表=**折叠等价**（表值与 ROM 逐字节同，见下）。`DrawObject` StructPtrTable 经 $3D/$3E（:8640-8700 `lda (AreaPointers_RAM.StructPtrTable),y` 后 `jsr DrawStruct($EF8C)`）✓。`DrawMetatile: ; 07:EF2B` `lda (AreaPointers_RAM.MetatileDefs),y`（$3F/$40）✓。任务卡所称三处间接锚全部属实。
- **ROM 独立 python 直算**（file off = 16 + bank*0x4000 + (0x959A-0x8000)，bank=1..5，自算未抄 commit）：
  - bank1 `14 a3 72 a3 …`（Brinstar）
  - bank2 `1b a2 77 a2 …`（Norfair）
  - bank3 `d1 a7 fb a7 …`（Tourian）
  - bank4 `d5 a1 1f a2 …`（Kraid）
  - bank5 `7f a1 d3 a1 …`（Ridley）
  与 `AREA_POINTERS_BY_AREA`（mainloop.rs:3027-3033）五行逐字节一致；**行序 [0..4]=[Brinstar,Norfair,Kraid,Tourian,Ridley]=[bank1,2,4,3,5]，Kraid 行 d5 a1（bank4）、Tourian 行 d1 a7（bank3）交错正确**——任务卡最担心的区序坑逐行核过无误。`BankTable: ; 07:CA30` = `.byte $01+1,$02+1,$04+1,$03+1,$05+1` 与行序自洽。
- **烘焙件全量比对（python 独立提取 vs ROM）**：graphics.rs ROOM_PTRS 47 项 $A314 ✓ / STRUCT_PTRS 50 项 $A372 ✓ / METATILE_DEFS 260B $AEF0 ✓；norfair_graphics.rs 46 项 $A21B ✓ / 49 项 $A277 ✓ / 248B $AEEC ✓；**房流+结构流逐件比对（ROM 读到首个 0xFF）：Brinstar 0/47、0/50 错；Norfair 0/46、0/49 错——全部逐字节相等**。锚值抽验：Brinstar 首房 0xA441、Norfair 首房 0xA3AB、Norfair MetatileDefs 首组 F1 F1 F1 F1 与 ROM 一致。

### B. 防弱化审计 — ✓ 无弱化

- `git show cde9ba9c` 全 diff 8 文件：**gates/ 零改动**；**`data/status/l4_ratchet_baseline.json` 不在 commit 文件清单**（`git log -- 该文件` 最后触碰=87e48ca2，非本 commit）；基线只动 rust_test_count 三件套（SHA256SUMS/baseline.json/baseline_history.jsonl，531→536 + history 追加 reason）。
- 无删格、无调阈值、无把 diff 格锚成 match：14 用例 110 格分母不变，白名单条目 0（gates l4 行原文）。
- **未烘焙区 fail-loud 真实存在**：copy_area_pointers `.get(area).unwrap_or_else(unported!("bank07:959A-area"))`；setup_room `unported!("bank07:959A-RoomPtrTable")` + oob `unported!("bank07:A314-RoomPtrTable-oob")`；room.rs draw_room/draw_struct/metatile 三处 None→`unported!("bank07:959A-room_streams-area"/"structures-area"/"metatile-area")`。**无静默回退 Brinstar**（D7 合规）；锚测 `copy_area_pointers_unbaked_area_fails_loud`（InArea=0x05，should_panic）实跑通过。

### C. 独立复跑 — ✓ 全部通过（实测原文）

1. `cargo test --workspace`（/tmp/r9_cargo_test.log，exit 0）：
   - **runtime：`passed=532 failed=0`**（26 行 test result 聚合）
   - **static `#[test]` = 536**（parent 531 + 本 commit +5：anchor 4 + norfair_graphics mod tests 1）
   - 任务卡「应 536 passed」**口径不精**：536 是静态 #[test] 数（gate 口径），runtime 实跑 532——差 4 = `crates/game-core/src/render/sprite.rs` 4 测未接进模块树（render/mod.rs 只有 :521 内联 `mod sprite_tests {`，无 `mod sprite;`），**既有遗留（r9-F2 已记），非本 commit 引入**。gate_rust_tests（gates/cli.py:795-799）跑 cargo test 通过后用静态 current_metrics 对 floor，报「测试数 536（基线 536）」。
   - area_dispatch_anchor.rs 4 测 + norfair_graphics 测全 ok。
2. `python3 tools/l4_diff_run.py run case-password-justin`：**一致 9 · 失配 3 · 缺口阻塞 0 · 错误 0**；失配=700 mid-1 **71px**、1100 mid-4 **90px**、1190 submit **2063px**；**f1300/f1430/f2390 全部 0（match）** ✓ 与 commit 声明吻合。
3. `python3 tools/l4_diff_run.py run-all`：**用例 14 · 格 110 · match 107 · diff 3 · blocked 0 · pass 107/110**（另有 10 格 flaky 不计分母不锚值）→ `shots_rust/matrix.json`。
   - **逐格对照 l4_ratchet_baseline.json：match→非match 回归 = 0**；非match→match 改善 = 7（530/850/950/1150/1300/1430/2390）；baseline 格缺失=0。
   - 当前非 match 3 格全在 case-password-justin：700=71、1100=90、1190=2063（1190 像素较基线 769 升但 state 仍 diff，ratchet 锚 state）。
   - commit 声明 104→107/6→3 与实测 107/3 吻合（基线记录 100/10 是滞后下锚，非矛盾；2390 基线 14434 vs 任务卡 14157 同理——9ce4c852 自记 14434→14157，基线未回写）。
4. `python3 gates/cli.py check`：**`SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2` / `CHECK: PASS`**。关键行：`[PASS] rust-tests … 测试数 536（基线 536）`；`[PASS] l4 … 用例 14 · 分母 110 · match 107 diff 3 blocked 0 · 白名单条目 0`。TOLERATED=2（trace-diff l3-samus-move3 oracle 载体缺损 + visual-diff 采集面缺口）均既有登记；SKIP=proof-checks（delta 空）。`gates/cli.py status` 同：`SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2 / STATUS: OK`。机制核（cli.py:567-1012）：SKIP 显式前置、TOLERATED 需 --reason+--bug、SUMMARY 显式计数——无静默通过。
5. `git status --porcelain`：**空**（0 行，未留任何写入）；HEAD=cde9ba9c。

### D. 新问题扫描 — ✓ 无新问题（1 项遗留假设，见 §4）

- **area 语义**：rust 五处 `mem.read_ram(IN_AREA) & 0x0F`（mainloop.rs copy_area_pointers/setup_room、room.rs draw_room/draw_struct、mod.rs 五函数 `match area & 0x0F`）= asm `SwitchBank` `and #$0F`。高位 $10 区码（电梯 `ora #$10` 标记）被 `&0F` 剥掉，与 BankTable 取索引一致 ✓。IN_AREA 写点：is_engine_running SwitchBank、title_bridge.rs `pb8 & 0x3F` = asm prg0_title `and #$3F/sta InArea` 逐行 ✓。
- **area>4 fail-loud 可达性**：合法游戏内 area 只有 0-4。唯一能造 `InArea&0F>4` 的是伪造密码域（title_bridge PB[8]&$3F），但该路径**更早**就炸在既有 `c4de_unit` `idx=pending-1` → `_ => unported!("bank00:C51F")`（e3da2288 引入，非本 commit）——copy_area_pointers 的 area>4 unported! 与既有守卫同向，不构成新误触发面。L4 password 用例 **0 blocked** 佐证。
- **oob unported! 对合法房号**：Brinstar 表 47 项（0..46）、Norfair 46 项（0..45），WorldMap 值域 0..46。room46 全图仅 (20,2) 一格。**若该格属 Norfair 则 `ptrs.get(46)`=None→unported! panic，而 asm SetupRoom 无界读 StructPtrTable($A277) 头两字节当房指针（确定性垃圾但不炸）**。区界项目内无机械数据（WorldMap 511 非空格单一连通分量；spec/findings 无 area 分区表；电梯坐标系与格值不同源，均核过）——**未决**。缓解：①现有 14 用例无一走到该格（0 blocked/0 error）②fail-loud 本身符合 D7 unported! 纪律（缺口确定性爆优于静默错渲）③分区分编号下46 更可能属表长 47 的 Brinstar。**登记为遗留假设，不构成 needs_revision**（见 §4）。
- **测试锚值**：A441/A3AB/F1 组与 ROM 直算一致（§A）；锚测 AREA0..4_PTR 五值 = 五区 $959A 头两字节 ✓。
- 次要观察（不阻断）：mainloop.rs:3125 `ROOM_PTR_TABLE` 94B 常量成死代码（编译 warning）；room.rs:148 `let _ = game_data::area::struct_ptrs(area);` 为「读了不用」占位（asm 查 StructPtr 定位结构体、rust 结构流按 struct_idx 直取——折叠等价依赖烘焙 STRUCTURES 展开，§A 已证逐字节相等，语义不同形但行为等价）。

## 3. 实测原文汇总

```
cargo test --workspace        → passed=532 failed=0（static #[test]=536，parent 531）
l4 run case-password-justin   → 一致 9 · 失配 3 · 阻塞 0 · 错误 0（700=71px / 1100=90px / 1190=2063px；1300/1430/2390=0）
l4 run-all                    → 用例 14 · 格 110 · match 107 · diff 3 · blocked 0（逐格回归=0，改善 7）
gates/cli.py check            → SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2 / CHECK: PASS
gates/cli.py status           → SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2 / STATUS: OK
git status --porcelain        → 空；HEAD=cde9ba9c
```

## 4. 遗留假设（非本 commit 回归）

1. **room46@(20,2) 区归属未决**：若属 Norfair（表 46 项），合法走到该格时 rust unported! panic vs asm 读垃圾指针继续——区界无机械数据不可判；现无用例可达；fail-loud 符合 D7。建议后续补区界考证（door 连通或 AreaMapPos 反推）后裁一次。
2. **render/sprite.rs 4 测未接模块树**（static 536 vs runtime 532）——既有遗留（r9-F2 已记），非本 commit；gate 用静态计数属既有口径。
3. **基线滞后**：l4_ratchet_baseline.json 仍记 100 match/10 diff（87e48ca2 最后回写），实际已 107/3——ratchet 下锚机制允许 diff→match 不回写，非违规。
4. L4 现有用例均不进未烘焙区 2/3/4（sc-elevator-ride 探跑 InArea 恒 0），未烘焙区 fail-loud 只被锚测 should_panic 覆盖，无像素级验证。

## 5. Verdict

**pass** · Confidence **90/100**

- A ✓（asm/ROM 逐行 + 五区 14B 直算 + 烘焙件全量逐字节）、B ✓（gates/零改动、基线未动、fail-loud 真实）、C ✓（532/0、9/3、107/3 回归 0、PASS=13、status 干净）、D ✓（area 语义一致、area>4 有既有守卫、锚值合 ROM；1 项 oob 遗留假设登记不阻断）。
- 对任务卡两处口径的原地更正：①「应 536 passed」→ runtime 532 passed / static #[test] 536（差 4=既有未接线 sprite 测）；②2390px「14157」与基线「14434」均对——前者是 9ce4c852 后实测，后者是基线旧下锚，非实施方错误。
