# 端口 backlog：unported 603 按 bank 分批 + 首批 20 条高价值例程

> 用途：把 `routine_inventory` 的「unported 603」变成一个能逐批派活的清单。每条都带 `bank:addr`、调用者、系统、影响面与最小验收命令，独立 claim、独立验收。
> 边界：本文件只做清单，不改代码/门禁/`data/status/`。状态口径以 `tools/routine_inventory.py --stats` 为准（不是落盘的 `data/status/routine_inventory.json`，后者是陈旧快照 380/147/608）。

## 0. 口径（先读这段，否则下面数字对不上）

**分母是什么**：`refs/m1disasm/SRC/**/*.asm` 的**源码级 code label**（1135 条），不是每个 bank 的装载实例。`areas_common.asm`、`sound_engine.asm`、`enemies/*.asm`、`reset.asm` 会被 bank0–5 各 `.include` 一次，按实例数会放大到 5–6 倍且毫无信息量，所以**每个 label 只计一次**。

**三档状态**（定义见 `tools/routine_inventory.py` 头注释）：
- `ported`：`data/status/routine_inventory_overrides.json` 里显式登记了 Rust 符号（人工考证过的映射）。
- `candidate`：label 的 snake_case 与某个 Rust `fn` 同名——**同名不是等价**，本文件**不把 candidate 当 ported**，也不把它算进 backlog 的高价值条目。
- `unported`：两者都没有。

**统计命令（本文所有汇总数的来源）**：

```bash
python3 tools/routine_inventory.py --stats          # 603 这个数
python3 tools/routine_inventory.py --list unported --limit 1000 | wc -l   # 603（逐条）
```

**bank 归属怎么来**：
1. 源码文件 → 装载 bank：`prg0_title.asm`→bank0、`prg3_tourian.asm`→bank3、`prg4_kraid.asm`→bank4、`prg5_ridley.asm`→bank5、`prg7_engine.asm`→bank7；bank0–6 映射到 `$8000`，bank7 固定在 `$C000`。
2. 被多个 bank `.include` 的文件（`areas_common.asm`/`sound_engine.asm`/`enemies/*.asm`/`reset.asm`）标为 **shared**：源码算一条，二进制里在 bank0–5 各有一份**同址副本**。
3. 具体地址取 `refs/m1disasm/out/M1_NES_NTSC.sym`（每行 `bank:addr label`）：

```bash
grep -E " (Amul16|ScreenOff|CommonJump_DrawEnemy)$" refs/m1disasm/out/M1_NES_NTSC.sym
# 07:c2c5 Amul16 / 07:c439 ScreenOff / 01:803c CommonJump_DrawEnemy（02–05 同址）
```

**每个文件贡献多少 unported（本文件的分组依据）**——直接调工具自己的 `build()`，避免读陈旧快照：

```bash
python3 - <<'PY'
import sys,collections; sys.path.insert(0,'tools')
import routine_inventory as ri; from pathlib import Path
inv=ri.build(Path('.')); print(inv['counts'])
c=collections.Counter(r['file'].split('/')[-1] for rows in inv['entries'].values() for r in rows if r['status']=='unported')
print('sum',sum(c.values())); [print(n,k) for k,n in c.most_common()]
PY
```

实测输出（2026-11-25）：`code 1135 / ported 380 / candidate 152 / unported 603`；按文件：`prg7_engine 348 / areas_common 64 / prg0_title 59 / sound_engine 56 / prg3_tourian 22 / kraid 13 / metroid 6 / prg2_norfair 5 / prg5_ridley 5 / ridley 4 / rinka 4 / rio 4 / prg1_brinstar 3 / prg4_kraid 3 / sidehopper 3 / dragon 1 / pipe_bug 1 / polyp 1 / reset 1`，合计 **603**。

> 不要用 `data/status/routine_inventory.json` 数这个：它是落盘快照（608/147/380），比 `--stats` 旧 5 条。

## 1. 按 bank 汇总

