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
