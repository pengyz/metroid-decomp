# G2 boss face 素材预位（ROM 考证面）

> G 系收官后 G2 首件（🔴 序）——窄交付逐件（①房/②对象/③行为）。
> Evidence 级：A（bank:addr 锚）+B（域结构推断）。2026-10-19 decomp。

## ①boss 房/域结构（A 级：prg 域锚）

- **五域结构**（m1disasm 域命名实证）：
  - prg1_brinstar（01:95D0 AreaItemRoomNumbers）——Brinstar 域
  - prg2_norfair（02:95D0）——Norfair 域
  - prg3_tourian（03:95D0）——**Tourian 域=Mother Brain boss 面**
  - prg4_kraid（04:95D0）——**Kraid 域=boss 面**
  - prg5_ridley（05:95D0）——**Ridley 域=boss 面**
- **boss 房=域内特定房**（AreaItemRoomNumbers 表指域内房号——boss 房号=
  各域表内特定项——待②对象面关联定）；
- **地图格→域**（B 级）：WorldMap 格值=房号（GetRoomNum 实证）——boss 域=
  格值域段（房号 46 总量内分段——域边界=各 prg 载入面）。

## 遗留假设（C 级——禁 rename）

- boss 房号精确清单待②（boss 对象面——敌类型→房关联）；
- Mother Brain=Tourian 核心房（it_MotherBrain=$6 常量——statue/item 类关联待）。

## ②boss 对象面（A 级：constants_ram 锚）

- **Statue 对象族**（$0360-0370 RAM 图——constants_ram 851/900-903 行）：
  - `Statue instanceof Object`（$0360 起——对象结构同 Samus/敌族）
  - `KraidStatueRaiseState = Statue.animDelay`（#01=未升/#02=升中/bit7=已升）
  - `RidleyStatueRaiseState = Statue.animResetIndex`
  - `KraidStatueIsHit = Statue.animIndex`（#00=未击/#01=已击）
  - `Temp00_KraidStatueStatus=$00`/`Temp02_RidleyStatueStatus=$02`（临时态）
  - `StatuesBridgeIsSpawned=$54`（桥生成标志——Tourian 桥面）
- **boss 面=statue 门机制**（B 级）：Kraid/Ridley 雕像=Tourian 门（击破两
  boss→雕像升→桥生→Tourian 通）——**boss 面核心=statue 状态机**（$D7B3 同族
  面——elevator/statue=对象状态机两例）；
- **遗留假设**（C 级）：statue 对象调度入口（UpdateStatue?）待③行为面；boss 敌
  对象类型码（EnemyInit 表）待关联。

## ③boss 行为面（A 级：prg7 指令锚）

- **statue 行为链**（prg7:5207-5254 UpdateStatue 状态机）：
  - `UpdateStatue`（07:5252 调——Kraid statue 索引 `Statue-Objects`）
  - `UpdateStatueBGTiles`（5207/5211/5231/5239 多点调——**BG 瓦更新**=
    雕像升起的视觉面）
  - Kraid statue 索引计算：`KraidStatueStatus-KraidStatueStatus+(Statue-Objects)`
    （5251 行——对象槽定位）+ Ridley 同形（5254）；
- **行为链定**（B 级）：Kraid/Ridley 击破→`KraidStatueStatus`/`RidleyStatueStatus`
  （$00/$02 临时）→`RaiseState`（bit7=已升）→`UpdateStatueBGTiles`（BG 瓦=升起
  动画）→`StatuesBridgeIsSpawned`（桥生成）→**Tourian 通**；
- **G2 素材线**（boss face 三件落）：①五域结构/②statue 对象 RAM 图/③行为链
  ——**boss 面素材预位完**（🔴 序首件）——下一：elevator 家族扩（🟡）/物品真径；
- **遗留**（C 级）：UpdateStatue 全指令流（状态转移细义）+boss 敌行为（Kraid/
  Ridley 战斗面）=G2 后续件。
