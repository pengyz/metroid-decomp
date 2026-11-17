# GameOver 屏：`InitPasswordFontGFX` 缺失 ⇒ "GAME OVER" 画成噪点

## 现象（100% 录像 f4023 起恒定 167px）

像素 bbox `x96–159 / y96–103` = 8 字×8px 的居中文字带。状态面两侧**完全相同**
（`GameMode=1`、`MainRoutine=4`、`TitleRoutine=$19`、`$FE=0x1E`、`status=8`、`animDelay=0`），
nametable `$218C` 两侧也**逐字节相同**（`10 0A 16 0E FF 18 1F 0E 1B`），
但 CHR 从 `0xC00` 起大段不同（1828 字节差异）⇒ tile 号对、字形错。

## 根因（A 级）

`GameOver ; 00:939E`（`refs/m1disasm/SRC/prg0_title.asm:2861`）三步：

```
jsr ClearAll                       ; 清屏/清精灵/清名表
ldx/ldy @VRAMStruct → PreparePPUProcess_   ; "GAME OVER" → $218C
jsr InitPasswordFontGFX            ; ← 缺这一步
jsr NMIOn / Timer3←$10 / TitleRoutine←DisplayPassword / jmp ScreenOn
```

`InitPasswordFontGFX ; 07:C6D6`（`prg7_engine.asm:1655`）只有两件：
`ldy #GFX_Font_Complete / jsr LoadGFX` → `ldy #GFX_Solid_1FC0 / jmp LoadGFX`。

rust 侧 `crates/game-core/src/title/bridge.rs` 的 shell 分派对
`ShellCall::InitPasswordFontGfx` 落到通配臂 `_ => {}`（注释留"深件随批"）⇒ 字体从未装载，
CHR 里遗留的是房间/上一屏图形，于是同一批 tile 号被画成噪点。

## 修法

`InitPasswordFontGfx => { load_gfx(FONT_COMPLETE); load_gfx(SOLID_1FC0); }`
（`crates/game-core/src/render/gfx.rs::load_gfx` 已按 `LoadGFX ; 07:C7AB` 的描述子径端口）。

## 验证

- `case-movie-100p-b26`（f4018–f4026）：f4019–f4026 **8/8 全 0px**（修前 f4023 起 167px）。
- 矩阵 15 案/132 格 match 126 · diff 6 · blocked 0 逐格无回退；测试 702 passed / 0 failed。

## 遗留（如实记账）

f4018 单独一帧仍有 12404px：两侧 `$FE` 同为 `0x06`（BG/OBJ 关），emu 截图全黑而 rust 仍输出上一屏
⇒ 属**关屏那一帧的采样相位**（rust 的 PNG 取自 `on_spin_exit` 缝，早于本 tick 的 mask 落寄存器），
不是游戏逻辑差；f4019 起立刻回到 0px。待单独定谳，未当绿。

## 五、【2026-11-17 补】f4018 关屏帧采样相位：两次尝试均被矩阵否决（负结果留档）

**现象**：`case-movie-100p-b26` 的 f4018 单帧 12404px；两侧 `$FE`/`$00FF`/`GameMode`/`MainRoutine`/`TitleRoutine`
全同，emu 截图全黑（61440），rust 仍输出上一屏。

**尝试 1（否决）**：`render_capture` 的 PPUMASK 一律取收口活体。
- 结果：f4018 **0px**，但矩阵 126/6 → **125/7**：`case-password-justin` f1190 submit 0 → **2063px**。

**尝试 2（否决）**：仅当「显示位被清掉」时取活体（`(seam & $18) & !live != 0`）。
- 结果：f4018 **0px**，矩阵仍 **125/7**，`case-password-justin` f1190 依旧 2063px。

**结论（重要）**：f4018 与 f1190 在**我们可见的数据里签名相同**（seam 亮 / 活体暗），
但 emu 地面真值相反（4018 黑、1190 亮）⇒ 现有相位模型缺一个区分维度，
**不是「changing direction」能解决的**。已 `git checkout` 还原，矩阵回到 126/6/0。

