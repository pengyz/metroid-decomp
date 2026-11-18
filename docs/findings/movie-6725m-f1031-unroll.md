# 6725M f1031 退球分歧定界：SamusRoll 不是根因（2026-11-18）

> 任务来源：6725M（any%）全片 RAM 对拍，BizHawk 2.10 + quickerNES ↔ Rust，
> 「首个实质性分歧 f1031：BizHawk 退球成功（status=04），Rust 留在球里（status=03）到 f1081」。
> 本文回答三件事：**asm 在 f1030/f1031 会怎么判**、**Rust 判的是不是同一条 asm 语义**、
> **真正让两侧分岔的是什么**。结论分级：A=硬证据（字节/trace/逐帧数值），B=可复现静态推理，
> C=假设。未证实项集中在 §7，不与结论混写。

## 0. TL;DR

| 问题 | 结论 | 级 |
|---|---|---|
| f1031 是 `SamusRoll` / `CheckMoveVertical` 判据错吗？ | **不是**。两侧判据、地址换算、瓦分类完全一致，只有**坐标不同** | A |
| 那 Rust 留球对不对？ | 在 Rust 自己的 x 上**对**：探针 0x66DE/0x66DF = `$00/$01`（<$80 固实）⇒ asm 同样会回滚动 | A |
| 根因归类 | **(d) 根本不是这个例程** | A |
| 真正分岔 | 两侧**输入生效相位差一帧**（Rust 对同一录像输入的反应比 quickerNES 晚一帧），在 f528 起以「1 帧」出现、f680 起变成 1~3px 漂移、f961..f991 拉到 16px，f1031 只是它撞上「探针落在固实列」的那一帧 | A（机制证伪/证实见 §5） |
| 附带的 asm 保真修 | `CheckMoveVertical ; 07:E7DE` 的块对齐闸（双臂）、下行回卷 `+0x0F`→`+0x10`（`; 07:E912` C 恒 1）——**与本分歧无关**，本片 28308 帧行为零变化 | A |

## 1. 现象（可复现）

复现命令（本会话实跑；产物只落 /tmp）：

```bash
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog \
    --frames 1200 --out /tmp/m6725-full.sched            # 已存在则直接用
./target/debug/rust_screenshot --schedule /tmp/m6725-full.sched \
    --out /tmp/shots-f1031 --state-out /tmp/rust-f1031-before.jsonl
python3 tools/movie_ref/compare_bizhawk_rust.py \
    --bizhawk /tmp/bh-full-28408.jsonl --rust /tmp/rust-f1031-before.jsonl \
    --start 1020 --end 1200
```

**行相位**（实测口径，非同相——这是本分歧的关键，见 §5）：

- Rust `--state-out` 行 `fN`：采样缝在 `crates/game-core/src/frame/engine.rs:112-117`
  （`!iter_span_left && !title_continuation() && phase∈{PostSpin,MainLoop}` 才采，故 **span 帧无行**），
  位置在 tick(N) 内 **post-NMI(N) / pre-work(N)** ⇒ 行内 `$0300-$0316` = **work(N-1) 的结果**、
  `$0014` = NMI(N) 刚锁存的 **输入行 N**。
- quickerNES（`tools/movie_ref/bizhawk_ref.lua`，先读 RAM 再 frameadvance）行 `fN`：`$0014` 也是
  **输入行 N**（实测 f1030=01、f1031=09 与 inputlog 第 1030/1031 行一致），但该行的状态里**已经含有
  消费输入行 N 的那次 work**（实测 f528：`$0014=0x82` 且 y 当行已 C1→BD）。
- ⇒ 两腿的**锁存面同相、work 面对齐差一行**：等价于「Rust 在 tick N 应消费输入行 N+1」。
  §5.2 的 +1 帧实验正是这条映射的机械验证（1152/1152 全同）。**因此本文所有逐帧比较都在
  「同 f 标签」下做，但不可把同 f 当成同 work 相位**。

f1020..1200 窗口（修复前后相同）：可比 181 / 全同 97 / 失配 84，首个失配 **f1031 status**。
两侧关键字段（同相位同源取值）：

