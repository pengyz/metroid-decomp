# r9 对抗复核：4511268f「work_cycles 六处累加改 6502 逐条周期模型 + 显式债表」

- 复核日：2026-11-13
- 复核员：r9 verifier（证伪立场，非实施方）
- 对象：`4511268f06bf1ed9330271449fd1559a21802a6f`
- 方式：只读。未跑 cargo / gates / emu。周期表与 asm 逐条对照 + python 独立重算全部模型值与 `ship()`。
- 时限约束：任务书要求 8 分钟内交付，本次按「四面各取最锋利证伪点」执行，未做穷举。

## 1. verdict

**needs_revision**（行为安全，审计面不成立）。

一句话：**`model_*` 的数值都被精确复现了，但「逐条」这个宣称在 3 处站不住 —— 其中 2 处是模型真的少数了周期（MT_FULL 少 8c、ENEMY 族少 ≥13c），1 处是逐条注释自相矛盾（UA 全径分项注释合计 133 ≠ 代码 128）。同时债表 `legacy − model` 是残差而非独立测量，因此它把上述模型错误全部吸收掉，还给出了错误的因果归因。**

无 must-fix 的**行为**问题：8/8 `ship()` 值独立验算等于旧标定（见 §3 C）。必须修的是注释/归因/文档一致性（见 §5）。

## 2. A 面：asm 忠实度

### A0 先说结论清单（逐条重算，全部与测试钉值一致）

用 python 独立复算（不跑 cargo），`cycles.rs:88-238` 全部模型值 = 测试钉值：

| 模型 | 代码值 | 复算 | 测试钉值 |
|---|---|---|---|
| model_fill_room_ram | 8503 | 8503 | 8503 |
| model_attrib_fill | 703 | 703 | 703 |
| model_init_tables_shell | 56 | 56 | 56 |
| model_init_tables | 9262 | 9262 | 9262 |
| model_draw_metatile_fast | 214 | 214 | 214 |
| model_update_attrib_full_body | 128 | 128 | 128 |
| model_draw_metatile_full | 333 | 333 | 333 |
| model_draw_object_shell | 114 | 114 | 114 |
| model_draw_object_y_unit | 17 | 17 | 17 |
| model_draw_struct_row | 72 | 72 | 72 |
| model_load_enemy | 274 | 274 | 274 |

即：**算术层面无抄写错误**，`cycles.rs:310-322` 的钉值测试不是空断言。

### A1 FillRoomRAM（`cycles.rs:88-93`）——**通过**，任务书三个疑点全部证实为真

asm `refs/m1disasm/SRC/prg7_engine.asm:9881-9885`：

```
@loop_outer: ; 07:F029
    @loop_inner: ; 07:F029
        sta (Temp00_RoomRAMPtr),y
        dey
        bne @loop_inner
    dec Temp00_RoomRAMPtr+1
    inx
    bne @loop_outer
```

- 内环确实 `sta (zp),y / dey / bne` ⇒ 写 6 + dey 2 + bne 跳 3 = 11/字节；末字节 bne 不跳 10。256 次 = 255×11+10 = 2815 ✓（`cycles.rs:90`）。
- `sta (zp),y` 是**写**，页交叉不加罚（store 恒 6），`INDY_ST=6` 用得对。
- X 起点验算：InitTables `lda RoomRAMPtr+1 / tay / tax / iny×3`（`prg7_engine.asm:9836-9841`）⇒ X=$60、Y=$63；FillRoomRAM 内 `txa / sty Temp00+1 / clc / sbc Temp00+1` ⇒ A=$60−$63=$FD ⇒ **X 从 $FD 起，$FD→$FE→$FF→$00 = 3 页** ✓。注意 asm 原注释写 `#$FC`（`prg7_engine.asm:9870`）是**上游注释算错**，$FD 才对；本次代码取 3 页是对的，未被上游注释误导。
- 外环 3 次：`(5+2+3)×2 + (5+2+2) = 29` ✓（`cycles.rs:91`）。

- **唯一瑕疵（+1）**：壳项 `cycles.rs:89` `const SHELL = PHA + IMP + ZP + IMP + IMM + IMP + PLA + IMM + ZP` 把 `sbc Temp00+1`（zp，3c）记成 IMM(2c)。asm 9 条指令为 `pha(3) txa(2) sty zp(3) clc(2) sbc zp(3) tax(2) pla(4) ldy#(2) sty zp(3)` = **24**，代码得 23。这也解释了 `model_init_tables() = 9262` 与旧值「精确相等」的巧合：8503+703+56 = 9262，而壳若为 24 则应为 9263。**「旧估值本来就是逐条数的」这一结论只对到 1c 精度。**

