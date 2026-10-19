# G2 物品真径素材（🟡②——ROM 考证面）

> G2 素材线第二件（🟡 序）——窄交付单件。Evidence：A（表锚）+B（径推）。
> 2026-10-19 decomp。

## 物品径面（SpecItmsTbl 续——elevator-branch-design §5 锚复用）

- **SpecItmsTbl（Bank01 $A3D6）**（A 级）：
  - 长条目=[Y][next-Y ptr word][X][offset][type][data…$00]
  - 短条目=[X][offset][type][data…$00]
  - type=$04=elevator（三电梯）——**物品 type 族**（missile/energy/tank 等=其他
    type 码——待 type 表全解）；
- **ItemData word 打包**（A 级——task-16 已解）：`IIIIIIXX XXXYYYYY`（type 6b+
  X 5b+Y 5b）——Item3 $2882→type 10=missile door（X~4-5/Y=2 ✓）；
- **物品真径**（B 级）：场景=item 拾取面（sc-pickup-real 🔴 待采清单）——
  物品径=SpecItmsTbl 项→房坐标→拾取事件核（GetItem/ItemGfx 链=后续件）；
- **遗留**（C 级）：type 表全解（type 码→物品类）+拾取事件链（GetItem 入口）
  =后续件；sc-pickup-real 场景（🔴 待采）=G2 素材线消费方。

## 【2026-10-19 触达战役素材归档】（v1-v22——sc-pickup-touch 全档案）

- **碰撞机制**（A 级：prg7:F172/F186）：hi XOR PPUCTRL &1 后比较——触达前提=
  raw hi 相等+Δx≤7（半径和 4+3）+Δy≤18（15+3）；
- **hi 翻转机制**（A 级：prg7:E249 ToggleSamusHi）：x/y wrap 专属翻转——门过渡不设 hi；
- **物品存活窗**（A 级实证）：RemoveIfOffScreen=MapX 界 crossing 触发——西行无界
  （存活窗解）/东行 MapX 3→4 死点；
- **房 23 几何全解**（rooms.asm:682）：双屏 12 结构+5 敌清单+门(222,5)+物品台地
  （struct01@(63,9) 顶条 y=152 吻合）——详 object-type-face.md §5/§6（GetItem 五环链
  +pu_* 全表）；
- **三条件同窗实证**（v20）：存活/hi/带同窗首证——触达差=wrap 侧 39px 带（v22 预算
  封顶收束）；
- **遗留**（C 级）：wrap 侧 39px 带径+GetItem SRAM 写链尾环。
