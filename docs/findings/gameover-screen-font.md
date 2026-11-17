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