### A2 DrawStructRow 的 AdvanceRow 是否重复计入外层——**未重复，但「行」桶少算 DrawStruct 入口 ≈17c**

asm 结构（`prg7_engine.asm:9588-9718`）：`DrawStruct(EF8C)` 每行 ⇒ `jmp DrawStructRow(EF13)`（行头）⇒ 元 tile 环 ⇒ 行尾 `jmp AdvanceRow(EF78)` ⇒ AdvanceRow 末尾**顺序落入** `DrawStruct`。

- `cycles.rs:211-213`：`ROW_HEAD(50) + ADVANCE(22) = 72`，AdvanceRow **只在 ROW 里算一次**；元 tile 模型 `cycles.rs:142` 的 TAIL 只到 `and #$1F / bne LEF72 / dec Temp0E / bne DrawMetatile`，**不含 AdvanceRow** ⇒ 不重复 ✓。
- 消费面 `room.rs:155` 每行加一次 `LOAD_ROW_CYCLES`，`room.rs:169-170` 每元 tile 加一次 MT，一行的行头只加一次 ✓（`room.rs:149-164` 循环结构正确）。
- **未计入项（登记级）**：`DrawStruct` 自身每行入口 `ldy #$00 / sty Temp10 / lda (StructPtr),y / cmp #$FF / beq / jmp DrawStructRow` ≈ 17c，以及行尾 `lda Temp10 / clc / adc Temp0E / sec / sbc #$01 / jmp AdvanceRow`（`prg7_engine.asm:9708-9718`）≈15c，**都不在 ROW=72 里**。所以「DrawStructRow 单行 72」是**行内**成本，不是「结构行总成本」。再加上元 tile TAIL 按「bne 跳转」计，而一行最后一个元 tile 走的是 `dec/bne 不跳/lda Temp10`（−6c），每行还有 ±10c 的结构性漏算。这些量级小，但方向是先漏后补，不是精确。

### A3 LoadEnemy 尾跳是否把 FORCE_SPEED 算重——**没算重，但整族少算 ≥13c**

asm（`prg7_engine.asm:8741-8760`、`8798-8815`、`11530-11553`）：

- `LoadEnemy(EB06) = jsr GetEnemyData / jmp EnemyLoop`。尾跳是 **`jmp EnemyLoop`（3c）**，不是 `jmp InitEnemyForceSpeed…`。`InitEnemyForceSpeed…` 的调用点在 **LEB4D 尾**：`LEB6E: ldy EnsExtra.0.type,x / asl Ens.0.data05,x / jsr InitEnemyFacingDirectionAxisAndDelay / jmp InitEnemyForceSpeedTowardsSamusDelayAndHealth`（`prg7_engine.asm:8810-8813`）。
- `cycles.rs:237` `JSR + GET_ENEMY_DATA + FORCE_SPEED + JMP_ABS`：FORCE_SPEED **只计一次**（作为 LEB4D 尾跳的被调方，`jmp` 3c + 被调体），**不存在 jmp 尾跳后又多算一个 jsr/rts 的重复** ✓。`GET_ENEMY_DATA` 里的 `PHA/PLA/IMM #$03/RTS`（EB24-EB26）也是真实指令，非重复。
- **但 ABSX 周期表口径错**：`cycles.rs:49` `ABSX = 4` 是 **`lda abs,x` 读**的周期；`sta abs,x` 是 **5c**，`ror/asl abs,x` 是 **7c**。ENEMY 族恰好全是 store/shift：
  - `LEB4D`（`cycles.rs:230-232`）：`sta Ens.0.y,x`(5) `sta Ens.0.x,x`(5) `sta EnsExtra.0.status,x`(5) `sta Ens.0.isHit,x`(5) `sta EnsExtra.0.hi,x`(5) 各被记 4（−5），`asl Ens.0.data05,x`(7) 记 4（−3）⇒ **LEB4D 少 8c**（133 vs 141）。
  - `InitEnemyFacing…`（`cycles.rs:228` `FACING`）：`ror Ens.0.data05,x`(7) 被记成 `INCDEC_ZP`(5) −2，`sta Ens.0.delay,x`(5) 记 4 −1 ⇒ **43 应为 44**。
  - `FORCE_SPEED`（`cycles.rs:229`）：`sta data0D,x`、`sta health,x` 各 −1，**`asl A`(2c) 整条漏计**，`bpl` 按不跳(2)计而常见径（非 mini-boss）是**跳**3 ⇒ 32 vs 37（−5）。
  - `GetEnemyType`（`cycles.rs:234`）：`bpl Lx228` 在「非 mini-boss」径是**跳转**（`prg7_engine.asm:8805`，bit6 清 ⇒ 跳），代码记 `BR`(2) ⇒ 29 vs 30（−1）。
  - 合计：**274 实际约 287（+13）**，而 `cycles.rs:218` 声明「槽未占用、非 mini-boss 的常见径」——该声明与 `bpl` 取不跳自相矛盾。

