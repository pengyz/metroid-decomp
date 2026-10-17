# findings：SamusGear 能力门径图（T45-4 考证窗——R-mv 能力回环径补面）

> 状态：**门径图定谳**（2026-10-17，asm-first 内容锚 Bank07 指令流）。
> 与 porter Statue 窗对表面见 §3。复现：`grep -n "SamusGear" …/Bank07.asm`+逐例精读。

## 1. 能力位→机制门径全图（A 级）

| 位 | 常量 | 门径（机制） | 内容锚 |
|---|---|---|---|
| $01 | gr_BOMBS | **炸弹发射**（CheckBombLaunch 门 1：lsr→carry）+形态球+FIRE 三闸 | 07:D151 |
| $02 | gr_HIGHJUMP | **重力 $18→$12**（低重力=高跳） | 07:LCFF3 |
| $04 | gr_LONGBEAM | **光束射程/行为**（lsr lsr 双移位=射程档） | 07:D306+ |
| $08 | gr_SCREWATTACK | **旋风攻击**（空翻态 an_SamusSalto 时清 carry=敌毁）+跳跃 SFX | 07:LCDA3/2686 |
| $10 | gr_MARUMARI | **形态球**（morph 门——隧道径能力键） | 07:LD0B5 |
| $20 | gr_VARIA | **减伤 ½**（HealthLoChange lsr） | 07:LCEAC |
| $40 | gr_WAVEBEAM | 波束弹型（wa_ 面） | 07:D306 族邻 |
| $80 | gr_ICEBEAM | **冰束弹**（bpl 判位 7→wa_IceBeam） | 07:D3185+ |
| $FF 全置 | — | **密码屏全装载**（新游戏+全道具+导弹 5） | 07:LC93D |

## 2. 能力回环径（R-mv 补面——世界路径×能力门）

| 世界屏障 | 能力键 | 实证（本库） |
|---|---|---|
| 爆破瓦（$4E/IsBlastTile） | gr_BOMBS | ✅ 炸弹机械实证（task-37 三斩）；顶棚非爆破瓦=假说降级（collision-family §8） |
| morph 隧道 | gr_MARUMARI | ✅ 形态球+滚行实证（batch-5 双包） |
| 高跳径（垂直通道） | gr_HIGHJUMP | 🟡 重力参数实证（$18/$12）；路径场景待采 |
| 敌群屏障 | gr_SCREWATTACK/光束族 | 🟡 机制实证；combat 变体场景待采（matrix §2） |
| 双壁密室（拾取 (104,152)） | **未定**（回环候选：炸弹径证伪后=异房入口/morph 径） | 🔴 开放（task-34 批 2 终局） |
| Tourian 入口 | Statue 门（Kraid/Ridley） | 🟡 对表面见 §3 |

## 3. 与 porter Statue 窗对表

- Statue 面（porter 移植窗）：KraidStatueStatus($687B bit0=雕像激活闪烁)+门状态机
  （Door/Elevator 状态机壳=10 态批 ba73f10）——**对表点=Tourian 入口门的前置
  条件**（击杀双 boss→雕像→门开）；
- R-mv 补面本图供：能力→机制映射（表 1）+世界屏障→能力键矩阵（表 2）——
  porter 做 Statue 门判时的能力前置引用源；
- 开放对表项：Statue 门开启的精确条件（SamusGear 检查 or 独立状态位？）——
  随 Statue 窗考证合龙。
