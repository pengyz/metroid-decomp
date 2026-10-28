# R9 对抗复核 verdict——commit 6866842d（UpdateAttrib hi 撤正：$63|(ptr_hi&4) 原形恢复）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-28
- 对象：`6866842d` fix(core),docs: 证伪更正·二——UpdateAttrib hi 公式撤正
- **verdict：pass**
- **教训登记（对我此前 2563b02b pass 判定的更正）**：该 pass 系同一截读误判产物，
  已在 r9-task-update-attrib-quirk-review.md 原地标注作废（本日追注）。

## 1. asm 全例程通读（:9723 起）—— 写尾 OR 确认

UpdateAttrib 全例程（prg7_engine.asm :9723-:9824，本次逐行通读）关键段：

- :9768-9770 `lda #$63 / sta Temp02_AttribPtr+1` —— 初值 $63；
- :9798-9801 **写尾 OR**：`lda Temp00_RoomRAMPtr+1 / and #$04 / ora
  Temp02_AttribPtr+1 / sta Temp02_AttribPtr+1` —— 终 hi = **$63|(ptr_hi&4)**；
- :9802-9810 `lda AttribMaskTable,x / ldy #$00 / and ($02),y / sta ($02),y`
  （AttribMaskTable :9826，$EFF4）——位清除后写回。

**ROM 字节直证（补写尾段）**：bank7 基（16+7*0x4000）下

- $EFC2 = `A9 63`、$EFC4 = `85 03`（初值段，前证保留）；
- **$EFD5 = `A5 01 29 04 05 03 85 03`**（lda $01 / and #$04 / ora $03 / sta $03
  ——Temp00+1&4 | Temp02+1 → Temp02+1，截读误判漏证段，本次补齐）；
- $EFDD = `BD F4 EF A0 00 31 02 91 02`（掩码清除写回段）。

反汇编↔ROM 逐字节全段一致。emu 执行即这些字节 ⇒ B 半区（$64xx）属性终写 $67xx，
非 quirk、非 bug——**原 rust 形即正**。

## 2. rust 现形对照 + TDD 测 mutation 有效性 —— PASS

- room.rs:603 `let hi = 0x63u16 | ((ptr >> 8) & 0x04);` —— 与全式逐条同形
  （Temp00_RoomRAMPtr+1 ↔ (ptr>>8)；and #$04 ↔ &0x04；ora/sta ↔ |）。
- 新测（room.rs:1095-1105）：`update_attrib(m, 0x6440, 2, 0)` 断言
  `read_wram(0x67C0) != 0` 且 `(b>>4)&3 == 2`。**mutation 有效性**：
  若回退 2563b02b 固定 $63 形，写落 $63C0，$67C0 读 0 ⇒ assert_ne 红。
  （位算复核：0x6440 bit6=1⇒x=2；palette=2 左移落 bit4-5 ⇒ (b>>4)&3=2 ✓。）

## 3. findings §4 原地证伪标注 —— 合规

- findings/scroll-content-mapping.md 新增 §4 卡：明记「前节『$63 固定 quirk』=
  **截读误判**…2563b02b『quirk 修』=误改回归（已撤正）」+写尾 OR 实录+A 级链全验
  +残差定性（emu 侧 PPU 时序/渲染色彩链，停待裁定）。
- room.rs 注释原地重写为【证伪更正·二】（被否主张原位替换，非静默删除）。
- 附（复核自查）：我方已否 pass 档 r9-task-update-attrib-quirk-review.md 已原地
  追加更正标注（§3.6 合规），verdict 作废并指向本档。

## 4. 必跑验证 —— PASS

- `cargo test --workspace`：**438 passed / 0 failed**（game-core 404+3+27+4；
  2563b02b 前后同数——+1 测撤 1 探针净零）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0 SKIP=1，CHECK: PASS**。
- L4 复跑 run-all：**match 23 / diff 16 / blocked 0（23/39 持平）**；
  f1000=**11267**、f1150=**11256** 未动（attrib 写址变化对该两点零像素影响——
  start 房同调早退免疫路径，与 2563b02b 前后两次实测一致）。

## 裁定

| 点 | 结论 |
|---|---|
| 1 全例程通读+ROM 写尾直证 | pass |
| 2 rust 同形+测 mutation 有效 | pass |
| 3 findings 原地标注合规 | pass |
| 4 验证（438/0；gates 13/0；L4 23/39 持平） | pass |

**verdict = pass**（2563b02b 误改完全回退，原形恢复正确；撤正链留档合规）。

Evidence: prg7_engine.asm :9723-:9824（全例程）/ :9798-9801（写尾 OR）/ :9826
（AttribMaskTable）；ROM hexdump $EFC2/$EFD5/$EFDD（bank7 基 16+7*0x4000）；
room.rs :603/:1095-1105；scroll-content-mapping.md §4；cargo test --workspace（438/0）；
gates/cli.py check（PASS=13 FAIL=0）；shots_rust/matrix.md（2026-10-28 第三次 run-all）。
