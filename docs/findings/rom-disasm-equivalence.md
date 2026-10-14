# 发现：m1disasm ↔ 原版 ROM 等价性验证（S0 取证）

- 日期: 2026-10-14
- 结论: **✔ 等价确认**。metroidret/m1disasm 的 `NES_NTSC` 构建产物与
  `rom/Metroid (USA).nes` 的游戏内容（headerless PRG+CHR，131,072 字节）**逐字节一致**。
  差异仅存在于 16 字节 iNES header（元数据，无功能影响）。
- 据此采用 D3 真值模型：**m1disasm 作为参考投影 + ROM（Mesen2 运行时）作为最终真相，并存互证**。

## 1. 被验证对象（版本锚点）

| 项 | 值 |
|---|---|
| ROM | `rom/Metroid (USA).nes`，131,088 bytes，MD5 `d77c8053168da14b360bf5caeccc5964`，整文件 CRC32 `A2C89CB9` |
| m1disasm | `refs/m1disasm` @ `eb17d3b91549de26455121bff4427521e5c7faa6` |
| 工具链 | WLA-DX v10.8a @ `7ae11df1657dafab1f5fa662889d005709fec15e`（源码自建，满足 ≥v10.7 要求） |
| 复现 | `cd refs/m1disasm && python3 build.py` → `out/M1_NES_NTSC.nes`；比对跑 `python3 tools/check_disasm_equivalence.py` |

## 2. 逐字节比对（裁定证据）

| 检查 | 结果 |
|---|---|
| headerless PRG+CHR 131,072 字节 | **逐字节一致**（双方 CRC32 均为 `70080810`） |
| iNES header | 不一致（见 §4，元数据层面） |

构建产物 MD5 `d7da4a907be0012abca6625471ef2c9c` = m1disasm 自带期望值（"matches vanilla ROM"），
与我们 ROM 的 MD5 差异**全部来自 header**。

## 3. 手动抽检（A 级证据，独立于构建链）

| # | 检查点 | ROM 实测 | 期望（m1disasm 源码） | 结果 |
|---|---|---|---|---|
| 1 | 中断向量 `$FFFA` | `D9 C0 B0 FF B0 FF` → NMI=$C0D9, RESET=$FFB0, IRQ=$FFB0 | `prg7_engine.asm:12925` 向量段 `.word NMI/ROMFIXED_RESET` | ✔ |
| 2 | reset 例程 `$FFB0` 37 字节 | `78 D8 A2 00 8E 00 20 8E 01 20 AD 02 20 10 FB … 4C 1A C0` | `reset.asm`：sei/cld/ldx#/stx $2000/stx $2001/lda $2002/bpl…/jmp Startup | ✔ 逐字节一致，jmp 目标 $C01A=Startup |
| 3 | 尾部数据块 "METROID" ASCII | file offset 131065 = CPU `$FFE9` | `reset.asm` `.byte` 表内 `4D 45 54 52 4F 49 44` | ✔ 位置与上下文字节一致 |
| 4 | NMI 入口 `$C0D9` | `08 48 8A 48 98 48 A9 00 8D 03 20 A9…` | `prg7_engine.asm:233` NMI: php/pha/txa/pha/tya/pha/lda #$00/sta $2003 | ✔ 一致 |

## 4. header 差异说明（为何可忽略）

| 字节 | 我们的 ROM | m1disasm 构建 | 含义 |
|---|---|---|---|
| byte6 (flags6) | `10` | `11` | 低 4 位 mapper=1 相同；bit0 镜像位不同（0 vs 1） |
| byte10-15 | `00×6` | `4E 49 20 31 2E 33` ("NI 1.3") | 构建所对照 dump 的工具标签残留 |

- **镜像位**：MMC1 (SxROM) 的镜像由游戏运行时写 mapper 寄存器（`$8000`，见 reset 例程
  `sta MMC1CTRL`）动态控制，header 镜像位对 MMC1 无功能影响——这正是社区以
  headerless PRG+CHR CRC32 (`70080810`) 为内容指纹的原因（MetroidNESRecomp 同此）。
- **"NI 1.3"**：m1disasm 期望 MD5 所对照的历史 dump 的 header 标签，非游戏数据。
- 结论：header 差异属 dump/工具元数据，不构成版本分歧。

## 5. 版本澄清（对"银河战士 1 只有一个官方版本"）

NES USA 卡带**确为单一版本**（无 v1.0/v1.1 修订），本次构建唯一命中 `NES_NTSC`。
但官方发行整体存在多版本，m1disasm 均可构建且各自 "matches vanilla ROM"：

| 构建目标 | MD5（构建已验证） | 说明 |
|---|---|---|
| NES_NTSC | `d7da4a90…` | USA 卡带版（= 我们的 ROM） |
| NES_PAL | `442fcb92…` | 欧版 |
| NES_MZM / NES_MZM_G | `8b95635a…` / `b27e4612…` | 《零点任务》(GBA) 内嵌版 |
| NES_CNS | `164fe605…` | Classic NES Series 版 |
| （fds-independant 分支） | — | 日版 FDS《メトロイド》（初代原生平台） |

Stage A 范围锁定 **NES_NTSC**；其余版本差异不进保真期。

## 6. 后续动作

- [x] 等价性确认 → 采用「disasm 参考投影 + ROM 运行时校验」双轨并存
- [ ] `reference/seed/` 冻结：以 IDA 8-bank export 归一化为 seed 真身，m1disasm 作对照注释源
- [ ] `SHA256SUMS` + rom-integrity 门禁
- [ ] Mesen2 oracle 首场景（reset → 标题）