| f | 侧 | status | radiusY | y | x | hi | Joy1Change | Joy1Status | ScrollX | ScrollDir |
|---|---|---|---|---|---|---|---|---|---|---|
| 1030 | BizHawk | 03 | 08 | C8 | **06** | **00** | 00 | 01 | 77 | 3 |
| 1030 | Rust | 03 | 08 | C8 | **F6** | **01** | 00 | 01 | 67 | 3 |
| 1031 | BizHawk | **04** | 0F | **BD** | 06 | 00 | 08 | 09 | 77 | 3 |
| 1031 | Rust | **03** | 08 | C8 | F8 | 01 | 08 | 09 | 67 | 3 |

⇒ 同一时刻、同一 `Joy1Change=0x08`（UP 新按下）、同一 `status&DOWN=0`：BizHawk 退球并上移 11px
（C8→BD），Rust 不退球，`x` 继续以 1~2px/帧 前进（F9→FB→FC→FE→FF→01@f1037），f1041 前后才站起。

## 2. asm 真值推演（`refs/m1disasm/SRC/prg7_engine.asm`，A 级）

### 2.1 路径

`SamusRoll ; 07:D0E1`（:3618）——`Joy1Change & BUTTON_UP` 或 `Joy1Change bit7`(JUMP) 且
`Joy1Status & BUTTON_DOWN == 0` 才进：

```
lda Samus.radiusY / clc / adc #$08 / sta Samus.radiusY   ; 球 8 → 16
jsr ObjectCheckMoveUp                                     ; ; 07:E7A2（:8141）
bcc Lx032                                                 ; 不可站 → 回滚动保持
（可站）lda #-$0B→Temp04_SpeedY / ApplySpeedToPosition（上移 11px）
        jsr SetSamusStand / dec Samus.animIndex / jsr StopVertMovement / lda #$04 → SetSamusData
```

`ObjectCheckMoveUp`：`lda Samus.radiusY / clc / adc #$08`（16+8=**24**）→ `ObjectCheckMoveVertical`：
`sta Temp02_DistToCenterY` → `lda Samus.radiusX` → `CheckMoveVertical ; 07:E7BD`。

### 2.2 f1030/f1031 逐格

起点（两侧共有的实况）：`PositionY=$C8(200)`、`PositionX=06`、`hi=00`、`radiusY=08`、`radiusX=04`、
`ScrollDir=3`（横滚）。

| 步骤 | asm | 本帧取值 |
|---|---|---|
| dist | `radiusY+8`（`; 07:E7A2`）→ `+8`（SamusRoll） | 8+8+8 = **24** |
| 撞点 | `CalculateFirstBGCollisionPoint ; 07:E90F` Lx213 支：`PositionY − DistToCenterY`，`and #$07 → Temp00_CollisionPointYMod8` | 200−24 = **176**；176&7 = **0（对齐）** |
| 闸 | `; 07:E7DE lda Temp00 / bne Lx202` | **不跳过**，进取瓦环 |
| 块数 | `GetNumBlocksToCheck ; 07:E8CE`：`A=(x−rx)&7 = 2` ⇒ `right_rem=(-2)&7=6`；`2·rx=8`；`8−6=2`；`left_rem≠0 ⇒ inx` | 列起点 `x−rx=2`，**2 块**（列 2、列 10） |
| 地址 | `MakeRoomRAMPtr ; 07:E96A`：`0x6000 \| (hi&1)<<10 \| (((y&0xF8)<<2) \| (x>>3))` | y=176 ⇒ `(176<<2)=0x2C0`；列 2/10 ⇒ `x>>3=0/1`；hi=0 ⇒ **0x62C0 / 0x62C1** |
| 瓦值 | `lda (Temp04),y / cmp #$4E / CheckBlastTile: bcc Exit16（<$80=固实） / cmp #$A0 bcs IsWalkableTile` | **0xFF / 0xFF** ⇒ ≥$A0 可走 ⇒ 无碰撞 |
| 出口 | `Lx202: sec / rts` ⇒ `bcc Lx032` 不成立 ⇒ 站起 | y 200−0x0B = **0xBD** ✓ 与 BizHawk f1031 完全一致 |

⇒ **asm 在 BizHawk 的坐标上必然退球**，且探测地址/瓦值 = 0x62C0/0x62C1 = FF/FF（A 级：
`/tmp/bh-deep-f1031.jsonl` 的 System Bus `$6000-$67FF` 全 2KB dump 实读）。

