# Oracle 可信度独立检验：AccuracyCoin 基线（Mesen2 2.1.1）

- 日期：2026-11-09
- 角色：verifier（oracle 可信度检验）
- 性质：**已证实**（Mesen trace / RAM dump A 级； FAIL 项定性区含少量假设，显式标注）
- 复现链：`tools/oracle/accuracycoin/`（run_accuracycoin.py → accuracycoin_dump.lua → decode_accuracycoin.py）

## 1. 目的

D3 锚点声明「ROM 字节 + Mesen2 执行是最终真相」。本卡用第三方标准具
**AccuracyCoin**（100thCoin/AccuracyCoin，147 项 NES 精度测试）对本机
Mesen2 2.1.1（`tools/mesen2/Mesen`）做独立可信度检验：跑全 battery、
逐项判读、**FAIL 不吞不改判据**，为 oracle 的适用边界留下量化基线。

## 2. ROM 采证链（provenance）

- 文件：`analysis/accuracycoin/AccuracyCoin.nes`（40976 B，不入库）
- sha256 `9332c3735dd7dea2f4711e670d6478d8b0085edeacdd4f9f300fb04a8bbe3f88`
- md5 `6cab1c9cc644e51ff4830fbe779f66d9` = doublegate/rustynes `RUNTIME.md` 记录的
  **AccuracyCoin v3.6.2**（RustyNES vendored 副本，upstream commit 66e87112；上游
  releases 无资产，atom feed 为空——vendored 副本是可得的最优 provenance 链）
- 判读所据源码：`analysis/accuracycoin/AccuracyCoin.asm`（master 快照，2026-11-09 抓取，
  sha256 `55d90f293a12d7b184634a1c563cde3253face1a613a5e5b8284dd121cd2591e`）。
  源-ROM 互证：源内 result_* 槽表（asm:144-321）解码出的通过数与 ROM 屏显/RAM 计数
  完全一致（128+19=147，见 §4）——槽表与该 ROM 版本匹配的 A 级交叉验证。

## 3. 运行环境定谳（本机 Mesen2 headless 陷阱清单）

以下均为本轮实测（strace / 进程证据，A 级），供后续 headless 采证复用：

1. **Mesen2 per-user 单实例**：已有实例在跑时新进程把 ROM 参数经
   `/tmp/CoreFxPipe_*` sendto 转发后 `exit 0` 静默退出（UI/Program.cs:78
   SingleInstance）。表现=「启动即秒退、无任何 stderr」。
2. **全新 HOME 无 settings.json** → 首次运行向导分支 `Process.Start` 重启后
   `return 0`，同样静默秒退。处置：先向一个 HOME 拷入 `settings.json`
   （`~/.config/acc-home/.config/Mesen2/`）。
3. **`tools/scenario_run.py:754-788` 孤儿清扫误杀**：`pgrep -f "tools/mesen2/Mesen"`
   等模式匹配到**cmdline 含同串的观察进程自身**（含 bash -c 内联命令）并 SIGKILL。
   对抗：把 Mesen ELF 拷到不含特征串的路径（如 `/tmp/accbin/accmesen`）、
   Xvfb 用非默认几何（1024x768x24）、内联 `pkill -f` 一律改为脚本文件执行。
4. **Lua `emu.read` 签名**（LuaApi.cpp:244-262）：`read(returnSigned, memType,
   address)`——**memType 必须显式传**；缺省 0=SnesMemory 对 NES 全返回 0
   （首轮采证 RAM 全零的根因）。NES 取值：8=NesMemory（CPU RAM）、
   9=NesPpuMemory（PPU 空间：nametable $2000-$23FF、CHR $0000-$1FFF）。
5. AccuracyCoin v3.6.2 **开机自动跑全 battery**（AutomaticallyRunEveryTestInROM），
   无需逐项选择；菜单位常显最近一次全测计数。

## 4. 结果（A 级，f=600 dump 与其后各帧一致，battery 启动后即收敛）

- RAM 计数：`$0037`=PostAllTestTally=**147**、`$0038`=PostAllPassTally=**128**
  （AccuracyCoin.asm:80-81 变量名对源吻合）。
- 屏显（nametable 字形解码）："TESTS PASSED- 128 / 147"。
- 结果槽数组 `$0400-$0498`（result_* 槽表），槽值语义（asm:1000-1015）：
  `AND #01`→bit0=1=PASS；bit0=0=FAIL，错码=byte>>2；`$0421` 恒 $00 系
  SRE_47 测试的哨兵要求（asm:159）；CPUInstr 槽占 `$0401-$0402` 双字节；
  DRAW 页结果在 `$03FF` 不计入 tally。
- 解码（`decode_accuracycoin.py`，128+19=147 与 tally 精确闭合）：

