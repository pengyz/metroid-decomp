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
