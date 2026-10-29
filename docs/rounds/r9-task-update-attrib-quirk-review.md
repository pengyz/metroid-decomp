# R9 对抗复核 verdict——commit 2563b02b（UpdateAttrib $EF9E 高字节 D4 quirk 保真回修）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-28
- 对象：`2563b02b` fix(core),docs: UpdateAttrib 高字节 quirk 保真（$63 固定（D4））
- **verdict：pass**

> 【2026-10-28 更正（6866842d 撤正后复核追加）】本档 verdict **作废**——「$63 固定」
> 系我方截读误判：asm UpdateAttrib 写尾（:9798-9801）`lda Temp00_RoomRAMPtr+1 /
> and #$04 / ora Temp02_AttribPtr+1 / sta Temp02_AttribPtr+1` ⇒ 终
> hi=**$63|(ptr_hi&4)**（B 半 $67），2563b02b 与本 pass 判定均错，原 rust 形即正。
> ROM 字节直证补全：bank7 基下 $EFD5-EFDC = `A5 01 29 04 05 03 85 03`
> （lda $01/and #$04/ora $03/sta $03）。教训：asm 大例程必须**全例程逐行读完**
> 再下「固定/无位运算」结论（$EF9E 跨 :9723-:9824 多段，本次截读于 :9790 前）。
> 撤正复核见 r9-task-update-attrib-quirk-revert-review.md。

## 逐条复核

### 1. asm 核 + emu 同形 —— PASS

- asm `prg7_engine.asm`：UpdateAttrib `; 07:EF9E`（**:9723**），高字节段
  （:9768-9770 实录）：`lda #$63 / sta Temp02_AttribPtr+1` —— **固定 $63，无任何
  ptr 高字节位运算**（Temp00_RoomRAMPtr+1 未被再读入）。
- **ROM 字节 A 级直证**（复现命令：python 读 `rom/Metroid (USA).nes`）：
  bank7 基 `16+7*0x4000`，`$EFC2 = A9 63`（lda #$63）、`$EFC4 = 85 03`
  （sta Temp02_AttribPtr+1）——反汇编与 ROM 逐字节一致。
- emu（Mesen2）一致性：emu 执行的即上述 ROM 字节，B 区房属性同样落 $63xx——
  quirk 面 emu≡ROM≡修后 Rust（构造性成立；本面未另行独立 trace）。
- 修后 Rust（room.rs:603）：`let hi = 0x63u16;` —— 同形。

### 2. 旧形起源考古 —— 无 A 级证据，回退正当

- `git log -S '$63u16 | ((ptr >> 8) & 0x04)'`：唯一引入点 **22bae778**
  （2026-10-25「DrawRoom 引擎端口」），行内注释自称「asm 算式：…高字节
  $63|(ptr_hi&4)」——但 asm 原文（:9723 面/ROM $EFC2）**无此 OR**。
- 该 commit Evidence 字段仅泛引「prg7_engine.asm 全读」，对 OR 项无独立指针
  ⇒ **C 级伪语义「顺手修 bug」**（把 ROM quirk 当 bug 修，违 D4）。
- 2563b02b 回退为 asm 保真 + TDD 测留档——符合 §3.6 证伪更正文化。正当。

### 3. L4 影响核 —— PASS（复跑实测）

复跑 `python3 tools/l4_diff_run.py run-all`（fresh/fresh、blocked 0）：

- 矩阵 **match 23 / diff 16**（23/39，与修前持平——无回退）。
- f1000 neighbor-room = **11267**、f1150 end = **11256** —— 未动 ✓
  （attrib quirk 修对该族零像素影响，「非该族根因」论证与实测一致）。
- 门链五点（case-002 f420/620/650/700/790）全 match；落地三点
  （case-001 f715/725/735）全 match。

### 4. 必跑验证 —— PASS

- `cargo test --workspace`：**404+3+27+4 = 438 passed / 0 failed**
  （= 修前 437 + 本 commit 新增 1 测 tests_update_attrib_quirk）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0 SKIP=1 TOLERATED=2，CHECK: PASS**。

### 附：新测有效性核（证伪立场追加项）

`attrib_hi_fixed_63_for_b_half`（room.rs:1081-1106）：`update_attrib(m, 0x6440, 2, 0)`
（B 半区、异调⇒写径），断言 `$67C0==0`（B 区不被写）+ `$63C0` quadrant 位=2。
**mutation 有效性**：若回退旧形 `0x63|(0x6440>>8)&4 = 0x67`，写落 $67C0，
`$63C0` 读 0 ⇒ 测红。强测成立。

## 裁定

| 点 | 结论 |
|---|---|
| 1 asm 核（:9723 lda #$63 固定 + ROM $EFC2 A9 63） | pass |
| 2 旧形考古（22bae778，无 A 级据，伪语义回退正当） | pass |
| 3 L4（23/39 持平；f1000/f1150 未动；门链/落地保持） | pass |
| 4 验证（438/0；gates PASS=13 FAIL=0） | pass |

**verdict = pass**。

Evidence: prg7_engine.asm :9723/:9768-9770；ROM hexdump（$EFC2 A9 63/$EFC4 85 03，
bank7 基 16+7*0x4000）；git log -S（22bae778 唯一引入）；room.rs :603/:1081-1106；
shots_rust/matrix.md（2026-10-28 复跑）；cargo test --workspace（438/0）；
gates/cli.py check（PASS=13 FAIL=0）。
