# findings：场景库族矩阵+B 面分母声明载体（task-40，声明面收官）

> 状态：**声明面收官**（2026-10-17）。B 行为面分母=本声明清单（D7：行为空间无
> 机械分母——声明性覆盖，对拍只裁「对不对」不证「全不全」）。
> 对账基准：`oracle/traces/l3_manifest.json`（14 案）× `analysis/bundles/`（45 bundle）。

## 1. 场景族矩阵（14 案声明清单，三态制）

| 族 | 案 | 帧 | 事件核 | 态 |
|---|---|---|---|---|
| **title 基本** | boot-title | 300 | 标题显示+按键注入面 | ✅ 已备 |
| **title 菜单** | sc-title-menu2 | 900 | 0x15/16 菜单屏+确认双分支 | ✅ 已备 |
| **title 密码** | sc-password-entry3 | 1200 | 0x17/18 密码屏+字符录入「22」实证 | ✅ 已备 |
| **title 终局** | sc-death-chain4 | 1800 | 0x19/1A/1B/1C 死亡→终局全链 | ✅ 已备 |
| **move 基本** | samus-move | 900 | 移动 RAM 反应面（G4 切片） | ✅ 已备 |
| **move 基本** | samus-move3 | 1400 | 解锁链+行走/受击（f1188 命中实证） | ✅ 已备 |
| **move 深域** | sc-roll-nav | 1500 | 球形滚行动力学（y=200 底层+x 回绕穿屏） | ✅ 已备 |
| **move 深域** | sc-morph-tunnel | 1600 | 球体底层连续穿屏（可达域差异） | ✅ 已备 |
| **转场族** | sc-door-transition | 1100 | 门转场 gap 1002-1003 实证 | ✅ 已备 |
| **转场族** | sc-room-scroll | 1100 | 滚换相（gap 边界 bundle） | ✅ 已备 |
| **敌遭遇** | sc-enemy-encounter | 1350 | 敌槽变更 366/1331+动画/速度活动 | ✅ 已备 |
| **道具拾取** | sc-item-pickup | 1400 | 【如实降级】搜寻路径素材（pw 全 00） | 🟡 降级 |
| **夹具机制** | fixture-test | 300 | 三条件断言（(b) 读回+(c) 持久） | ✅ 已备 |
| **任意起点** | bundle-ff | 240 | 装载快进 102 帧逐字节忠实 | ✅ 已备 |

## 2. 待采清单显式（玩法面矩阵补全）

| 面 | 待采场景 | 前置 | 态 |
|---|---|---|---|
| **道具真达成** | sc-pickup-real（爆破瓦径 or 异房入口） | 真径未锁（双壁+假说降级——collision-family §8） | 🔴 开放 |
| **受击主动径** | sc-hit-deliberate（迎撞定点） | 敌接触定点设计（LF034 门勘后） | 🟡 材料在 |
| **door 深径** | sc-door-deep（开门动画/门内态） | 门转场 gap 内相（采样盲区） | 🔴 开放 |
| **combat 变体** | sc-combat-missile / sc-combat-beam | 武器面能力（fixture 可注 gear） | 🟡 可采 |
| **boss 面** | sc-boss-kraid / sc-boss-ridley | boss 域移植后（远期） | 🔴 远期 |
| **电梯族** | sc-elevator-ride | 电梯场景设计（$D7B3 面） | 🟡 可采 |
| **密码提交链** | sc-password-submit（提交→game 校验） | 录入缓冲编码考古（batch-3 开放项） | 🔴 开放 |
| **音频/时钟面** | sc-audio-sfx（SFX 触发面） | 声引擎移植（B31B=task-30） | 🔴 随 task-30 |

## 3. bundle 索引（任意起点素材库目录，45 件全带三直映）

| 场景→bundle 对照 | 件数 | 用途 |
|---|---|---|
| 案内 bundle（manifest 14 案随包） | 14 | bundle-ff 模式即用（对拍起点任意化） |
| 拾取攻防战 v1-v18 路径变体 | 18 | 几何考古原始数据+复现路径 |
| 死亡链 v1-v4 + game-over + bomb-blast | 6 | 死亡/终局/能力注入径 |
| title 迭代（menu v1-3/password v1-3/recon） | 7 | title 导航考古原始数据 |
| 基础移动（mid-move） | 1 | 任意起点首发实证（102 帧忠实） |

- 全 45 件 `analysis/bundles/*.mss`+`*.json`（MSS blob+ram/wram/regs 三直映）；
- 复用协议：`scenario_run` bundle_load 指令 or `l3-dump --bundle`（game-core 腿）；
- 对拍锚：bundle-ff 案（manifest）=任意起点机制的活体验证案。

## 4. 对账声明（判据）

- ✅ 清单完备：14 案逐案对账 manifest（族/帧/事件核/态全列）；
- ✅ 待采显式：8 面待采清单（三态制：🔴 开放/🟡 材料在/远期）；
- ✅ bundle 索引：45 件全录（三直映全 ✓）；
- **B 面分母=本声明清单**（声明性覆盖——随新批增补行）。

## 【2026-10-19 场景批 6 落】待采清单收口（8 面 → 6 余）

| 面 | 状态 | 实证 |
|---|---|---|
| sc-door-deep（门深径） | ✅ 落 | 双跑 634db9e5 ✓+gap@987 转场窗实证（UpdateAllDoors 实达面） |
| sc-combat-variant（受击径） | ✅ 落 | 双跑 f858721a ✓+血量递减+$030A 接触实证（碰撞族实达面） |

- L3 manifest 14→**16 案**（新双案 sha 锚 634db9e504dd/f858721aa32a）；
- G2 玩法系统对拍素材预备面+2（门电梯深径/受击径=已定谳链的实达面）；
- D7 如实：B 行为面声明覆盖（场景清单制）——两案=声明性覆盖非机械完备。
