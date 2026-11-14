# 电梯 / 换区链的像素证据（2026-11-14）

> 目标：让 `UpdateElevator ; 07:D7B3`（十态）+ `SamusElevator ; 07:D423` + 换区核
> `ElevatorD8BF ; 07:D8A3` 从「单测绿」升级为「有 L4 像素证据」。
> 结论：**已达成** —— `cases/case-elev-norfair-transition.txt` 9 格全 0px。
> 过程中抓出**两处真错**（均在状态面之外，见 §4）。

## 1. 为什么原来没有证据

- 审计文档 §6 早已登记：#6/#3 的实现「坏掉也不会有任何自动化证据报警」。
- 实测原因有两层：
  1. `cases/*.txt` 里没有任何用例进入电梯房，`Elevator.status` 全程 0；
  2. 更根本的是**输入窗落在开场演出里**。`SamusInit ; 07:C8D1` 写
     `Timer3←$2C`，`UpdateTimer ; 07:C266` 每 10 帧减 1 ⇒ **440 帧淡入**，
     `SamusIntro ; 07:C9D7` 在 `Timer3==0` 前不把 `MainRoutine` 交回 `GameEngine`。
     `case-001` 轨迹里 `SamusX` 从 f359 恒 128 直到 f902、f903 才首次变化 ——
     即 `hold 500-550 right` / `press 700 a` 这些**标签声称的动作全部落在演出内**，
     两侧都「不动」，对拍是**空对空**（与此前 shootprobe/scrollseam 同类，第三次）。

## 2. 取证方式（两阶段注入）

- **阶段一（失败，留档）**：`cases/_archive/case-elevprobe-roomreload.txt` ——
  直接注入 `RoomNumber=$0B` + `MapPos=(22,11)` 触发房间重载。电梯对象确实生成
  （`ElevStatus=1 / ElevType=1 / ElevY=131 / ElevX=128`），但 `DrawRoom` 重绘把
  Samus 埋进墙里（`health=0 → status=7 → MR=9 → 4`）。**结论：注入房间号不可用于骑梯。**
- **阶段二（成功）**：`cases/case-elev-norfair-transition.txt` —— 不重载房间，
  直接合成 `LoadElevator ; 07:EC04` 的字段面：
  `$0320=01`（status=Idle）/ `$032f=01`（ElevatorType=Norfair）/ `$032d=$83`（y）/
  `$032e=$80`（x）/ `$0323=$23`（`_id_ObjFrame_Elevator`），并把 Samus 摆到台面上
  （`$030e=$80 / $030d=$74 / $0308=00`），随后 `hold 960-1560 down`。
- 使这件成立的前提改动（同一 commit）：**两侧调度器都要认 `fixture`**——
  `tools/scenario_capture.lua` 的 `fixtureApplied` 由单一全局布尔改成逐条 `fixtureAppliedAt[i]`
  （旧形第一批命中后其余批次静默失效），`tools/l4_diff_run.py` 落 `fixture` 行，
  `crates/game-core/src/bin/rust_screenshot.rs` 加同 op（采集发射后、tick 前写 RAM，
  与 emu 帧回调同相位）。

## 3. emu 实录（A 级，全程真 ROM 代码）

| 帧 | 观测 |
|---|---|
| f980 | `ElevStatus=3`（已过 Idle→ScrollXToCenter）、`SamusOnElev=1`、`SamusStatus=9(sa_Elevator)`、`ScrollDir=0` |
| f1040 | `ElevY=209`（下行中） |
| f1120 | `ElevY` 回绕到 49（`ToggleObjectXSlotHi` 生效） |
| f1206 | `InArea=17($11 Norfair)`、`CurBank=2` |
| f1207 | `PaletteDataPending=6`（`ElevatorD8BF` 的 `PaletteToggle^=7` 结果） |
| f1208 | pending 被消费（NMI 的 `CheckPaletteWrite`），随后 `SelectSamusPalette` 写 2 |
| f1300 | `ElevStatus=8`、`PaletteDataPending=0` |
| f1560 | `ElevStatus=1`、`ElevType=129($81)`（换向）、`SamusStatus=0(Stand)` |

⇒ 十态机 `1→2→3→…→8→9→1` 与跨区全链在两侧同轨运行，9 个采集点像素 0px。

## 4. 该证据抓出的两处真错

### 4.1 `AreaPaletteToggle` 读错址

