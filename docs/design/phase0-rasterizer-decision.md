# Phase 0 架构设计：光栅器裁定项（需用户裁定）

> 2026-10-22 · 用户裁定（认可（按计划推进））· 纪律强化（refs 引用必写+更新留档可追溯）·
> 四项核（对抗复核核（证伪立场（独立重导证据）））· Evidence A 级（bank:addr 引证）Confidence 95

## 0. refs 实证（OAM DMA 语义 + 8-sprite/scanline 语义）

- **NMI OAM DMA**：bank7:$C0D9（prg7_engine.asm:233 `NMI: ; 07:C0D9`）——NMI 入口先
  `sta OAMDMA`（:249 段内；OAMDMA=$4014 Sprite RAM DMA register（hardware.asm:84）），
  从 **$0200 sprite 页**（constants_ram.asm:825-827 `.enum $0200 export ;$0200 thru $02FF`）
  整页 DMA 至 PPU OAM；随后 `lda NMIStatus / bne @endIf_A`（:251-252）门控主 NMI 体
  （NMIScreenWrite/CheckPaletteWrite/CheckVRAMStructBufferWrite/WritePPUCtrl/WriteScroll/
  ReadJoyPads/SoundEngine），收尾 `sty NMIStatus`（:280）。
- **8-sprite/scanline 丢弃**：PPU 硬件行为（每扫描线超 8 sprite 丢弃余者）——refs asm 无
  「实现」（硬件语义），D4 保真纪律=原版 quirk 保留（bug-ledger 登记 8 sprites/scanline）。
- **$0200 sprite 页清屏径**：prg7_engine.asm:449/471/476（`Stores #$F0 in $0200 thru $02FF`）。

## 1. 两案对比（成本/收益/风险）

| 维度 | 案 A：单列软件 PPU 光栅器 | 案 B：L4 降级 OAM/nametable diff+白名单 |
|---|---|---|
| 内容 | 逐扫描线模拟 PPU（含 8-sprite 丢弃/优先级/抖动） | 不做像素合成；diff $0200 OAM 影子页+nametable 写集+调色板 |
| 成本 | **高**（扫描线级时序建模+CHR-RAM 图案表合成+调色板映射，工程量≈核心逻辑量级） | **低**（写集 diff 沿 L3 RAM 快照径扩展，复用既有 shim 写集） |
| 收益 | L4 像素 diff=最终判据（Phase 0 设计 §2）原样达成；8-sprite 丢弃可视化验证 | 行为等价代理验证（写集等价⇒像素等价在保真渲染前提下成立） |
| 风险 | PPU 时序建模偏差引入**伪 diff**（帧相位/NMI 时序敏感）；维护面大 | **像素面漏判**（合成器差异不可见）；8-sprite 丢弃仅间接验证（OAM 写集一致≠丢弃一致） |
| D4 保真 | 8-sprite 丢弃可直接复现验证 | 8-sprite 丢弃=白名单豁免或 OAM 层间接验证（quirk 可见性降级） |

## 2. 案内折衷（若裁定案 B）

- 白名单限字体/调色板映射（AGENTS §5 唯一允许例外类）——8-sprite 丢弃**不进白名单**（D4 保真
  禁修），以 OAM 写集+帧预算记账间接守恒（a3-028 洞类教训：捕获窗覆盖主循环写段）。
- 像素面终检保留一次性验收径：**Mesen2 framebuffer 截图**（L4 调研工单①产物（256×240 PNG））
  与 Rust 渲染截图对拍——降级不豁免最终像素判据，只降**常态化**像素 diff 频度。

## 3. 判定与请裁定

- **建议**：**案 B 为主 + 像素面一次性终检**（成本可控；像素最终判据不豁免仅降频；8-sprite
  quirk 走 OAM 写集间接守恒+bug-ledger 保真登记）。
- **需用户裁定项**：①案 A/案 B 择一（或 A 后置为 Stage B 项）②8-sprite 丢弃的 L4 验证形态
  （直接像素 vs OAM 间接）③像素终检频度（每簇一次 vs 里程碑一次）。

## 4. refs 引证汇总（bank:addr）

prg7_engine.asm:233（NMI 07:C0D9）/:249-280 段（OAMDMA+NMIGate+NMIStatus 门控）/
:449/:471/:476（$0200 sprite 页清屏）；hardware.asm:84（OAMDMA=$4014）；
constants_ram.asm:825-827（$0200-$02FF sprite 页）。

Evidence: 上述 bank:addr 引证（m1disasm SRC 实测行号）。
Confidence: 95