### 2.3 同一组字段搬到 Rust 的坐标

Rust 在它自己的 work(1030) 起点是 `x=0xF6(246)`、`hi=01`（y/radius/status/输入全同）：

- 撞点仍 176（对齐，闸不跳过）；
- `x−rx = 242` ⇒ `left_rem=2 / right_rem=6 / 2·rx=8` ⇒ **2 块**：列 242、250 ⇒ `x>>3 = 30 / 31`；
- `hi&1=1` ⇒ page `0x400` ⇒ 地址 **0x66DE / 0x66DF**；
- 实读瓦值 = **0x00 / 0x01**（<$80 = 固实，`$00` 是该房间地板砖，row26 = `00 01 08 09…`）⇒ 碰撞 ⇒
  `CheckBlastTile bcc Exit16` ⇒ 进位 = 1 ⇒ `bcc Lx032` 成立 ⇒ **回滚动保持**。

⇒ Rust 的行为**符合 asm**。两侧差在 `x`/`hi`，而 `$6000-$67FF` 房间 RAM 在 f1030 **逐字节相同**
（System Bus dump × Rust state 行 `wram` 前 2KB：**2048 字节 diff=0**），所以不是「瓦表不同」。

## 3. Rust 现状（工作树 `crates/game-core/src/samus/`）

- `handler.rs:1149 samus_roll`：`(change&UP)||(change&A)` 且 `status&DOWN==0` → `radiusY+=8` →
  `object_check_move_up` → 成功则 y−=0x0B / `set_samus_stand_no_footstep` / `animIndex−1` /
  `stop_vert_movement` / A=4 → `set_samus_data`（status 落 **4** = sa_PntUp，与 BizHawk f1031 的 04 同）。
- `physics.rs:748 object_check_move_up`：`check_y = y − ry − 8`；`y < ry+8` ⇒ `−0x10` 且
  横滚房 `hi ^= 1`；`x < rx && 横滚房` ⇒ `hi ^= 1`；`rx==0 ⇒ true`；`vertical_span_cols` 逐列
  `collision_point_samus`（`idx=((y&0xF8)<<2)|(x>>3)`，`tile==0x4E → false`，`tile<$80 → false`）。
- 与 asm 的**行为差**（本会话修掉，见 §6）：①缺 `; 07:E7DE` 的块对齐闸（双臂）；②下行回卷写
  `+0x0F`（asm `Lx212: txa / adc #$0F` 入场 carry 恒 1 ⇒ 实际 `+0x10`）。

## 4. 根因分类：**(d) 根本不是这个例程**

排除链（每条都是数值证据，不是推理）：

- **(a) 探测几何错**：几何式两侧一致（`y−ry−8`、`±0x10` 回卷、`hi` 翻转条件、`GetNumBlocksToCheck`
  列跨度），且同参数下两侧算出**同一个撞点 176**；差异只在输入坐标 `x/hi`。→ 排除。
- **(b) mod8 闸错**：本帧 `176&7=0`，闸开不开都不改变结论（单测 `up_check_reports_solid_when_*`
  已钉死「对齐时必须取瓦」）。→ 本帧排除（闸缺失本身是真缺陷，但不是 f1031 的原因）。
- **(c) 瓦固体性分类错**：房间 RAM 逐字节相同；`$00/$01 < $80` 是 asm 明写的固实（`CheckBlastTile`
  注 "CF=0 if tile # < $80 (solid tile)"）；BizHawk 那边读到 `$FF` 是**可走**，也是 asm 真值。→ 排除。
- **(d) 上游分岔**：见 §5。

## 5. 机制：一帧的输入生效相位差（决定性实验）

### 5.1 逐字段首个差异（`/tmp/bh-pos2.jsonl` × Rust state，f30..1200）

| 字段 | 首个差异 | 现象 |
|---|---|---|
| y / spdY | **f528** | 输入 `0x82`(A+Left) 当帧，BizHawk 已跳（y C1→BD），Rust 下一帧才跳 |
| animD / spdX / accX 等 | f662 / f680 | 松键后减速相位错一帧 ⇒ 水平 1~3px 漂移 |
| status | **f952** | DOWN 变球：BizHawk f953 已 03，Rust f954 才 03（同一 `$0014=04` 下） |
| y (球下落) | f961..f991 | BizHawk f981 落地（y=C8），Rust f992 才落地 ⇒ 差 11 帧、dx 拉大到 16px |