### A4 DrawMetatile / UpdateAttrib——**MT_FULL 少 8c，属结构性漏项**

- MT_FAST 边界判（`prg7_engine.asm:9615-9627`）：`lda hi/cmp/beq(不跳)/cmp/bcc(跳)` = 12 ✓（`cycles.rs:135`）。
- 4 tile 环 `ldx #$03 / [ldy/lda (MetatileDefs),y/inc/ldy TilePosTable,x/sta (zp),y/dex/bpl]`：3×28 + 27 = 111 ✓（`cycles.rs:137-138`）。
- 早退 UpdateAttrib = jsr(6) + `lda ObjectPalette(3)/cmp RoomPalette(3)/beq(跳)3` + rts(6) = 21 ✓（`cycles.rs:139`）。
- **全径 UpdateAttrib（`cycles.rs:160-169`）的 body 从 `lda Temp00_RoomRAMPtr`（`prg7_engine.asm:9760`）起算，正好跳过了 `lda ObjectPalette / cmp RoomPalette / beq(不跳)` 这 3+3+2 = 8c 前缀**；而 `UA_FULL = JSR + body + RTS`（`cycles.rs:174`）只加了 6+128+6 = 140，早退径的 21 里却含前缀。⇒ 全径 UpdateAttrib 实为 148，**`model_draw_metatile_full` 应为 341 而非 333**（`cycles.rs:172-176`）。
- `PAGE_CROSS_METATILE = 6`（`cycles.rs:151`）按旧口径固定补偿，注释已自承「期望值不是保证值」——诚实标注，可接受（登记级）。

### A5 DrawObject——壳 114 逐条对得上，y 单元口径偏乐观

- PROLOGUE 42 / XPOS 15 / LEA8D 57 逐条核对 asm（`prg7_engine.asm:8609-8660`）**全部吻合** ✓（含 `beq LEA80` 按不跳计）。
- `model_draw_object_y_unit = 17`（`cycles.rs:199-201`）：asm `lda/clc/adc #$40/sta/bcc(跳)/dex/bne(跳)` = 3+2+2+3+3+2+3 = 18？——代码是 `ZP+IMP+IMM+ZP+BR+IMP+BR_TAKEN` = 3+2+2+3+2+2+3 = 17，即 **`bcc` 按不跳(2)**。常见径不进位 ⇒ `bcc` **跳**（3）⇒ 应为 18。且末次 `bne` 不跳(−1)，平均 17.5。`cycles.rs:200` 注释自承「无跨字节进位的常见径」，但进位时是 `bcc` 不跳 2 + `inc abs` 6 = 8。方向：模型对单个单元 −1c，对 yu 个单元再 −1（末次）。

## 3. B 面：防弱化——**结构通过，1/8 断言是恒真**

- `git show --stat`：改动 7 文件 = `cycles.rs`(新) + `room.rs` + `mainloop.rs` + `mod.rs` + baseline 三件。**`gates/` 零改动**，未动门禁、未加白名单、未调阈值 ✓。
- baseline 仅三件且内容最小：
  - `data/status/baselines/baseline.json`：唯一变化 `rust_test_count 542 → 545`，其余 metric（naming 738 / knowledge 47 / completeness 57）不动 ✓。
  - `baseline_history.jsonl`：追加一行，`old 542 / new 545`，reason 明确写「新增 3 条钉值/债表测试」，带 `baseline_sha256: 0d4e0c61…` ✓（与 SHA256SUMS 新值一致）。
  - `SHA256SUMS`：仅 baseline.json 与 baseline_history.jsonl 两行哈希更新，`rom/Metroid (USA).nes` 行未动 ✓。