**下一步（未做）**：在同一 tick 内记录 `$2001` 写入相对「帧渲染窗口关闭」的**先后**——
即给 `write_io(0x2001)` 加调用点标记（`ScreenOff` 直写 vs `apply_deferred_screen_on` 队列兑现），
并用 1190 / 4018 两帧对照，确认是「写入发生在 NMI 前后」而非「mask 值差」。
在拿到这个区分之前，不得再用「活体 mask」一刀切（已两次被矩阵否决）。

## 六、取证小件：`$2001` 写入面清点（2026-11-17）

### 6.1 手段
`Manager` 加诊断旗 `trace_mask`（`--trace mask` 启动参数，非逻辑层钩子），在 `write_io(0x2001)`
逐次打印 `seq`（累计写入序号）/`fc`（`$002D`）/新值/`Location::caller()`；采集点再打印
`seam`（缝快照 mask）/`live`（收口活体）/`fe`（影子）。取完即回滚（工作树 clean）。

### 6.2 结果：真寄存器只有三个写入点

| 写入者 | 语义 | 频率 |
|---|---|---|
| `crates/game-core/src/frame/nmi.rs:343` | `WritePPUCtrl ; 07:C44D` 的 `sta PPUMASK` | 每帧一次（主路径） |
| `crates/game-core/src/world/room_load.rs:117` | `screen_nmi_off`（`ScreenNmiOff ; 07:C45D` 的同 tick 近似） | 换区/进场/密码进场 |
| `crates/game-core/src/frame/tick.rs:395` | `prepare_game_over`（`ScreenOff` 的同 tick 近似） | 死亡→GameOver |

后两处都是**主线程内的「NMI 落点近似」**（注释已写明：asm 只写影子 `$FE`，真寄存器由下一个
NMI 的 `WritePPUCtrl` 落；本层原子执行，故此刻即写真寄存器）。

### 6.3 两个样本的逐次序列（关屏那一次）

```
# case-password-justin（emu f1190 = 亮）
seq=1186 fc=153 v=0x1e  nmi.rs:343        # 上一帧的 NMI 落影
seq=1187 fc=154 v=0x1e  nmi.rs:343
seq=1188 fc=154 v=0x06  room_load.rs:117  # ← 关屏（主线程近似）
seq=1189 fc=155 v=0x06  nmi.rs:343        # 下一个 NMI 再写同值
seq=1190 fc=155 v=0x06  room_load.rs:117

# case-movie-100p-b26（emu f4018 = 黑）
seq=4002 fc=151 v=0x1e  nmi.rs:343
seq=4003 fc=151 v=0x06  tick.rs:395       # ← 关屏（主线程近似）
seq=4004 fc=152 v=0x06  nmi.rs:343        # 下一个 NMI 再写同值
```

### 6.4 定谳：这两个样本在我们的模型里**不可区分**

采集点打印：

```
case-password-justin  cap frame=1190 seam=1e live=06 fe=06
case-movie-100p-b26   cap frame=4018 seam=1e live=06 fe=06
```

三个维度全同：①写入者都是「主线程近似 NMI 落点」②关屏后都有一次 NMI 重写同值
③`seam/live/fe` 三元组一致。而 emu 真值相反（1190 亮、4018 黑）。
⇒ **判别信息不在 rust 侧的可见状态里**，必须从 emu 侧取（帧末 `$2001` 真值），
或改模型本身（见 6.5）。

### 6.5 下一步的两个候选（都需先证后改）

1. **去掉两处「同 tick 近似」**，让 `$2001` 只由 NMI 的 `WritePPUCtrl` 写（与 asm 同构）。
   代价：暗窗建模（`deferred_screen_on`、elevprobe f1206/f1207 的 61440 px 证据）可能回退，
   须逐格验证「换区暗窗 / 密码进场 / GameOver 屏」三类转场。
2. **给 emu 腿补帧末 `$2001` 采样**（`analysis/traces` 的 `ppu.hex` 尾 3 字节当前恒 0），
   用真值判定「该帧关屏是否已生效」，再据此定采样相位。

约束（沿用第五节）：在候选实现前后都不得用「活体 mask 一刀切」——已两次被矩阵否决。
