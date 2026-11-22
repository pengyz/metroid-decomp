# 名表镜像不是常量：`MirrorCntrl($FA)` bit3 逐房间切换（2026-11-22）

## 0. 结论摘要

`$2000-$2FFF` 四页在真实硬件上并非固定排布。游戏在**换房/换区/滚动向切换**时改写
MMC1 的 CIRAM 镜像位，得到两种不同排布：

| 形态 | `$FA` bit3 | 等价关系 | 典型场景 | 两页的用途 |
|---|---|---|---|---|
| 竖镜（vertical） | 0 | `$2000≡$2800`、`$2400≡$2C00` | 横滚房（Brinstar 主流） | 左右两屏（按 X 选） |
| 横镜（horizontal） | 1 | `$2000≡$2400`、`$2800≡$2C00` | 竖滚房（Norfair 竖井/电梯房） | 上下两屏（按 Y 选） |

rust 旧实现把排布当常量（恒竖镜），且像素取面只看 `ppuctrl` bit0 与 `wx`——这两条合起来
使得**竖滚房的第二竖屏永远取不到**，表现为 Norfair 全房大面失配（整屏级、随时间扩大）。

## 1. 证据（A 级）

### 1.1 硬件观测量（emu PPU 总线）

`tools/scenario_capture.lua` 的 `dumpPpu` 用 `emu.read a, 9` 读 **PPU 总线** `$2000-$2FFF`，
因此四页是"镜像之后"的视图。同一采集 tag（`f60`）的四页 `$FF` 计数与互等性：

| 场景 | 四页 `$FF` 计数 | 互等 | 判定 |
|---|---|---|---|
| Brinstar `a0_r21` | `[646, 796, 646, 796]` | `p0==p2`、`p0!=p1` | 竖镜 |
| Norfair `a1_r00` | `[646, 646, 882, 882]` | `p0==p1`、`p2==p3`、`p0!=p2` | 横镜 |

修前 rust 侧两例都是 `[A,B,A,B]`（恒竖镜），故 `a1_r00` 的四页计数为
`[646, 882, 646, 882]`——与 emu 完全不同的排布。**这不是 dump 记账错**：同一段代码在
Brinstar 上与 emu 逐字节相同。

### 1.2 asm 定谳

- `refs/m1disasm/SRC/constants_ram.asm:811-813`：
  `MirrorCntrl db ;$FA ;If bit 3 is set, PPU set to horizontal mirroring --> else if bit 3 is
  clear, PPU is set to vertical mirroring. No other bits seem to matter.`
- `refs/m1disasm/SRC/prg7_engine.asm:1250-1290`：
  `PrepVertMirror ; 07:C4B2`（`lda #$47` 入 A）→ `SetPPUMirror ; 07:C4B6`
  （`lsr lsr lsr` 取 bit3 → `and #$01` → `sta $00` → `lda MMC1CTRL_ZP` / `and #$FE` / `ora $00`
  / `sta MMC1CTRL_ZP` → 串行写 5 次 `MMC1CTRL`）；`PrepPPUMirror ; 07:C4D9`
  （`lda MirrorCntrl` / `jmp SetPPUMirror`）——即"切换镜像"的唯一出口。
- `ToggleScroll ; 07:E252` 在切换滚动向的同一拍做 `MirrorCntrl ^= $08`。
- 三处显式切换点：`prg7_engine.asm:2001`（电梯下行，`ldy #$2F` 落横镜）、
  `:4866`（注释原文 "set mirroring to horizontal, to place the nametables in a vertical
  configuration"）、`:7059`（`lda #$47` 落竖镜）。

### 1.3 rust 侧缺口（修前）

- `crates/game-core/src/render/mod.rs` 的 `nt_offset` 是**固定**映射
  `(nt_plus1-1 & 1) * 0x400`（= 竖镜形），无入参可表达另一形；`write_ppudata` 的
  `& 0x7FF` 同病。
- `crates/game-core/src/render/mod.rs` 的 `bg_lookup` 只按 X 选页
  `(nt0 + (wx/256)*0x400) & 0x7FF`，**完全丢弃 `ppuctrl` bit1 与 `wy`**。
