# r9 复核：电梯换区像素证据（`02c70799` → 处置 `P0/P1/P2`）

> 复核对象：commit `02c70799`（电梯换区首个像素证据 + 两处真错修复）。
> 复核方式：独立 verifier 子代理，**只读**（`git show`/Read/grep，禁跑 cargo/gates/L4），
> 立场=证伪。结论：**fail**（无 P0，两处代码修复成立），9 条判据里 2 条「不等价」、4 条
> 「有条件等价」；问题集中在**证据主张过头**与**工具对称性**，不在行为实现。
> 本文件记录复核原文要点 + 我方逐条处置（并附我方对复核结论的一处反证）。

## 一、复核结论摘要（原文判定表）

| # | 判据 | 判定 |
|---|---|---|
| 1 | ROM `$95DA` 五 bank 字节 + 区/bank 映射 | 等价 |
| 2 | `extract_area_pal_toggle` / `assets::area_pal_toggle` | 等价 |
| 3 | `more_init` 消费点 ↔ `MoreInit 07:C81D` | 等价（但 commit 未写出「lower bank ≡ BankTable[InArea&0x0F]」前提）|
| 4 | 时序主张（pending=6 先于 2 被消费） | 有条件等价 |
| 5 | 三条新测独立性 | 有条件等价（1 条不能证伪旧错；映射未被钉住）|
| 6 | fixture 双侧通路 / 既有 14 案字节 | 有条件等价 + **未登记的 1 采集索引相位差** |
| 7 | 9 格证据完整性 | **不等价**（像素只覆盖 4 个状态；两侧 RAM 行未对齐）|
| 8 | 基线正当性 | 有条件等价（无回退 ✔，但「全为新增格」不实）|
| 9 | 文档一致性 | **不等价**（把「走完十态」写成「像素已证十态」；称缺口「已闭合」）|

verifier 的 P1 四条：①像素只覆盖状态 {0,1,3,8}；②fixture 注入相位差 1 采集索引 +
rust `==frame` vs lua `>=frame` 不对称；③`area_palette_toggle_matches_rom_anchor` 不能证伪
旧错、`AREA_BANKS` 的 Kr/To 顺序未钉；④`elevator` 测未钉「6 先于 2」的顺序。
P2 五条：基线说明「全为新增格」不实、证据指针路径错、`WaitNMIPass_` 修复是单点非语义统一、
审计文档「已闭合」应降级、`area_pal_toggle` 的 bank 不变量未写出。

## 二、我方处置（逐条）

### 已修（P1）

**② fixture 相位 + once 语义不对称** — `crates/game-core/src/bin/rust_screenshot.rs`：
注入移到**采集发射之前**（emu 侧在帧回调里、采样前注入；轨迹实证 emu f900 那行已含注入值），
并把 `== frame` 改为「逐条 `applied` 表 + `frame >= fx.frame`」——与 `tools/scenario_capture.lua`
的 `fixtureAppliedAt[i]` 完全同构（跳帧也不丢、只注入一次）。

**③ 映射钉住** — 新增 `assets::palette_tests::area_banks_match_bank_table_rom`：
断言 `extract::AREA_BANKS == [1,2,4,3,5]`（= ROM `BankTable ; 07:CA30` 字节 `02 03 05 04 06` 减 1）
+ 五个区值 `[01,01,06,06,06]`。**并显式写出「Kr/To 值同为 `$06` ⇒ 二者互换在值上不可观测」**
——这正是复核要求的「钉映射」而非「钉值」。

**④ 顺序钉** — `transition_consumes_area_palette_at_wait_nmi` 追加断言
`PALETTE_PENDING == 2`（= `SelectSamusPalette` 的覆写值）。理由：`select_samus_palette`
**只写 pending 不做写入**，若把它提前，flush 会消费 2 ⇒ BG 变 pal02 且末值为 0，双红。
**变异验证**：交换两行调用 ⇒ 该测当场红（报错 `WaitNMIPass_ 处须先兑现 pending=6…`）。

### 已修（复核发现之外、由本次对齐暴露的第三处真错）

**`init_bank1..5` 从未建模 `ScreenNmiOff` 的寄存器面，也从未执行 asm 尾的 `jmp NMIOn`。**
- 旧形只写 `NMI_STATUS=1` + `iter_wait()`（建模「等待形」），不写影子 `$FE`、不写 `$2001`、
  不清 `$FF` bit7 ⇒ **换区/进场窗口的整屏暗窗完全缺失**（elev 案 f1206/f1207：
  emu 61440/61440 全黑 vs rust 13115/12777 px 亮屏）。