两侧 `$0012/$0014` 在每一行**数值相同**（f524..f535 全同），说明「看到的输入」一样，**生效时机不同**。

### 5.2 把输入整体提前一帧 ⇒ 全窗 0 失配

保持录像不变，只把调度表里的 `joy N` 换成 `inputlog 第 N+1 行`（f≥40 起），其余全同：

| 运行 | 窗口 | 可比 | 全同 | 失配 | 首个失配 |
|---|---|---|---|---|---|
| 原调度 | 30..1200 | 1164 | 1072 | **92** | f940 scrolldir |
| 原调度 | 1020..1200 | 181 | 97 | **84** | f1031 status |
| **+1 帧（f≥40）** | 40..1200 | 1152 | **1152** | **0** | 无 |
| +1 帧（f≥40） | 40..28408 | 28265 | 2326 | 25939 | f1566 status(01/05) |

⇒ f940/f944/f948/f952/**f1031..1081**/f1093..f1123 **全部消失**，且不是靠放宽判据（字段集不变）。
⇒ 但「整体 +1」不是全片常数：f1566 起又分岔（说明两腿的帧轴/输入轴是**分段**对齐的，不是单一平移）；
boot/title 段（f<40）用 +1 反而崩（title 输入相位敏感，实测 25943 失配）。

### 5.3 与并发结论的一致性（跨模拟器互证）

`docs/findings/movie-bizhawk-6725m.md` §11（并发会话本轮补做、本会话不重复写）给出
Mesen2 ↔ quickerNES 的同输入配对：boot 段 quickerNES 晚 2 帧（`$5A→09` 24/26、`status→$20` 28/30），
f166..f487 段 Δ0，**f672 起出现 ±1 帧漂移**。本会话从「输入生效相位」独立测到同一现象
（§5.1/§5.2），两者互证：**Rust 侧与 Mesen2 语境的既有基线对齐；f1031 段的分岔是 quickerNES 侧
帧轴/输入轴与 Rust 的错位，在输入跳变帧显形，再经跳跃/落地/滚动累积成 16px 位置差。**

> **对 `movie-bizhawk-6725m.md` §6.1/§6.3 的更正建议**（该文件正被并发会话编辑，本会话不写入）：
> `$0014` 967/967 全同**只证明「行号↔锁存值」对齐**，不证明「行号↔该行状态所消费的输入行」对齐。
> 本会话实测：两腿行 `fN` 的 `$0014` 都是输入行 N，但 Rust 行 `fN` 的状态来自 work(N-1)（消费行 N-1），
> quickerNES 行 `fN` 的状态已含消费行 N 的 work。故「锁存全同」与「work 输入差一行」可以同时成立
> （本会话实测即是）。判定 work 相位必须用**输入跳变帧的行为响应**（§5.1），或按 §5.2 做 ±1 对照。
> 若只按锁存面判「输入无问题」，f1031 这类分歧会被误判成核心逻辑 bug。

## 6. 修复（asm 保真，非 f1031 之因）与数字

改 `crates/game-core/src/samus/physics.rs`：

1. **块对齐闸（双臂）**：`CheckMoveVertical ; 07:E7DE`（原注 "skip collision if object boundary is
   not at block boundary"）——撞点 Y 未对齐 ⇒ `Lx202: sec/rts`（无碰撞）。`Temp00 =
   (y−DistToCenterY)&7`，回卷 ±0x10 不改 mod8 ⇒ 用未调整 `check_y`。
2. **下行回卷 +0x0F → +0x10**：`Lx212: txa / adc #$0F` 两条入场径（`bcs Lx212`、`cpx #$F0/bcc` 落空）
   carry 恒 1 ⇒ 实际加 0x10；旧 `+0x0F` 在对齐帧上会落到前一行（例 X=0xF8：asm 行 8，旧形行 0）。

TDD：先加 5 条锚测（`physics.rs::tests_block_align_gate` 3 条 + `handler.rs` 2 条），把闸改回
`false &&`/把 +0x10 退回 +0x0F 时**实测 3 条红**（含 handler 的 `samus_roll_unrolls_...`：
`status left:3 right:0`——单测里复现了「该退球却没退」的形态），修复后 9/9 绿。另把 3 条建立在
「非对齐几何」上的旧测试改成对齐几何（保持原意图）。

