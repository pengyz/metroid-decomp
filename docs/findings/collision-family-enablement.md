# findings：碰撞族使能考古定谳（task-37，UpdateItems 门条件+触摸槽路径+移植输入）

> 状态：**使能条件定谳**（2026-10-17）——附 batch-2 根因更正（§7 同日）。
> 判据：nmikstas 指令流锚×ROM 字节×写点级 probe（装载/未装载双态判别）。
> 复跑：`scenario_run run sc-touch-gate`（装载态槽探针）+
> `scenario_run run sc-touch-gate2`（几何径实验）。

## 1. 使能链全图（A 级，UpdateWorld $CB29 帧更新 20 连环）

```
UpdateWorld($CB29):
  UpdateEnemies → UpdateProjectiles → UpdateSamus → AreaRoutine
  → UpdateElevator → UpdateStatues → $FA9D → LFC65 → LF93B → LFBDD
  → SamusEnterDoor → $8B79 → UpdateTiles
  → **LF034（Samus<enemies crash detection——敌接触判定！）**
  → DisplayBar → LFAF2 → CheckMissileToggle
  → **UpdateItems($DB37)（道具显示+触摸）** → LFDE3 → 清 SpriteRAM
```
- UpdateWorld 每帧全跑（敌更/移动/门转场实证并行）——**使能链本身无总闸**；
- 阻断点在子例内部门（下表）。

## 2. UpdateItems 门模型定谳（A 级指令流+写点实证）

```
CheckOneItem($DB42):
  PowerUpType,x==$FF? → [门0 早退]（未装载态=此门！batch-2 探针误读的真相）
  → 填槽：PowerUpYCoord→PowerUpY($034D)/XCoord→PowerUpX($034E)/NT→PowerUpHi($034C)
  → GetObjCoords($D79F)（读槽→MakeCartRAMPtr 房间 RAM 指针）
  → [门1] lda ($04),y / CMP #$A0 / BCC exit   ← 实心瓦判定（道具位瓦片）
  → [门2] DrawFrame / CMP SpritePagePos / BEQ exit  ← 绘制成功判定
  → AreObjectsTouching($DC7F)=LF186+LF172+LF1A7+LF1FA（F1xx 几何族）
      X=0(Samus 对象) × Y=$40(道具对象槽)——槽坐标+半径几何比对
```

**写点实证**（sc-touch-gate 装载态，289 轮/帧）：
- $034D=152/$034E=104/$034C=1（槽坐标+NT——与 PowerUpYCoord 解码值互证）；
- $0341/$0342=3/3（ObjRadY/X=触摸半径 3px——**小箱**）；
- $0343=84（PowerUpAnimFrame——绘制面在跑）；
- **PowerUpType 时间线：00→FF(f549 清)→04(f1013 装载)**——门 0 判据实证。

## 3. 触摸几何定谳（负结果真因重定）

- 装载态最近逼近：Samus (104,177) vs 道具 (104,152)——**dx=0, dy=25**
  （道具箱 y=149-155 vs 走廊地板态——正下方掠过无重叠）；
- 道具位=**顶棚顶面**（x≈76-128 顶棚结构，顶 152/底 175；左壁 x=76 高柱
  （v8 实证 y=111-177 横向阻断）；天窗 x=4-75 全高跳区（62px 弧实证））；
- **几何径未竟**（v1-v17 轨迹库）：最近逼近 (64,152)=Y 精确命中（天窗跳+右飘段）
  ——x 差 40px（左壁阻断）；右缘径（x=130+ 起跳左飘落顶）待试；
- 受击/死亡侧：LF034 敌接触判定在使能链内（每帧跑）——敌邻位 (224,176) vs
  (236,193) 未叠（12+17px）——**是否同因（几何未真叠）待 LF034 内部门勘**
  （EnStatus 门控=task-19 面）。

## 4. UpdateItems 移植任务卡输入（porter 面）

| 件 | 规格事实 |
|---|---|
| 槽模型 | PowerUp RAM=$0340-034F（对象槽 $40 field-major 区）+ $0748-0753（双道具槽 +8）；槽填=每帧从 $0749/x 拷贝（LDB4A-56） |
| 门 0 | PowerUpType==$FF 早退（未装载） |
| 门 1 | 实心瓦判定（房间 RAM 瓦 <$A0 退——道具只在可通行瓦上渲染/可拾） |
| 门 2 | SpritePagePos 绘制成功判定 |
| 触摸 | AreObjectsTouching=F1xx 族四连（LF186/LF172/LF1A7/LF1FA）×槽半径 3px 小箱 |
| 二槽 | ItemIndex=$00/$08（双道具槽并存——WaveBeam 备用槽同机制） |
| 动画 | PowerUpAnimFrame=$0343（*2=FramePtrTable 索引）+PowerUpAnimIndex=$074F |

