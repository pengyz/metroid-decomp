# findings：电梯垂直支路设计输入（$D7B3 面考证——G4 escape 井同族）

> 状态：**电梯面定谳+径设计**（2026-10-19，m1disasm prg7 内容锚×探跑实录）。
> 上游：sc-elev-probe 探跑（右行主径无电梯——InArea 恒 0）。
> 复现：`sed -n '4165,4230p' …/prg7_engine.asm`（SamusElevator）+
> `sed -n '4803,4850p'`（UpdateElevator）。

## 1. 电梯系统定谳（A 级）

### 1.1 对象模型（UpdateAllDoors 同构！）
- **电梯=房间对象**（Objects 槽族——Elevator 专用槽）；UpdateElevator($D7B3)
  =对象状态机 10 态 JumpEngine（door 7 态同族扩展）：
```
0=无/1=ElevatorIdle/2=ScrollXToCenter/3=ElevatorMove/4=ScrollY/
5=FadeOut(退化)/6=D8BF/7=FadeIn(退化)/8=ElevatorMove/9=ElevatorStop
```
### 1.2 激活链（scenario 设计关键）
```
SamusOnElevator（站上平台）+ 方向键按下（ElevatorType 位 7 定向：
  位 7=1→UP 键激活；=0→DOWN 键激活）→ ElevatorIdle→ScrollX 对中→Move 骑行
```
### 1.3 骑行力学（SamusElevator $D423）
- status 3/8=骑行中；**ElevatorType 位 7=方向**（1=up）；
- 下行：Samus.y 逐帧 +1（ScreenY≥$84 触 ScrollDown）；上行：逐帧 -1（<$64 触
  ScrollUp）；Y 环回=ToggleSamusHi（第 9 位翻转）；
- 动画帧隔帧（FrameCount lsr）。

## 2. 垂直支路径设计（sc-elevator-ride 输入）

- **前提**：含电梯对象的房间（房间数据 object 列表——Elevator 槽位）；
- **探跑事实**（sc-elev-probe）：右行主径（f1001 门/f1173 敌房/f3050 深门转场）
  无电梯（InArea 恒 0）——**电梯房=垂直支路**（下行支路/深区连接）；
- **设计径**（候选二选一）：
  ①f3050 深门后房间续探（下探 sc-elev-probe2——右行+periodic 下键踩台）；
  ②地图考证径（房间对象表反查电梯对象房间——ItemData 表旁证：(05,02) Tourian
  电梯红门=深区电梯井入口）；
- **scenario 骨架**（径锁定后）：站台（SamusOnElevator）→按方向键→骑行序列
  （capture 骑行前/中/后+bundle）——事件核=$D7B3 状态机写面（status 序列 1→2→3
  →8→9）+Samus.y 逐帧变+ScrollY 跟随。

## 3. T45-6 escape 电梯井（G4 通关面同族）

- **escape 井=电梯族**（Elevator 对象同机制——方向/骑行/状态机全同）；
- T45-6 设计=本面力学+escape 专径（Tourian 逃逸时限面——后续窗）；
- 同族共用力学表（§1）=G2/G4 双面的对拍素材底。

## 4. 遗留

1. 电梯房定位（候选径①/②二选一——下探 or 对象表反查）；
2. SamusOnElevator 置位点（站台几何——对象碰撞族同源）；
3. ElevatorType 来源（房间对象定义——对象表反查随 1）。

## 5. 【2026-10-19 补·置放表定谳】对象表反查径收（候选②胜出）

**SpecItmsTbl（Bank01:$A3D6 起）**——特殊物件置放表（房间级）：
```
条目形=[房间号][链指针][物件]*[$00 终止]；物件型 $04=Elevator（型表 $01-$0A）；
电梯参数字节=目的地 ID（$01/$02/$03）。
```
| 电梯 | 房间号 | 参数 | 备注 |
|---|---|---|---|
| **Elevator to Tourian** | $02 | $03 | Item3 红门 (05,02) 同房——需导弹轰门 |
| **Elevator to Norfair** | $16 | $01 | **无需导弹**（首选 sc-elevator-ride 径） |
| **Elevator to Kraid** | $12 | $02 | boss 域 |

- **径锁定判**：sc-elevator-ride=**room $16（Norfair 电梯）**——导弹无关径；
- 最后一步=room $16 的地图坐标（map 表反查——房间号↔坐标映射）→场景输入序
  （站台+DOWN 键——ElevatorType 位 7=0 下行）；
- 探跑三轮实录（右行/深门/坑区下降——InArea 恒 0）=电梯井不在已探支路
  （与置放表房间号法互补：探跑证否+静态定址）。
