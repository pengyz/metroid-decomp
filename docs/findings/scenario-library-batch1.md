# findings：场景库扩面·批次 1——玩法场景族四包采集（task-34）

> 状态：**四包入库**（2026-10-16）——spin-exit 采样+双跑对称+manifest 登记+bundle 随包。
> 复跑：`python3 tools/scenario_run.py run <名>` ×2（.run1/.run2 对称）+
> `cargo run -p game-oracle --bin l3-dump -- --scenario <名> --bundle analysis/bundles/<名>`。

## 1. 四包清单（B 面分母声明性清单=manifest cases）

| 包 | 族 | 双跑 | 行数 | 事件核实证 | bundle |
|---|---|---|---|---|---|
| sc-door-transition | 门转场（SamusEnterDoor） | ✓ 684eb723 | 1083 | **转场 gap 1002-1003 实证**（spin 空窗=门穿越面） | 门前态 f990（12514B） |
| sc-room-scroll | 房间滚换（SetupRoom/GetNameTable） | ✓ 684eb723 | 1083 | 同路径滚换相（gap 边界 bundle=f1002） | 滚换相 f1002（12602B） |
| sc-enemy-encounter | 敌遭遇 | ✓ 39d58f38 | 1331 | **敌槽区 $300-35F 帧间变更 366/1331**+AnimResetIndex{00,04,07,0C}+ObjVertSpeed{00,01}（有敌活动无接触伤害）；转场 gap 1002-1003/1188-1189 | 遭遇中段 f1200（11824B） |
| sc-item-pickup | 道具拾取（best-effort） | ✓ 758348ab | 1381 | **【如实降级】pw 面 $6988-698F 全 00=拾取未达**→降级「搜寻路径 bundle 素材」（D7 不冒领）；转场 gap 1002-1003/1207-1208 | 搜寻段 f1250（11667B） |

## 2. 事件核方法（锚=spin 空窗 gap 群+变量活动面）

- **转场类事件**=行集 gap（spin 出口空窗）——1002-1003 三包共现（同穿门）；
  敌包 1188-1189/道具包 1207-1208=延伸路径的二次转场；
- **敌遭遇**=敌槽区帧间变更计数（366 次）+动画/速度变量活动；
- **道具拾取**=PasswordByte00（$6988，「Stores status of items 0 thru 7」定义锚）
  位图变更——**事件核标注更正**（原 $69/$6A=TempX/TempY 临时量，标注错已原地更正）。

## 3. 每包内容（可复跑命令在头部）

- `oracle/scenarios/sc-*.txt`（spin-exit 采样+bundle_capture 指令）；
- `analysis/traces/sc-*.{run1,run2}.frames.jsonl`（双跑对称件）；
- `analysis/bundles/sc-*.mss + .json`（MSS 快照+三直映——任意起点素材库）；
- `oracle/traces/l3_manifest.json`（4 案登记：sha256/行集/missing/head_skip/事件核注）。

## 4. 遗留

1. 道具拾取真达成需路径设计（SpecItmsTbl 世界坐标×定位）——现包降级素材；
2. 敌接触伤害面（SamusHit=$030A 写）未触发——受击场景待设计（主动迎撞路径）；
3. 移植线到域后 bundle-ff 模式即比（游戏腿接口=l3-dump --bundle 已就绪）。