| bank | 映射 | 独占 unported | 说明 |
|---|---|---|---|
| 0 | `$8000` | **59** | 全在 `prg0_title.asm`：标题/密码/结局/存档码 |
| 1 | `$8000` | **3** | `prg1_brinstar.asm` 自有 3 条 |
| 2 | `$8000` | **5** | `prg2_norfair.asm` 自有 5 条 |
| 3 | `$8000` | **22** | `prg3_tourian.asm`：Mother Brain/Zebetite 等 |
| 4 | `$8000` | **3** | `prg4_kraid.asm` |
| 5 | `$8000` | **5** | `prg5_ridley.asm` |
| 6 | `$8000` | 0 | `prg6_graphics.asm` 是图形资产，无 code label 缺口 |
| 7 | `$C000` | **348** | `prg7_engine.asm`：主循环、Samus、敌人引擎、门/滚动、PPU/VRAM |
| shared | bank0–5 各一份 | **158** | `areas_common 64` + `sound_engine 56` + `enemies/* 37` + `reset 1` |

合计 59+3+5+22+3+5+0+348 = 445（独占）+ 158（shared）= **603**。

**读法**：bank7 占 348/603（57.7%），是缺口主战场；shared 的 158 条虽然「只算一条」，但一条要写 5–6 份副本才能让各区域 bank 一致——移植时按源码写一份、按 bank 验证多份。
> 【2026-11-25 更正】此处初稿写「155」（沿用旧的 shared 计数）；按 §1 表与合计应为 **158**（64+56+37+1）。对影复核时指出，已改。

## 2. 高价值条目（24 条，按批分组）

选择依据：`jsr/jmp` 交叉引用次数（越高越底层/越被依赖）+ 是否落在 L4 已覆盖的行为面上 + 是否卡着其它未端口例程。xref 计数命令：

```bash
python3 - <<'PY'
import re,pathlib
t='\n'.join(p.read_text(errors='replace') for p in pathlib.Path('refs/m1disasm/SRC').rglob('*.asm'))
for l in ['Amul16','Adiv16','TwosComplement','LoadTableAt977B','CommonJump_DrawEnemy','ScreenOff','InitSFXData']:
    print(l, len(re.findall(r'\b(?:jsr|jmp)\s+'+l+r'\b',t)))
PY
```

### 批 1：PPU/帧时序原语（6 条，bank7 + 1 条 bank3）

| # | `bank:addr` | 例程 | 调用者（asm 行） | 系统 | 影响面 | 最小验收 |
|---|---|---|---|---|---|---|
| 1 | `07:C42C` | `WaitNMIPass` | `ClearAll`(0:2331)、`DisplayPassword`(0:2811)、`PasswordToScreen`(0:2888) | ppu/帧时序 | 关屏后等 NMI 的同步点；密码屏/转场相位 | `python3 tools/l4_diff_run.py run cases/case-password-justin.txt` |
| 2 | `07:C439` | `ScreenOff` | `DrawIntroBackground`(0:188)、`TitleScreenOff`(0:607)、`MoreInit`(7:1871)、`PrepareGameOver`(7:2134)、`GoPassword`(7:2182)、`ClearScreenData`(7:2481) | ppu | 显示开关；暗窗/转场的起点（#31 的 f1191） | 同上 |
| 3 | `07:C447` | `ScreenOn` | `DrawIntroBackground`(0:209)、`TurnOnDisplay`(0:2354)、`DisplayPassword`(0:2818)、`GameOver`(0:2877)、`LoadEndGFX`(0:3787)、`RollCredits`(0:3987)、`ElevatorD8BF`(7:5036) | ppu | 显示开关的另一半，与 2 成对 | 同上 |
| 4 | `07:C2A8` | `AddYToPtr00` | `VRAMStructWriteData`(7:852)、`WriteVRAMString`(7:925)、`WritePaletteStringByte`(7:1009)、`UpdateNameTable`(7:7809)、`DrawStructRow`(7:9837) | ppu/VRAM | 16 位指针加偏移；所有 VRAM 写入的寻址基座 | `python3 gates/cli.py visual`（PPU 面） |
| 5 | `07:F1EE` | `Yplus16` | `SearchOpenSamusProjectileslot`(7:3824)、`CollisionDetection`(7:10106/10171/10212)、`SpawnEnProjectile_FindSlot`(7:11821) | collision | 16 位 Y 加偏移；Samus/子弹/敌人碰撞盒 | `python3 tools/l4_diff_run.py run cases/case-collide-probe.txt` |
| 6 | `03:9EF9` | `Xplus16` | `MotherBrain_Killed`(3:1213)、`MotherBrain_Disappear`(3:1251)、`MotherBrain_Idle_HandleBeingHit`(3:1553)、`UpdateBullet_CollisionWithZebetiteAndMotherBrainGlass`(3:1751) | collision（Tourian） | 16 位 X 加偏移；Mother Brain/Zebetite 碰撞 | `python3 tools/l4_diff_run.py run cases/case-001-game-start-samus-interaction.txt`（Tourian 段无专属 L4 格，见 §5 缺口） |