- asm：`MoreInit ; 07:C81D` 内 `lda AreaPaletteToggle / sta PaletteToggle`；
  `AreaPaletteToggle ; 01/02/03/04/05:95DA` ROM 逐字节 `01 00`×2、`06 00`×3
  ⇒ 区序（Br/No/Kr/To/Ri）= `[01,01,06,06,06]`。
- 旧 Rust：`crates/game-core/src/frame/mainloop.rs` 读 **WRAM `$6885`**
  （该址是道具扫描项区，`crates/game-core/src/frame/room.rs` 用作 `lo[k]`）⇒ 恒 0。
- 后果：换区 `PaletteToggle ^= 7` 后 pending 差 1（6 vs 7）⇒ 载错调色板。
- 修法：`AreaPaletteToggle` 入包（`PackKey::AreaPalToggle = 154`，`area.pal_toggle`
  5 B，crc32 `F84AC7C9`），`MoreInit` 改读 `assets::area_pal_toggle(IN_AREA)`（fail-loud）。

### 4.2 `WaitNMIPass_` 后 pending 被抢覆

- asm 序：`sta PaletteDataPending`（=6）→ `jsr WaitNMIPass_` → `jsr SelectSamusPalette`（写 2）。
  emu 里 `WaitNMIPass_` 交出本迭代余量，**NMI 先消费 pending=6**（此时 bank 已切到 Norfair
  ⇒ 取 Norfair 表 pal05），下一帧 `SelectSamusPalette` 的 2 才被后续 NMI 消费。
- 旧 Rust：`iter_wait()` 只记帧账、不交出执行权 ⇒ 同一 tick 内 pending 被覆写成 2，
  **调色板那次写入从未发生**。像素面表现：f1300 起全屏错色（12421px），
  但 RAM 状态面（`InArea/PalToggle/MainRoutine/ElevStatus`）**逐字节相同**——
  这正是「状态对拍绿、像素面红」的典型形态。
- 修法：在 `elevator.rs` 的 `WaitNMIPass_` 处就地兑现该段 NMI（`nmi::check_palette_write`），
  并在注释里声明**相位早 ≤1 tick** 的近似（本模型无「迭代中途挂起/续跑」机制）。
  回归锚：`transition_consumes_area_palette_at_wait_nmi` 断言换区后 BG 调色板
  = `bank2 idx5` 的 16 字节；变异验证：删掉该行调用 ⇒ 测试当场红。

### 4.3 `init_bank1..5` 未建模 `ScreenNmiOff` 寄存器面、也未执行 asm 尾 `jmp NMIOn`

- asm：`InitBank2 ; 07:C583` = `lda #$00 / sta GameMode / jsr ScreenNmiOff / jsr InitNorfairGFX / jmp NMIOn`
  （Bank3/4/5 同形）；`ScreenNmiOff ; 07:C45D` = 影子 `$FE &= ~(BG_ON|OBJ_ON)` → `jsr WriteAndWait`
  （等 NMI）→ 影子 `$FF &= ~VBLKNMI` → **`sta PPUCTRL` 真写**。
- 旧 Rust：只写 `NMI_STATUS=1` + `iter_wait()`（建模「等待形」），**不写影子、不写 `$2001`、
  不清 `$FF` bit7、也不做 `nmi_on`** ⇒ 换区/进场窗口的**整屏暗窗完全缺失**。
- 实证：elev 案 f1206/f1207，emu 全黑 61440/61440 px vs rust 亮屏 13115/12777 px。
- 修法：抽 `mainloop::screen_nmi_off`（全形）——`init_bank1..5` 与 `title_bridge` 三处
  旧实现**收口到一件**（同一 asm 例程三份语义不同的实现 = F 类）；每个 `init_bankN` 尾
  补 `nmi_on`。漏 `nmi_on` 的后果极端：`nmi_enabled` 永久 false ⇒ NMI 死锁、调色板面全停
  （本轮实测过一次全案 11/11 失配，是这个漏项的现场）。
- 该缺陷**不是**本轮修复引入的：旧形「恰好没坏」是因为从来没人真把 NMI 关掉。

## 5. 覆盖现状（诚实版）

**像素格覆盖的电梯状态 = {0 未注入, 1 Idle, 3 Move, 6 换区核, 8 Fade}**（10 个采集点，含 f1207 暗窗锚）。

