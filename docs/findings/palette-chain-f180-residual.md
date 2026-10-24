# findings：boot-title 残差（f180 闪光链 + 文本色链）根因证据链与工作单

> 【2026-10-24 结案】**七金面全零像素差达成**（f30/60/120/180/240/299/300 = 0/61440）。
> 两个真根（均经 A 级证据定谳后修复）：
> ① **BG 属性表象限位序互换**（`render_frame_bg`/`bg_pix_grid` 的 q 位序 = 低位 y/高位 x，
>    标准应为低位 x/高位 y）——非对称 attr 字节下选错组 = 58px 文本链；
>    例：tile(12,19) 块内偏移(0,3) → 标准 q=2（attr 0xAF 位段=组2）而互换式得 q=1（组3）。
>    调色板链全排除后由 `$1C` 写点探针（`oracle/scenarios/pal1c-boot.txt`）反证：
>    golden 侧 8 次渐显写与我方逐字节同、烘焙与 ROM 逐字节同 ⇒ 唯余属性读取面。
> ② **渲染采样相位**（tick≡oracle 帧+2——启动段 2 tick 不占帧号）：金面帧 F 捕于 tick F-2。
>    f180（渐显中途、16 帧节拍敏感帧）曾 9970px = 采样点早 2 tick。
> 复核测试：`attr_quadrant_bit_order_standard`（非对称 attr 四象限钉死）。

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

## 4. 【2026-10-24 二轮更正】矛盾收窄到单事件

> 【2026-10-24 更正】一轮"Timer3 单位复核/渐显表序"两个假设**均被证伪**：
> PalSelectTbl ROM 直读（`rom/Metroid (USA).nes` prg[0x0A9A] = `02..0C,0C FF`）与烘焙
> **逐字节一致**；调用记账（`analysis/_diag_palidx.txt` 实录）`idx=5,6,7,8,9,10,11,11`
> =8 次调用忠实于 asm（表内 `0C 0C` 重复项=末两次同写 ptr[11] 自证）；`$C1` 轨迹双侧对齐
> （4→0C=8 步）⇒ **双方渐显写序完全相同**。

**残差=单一缺失事件**：golden 在 f180-240 间（TR∈{3,4} 窗）有一笔 **Palette0C 族写**
（`$3F0E←0x31`——文本行 536px 双侧轨迹：f120=02/f180=11/f240=31（golden）vs 我方 11 后停止；
金面逐帧反推=`analysis/_slotprobe.py`）。写者身份未定——已穷举候选：

| 候选 | 排除依据 |
|---|---|
| METROIDFadeIn/LoadPalData | 双侧调用序相同（对账闭合）✓排除 |
| FlashEffect(0x05)/MessageFadeIn(0x0D)/StartContinue(0x15/1B) | TR=4 未达（行对齐 $1F 双侧=4）|
| FlashIntroScreen（$8AA7） | 表=[0x11/0x01]=Palette10/00 族，非 Palette0C ✓排除 |
| StarPaletteSwitch | 写 $3F19/$3F1D 精灵槽 ✓排除 |

⇒ **定位判据**：写者=某条 **RAM 状态全对齐却产生 PPU 写差异**的路径（丢写径/未端口/
解码错位）。下一步（精确工作单）：Mesen Lua 对 golden 侧 **$1C(PaletteDataPending) 写点
+ $3F0E 写值**做 NMI 级 watch（把写者当场抓获）；或逐臂核 `PENDING_PAL` 事件到
`palette_write_pending` 的消费链（重点：shim 的 tag→addr 映射有无 `0x0D` 丢写、
`merge_writes` 同址覆写）。

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