**PASS 128**：全部 CPU 指令语义类——RAMMirror/PPURegMirror/ROMnotWritable/
DummyReads/DummyWrites/OpenBus/**全部 68 项未文档化指令**（SLO/RLA/SRE/RRA/SAX/
LAX/DCP/ISC/SHA/SHS/SHY/SHX/LAE/ANC/ASR/ARR/ANE/LXA/AXS/SBC_EB，SRE_47 哨兵 OK）/
PC_Wraparound/PPUIOOpenBus/DMA+2007R+2007W+4015R+4016R+OpenBus+2002R/
VBlank_Beginning+End/NMI 全五项/Sprite0Hit/ArbitrarySpriteZero/SprOverflow/
MisalignedOAM_Behavior/APURegActivation/ControllerStrobing/InstructionTiming/
NmiAndBrk/NmiAndIrq/APU LC+LengthTable+FrameIRQ+4step+5step/DMC/DMA+OpenBus/
AddrMode 全六项/DecimalFlag/BFlag/PPUReadBuffer/DMCDMAPlusOAMDMA/ImplicitDMAAbort/
ExplicitDMAAbort/ControllerClocking/JSREdgeCases/AllNOPs/PaletteRAMQuirks/
INC4014/AttributesAsTiles/tRegisterQuirks/CHRROMIsNotWritable/RenderingFlagBehavior/
Rendering2007Read/2002FlagClearTiming/MisalignedOAMDMA/DMALandingOnWrite/
DMCPlaybackRate。

**FAIL 19**（槽:值→错码）：

| 槽 | 测试 | 错码 |
|---|---|---|
| $045B | Address2004_Behavior | $0A |
| $0461 | IFlagLatency | $0E |
| $046B | DMABusConflict | $02 |
| $046D | ImpliedDummyRead | $22 |
| $047B | OAM_Corruption | $02 |
| $0483 | StaleBGShiftRegisters | $03 |
| $0484 | Scanline0Sprites | $02 |
| $0487 | BGSerialIn | $02 |
| $0489 | SuddenlyResizeSprite | $04 |
| $048B | BranchDummyRead | $05 |
| $048C | 2004_Stress | $02 |
| $048E | 2007_Stress | $02 |
| $048F | StaleSpriteShiftRegs | $03 |
| $0490 | InternalDataBus | $02 |
| $0491 | ALERead | $02 |
| $0492 | HybridAddresses | $02 |
| $0493 | FrozenOAM2Inc | $02 |
| $0495 | MisalignedOAM2Addr | $04 |
| $0497 | DMCReloadTiming | $01 |

## 5. 定性（oracle 适用边界）

- **CPU 指令语义 100% 通过**（含全部 68 项未文档化指令）——本 oracle 的一切
  CPU 行为级差分（6502 指令语义、RAM 布局、NMI/VBlank 节拍、APU 计数器、
  手柄时序）有第三方标准具背书。
- 19 项 FAIL **全部聚集在 PPU 渲染管线内部时序 / 虚拟读(dummy read)/DMA
  冲突前沿**：$2004/$2007 应力、BG/Sprite 移位寄存器残态、branch 虚读、
  internal bus/ALE/HybridAddresses（CPU 内部总线行为，测试 ROM 自述属
  「超出总线可观测性」级）、OAM 损坏/冻结、DMC reload 时序。
  —— 对《银河战士》行为面差分（帧级 RAM 状态、游戏逻辑语义）不构成可信度
  削减；但对**扫描线内周期级 PPU 渲染保真**的差分主张，应视为 Mesen2 2.1.1
  的已测短板，不做该粒度的 oracle 主张。
- 【假设区，未逐项溯源】这批前沿项的失败属于「绝对前沿测试」（ AccuracyCoin
  上游把部分项标注为对任何模拟器都近乎不可测）与 v3.6.2 时点 Mesen 回归的
  混合；区分二者需逐项读 asm 判据，本轮不做。RustyNES 等其他模拟器同版本
  得分对比未取（无网侧证据），不引用。

## 6. 基线与复跑

- 基线向量：§4 的 128/147 + 19 项 FAIL 错码表。复跑
  `python3 tools/oracle/accuracycoin/run_accuracycoin.py`（持
  `analysis/.scenario.lock`，勿与 L4 harness 并发——单实例+孤儿清扫双约束）；
  解码 `python3 tools/oracle/accuracycoin/decode_accuracycoin.py <dump.log>
  analysis/accuracycoin/AccuracyCoin.asm`。
- 基线升不降：若未来 Mesen 升级后 FAIL 集扩大或 tally 下降，按门禁纪律定位
  （Mesen 版本差异 vs 采证环境差异），不得静默放宽判读。
- 原始产物：`analysis/accuracycoin/`（acc_run5.log=CHR/NT/RAM 多帧 dump、
  ROM、asm 快照；均不入库，由 §2 链可再生）。
