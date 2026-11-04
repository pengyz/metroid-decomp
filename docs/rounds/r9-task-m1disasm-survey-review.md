# R9 对抗复核 verdict——m1disasm 整体架构 survey（docs/spec/m1disasm-architecture.md）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-04
- **verdict：需修订**（§1.4/§6.2/§7.3/抽样/分级纪律五面中四面 PASS；**§7.1 热图数字
  不可复现=违 §0 机械复现自标口径**，须修 §7.1/§7 相关段后可用）

## 1. §1.4 NMI/窗口 bank 契约 —— PASS（B 级推断+栅栏合规）

- A 级事实逐项复现：NMI（bank7）`jsr SoundEngine`（prg7_engine.asm:272，注释
  「($B3B4)」）↔ sound_engine.asm:384 `SoundEngine: ; $B3B4`（窗口区 ✓）；
  include 结构=**prg0/1/2/3/4/5 六份副本**（逐文件 grep 在案）、prg6/bank7 无
  include ✓；bank6=GFX_* 纯 .incbin CHR（§1.3）✓。
- 推论「NMI 期间窗口≠bank6」逻辑必然（窗口=bank6 时 jsr $B3B4=跳进 CHR 数据）
  ——CopyGFXBlock 调用点（:1778-1786）仅见 BankSwitch+PPUCTRL 设置、NMI 关闭
  处置未见明文 ⇒ survey 标【假设】待 Mesen2 trace——**栅栏恰当**（A 级事实与
  B 级推断分离纪律合规）。

## 2. §6.2 六陷阱抽核 —— PASS（三陷阱属实验证）

- 「门码在 areas_common 实为 world 域」：属 实——SamusEnterDoor($8B13)/
  UpdateAllDoors($8B79)/DrawDoor($8BB1) 物理位于区域共享文件、消费方=bank7
  引擎 ✓。
- 「prg0 四域混居」：属实——title/intro/密码/结局演出/世界地图+曲+engine 尾
  同文件（§1.2 row0 与源一致）✓。
- 「ZP Temp 复用组」：属实——constants_ram Temp00_* 同址多义族（Temp00_RoomRAMPtr/
  Temp02_AttribPtr/Temp00_Diff… 本复核历史轮多见）✓。
- 「{AREA}+`.if BANK`」：sound_engine.asm:2256 族声明 A 级 ✓（未逐条枚举已栅栏）。

## 3. §7.3 覆盖空白 —— 抽样两点 PASS（但依赖 §7.1 的量化底座见 §6）

- 「五区 rooms/structures/metatiles bank1-5 $A000-$B1FF 引证≈0」：机械复现——
  `grep bank[1-5]:A###` 全 findings **0 命中** ✓。
- 「enemies/*.asm 无址标注」：crawler.asm `grep "07:"`=**0**（195 行全无
  bank:addr 注址）✓——「敌 AI 最大无址标注盲区」定性成立。

## 4. 抽样（文件+导航表） —— PASS（10/10 全对）

- 文件行数 6 例全对：prg0_title **5326**（doc 5,326）/prg7_engine **12930**
  （12,930）/areas_common **1750**（1,750）/common_sprite_data **1635**
  （1,635）/crawler **195**（195）/brinstar rooms **935** ✓。
- 导航三元组 5 例全对：RandomNumbers **07:C000**（:33）/MainLoop **07:C0BC**
  （:199）/NMI **07:C0D9**（:233）/ObjectAnimIndexTbl **$8572**/SamusEnterDoor
  **$8B13** ✓。

## 5. §7.1 覆盖热图 —— **FAIL（不可复现）**

- 声明方法（§7.1）：扫 `bankN:XXXX`/`bankN:$XXXX` 引证按 bank×1KB 聚合。
- **机械复算不一致**（同口径 grep）：
  - bank1 $8800 格：声明 **473/55** ⇒ 声明口径实测 **5/4**；放宽含 `01:88xx`
    形 **63/12**；全形裸 `88xx` 总量仅 **86**——三种口径均远低于 473。
  - bank7 $CC00 格：声明 **188/10** ⇒ 声明口径 **13/3**；`07:CCxx` 形 **3/2**；
    全形裸 `CCxx` 总量 **67**。
  - 附带：findings 文件数声明「250 份」实为 **265**（陈旧）。
- ⇒ §7.1 数字与自报口径**数量级不符**（疑似计数脚本口径漂移/未随文档入库）——
  违 survey 自标「A 级机械统计」。§7.3 排序以此为底座，量化结论不可采。
- **处方**：①热图重算脚本入库（tools/ 或 spec 附录）+按现仓 265 findings 重跑
  落数；或 ②§7.1 降级定性（方向性叙述），删除具体数字；③同步修正 findings
  计数。§7.3 的两个已验证盲区（rooms≈0/enemies 无址）可保留（独立 grep 证实）。

## 6. 工程质量 —— PASS（一处例外）

- A/B/C 分级纪律合规（A 级标统计源、B 级推断标注、C 级引用明示「未独立复算」）；
- §8 遗留假设 8 条显式栅栏 ✓；D5「未做 rename」声明 ✓；
- 例外=§7.1（见 §5）——数字面未达自标口径。

## 裁定

| 面 | 结论 |
|---|---|
| 1 §1.4 NMI/窗口契约（A 级事实+栅栏） | pass |
| 2 §6.2 六陷阱抽核（三陷阱属实） | pass |
| 3 §7.3 盲区抽样两点（rooms≈0/enemies 无址） | pass |
| 4 抽样（6 文件行数+5 导航三元组） | pass（10/10） |
| 5 §7.1 热图 | **fail**（数字不可复现，数量级不符） |
| 6 工程质量 | pass（例外 §7.1） |

**verdict = 需修订**：§7.1 热图重算入库（或降级定性）+findings 计数修正后复审
即 pass；其余各节（§1.4/§6.2/§7.3 定性面/§2/§4/§8）可用。

Evidence: prg7_engine.asm :265-275/:272（jsr SoundEngine）/sound_engine.asm :384
（$B3B4）；include 六副本 grep（prg0-5）；CopyGFXBlock 调用点 :1770-1790；
crawler.asm 无址 grep=0；docs/findings bank[1-5]:A### grep=0；行数 6 文件实测；
导航 5 三元组锚实测；§7.1 复算三口径（5/4、63/12、86 vs 473/55；13/3、3/2、67
vs 188/10）；findings 计数 265 vs 250。