- `shipped_values_match_legacy_calibration`（`cycles.rs:328-338`）确实会失败，不是空断言：它比的是 7 个 `const`（编译期求值）与 `legacy::*` 常量，任一处模型/债项改 1 都会翻红；`room.rs:1250-1258` 另有掉帧窗三房断言兜底。
- **但 8 条断言里有 1 条恒真**：`cycles.rs:337` `assert_eq!(LOAD_SHELL, legacy::SHELL)`，而 `cycles.rs:288` `LOAD_SHELL = legacy::SHELL` —— 同一常量自比，永远绿。「SHELL 未分解」恰好**没有被安全网覆盖**（这也是全文件唯一没被 `model_*` 支撑的上线值）。

## 4. C 面：零行为变化——**通过（独立验算 8/8）**

python 独立重算，`ship(model, balance)` 与 `legacy::*` 全等（未依赖 Rust 编译）：

| 项 | model | balance | ship | legacy |
|---|---|---|---|---|
| INIT | 9262 | 0 | 9262 | 9262 |
| OBJ_SHELL | 114 | +66 | 180 | 180 |
| Y_UNIT | 17 | +3 | 20 | 20 |
| ROW | 72 | −42 | 30 | 30 |
| MT_FAST | 214 | 0 | 214 | 214 |
| MT_FULL | 333 | +16 | 349 | 349 |
| ENEMY | 274 | +26 | 300 | 300 |
| SHELL | 1300 | — | 1300 | 1300 |

- 结构上更强：本 commit 在 `room.rs` **只改了常量定义块**（`room.rs:55-61` re-export），6 处累加点 `room.rs:113 / 117 / 155 / 170 / 506 / 629` 的**表达式一行未动**（diff 无这些行），`mainloop.rs:3145` 只把 `room::LOAD_SHELL_CYCLES` 换成 `cycles::LOAD_SHELL`，值同为 1300 ⇒ 零行为变化是**编译期保证**，不依赖我对 asm 的判读。
- `physics.rs:1384` 消费 `room::LOAD_BASE_CYCLES`（29000）未被触碰 ✓；`room.rs:63-64` BASE/FRAME 未变 ✓。
- 负债符号正确：`ship()`（`cycles.rs:277-281`）走 i64，`72 + (−42) = 30 ≥ 0`，`assert!` 未被触发；`cycles.rs:342-352` 另有方向断言（ROW<0、其余>0）✓。

## 5. D 面：新问题（按严重度）

