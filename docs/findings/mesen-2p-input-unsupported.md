# Mesen2 腿无法驱动端口 2（2P 键被 OR 进端口 1）——裁定链污染已定位并处置

【2026-11-23】100% any% 影片（`movies/eien86-6725M.inputlog`）从数据帧 **3265** 起有 242 帧
`p2 != 0`（`0x08` = Up）。此前的裁定腿（Mesen2）把 2P 键 **OR 进端口 1**，导致自 f3266 起的
对比全部被污染：f3268 出现「rust 起跳（`speedY=FC`，`SetSamusJump` 初速）/ emu 不起跳
（`speedY=00`、`y` 恒 `C1`）」的伪分歧，随后 `$005A` 载房请求时机不同（rust f3281 提前进入
`F1/F3/FF`、emu f3276 才请求 room `0x14`），f3279 起全屏崩塌。

## A 级证据（受控实验，同一 ROM、同一 Mesen 构建）

| 实验 | 输入 | emu `$0014 Joy1Status` | emu `$0015 Joy2Status` | rust |
|---|---|---|---|---|
| 一 | `p1=0`，f200–215 `p2=0x08` | **0x08** | 00 | 00 / 08 |
| 二 | `p1=0x02`，f200–215 `p2=0x08` | **0x0A**（=02\|08） | 00 | 02 / 08 |
| 三 | 原影片 f3266（`p1=82`，`p2=08`） | **0x8A** | 00 | 82 |
| 四 | 修好后（实验二重跑） | **0x02** | 00 | 02 / 08 |

- ROM 侧地址确认：`refs/m1disasm/SRC/constants_ram.asm:487-490` `Joy1Status=$14` / `Joy2Status=$15`；
  `prg7_engine.asm:572-596` `ReadOnePad` 用 `lda JOY1,x`（`JOY1=$4016`，x=1 ⇒ `$4017`）。
- `tools/scenario_capture.lua` 原形：`:1102 pcall(emu.setInput, btn, 0)` 与
  `:1113 pcall(emu.setInput, btn2, 1)`。
- Mesen2 内置文档（二进制内嵌 JSON）原文：「Sets the input state for the specified port.
  Buttons enabled or disabled via setInput will keep their state until the next inputPolled
  event.」+ 示例 `emu.setInput(input, 0)`，参数表 `{input, port, subPort}`。

## 已排除的三种「正确写法」

1. `emu.setInput(btn2, 1)` —— 落端口 1（`getInput(0)` 返回 `up+left`、`getInput(1)` 恒空）。
2. `emu.setInput(btn2, 2)` —— 同上。
3. `emu.setInput(btn2, 1, 0)`（显式 subPort）—— 同上。
4. 把隔离 HOME 的 `settings.json` 改 `Nes.Port2.Type = "NesController"` +
   `Nes.AutoConfigureInput = false`（原值 `None` / `true`）—— 行为不变，`$4017` 仍恒 0。

⇒ 该 Mesen 构建的 Lua `setInput` 端口参数不可用，裁定腿**无法驱动 2P**。

## 处置（`tools/scenario_capture.lua`）

- **不喂端口 2**：`p2` 列只触发一次 `WARN 2P` 日志；若用例声明 `require_joy2` 则直接 `error`（fail-loud）。
- 为什么这样不降低保真：ROM 对 `Joy2Status` 的**唯一**消费点是 PauseMode 的 2P `A+UP` 退出
  （`prg7_engine.asm:2141` `lda Joy2Status / and #BUTTON_A|BUTTON_UP / eor`），本影片的 242 帧
  只按了 `Up`（`0x08`，不含 `A`）⇒ 真机上该组合不成立、游戏行为与「2P 完全不按」等价。
- rust 侧仍按 inputlog 的 `p2` 列写入 `$0015`（与真机一致），只是 Mesen 腿读不到——两侧在
  「游戏实际行为」上等价，`$0015` 本身不进 L4 判据。

## 效果

- 修后 4000 帧裁定（同一影片，`p2` 保留在 inputlog 中）：`f3268` **0 px**（原伪分歧消失），
  新首分歧候选 `f2000=120 px`（瞬时，f3000 已回到 0 px）、`f3280=419`、`f3900=433`、`f4000=8611`。
- 结论：**f3268 那次分歧不是移植缺陷**；100% any% 影片自 f3266 起的对比在修复前全部无效。

## 纪律

- 裁定腿的输入通道本身也要被验证：任何「2P 参与」的用例，先跑受控实验确认端口分流，
  再拿来做判据。
- `p2` 置零只能当**诊断**（改输入即改路线），不能当判据。