### 批 2：定点数学与表访问（7 条，全 bank7）

| # | `bank:addr` | 例程 | xref | 调用者（代表） | 系统 | 影响面 | 最小验收 |
|---|---|---|---|---|---|---|---|
| 7 | `07:C2C5` | `Amul16` | 19 | `SetDoorEntryInfo`、`UpdateDoor`、`AddHealth`、`HorzAccelerate`、`DrawStructRow`、`Hex16ToDec` | math | 8×8→16 位乘；速度/加速度/坐标/门位移的公共算子 | `cargo test -p game-core`（新增单测） |
| 8 | `07:C2BF` | `Adiv16` | 17 | `WriteVRAMString`、`CheckHealthStatus`、`DisplayHUD`、`DrawStructRow`、`DrawTileBlast`、`DoSparkleSpriteCoord` | math | 16÷8 位除 + 商余；HUD/密码/坐标换算 | 同上 |
| 9 | `07:C2C6` | `Amul8` | 6 | `UniqueItemFound`、`SamusHasItem`、`DoOneDoorScroll`、`GetEnemyIsHitFlags`、`SamusProjectileHitDoorOrStatue` | math | 8×8→16 位乘；道具/存档码/门滚动 | 同上 |
| 10 | `07:C2C0` | `Adiv8` | 5 | `WriteDoorBGTiles_Common`、`DoSparkleSpriteCoord`、`UniqueItemFound`、`EndOfRoomHorizontal` | math | 8 位除法；门背景砖/滚动列 | 同上 |
| 11 | `07:C3D4` | `TwosComplement` | 18 | `EnemyMove`(areas_common:130/157)、`CheckHealthStatus`(7:3145)、`SamusMoveVertically`(7:7189)、`SamusMoveHorizontally`(7:7292)、`EnemyFlipAfterDisplacement`(7:12292) | math | 负数取补/符号处理；敌人移动、Samus 移动、受伤方向 | 同上 |
| 12 | `01–05:80B0` | `LoadTableAt977B`（shared） | 18 | `EnemyIfMoveFailed{Up,Down,Left,Right}`(areas_common:212/278/328/388)、`EnemyGetDeltaX/Y`、`UpdateEnemy_Hurt`、`EnemyReactToSamusProjectile` | enemy data | 按状态索引取速度/表值；敌人被击退与碰撞反弹 | `python3 tools/l4_diff_run.py run cases/case-killenemy-probe.txt` |
| 13 | `07:C27C` | `JumpEngine` | 17 | `GoMainRoutine`(7:346)、`GoSamusHandler`(7:2721)、`EnemyLoop`(7:8865)、`UpdateEnemy`(7:10688)、`EndGame`(0:3732) | control | 状态机跳转表分发；主循环/Samus/敌人/结局的分派基座 | `python3 gates/cli.py trace` |

### 批 3：敌人公共更新（7 条，shared，bank01–05 同址）