- 修法：抽 `mainloop::screen_nmi_off`（asm `$C45D` 全形：影子清位 → 真 `$2001` 写 →
  影子 `$FF &~$80` → 真 `$2000` 写），`init_bank1..5` 与 `title_bridge` 三处旧实现**收口到一件**
  （原先同一 asm 例程三份语义不同的实现 = F 类）；每个 `init_bankN` 尾补 `nmi_on(mem)`
  （asm `jmp NMIOn`）——漏这句会让 `nmi_enabled` 永久 false（NMI 死锁、调色板面全停，
  本轮实测过一次全案 11/11 失配）。
- 配套：`title_bridge` 的 NMIOn 旧形只写影子 `$FF`、不写真 `$2000`，改为共用 `boot::nmi_on`。

### 已修（P1-7 的「两侧 RAM 行未对齐」）

`tools/sem.py::_derive_sched` **漏传 fixture**（`schedule_text(..., [])` 无 `fixtures` 参数）
⇒ 注入类用例的 Rust 腿根本不注入、oracle 腿照注入 ⇒ 状态行整段假分歧。
已修并跑 `tools/sem.py case case-elev-norfair-transition`：**MATCHED 1191/1581 帧**
（15 列 `fc,gm,mr,tr,gp,nr,t3,cb,bsp,ia,rm,st,sy,sx,hp` 全同），首分歧 f1227 =
`fc: oracle=B9 rust=B7`（**已知 #8 FC+2 相位残差**，非本轮引入）。
即「两侧同轨」现在有 RAM 行证据，且残差可归因到既有未结项。

### 已修（P2）

- 基线说明：本轮重跑后矩阵为 15 用例 / 122 格 / match 120 / diff 2；**并显式记录**
  `case-password-justin` 两格随调色板修复**改善**（`f700 71→23px`、`f1100 90→42px`）——
  这是「修复波及既有锚且向好」的正面证据，不该被抹成「全为新增格」。
- 证据指针改为真实路径 `analysis/traces/case-elev-norfair-transition.frames.jsonl`
  （旧文写的 `.archive/…` 不存在）。
- 审计文档把「覆盖缺口**已闭合**」降级为「**已收窄**」，并写明剩余缺口。
- `more_init` 的 bank 不变量写进代码注释（`lower bank ≡ BankTable[InArea&0x0F]`，
  依据 `MainLoop` 顶部 `jsr CheckBankSwitch` 在 `GoMainRoutine` 之前）。

### 反证（复核该条不成立）

复核 P1-④ 称「`elevator` 测未钉 6 先于 2 的顺序（若把 `select_samus_palette` 提前，
BG 面仍是 pal05 ⇒ 照绿）」。**不成立**：`select_samus_palette` 只写 `PALETTE_PENDING`，
提前后 flush 会消费 2 并写 pal02 ⇒ BG 面变 pal02。实测变异（交换两行）该测**红**。
（仍按复核建议补了 `PENDING==2` 末值断言，使顺序证据更直接。）

## 三、覆盖现状（诚实版）

- 像素格覆盖的电梯状态：`{0 未注入, 1 Idle, 3 Move, 6 换区核, 8 Fade}`。
- **未覆盖**：2（ScrollXToCenter）/4（ScrollY）/5,7（Fade）/9（Stop）——这些状态在 ROM 里
  **各只持续 1 帧**（emu 帧级实录：f962→2、f1203→4、f1204→5、f1209→7、f1450→9），
  而采集缝约定 `capture(N) = tick(N-1)` 收口态，与 emu 帧 N 呈现面天然相差一帧 ⇒
  1 帧态**结构性不可截**（实测 f1206 边界帧给 13115px 假失配，已从用例移除并注释原因）。
  这五个状态目前只有 emu 帧级轨迹 + 十态机单测支撑，不宣称像素证据。
- 自然径入口仍缺（14 案输入窗落在 440 帧开场演出内，见 findings §1）。

## 四、verdict

复核 fail → 本轮处置后：行为面无 P0、无新增失配；证据主张已按复核降级并补齐
（RAM 行对齐 + 映射钉 + 顺序钉 + 指针修正）。建议**复评**关注：新抽的
`screen_nmi_off` 是否是 asm `$C45D` 的等价形（含 `sta PPUCTRL` 关 NMI 位）、
`init_bankN` 尾 `nmi_on` 是否与 asm `jmp NMIOn` 同位置、以及 sem 修复后的 1191/1581 读数
是否与 #8 的既有残差口径一致。
