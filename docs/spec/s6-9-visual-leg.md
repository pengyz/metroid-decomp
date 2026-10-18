# spec：S6-9 视觉腿实施规格（game-core 渲染输出面——L4 第二腿设计输入）

> 状态：**规格细化**（2026-10-18）。上下文=task-31 visual（diff_visual 工具面）
> ×task-42 rendering-chain spec（链契约）×S6-6/7 RenderState（PPU 地址空间建模
> ——porter 已落）。本节=帧缓冲产出面/像素对拍接口/白名单面衔接三面。

## 1. 帧缓冲产出面（RenderState→帧缓冲契约）

```
RenderState（S6-6/7：PPUADDR/CHR/名表建模）
  → 帧渲染（每帧一次）→ FrameBuffer:
     尺寸 256×240（NES NTSC 可见面）
     像素=调色板索引（64 色 NES 调色板）→ RGB 映射（映射表=白名单面，§3）
  → PNG 编码（8 位 RGB 非隔行——diff_visual 接受面，§2）
  → 落盘 shots/<场景>_f<帧>_<tag>.png（第二腿复采面——同金面命名契约）
```
- **确定性要求**：同输入（bundle/场景）→同帧缓冲（逐字节）——对拍前提；
- 渲染时机=spin-exit 缝采样后（帧完整态）——与 oracle 截图同相位（对齐契约
  l3-alignment-contract §1 层 2 相位语义适用）；
- OAM 面（精灵）：S6-4 落后接——当前腿可先 BG-only（金面选择含精灵帧时白名单
  处理或延后）。

## 2. 像素对拍接口（diff_visual 双腿衔接）

- **PNG 契约**（task-31 工具面硬约束）：8 位 RGB/RGBA **非隔行** IHDR（其余显式拒收
  ——纯 stdlib 解码器）；编码=filter 0+IDAT zlib；
- 命名契约：`<场景>_f<帧:05d>_<tag>.png`（金面 oracle/screens/×复采 shots/ 同名
  对照——第二腿=game-core 产 shots/ 同名件或独立目录+对照扩展）；
- 对拍矩阵（双腿启用后）：
  | 对 | 裁定面 |
  |---|---|
  | 金面×oracle 复采 | L4 现有（7→50 帧） |
  | 金面×game-core 复采 | S6-9 第二腿（同机制） |
  | oracle×game-core 直对 | 可选第三面（渲染等价直接证） |
- selftest 四控（正负对照）覆盖两腿（同帧全同/合成差异打回——判别力基线原则）。

## 3. 白名单面衔接（gates/visual_whitelist.json 唯一例外清单）

- 条目形（现契约）：`{file: glob, rect: [x,y,w,h], reason, approved_by}`——
  **白名单外 1 像素即 FAIL**（零容差）；
- 双腿共享同一清单（file glob 天然双腿通配——同名金面对）；
- 预期例外面（AGENTS §5：字体/调色板映射）：
  ①调色板映射差异（NES 64 色→RGB 的映射表选择——RenderState 输出 RGB 时
  与 Mesen 截图映射可能不同系——首例差异按 rect+reason 登记）；
  ②字体渲染（文字抗锯齿无——NES 逐像素应全同，差异=渲染 bug 非白名单）；
  ③精灵相位（OAM 160B 上传相位差——l3-alignment-contract §1 层 2 同源面）；
- 变更纪律：白名单条目新增=用户确认+记录（AGENTS §5）——**白名单不是修 bug 的
  出口**（渲染不一致=先修渲染）。

## 4. S6-9 验收判据

1. game-core 渲染腿产 PNG 全过 diff_visual 格式检（8 位 RGB 非隔行）；
2. 金面×game-core 对拍跑通（同机制）——首批差异分类（白名单/渲染 bug）；
3. selftest 四控两腿全过；
4. 确定性：同 bundle 两次渲染逐字节同。