| # | `bank:addr` | 例程 | xref | 调用者（代表） | 系统 | 影响面 | 最小验收 |
|---|---|---|---|---|---|---|---|
| 14 | `01–05:8000` | `CommonJump_UpdateEnemyCommon` | 8 | `UpdateEnemyCommon_Decide_{AREA}`(prg1:875/prg2:598/prg3:543/prg4:726/prg5:603)、`SkreeExit_Active`、`SwooperAIRoutine01_Swoop`、`RioExit_Active` | enemy | 敌人公共更新入口（移动+动画） | `python3 tools/l4_diff_run.py run cases/case-crawlprobe.txt` |
| 15 | `01–05:8003` | `CommonJump_UpdateEnemyCommon_noMove` | 13 | 同上 + `CrawlerExit_Explode`、`SqueepExit_Resting`、`RinkaAIRoutine`、`PipeBugAIRoutine` | enemy | 不移动版（仅动画/计时），被最多敌种调用 | 同上 |
| 16 | `01–05:8027` | `CommonJump_ApplySpeedToPosition` | 7 | `Cannon_ShootEnProjectile`(prg3:739)、`KraidUpdateProjectile`、`MetroidAIRoutine`、`RinkaAIRoutine`、`SqueeptAIRoutine`、`RidleyTryToLaunchFireball` | enemy | 把速度/子像素累加到坐标；所有发射物与飞行敌 | 同上 |
| 17 | `01–05:801B` | `CommonJump_EnemyFlipAfterDisplacement` | 6 | `Sidehopper_Common`(sidehopper:15)、`WaverAIRoutine`(waver:10)、`RidleyAIRoutine`(ridley:25)、`L9963`(prg2:623) | enemy | 位移后按 data1F/jump 符号决定是否翻向（#16 已修的同族逻辑） | `python3 tools/l4_diff_run.py run cases/case-jumpledge-probe.txt` |
| 18 | `01–05:803C` | `CommonJump_DrawEnemy` | 4 | `MotherBrain_DrawSprites`(prg3:1624/1633)、`DrawEndTimerEnemy`(prg3:2040)、`DrawCannon_Escape`(prg3:763) | enemy/render | 敌人精灵绘制入口 | `python3 tools/l4_diff_run.py run cases/case-scrollseam-probe.txt` |
| 19 | `01–05:803F` | `CommonJump_DrawTileBlast` | 5 | `MotherBrain_TimeBombMessage_ScrollBackOnScreen`(prg3:1379)、`MotherBrain_SpawnDoor`(prg3:1485)、`MotherBrain_Disappear_Disintegrate`(prg3:1684)、`UpdateZebetite`(prg3:2137) | enemy/render | 砖块爆破特效绘制（Mother Brain 战） | 同上 |
| 20 | `01–05:8563` | `GetOtherNameTableIndex` | 8 | `EnemyMoveOnePixel{Up,Down,Left,Right}`(areas_common:1026/1102/1175/1246) | room/ppu | 跨名字表判断（敌人越界到相邻 NT） | `python3 tools/l4_diff_run.py run cases/case-scrollseam-probe.txt` |

### 批 4：音频 SFX 与玩家/密码杂项（4 条）

| # | `bank:addr` | 例程 | xref | 调用者（代表） | 系统 | 影响面 | 最小验收 |
|---|---|---|---|---|---|---|---|
| 21 | `00–05:B452` | `InitSFXData`（shared） | 14 | `SpitFlameSFXInit`、`ScrewAttackSFXInit`、`GotoInitSFXData`、`HasIceBeamSFXInit`、`DoorOpenCloseSFXInit`、`SamusDieSFXInit` … | audio | SFX 初始化（通道/包络/音高） | `python3 gates/cli.py trace`（APU 面） |
| 22 | `00–05:B368` | `LoadSFXData`（shared） | 13 | `InitSFXData`(sound_engine:502–511)、`MultiSFXInit`、`BossHitSFXInit`、`SamusHitSFXInit`、`IncorrectPasswordSFXInit` | audio | 装载 SFX 序列数据；所有音效的取数基座 | 同上 |
| 23 | `07:CE84` | `IsSamusDead` | 7 | `CheckHealthStatus`(7:3116)、`SubtractHealth`(7:3209)、`CollisionDetection`(7:10076/10125/10179/10205) | player | 死亡判定；受伤/碰撞链路终点 | `python3 tools/l4_diff_run.py run cases/case-collide-probe.txt` |
| 24 | `07:E8BE` | `StoreObjectPositionToTemp` | 5 | `SamusRoll`(7:3637)、`PlaceBulletAtArmCannon`(7:3996)、`UpdateBullet_Move`(7:4531)、`ObjectCheckMove*`(7:8161/8330) | player | 坐标存临时槽；Samus/子弹移动的公共前段 | `python3 tools/l4_diff_run.py run cases/case-shootprobe-probe.txt` |

> 备选（同价值，留给下一批）：`ClearAll`(0:909F,5)、`PrepareWriteVRAMStringRAM`(0:9450,5)、`LoadRowAndColumn`(0:91BF,3)、`UpdateCrossMissile`(0:8963,7)、`InitPasswordFontGFX`(7:C6D6,3)、`ReadTableAt968B`(7:F74B,4)、`StoreObjectPositionToTemp` 的对偶 `Xplus16/Yplus16` 已在批 1。

## 3. 分批计划与依据