- `grep MIRROR_CNTRL crates/game-core/src/render/` 为空——渲染层从未读过 `$FA`。
  写入面却一直存在：`world/update.rs:75,83,92`、`world/elevator.rs:159,345`，
  测试 `crates/game-core/tests/samus_init_scrolldir.rs` 还断言过 `$FA` 的值（`0x2F`/`0x27`）。
- 竖滚的数据流送明确按两页写：`crates/game-core/src/world/camera/scroll.rs:142-156`
  的 `stream_row_kick` 目标地址是 `if y == 0 { 0x2000 } else { 0x2C00 } + off`
  （`y = ($00FF ^ ScrollDir) & 1`）——**渲染层却从不显示 `$2C00` 这一页**。

## 2. 修复

| 位置 | 改动 |
|---|---|
| `state.rs` | `RenderState.mirror: bool`（默认 false=竖镜，保持既有横滚房行为） |
| `render/mod.rs` | `nt_offset_with(nt_plus1, mirror)` 两形映射；`clear_name_table` 走它 |
| `render/mod.rs` | `bg_lookup` 按排布选页：横滚 `wx/256` 翻 bit0、竖滚 `wy/240` 翻 bit1；基页取 `ppuctrl & 0x03` |
| `render/ppu.rs` | `write_ppudata` 名表径按 `mirror` 映射逻辑页（页内偏移 `rel & 0x3FF` 保留） |
| `frame/memory.rs` | `sync_mirroring()`：`$FA` bit3 → `render.mirror` |
| `frame/engine.rs` | 帧门排空拍调用 `sync_mirroring()` |
| `bin/rust_screenshot.rs` | ppu.hex 四页按同一映射展开（竖镜 `[A,B,A,B]`、横镜 `[A,A,B,B]`） |

## 3. 验证

- `cargo test --workspace` 全绿（`render` 60 passed、lib 947 passed；总数 1056）。
- 锚测：`nt_offset_with_has_two_mirroring_forms`（两形逐项）、
  `clear_name_table_respects_mirror_bit`（哨兵 0xAA 证明真的读 `mirror`，且不碰对面页）。
- L4 矩阵 `run-all`：21 案 / 172 格 **match 166 · diff 6 · blocked 0**，与基线逐格一致
  （6 格为已挂账项，见下）。
- Norfair 单房（`room_sweep --areas 1 --rooms 0,1`）：**room00 0px / room01 0px**
  （修前 `f60=8757 / 6397`、`f120=17401 / 13323`）。
- `gates/cli.py check` PASS=23。

## 4. 未解决 / 边界

- 6 格矩阵残差与本次无关：`case-mechanism-input-conflict` f673=84 / f674=84 / f676=303
  （显示集卡=NMI 停⇒OAM DMA 时刻）、`case-password-justin` f700=23 / f1100=42
  （挂账 `oracle-present-pipeline-lag`）、`case-scrollseam-probe` f1100=266。
- `sync_mirroring()` 只在帧门排空点同步 ⇒ **同一 tick 内**若先切镜像再写名表，写入会按旧
  排布落页（差一拍）。既有场景未观察到影响（矩阵逐格不变），登记为潜在残差。
- Norfair 全房扫描进行中：room 04 起仍有失配（`f60=11280` 级），属**另一个**机制
  （非镜像），待独立解剖。

## 5. 读法纪律（可复用）

1. **emu `dumpPpu` 的四页是镜像后视图**，不是原始 CIRAM；判断"排布对不对"要先问
   "四个页里谁和谁相等"。
2. 一个"常量式实现"若同时被两处消费者读取（写入面 + 显示面），必须查**显示面**是否也读它；
   本仓 `$FA` 有写入有断言，唯独渲染层没读——这类"半接线"最容易伪装成"渲染 bug 不可解"。
3. 竖滚与横滚是**同一对物理页的两种用法**（不是两套缓冲）：横滚按 X 翻页、竖滚按 Y 翻页。
   只实现一种时，另一种的表现是"第二屏永远空白"，而不是"图像错位"。
