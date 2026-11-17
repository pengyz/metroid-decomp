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
