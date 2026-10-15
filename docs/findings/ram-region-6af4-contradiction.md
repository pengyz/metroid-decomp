# findings：$6AF4-$6BF3 区域的硬件矛盾（阻断 F152/F1BF 移植）

> 状态：**已定谳**（2026-10-15 当日二次考证）——$6000-$6Cxx = **WRAM 暂存区**。
> 判据：①写址实存（bank01 `$6Axx` abs,X 恰 7 处：81B4/820C `$6AFE`、81C3/81D7 `$6AFF`、838A `$6AFC`、83EA `$6AFD`、855F `$6AFB`；82AB `$6B01`/82CD `$6B03` 系 $6Bxx 另计）②m1disasm 布局
> 「ItemHistory …saved game data (not used)」+「16 slots of 16 bytes ($6AF4-$6BF3)」
> ③实跑证据（Mesen2=D3 真相源，非真机）：游戏内敌人正常运行（samus-move3 f1150 截图，再生命令见 samus-movement-observations.md）——若 $6AFx 为 open bus 敌槽
> 不可能工作。结论：卡带带 WRAM（作暂存，无电池存档功能——battery 位=0 与「(not used)」
> 自洽：WRAM≠battery save）。

## 矛盾（A 级观测三则）

1. 代码读址：bank07:D983 `BD F4 6A LDA $6AF4,X`、F152 `BD FB 6A LDA $6AFB,X`、
   F1BF `BD F5 6A LDA $6AF5,X`——绝对,X 寻址 $6AFx（X=$00-$50 扫描域 ⇒ $6AF5-$6B45）。
2. m1disasm 线索（constants_ram.asm:1271）：「16 slots of 16 bytes each ($6AF4-$6BF3)」
   ——按 RAM 敌槽数组标注。
3. 硬件事实：iNES flags6=0x10（battery 位=0，无 WRAM 声明）；MMC1 的 $6000-$7FFF
   非 PRG/非 RAM（SLROM 无 PRG RAM）⇒ 读 $6AFx 应为 open bus，写应无存储。

## C 级假设区（未证）

- H1：open bus 读值恰被游戏当确定性数据用（unlikely，但 Metroid 有 quirk 传统 D4）；
- H2：代码区间系 linear sweep 误判（F162-F171/F1B3-F1BE 已证实为数据岛被 seed 解成字节
  ——同区域存在解码争议先例）；
- H3：我方 iNES 头与实际卡带板型（如 SOROM/带 WRAM 变体）不一致的 dump 差异；
- H4：$6AFx 经由某未解的 mapper/镜像机制实际映射到 ROM 表（例如与 $6AF4 数据段
  prg[0x6AF5]=01 60 2b 00…（bank2 区）的关系待查）。

## 处置

F152/F1BF/F1FA 等读表件**移植冻结**，直至本矛盾定谳（走 hexdump → 实机 trace →
mapper 考证三步）；F1xx 族其余微切片（F1F4/F1EE/F1E0，纯 RAM）不受影响已移植。


## 定谳后处置（替代原「移植冻结」）

- L2 引擎补 **$6000-$7FFF WRAM 映射**（8KB 缓冲；L2In 扩 wram 域；写集契约：$6AFx 侧写
  记完整地址，2KB 镜像归一仅适用 $0000-$07FF）；
- F152/F1BF 读表件**解冻**（待 WRAM 域落引擎后移植）；
- 残留待考（低）：卡带板型精确名称/WRAM 实际尺寸上界（现按 8KB 保守建模）。