| 批 | 条目 | 写面（预估） | 分批依据 | 可否独立验收 |
|---|---|---|---|---|
| B1 | 1–6（PPU/帧时序 6） | `crates/game-core/src/frame/`（+ `ppu`） | 都是「一次调用、无状态机」的原语；成对（ScreenOff/On、WaitNMIPass）必须同批，否则密码屏会出现半套 | 是：`case-password-justin` 三位点可判 |
| B2 | 7–13（数学/表 7） | `crates/game-core/src/frame/`（数学工具，建议独立模块） | 纯函数、可单测、被 19/18/17 次引用；**B1/B3 都依赖它**（`TwosComplement` 被 `EnemyMove`、`SamusMove*` 调用） | 是：新增单测 + `cargo test -p game-core` |
| B3 | 14–20（敌人公共 7） | `crates/game-core/src/enemy/` | 全部 shared（bank01–05 同址），同一份 Rust 实现要覆盖 5 个 bank 的调用；`UpdateEnemyCommon` 与 `noMove` 必须同批 | 是：敌人相关 L4 案（crawl/killenemy/jumpledge） |
| B4 | 21–24（音频 + 玩家 4） | `crates/game-core/src/audio/`（+ `samus`） | 音频与玩家死亡/子弹是两条独立链；放最后是因为它们不阻塞 B1–B3 的验收 | 是：`gates/cli.py trace`（APU 面）+ collide/shoot 案 |

**首批建议顺序：B2（数学 7 条）→ B1（PPU 6 条）→ B3（敌人公共 7 条）→ B4。**

理由（按依赖与验收成本排）：
1. B2 是叶子：纯函数、输入输出可穷举，能写单测自证，不需要 L4 采集，也不依赖帧时序；`Amul16/Adiv16/TwosComplement/LoadTableAt977B` 的 xref 分别 19/17/18/18，是后续三批的公共依赖。
2. B1 里的 `AddYToPtr00` 被 `WriteVRAMString`(7:898) 与 `DrawStructRow`(7:9773) 直接调用，而这两处同一子程序里紧挨着调 `Adiv16`（B2 的条目 8）——所以 B1 的验收会连带把 B2 的除法口径暴露出来；先做 B2 可以让 B1 的失败定位干净。它的验收点（password-justin 的 1190/1191/1197）已被 #31 定过口径，判据现成。
3. B3 必须先有 B2 的 `TwosComplement`/`LoadTableAt977B`/`JumpEngine`，否则 `UpdateEnemyCommon` 只能写一半。
4. B4 与前三批无依赖，放最后是为了不打断 M4 的主线（敌人/房间）。

## 4. 逐条要点（便于直接开卡）

- 每条卡建议只含 1 条例程（或一对强耦合，如 `ScreenOff`+`ScreenOn`），`writeScopes` 限定到单个 Rust 文件，验收命令取本文件对应行的「最小验收」。
- `shared` 条目（12、14–22）要在卡面写明「一份实现覆盖 bank01–05 的 5 份同址副本」，验收时至少跑两个不同区域 bank 的 L4 案（如 crawl 在 bank1/tourian 段）。
- 三条与已修 bug 同族、容易误判：`EnemyFlipAfterDisplacement`（#16 修过 data1F 符号门）、`ScreenOff`（#31 的 f1191 暗窗）、`GetOtherNameTableIndex`（#24/#27 的载房窗精灵页）。

## 5. 未覆盖与待取证（如实写）

- **系统归属的证据级别**：上表「系统」列全部由**调用者名字与调用位置**推定（B 级）。没有对任一条做完整的数据流证明；`math`/`control`/`audio` 三类的名字本身来自第三方反汇编（m1disasm），按 D5 只作线索，**不作改名依据**。
- **xref 计数**只统计 `jsr/jmp` 字面引用，不含间接跳转（`JumpEngine` 的跳转表、指针表调用）；所以 xref 少的条目可能被低估。
- **无专属 L4 格的系统**：音频（SFX）没有像素/内存判等格，只能用 `gates trace` 的 APU 面兜；Tourian/Mother Brain 段没有专属案，B1-6、B3-18/19 的验收要借 case-001 或新开案，属**待补**。
- **未做的分母核对**：本文件沿用 `--stats` 的 603，没有独立重算「code label 判定」的边界（`.byte`/宏生成的表）；若有人改动 `routine_inventory.py` 的判定规则，本文件的分组需要重跑 §0 的两条命令。
- **candidate 152 条未纳入**：它们是「同名但未考证」的候选，按卡面要求不当作 ported，也不进 backlog；若后续考证通过会从 unported 侧转移，届时本文件的分布需重生成。