1. **【must fix，审计方法论】债表是残差，不是独立测量。** `balance = legacy − model`（7 项逐项 = 旧值减模型值，见上表），因此**任何模型误差都被债项自动吸收**，`shipped_values_match_legacy_calibration` 永绿。实证：MT_FULL 的 +16 被注释归因为「`bvc/beq` 分支走向 + 平移环轮数」（`cycles.rs:259-260`），但其中 **8c 只是漏掉的 `lda/cmp/beq` 前缀**（§A4）；ENEMY 的 +26 被归因为「旧值含 boss 径与槽占用径的加权估计」（`cycles.rs:265`），但其中 ≥13c 是**模型自己的 ABSX store 口径错**（§A3）。**债表的因果归因不成立**，需要拆成「模型缺陷」与「标定差额」两类，否则下一步「烧债」会把模型错误一起烧进 BASE。
2. **【must fix，注释自洽】`model_update_attrib_full_body` 的分项注释与代码不符 5 处**：`cycles.rs:161-162` 注释 53 实为 55；`163` 注释 13 实为 14；`164` 注释 18 实为 15；`165` 注释 17 实为 13；`167` 注释 12 实为 11。**分项合计 53+13+18+17+20+12 = 133 ≠ 代码 128**。总数 128 与测试一致，但「A 级逐行可复核」的宣称在这 6 行上不成立（读者按注释复核会得到 133，与钉值冲突）。
3. **【must fix，文档一致性】`room.rs:43-56` 的旧标定注释块未更新，且与 `cycles.rs` 直接冲突**：仍写「FillRoomRAM ≈8508 / AttribFill 704 / 壳 ≈50 / UpdateAttrib 全径 152 / DrawObject 壳 ≈180 / LoadEnemy ≈300」，并在 `room.rs:45` 宣称「常量=6502 逐条计数（asm 内容锚）」——这正是 r9-A1 已判为不成立的说法；现在同一份代码里两处注释互斥（`cycles.rs:6-9` 明说旧常量是手估）。半修比不修更危险：后来者按 `room.rs` 读会得到错误图景。
4. **【可登记遗留】`ABSX` 周期常量语义混用**（`cycles.rs:49`）：一个常量同时充当 `lda abs,x`(4) / `sta abs,x`(5) / `ror,asl abs,x`(7)。建议拆 `ABSX_R/ABSX_W/ABSX_RMW`，否则后续任何含 abs,x 写的例程都会系统性少数周期（本批 ENEMY/OBJ 两族已被污染）。
5. **【可登记遗留】命名双轨**：`room.rs:58-61` 把 `LOAD_SHELL` re-export 成 `LOAD_SHELL_CYCLES`，而 `mainloop.rs:3145` **绕过 re-export** 直接写 `cycles::LOAD_SHELL`。同一值两个名字、两条访问路径；`LOAD_MT_FAST/LOAD_MT_FULL` 在两处同名（无 `_CYCLES` 后缀），`LOAD_BASE_CYCLES` 又留在 `room.rs` 无对应 model —— 「哪些是模型量、哪些是标定量」在命名上不可辨。你担心的 `LOAD_BASE_CYCLES` 与 `LOAD_SHELL` 语义混淆**当前不存在**（BASE 是 29000 的标定基底，SHELL 是 1300 的例程壳，值差 22 倍），但**命名不足以自证**，是下一批的风险面。
6. **【可登记遗留】SHELL 未分解 ⇒ 「六处改成逐条」属过度宣称**：8 个上线值里 `LOAD_SHELL` 仍是纯旧值（1300），且安全网对它恒真（§3）。commit message 用「六处累加改 6502 逐条周期模型」，实际是 **7 个常量里 6 个有模型支撑、1 个（占载房壳成本）原样搬运**；建议措辞改为「房内热循环逐条化，壳待分解」。
7. **【可登记遗留】`pub mod cycles` + 全部 `pub const fn model_*`** 成为 game-core 公开面，无 `#[doc(hidden)]`/`pub(crate)`；未来易被误当权威周期表引用。

## 6. 问题清单

**must fix（不改行为，改注释/文档/归因）**
- M1 债表因果归因错误：MT_FULL +16 与 ENEMY +26 中含**模型缺陷**（§A4 8c、§A3 ≥13c），不能记作「旧估值偏差」。
- M2 `cycles.rs:161-167` 五个分项注释数值错（合计 133≠128）。
- M3 `room.rs:43-56` 旧注释块与 `cycles.rs` 冲突且重复 r9-A1 已否决的「常量是逐条数出来的」说法。

**可登记遗留**
- L1 `ABSX=4` 混用读/写/RMW；ENEMY 族 274 实际 ≈287、MT_FULL 333 实际 341（改与不改均不影响上线值，因债表吸收）。
- L2 `cycles.rs:337` SHELL 断言恒真，SHELL 未分解未被安全网覆盖。
- L3 FillRoomRAM 壳 `sbc zp` 误记 IMM（23 vs 24）——顺带动摇「INIT 9262 与旧值精确相等」的解读。
- L4 `GetEnemyType bpl` 与声明的「非 mini-boss 常见径」矛盾；OBJ y 单元 17 vs 常见径 18。
- L5 命名双轨 / `mainloop.rs:3145` 绕过 re-export / `model_*` 全公开。
- L6 「六处逐条」措辞过度（SHELL 未分解）。

## 7. Confidence

**80 / 100**

- 高置信：C 面 8/8 数值（python 独立复算 + diff 结构双重锁定，可到 95）；B 面基线三件与 gates 零改动（diff 直读，95）；A1 FillRoomRAM 与 A3 尾跳不重复（asm 原文逐行，90）。
- 中置信：A3/A4/A5 的少算量化（依赖 NES 标准周期表 `sta abs,x=5`、`asl/ror abs,x=7`，A 级文档知识；逐条映射由人工配对，±1c 级别的归属可能有争议，75）。
- 低置信：MT_FULL「平移环期望 2 轮」（`cycles.rs:166`）与 `SHIFT` 的 ×2 假设，未能从运行时数据证实（60）；旧标定 2026-11-05 的原始推导文档未在本次时限内复读（因此「旧值是手估」这一前提沿用 commit 与 `cycles.rs:242` 的说法，未独立验证）。
