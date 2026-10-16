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
