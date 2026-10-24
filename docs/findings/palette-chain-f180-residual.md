# findings：boot-title 残差（f180 闪光链 + 文本色链）根因证据链与工作单

> 状态：**残余定性完成**（2026-10-24，r15 取证）。七金面 3 帧 0px（f30/f60/f120）、
> f240/f299/f300 残 58px（文本色链）、f180 残 9970px（闪光链相位）。本文=残差的
> **逐层证据链 + 精确工作单**；修复实施待 LoadPalData 考证件（见 §4）。

## 1. 残差现象（A 级：逐像素裁判）

| 金面 | 差异 | 形态 |
|---|---|---|
| f30/f60/f120 | **0/61440** | 完美（含完整标题屏） |
| f180 | 9970 (16.23%) | 3 组色链对（`155fd9→1412a7` ×6002 / `c0dfff→155fd9` ×2105 / `002a88→1412a7` ×1863） |
| f240/f299/f300 | 58 | 单色链对 `155fd9→c0dfff`（y144-159 文本行"PUSH START BUTTON"） |

复现：`python analysis/_pixdiff.py boot-title_f00299_title.png`（工具留存 analysis/）。

## 2. RAM 面已全对齐（排除法）

L3 双腿对齐窗=**shift=2**（O fN ≡ G f(N-2)——启动段 2 tick 滞后实证）；对齐窗内
**恒差址仅 77 个**：`$18/$19`（RetrigDelay 输入连发计时——每帧 -1 恒差 1，无害）、
`$E6/$E7/$EA/$EC/$ED`（SoundEngine——F1 立项）、`$01EB-$01FF`（栈页尾）、`$0600-$068D`（音乐块）。
**调色板链 RAM（$C1/$C5/$C9/$1C/缓冲 $07A1+）全对齐** ⇒ 残差必在 **PPU 侧写径**。
复现：`python analysis/_legcmp.py`。

## 3. 槽位定位（A 级：逐帧变迁记账）

文本色差=BG 调色板槽 **`$3F0E`**（组3 色2）：我方 0x11（NES_PALETTE=155fd9）/
golden 0x21（c0dfff）。逐帧变迁（我方）：
`f1=0F → f100=01 → f116=02 → f132=01 → f148=12 → f164=02 → f180=11`（16 帧节拍=渐显链
`FrameCount&0xF` 闸）后**停止**。

PALETTE_BLOBS（nmi.rs）解码模拟对表（`python analysis/_paltrace.py`）：
我方写序=**ptr[6..11]**（METROIDFadeIn 渐显链 6 步）；golden 终值 0x21=**ptr[4]**——
渐显链序列中不存在 ptr[4] ⇒ golden 在渐显后有**另一条链覆写**（FlashEffect 的
`PaletteDataIndex&3 → PAL_SELECT_TABLE[0..3] → ptr[1..4]` 族，终落 ptr[4] ✓）。
f180 golden=0x02（ptr[7] 族）⇒ golden 渐显比我方**晚相位**，随后闪光链在 f180-240 间覆写至 ptr[4]。

## 4. 矛盾点=精确工作单（asm 边界）

asm 真值（prg0_title.asm:252-276）：
- `METROIDFadeIn`：Timer3≠0 门（80 帧）→ `&0F` 闸（16 帧）→ `jsr LoadPalData`（**Z=耗尽**）→
  完成⇒ `Timer3←$20`（注释明言"Delays flash by 320 frames"）→ inc → `LoadFlashTimer`；
- `LoadFlashTimer`：Timer3≠0 门（320 帧）→ `Timer3←$08`（80 帧）→ inc → `FlashEffect`。

**矛盾**：按 asm，闪光链最早 f≈500 才跑（boot-title 窗口=300 帧内不应出现）；但 golden
f240 已现 ptr[4]（闪光族）值。⇒ 考证靶点（按优先序）：
1. **LoadPalData（asm 本体）**：表序/起始索引（PaletteDataIndex=4 起 vs 表首起）、
   Z 语义（何时判"耗尽"）、写面（写哪些槽）——我方渐显写 ptr[6..11] 起点=4；golden 起点
   疑为 0（渐显表序错位）；
2. **"320 帧"延迟的真值核**：$20 的单位（10 帧/单位 vs 1 帧）——若单位=1 帧，
   f180+32=f212 闪光开跑 ✓ 与 golden f240 现 ptr[4] 相容！（constants_ram:496
   「Decremented every 10 frames after set」是 Timer3 通则，但本路径注释/单位需复核）
3. 我方 0x04/0x05 臂（title/mod.rs EffectDriver）与 asm FlashEffect（:226）逐步对表。

**主假设（待证）**：golden 时间线=渐显（f80-180）→ 32 单位延迟（若=32 帧 ⇒ f212）→
闪光 ptr[1..4] 循环（f212-299，`&3`+LoadPalData）→ 终值 ptr[4] ✓✓ 与全部观测相容；
我方 0x04 的 Timer3 门按 320 帧实现（Timer3 单位=10 帧通则）⇒ 闪光永不开跑。

## 5. 已收口件（本战役前几轮，均有独立证据）

- 图案表选 PPUCTRL bit4 / 精灵面（Y+1、8/行 quirk、优先级 pix 真值）/ C1BC 星槽范围 /
  迭代序门（四件冻结）/ $C3 IntroStarOffset 双自增（②③互斥+唤醒合并+00 单迭代）——
  见 commits 8edc3bed/26d8ba99 与 rounds/r14 复核账。

## 6. 证据指针

- 逐像素裁判：analysis/_pixdiff.py、_pixsample.py、_shiftprobe.py（邻域采样实证：f180 仅
  shift-2=58px——采样相位敏感；f240/299 全偏移恒 58px——真值差）
- 双腿对齐：analysis/_legcmp.py（shift=2 窗 77 恒差址全清单）
- 槽位追踪：analysis/_paltrace.py（PALETTE_BLOBS 19 项 ptr 全解码→$3F0E 写者表）
- asm：prg0_title.asm:211-276（FadeInDelay/FlashEffect/METROIDFadeIn/LoadFlashTimer）、
  prg7_engine.asm:507/654（CheckPaletteWrite/UpdateTimer）