## 5. 遗留

1. 右缘几何径（x=130+ 起跳左飘落顶）——一试件；
2. LF034 敌接触内部门勘（受击/死亡/终局三果归属）；
3. F1xx 几何族逐例解（LF186/LF172/LF1A7/LF1FA 契约——task-19 材料续）。

## 6. 【2026-10-17 终局·四果收账】死亡/终局链达成（sc-death-chain4）

**四果杠杆判定终谳**：
| 果 | 状态 | 证据 |
|---|---|---|
| 受击 | **✓ 达成**（翻案收账） | samus-move3 f1188 命中（$70 闪烁 50f 实锤）+death-chain 系 f1202/1281 确定性双击——**碰撞族活着**（blink 门 $70 全程 0=未挡） |
| 死亡 | **✓ 达成**（fixture×真实伤害合流） | fixture 压血 1（编码破：(ones,tens)=($0106 高半,$0107 低半)）→f1281 真击→**血尽 0000** |
| 终局 | **✓ 达成**（全链） | **f1410 $1F→0x1C GAME OVER → f1415 0x19 DisplayPassword → f1578 0x1A WaitForSTART**（死亡→游戏终局→密码显示→等待 START 全链实录） |
| 拾取 | 几何径未竟 | 顶棚顶面右缘径待试（v1-v17 轨迹库） |

**死亡链机制定谳**（B 级）：
- 血量编码：`(ones,tens) = ($0106 高半字节, $0107 低半字节)`（v3/v4 fixture 双态判别）；
- 敌伤害=8 HP/击（30→22→…实测）；受击闪烁 $70=0x32 起倒数 50f（不可击窗）；
- 死亡判定=**伤害应用路径完成态**（fixture 直接压 0 不触发——须真实伤害事件
  收尾——task-35「同根」修正为「伤害完成态检查」语义）；
- Game Over 链：死亡→0x1C（GAME OVER 显）→0x19（密码显）→0x1A（等 START）——
  title 仓 0x19/0x1A/0x1B/0x1C 四面素材**全达**（sc-death-chain4=bundle 随包）。

**更正链**（一并收账）：batch-2「碰撞族共用阻断」→task-37「几何不可达」→本节
「受击/死亡/终局全达成」——三次翻案全部原地留痕（更正文化自证）。

## 7. 【2026-10-17 拾取果收官·v1-v18 终谳】几何不可达=设计性屏障（能力限制如实）

**几何全图终谳**（A 级，v1-v18 轨迹库×双壁定谳）：
| 结构 | 带 | 证据 |
|---|---|---|
| 左壁 x=76 | y=110-**170**（60px 带） | v6 实证（apex 110@x=70 后 x=76 卡落 y=120-170）+v8/final2 三证 |
| 门楣 x=116 | y=99-155+ | final4 实证（apex 99 卡 x=116） |
| 走廊压跳区 | x≈78-128，顶 ~175 | pickup7/8 实证（JumpDsp 封顶 2） |
| 天窗 x=4-77 | 全高跳区（apex 110） | sc-touch-gate 实证（62px 弧） |

**终判**：左壁带顶 y=110=**恰好等于天窗区最大跳高 apex**——玩家无法越顶
（设计性屏障实锤）；道具 (104,152)=顶棚顶面在屏障后。
**拾取果=未达（能力限制类，D7 如实）**——非路径失败：真径候选=**爆破瓦穿透**
（UpdateItems 面 `cmp #$4E / IsBlastTile` 指令锚——走廊顶 x=95-107 疑爆破瓦，
需**炸弹**道具；炸弹为后续道具=回环解锁设计）/异房入口/形态能力（morph）。
- v1-v18 轨迹库+双壁几何图=完整交付（移植线障碍物物理面素材）；
- blast-tile 假说（B 级）留档：$4E 瓦判定指令流在 UpdateTiles 面——爆破瓦破顶径
  待炸弹能力获得后（或 fixture 注入炸弹能力！——下一试件）。

**四果终账**：受击 ✓ / 死亡 ✓ / 终局 ✓（三果收账）+拾取=能力限制径锁定
（fixture 炸弹注入=待试一击）。