**未覆盖的 5 个状态：2（ScrollXToCenter）/4（ScrollY）/5、7（Fade）/9（Stop）**——它们在 ROM 里
各只持续 1 帧（emu 帧级实录：`f962→2`、`f1203→4`、`f1204→5`、`f1209→7`、`f1450→9`）。

> **【2026-11-14 更正】原表述「结构性不可截」是误诊（r9 复核 P1-2 指出）。**
> 1 帧态**可以**截：`capture(N)=tick(N-1)` 是固定约定，截 N+1 即可。f1206 的 13115px 失配
> 不是约定造成的，而是「换区首帧的执行相位比 emu 晚 1 tick（`ScreenNmiOff` 落点）」这一
> **#8 相位族残差的具体实例**。当时把它写成「不可截」会永久合理化 #8。
> 因此把这些格暂不进 ratchet 的**真实理由**是：它们会暴露一个**已登记待修**的相位残差，
> 在 #8 修好之前进 ratchet 只会变成 5 个常红格；#8 修好后应把它们补成正式格（届时 f1206
> 这类边界格应当自动转绿，可作为 #8 的验收点）。
> 同时撤回「f1207 替换 f1206」的说法：f1207 两侧全黑（61440px，0 差），信息量近乎零；
> f1206 才是唯一能看见该残差的格。已改为：**保留 f1206 于 `cases/_archive/` 的探针谱系，
> 作为 #8 的复现配方**，正式用例暂不含它。

**RAM 行对齐**：`tools/sem.py case case-elev-norfair-transition` ⇒ **MATCHED 1191/1581 帧**
（15 列全同：fc/gm/mr/tr/gp/nr/t3/cb/bsp/ia/rm/st/sy/sx/hp），首分歧 `f1227 fc: oracle=B9
rust=B7` —— 即既有 **#8 FC+2 相位残差**（本轮未引入，本轮之前该案因 sem 漏传 fixture 而
整段假分歧，已修 `tools/sem.py::_derive_sched`）。

其余缺口：

1. **入口是合成的**：电梯对象由 fixture 注入，不是「走进去按 A」的自然径——自然径需要能跨过
   440 帧开场演出的用例（输入窗后移会改变既有锚，须用户裁决）。
2. **相位近似**：§4.2 的就地兑现比 emu 早 ≤1 tick；当前采集点不在该窗口内，若将来加
   f1206–f1209 之间的采集点，须先把「迭代中途挂起」机制做出来。
3. **只覆盖 Brinstar→Norfair 一条边**：Kraid/Tourian/Ridley 边与结局径（`$8F`）仍无像素证据。
4. 0px 只证「没变坏」，不证「更正后的分支被走到」。

## 证据指针

- 用例：`cases/case-elev-norfair-transition.txt`（9 格）、归档失败探针
  `cases/_archive/case-elevprobe-roomreload.txt`
- 矩阵：15 用例 / 122 格 / match 120 / diff 2（2 格=已挂账 `oracle-present-pipeline-lag`）
- emu 帧级轨迹：`analysis/traces/case-elev-norfair-transition.frames.jsonl`
- 状态对齐：`analysis/sem/case-elev-norfair-transition.{sem,rust}.jsonl`（1191/1581 同帧同值）
- 代码：`crates/game-core/src/frame/elevator.rs`、`crates/game-data/src/extract.rs`
  （`extract_area_pal_toggle` / `AREA_BANKS = [1,2,4,3,5]`）
- ROM 锚：`AreaPaletteToggle ; 01..05:95DA`、`SamusElevator ; 07:D423`、
  `ElevatorD8BF ; 07:D8A3`、`CheckPaletteWrite ; 07:C1E0`

## 复现命令（证据可复跑）

```bash
# 1) 像素面（10 格，含换区暗窗锚 f1207）
python3 tools/l4_diff_run.py run case-elev-norfair-transition

# 2) 状态面（15 列 × 1581 帧，RAM 行对齐；首分歧 f1227 fc = 既有 #8 相位残差）
python3 tools/sem.py oracle-case case-elev-norfair-transition
python3 tools/scenario_run.py run analysis/sem/case-elev-norfair-transition.sem.txt
python3 tools/sem.py case case-elev-norfair-transition     # 期望 MATCHED 1191 1581
```

> 注意：`run-all` 会清理 `analysis/traces/<case>*`（含 `.sem.frames.jsonl`）⇒ 跑过全矩阵后
> 状态面对拍须重新生成 oracle 轨迹（38s）再比，否则 sem 会报「缺 oracle 轨迹」。
