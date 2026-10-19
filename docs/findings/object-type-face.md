# 对象类型面+物品 type 码表（G2 深化——转正钥件）

> G2 后续件（item type 码表+对象辨识）。Evidence：A（bank:addr 锚）。
> 2026-10-19 decomp。

## 1. SpecItmsTbl type 码表（A 级：prg7:EDD6 派发锚）

**type 字节低 4 位=处理例程索引**（ChooseSpawningRoutine $EDD6→JumpEngine）：

| type | 例程 | 用途 |
|---|---|---|
| 0 | ExitSub($C45C) | 无操作 |
| 1 | SpawnMapEnemy($EDF8) | 敌（Squeept 族） |
| **2** | **SpawnPowerUp($EDFE)** | **Power-ups（物品！）** |
| 3 | SpawnMellows($EE63) | Mellows/Mellas/Memus（飞行敌） |
| **4** | **SpawnElevator($EEA1)** | **Elevators（电梯）** |
| 5 | SpawnCannon($EEA6) | MB 房加农 |
| **6** | **SpawnMotherBrain($EEAE)** | **Mother Brain** |
| 7 | SpawnZebetite($EECA) | Zebetites |
| 8 | SpawnRinkaSpawner($EEEE) | Rinkas |
| 9 | SpawnDoor($EEF4) | 部分门 |
| 10 | SpawnPalette($EEFA) | 背景调色板切换 |

- **条目形**（ScanForItems $ED98 解析实证）：`[Y][next-ptr lo][next-ptr hi][X]…[type][data…]`
  ——Y 匹配 MapPosY→X 匹配 MapPosX→type 派发；$FFFF ptr=链尾；
- **双 type 概念辨**（task-16 注精化）：条目 type 低 4 位=派发索引（本表）≠
  ItemData word 6 位 type（物品类打包——两字段勿混）。

## 2. 对象槽位图（A 级：constants_ram 锚）

- **Object×16**（$0300-$03FF，16B/槽）：Samus($0300)/Object0310($0310)/
  Elevator($0320)/Object0330($0330)/PowerUpDraw($0340)/Object0350($0350)/
  **Statue($0360)**/Object0370($0370)/Doors×4($0380-03BF)/Object03C0($03C0)/
  **SamusProjectiles×3($03D0-03EF)**；
- **Object 字段**（$0300+x）：status/radiusY/radiusX/animFrame/animDelay/
  animResetIndex/**animIndex($0306)**/data07/speedY/speedX/isHit/onScreen/hi/
  **y($030D)/x($030E)**/data0F；
- **敌族**（$0400-$04FF）：Ens×6/EnProjectiles×6/EnExplosions×4($04C0-04DF)/
  EnMotherBrain/EnMellowHandler（EnExtra 平行阵）；
- **物品真面=PowerUps**（$0748-$0757，2 槽×8B）：type(物品类)/y/x/hi/unused×3/data07。

## 3. 对象辨识（A 级：GetEnemyData $EB0C 写面）

- **obj1($0310)=房载敌**：GetEnemyData——房对象首字节**高 4 位=槽 nibble**（and #$F0→tax
  =槽偏移）→nibble 1=$0310；GetEnemyType($EB28) 载敌类型——多态 status 轮转=敌状态机 ✓
  （sc-pickup-deep 观测 obj1 status 0x86/0x22/0xC4/0x06/0x6F 多态=敌 AI 态）；
- **物品辨识**：物品=PowerUps 阵（type 字段=物品类）+PowerUpDraw($0340)=绘图工作面
  （status=0 时 y/x/hi churn=绘图 scratch——sc-pickup-deep $0340 churn 解释 ✓）。

## 4. 【2026-10-19 更正注】×3

1. **sc-boss-combat commit "obj4=f1278 beam 弹槽"=误**：$0340=PowerUpDraw（非
   beam 弹——SamusProjectiles=$03D0-03EF 三槽）——f1278 churn=PowerUpDraw 面；
2. **sc-pickup-deep "$0340 probe 假设存疑"→精化**：probe 地址对（PowerUpDraw）
   但非物品态面——**真物品面=PowerUps $0748-$0757**（下版场景改 watch）；
3. **animIndex echo 精度修正**（a1-001 fixture 约定——影响 c0 定案）：Object 结构
   **$030E=x 坐标/$0306=animIndex**——"c0 独需 $030E"=**x 字段**（非 animIndex）/
   "c1-7 槽面 $0306+x"=animIndex 槽位——**双面=异字段（x vs animIndex）**，
   非同字段异位（原 echo「animIndex 按路径异位」表述过粗——以此为准）。

## 遗留（C 级假设区）

- GetItem 事件链（拾取判定入口——PowerUps.type→SRAM $6988 位图写链）待考证；
- ItemData word 6 位 type→物品类全表（missile/tank/door 族）待解。