**行为影响（决定性）**：修前/修后各跑全片 28408 帧，逐帧解 `$0300-$0316` 23 字节比对
⇒ **28308 帧、不同帧 0**。即该修对 6725M 全片**零行为差**（符合「asm 语义修，命中面不在此片」）。

五条门禁数字：

| 门禁 | 结果 |
|---|---|
| `cargo test --workspace` | lib **663 passed / 0 failed**；全 target 绿（总 754，baseline 749→754 已登记） |
| `python3 gates/cli.py check` | **PASS=22 FAIL=0 SKIP=3 TOLERATED=1**（含 `nes-map` 行号 refs 更新、`ratchet:rust_test_count` 754） |
| `python3 gates/cli.py l4` | 用例 15 · 分母 132 · **match 126 · diff 6** · blocked 0（与基线一致，无退化） |
| 录像对拍（1020..1200） | 修前=修后：可比 181 / 全同 97 / 失配 84，首失配 f1031 → **未消除、未后移**（因为不是本例程） |
| 相位对照实验 | +1 帧（f≥40）40..1200：**1152/1152 全同、0 失配** ⇒ f1031 由相位差解释 |

## 7. 未证实（单列，不得当结论用）

1. **分段相位差的成因未定**：f40..1200 段需要 +1（已在 400..1200、40..1200 两个窗口证到 0 失配，
   且 +1 起 f100 与 +1 起 f40 结果相同），但 f<40 用 +1 反而崩（title 输入相位敏感）；f1566 起又漂移。
   候选（均未证）：①boot/title 入口差（实测 Rust 行 f28 已有 `status=14/y=B0/x=80`，quickerNES 到
   f30 才有 ⇒ 入口差 **2 行**，与 gameplay 段测得的 **1 行** 不等）；②`iter_span_left` 迭代跨帧
   （`engine.rs:112-117` 该帧不采行）与 emulator 掉帧/双帧计数的交互；③`inputlog` 行号 ↔ 录像帧的
   导入侧 off-by-one（`movie_sched.py` 头注约定 `joy N` = inputlog 第 N 行 = 录像第 N-1 帧）。
2. **「+1」的物理归属未定**：本会话只证「Rust 在 tick N 消费输入行 N+1 时与 quickerNES 逐帧全同」，
   未证这 1 行差异属于**哪一侧**（是 Rust 的输入注入错一行，还是 quickerNES 的 movie 行号语义差一行）。
   裁决方式是让 Rust 对 Mesen2 同输入跑同段（§8-2）；在裁决前**不要**把 +1 写进产物或门禁。
3. **像素面**未在本会话复验（L4 只跑既有 15 用例矩阵）；（原 findings §11 记 f670..f980 有 BG 1px
   水平相位差——是否同源于本相位差**未证**）。
4. **`accX/hMax/subX` 数值差**（Rust `accX=D0/hMax=18/subX=C0` vs BizHawk `30/12/20`）是否为同一相位差
   的下游、抑或另有速度/加速度实现差——**未证**。
5. **两种帧轴口径**（Mesen2 语境 shift=0 vs quickerNES 语境需 +1）谁代表「真机」——本会话不作裁决；
   按项目纪律，Mesen2 是 oracle，quickerNES 只作第二来源。

## 8. 下一步（按收益排序）

1. **把帧轴/输入轴做成机械可判**：给 `compare_bizhawk_rust.py` 加「按输入跳变帧的行为响应反解行相位」
   的标定（现在 `$0014` 锁存对齐会**假阳性**地判「输入面没问题」）。本次 §5.2 的 +1 实验就是它的
   手工版。
2. 用 Mesen2 同输入跑同一段（f500..1100），把「Rust vs Mesen2」与「Rust vs quickerNES」两条曲线
   叠起来：若 Rust 对 Mesen2 在 f528/f952 不必移，则 §5.3 的归因（quickerNES 侧错位）成立。
3. 迭代跨帧（`iter_span_left`）与录像帧数的账：统计全片 span 帧与 quickerNES 掉帧点，检验
   「每 span ⇒ 轴移 1 帧」假设（本片 f482..483、673..675、944..945… 的 span 点正好早于各次漂移）。
