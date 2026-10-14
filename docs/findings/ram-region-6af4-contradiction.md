# findings：$6AF4-$6BF3 区域的硬件矛盾（阻断 F152/F1BF 移植）

> 状态：**矛盾未决**（2026-10-15）——C 级假设区，禁止据此移植/命名（AGENTS §3.5）。

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
