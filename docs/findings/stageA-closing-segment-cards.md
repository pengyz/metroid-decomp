# findings：Stage A 收官段任务卡清单（task-45 通关面考证——asm-first）

> 2026-10-17；考证窗（m2 拼装间隙——读 asm=拼装规格源）。与 task-37 判定书对表
> （爆破瓦降级后的真实解锁径）。产出=任务卡清单+估时。

## 通关径结构（asm-first 勘定）

```
Brinstar/Kraid/Ridley（能力回环收集）
  → Statue 门控（双 Boss 击败=双像升起=Tourian 门开）← 爆破瓦降级对表点
  → Tourian（MB 区）→ Mother Brain 战 → escape 序列（倒计时+电梯井）
  → ChooseEnding（时长判定 1-5 型）→ ending
```

## 任务卡清单（估时=单元数（五环节奏））

| # | 卡 | asm 面 | 规模 | 估时 | 域 | 备注 |
|---|---|---|---|---|---|---|
| T45-1 | **Statue 门控族** | `UpdateAllStatues: 07:D9D4`/`UpdateStatue: 07:DA1A`/`UpdateStatue_Raise: 07:DA3D`/`UpdateStatueBGTiles: 07:DAB0`+StatueTileBlast 双表（DAD2/DAD6） | ~200 行+4 表 | **2-3 单元**（全仪式关键件——门控语义） | samus/ 还是 enemy/ 待对表 | **task-37 对表核心**：爆破瓦降级后 Statue 瓦爆=真实解锁径 |
| T45-2 | **ChooseEnding** | `ChooseEnding: 07:CAF5`（SamusAge 阈值环+AgeTable+JustInBailey 面） | ~30 行+表 | **1 小件**（字节自证可端大半） | samus/（SamusAge/JustInBailey 常量已在案） | **BUG 注保真**（SamusAge+3 漏检——D4）；EndingType=$? 址考证随件 |
| T45-3 | **GameEngine 主循环** | `GameEngine: 07:C92B`（游戏侧 GoMainRoutine 路——task-25 C11B-game 桩对面） | 大件 | **3-5 单元** | frame/（主循环相） | task-25 移动链交汇面 |
| T45-4 | **能力回环（SamusGear 门径）** | `CheckBombLaunch: 07:D150` 族+各 handler 内 SamusGear 检（`lda SamusGear/lsr/bcc` 面） | 门径图考证+2 单元 | **考证 1 窗+2 单元** | samus/physics（R-mv 挂点） | 能力门径图=解锁径规格（R-mv 补面） |
| T45-5 | **Tourian 入口径** | Statue 门开→Tourian 门 tile/scroll 面（ScrollDoor E1F1 族+门 tile $A0/$A1 面已锚 tile_class） | 考证件+1-2 单元 | **考证 1 窗+2 单元** | samus/scroll（待立） | 降级后真实径=考证判据 |
| T45-6 | **escape 序列** | `SetTimer: 07:C4AA`/`WaitTimer: 07:C494`/Elevator 族（`UpdateElevator: 07:D7B3`/ElevatorIdle/DrawElevator/ElevatorScrollXToCenter） | ~150 行 | **2-3 单元** | samus/scroll+计时面 | 倒计时=Timer 族（已折 UpdateTimer 面）；电梯井=Scroll 面 |
| T45-7 | **Boss 例程族**（协同件） | Kraid/Ridley/MB handler（enemy 域——port2 enemy/ 面） | 大件 | **协同**（其域+我钩对表） | enemy/（port2） | 双像击败触发=Statue 门控输入（T45-1 对表） |

**合计**：~10-15 单元+考证 2 窗（能力门径/Tourian 入口径）。

## task-37 对表注（爆破瓦降级真实解锁径）

降级面=爆破瓦销毁行为 L3 降级；真实解锁径=**Statue 瓦爆面**（StatueTileBlast 表
=像升起时的瓦片爆破写面）+门 tile 触发（$A0/$A1 已锚）——T45-1 考证判据=降级后
径可达性（门径图自证）。双 Boss 击败事件=Statue 升起输入（T45-7 对表）。

## BUG 注保真清单（D4——Stage A 保留）

- ChooseEnding：SamusAge+3 漏检（asm 注「BUG! Should be checking SamusAge+3」）。
- 其余随件考证注续记。

## 复现/考证命令

```
grep -n "UpdateAllStatues\|ChooseEnding\|GameEngine\|UpdateElevator" \
  refs/m1disasm/SRC/prg7_engine.asm
python3 -c "rom=open('rom/Metroid (USA).nes','rb').read(); \
  print(rom[0x10+0x1CAF5:0x10+0x1CAF5+48].hex())"  # ChooseEnding 字节区
```
