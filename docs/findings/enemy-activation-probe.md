# 敌人 Resting→Active 激活路径：像素证据尝试与四处注入陷阱

**日期**：2026-11-16
**目的**：给 `e64b795b`（`UpdateEnemy_Resting_TryBecomingActive ; 07:F7CA` 端口）补像素证据。
L4 全矩阵在该 commit 后逐格不变，但"不变"有两种解释——窗内没执行，或装载值恰等于既有初值；
两者无法用现有用例区分。

**结论摘要**

1. 激活路径的**写入值**已与模拟器逐字节一致（见 §3 表），可判 A 级。
2. 但同一帧之后出现**真分歧**：注入的空中 Sidehopper 在 Rust 侧 `subY` 恒 0、位置恒不动，
   模拟器侧正常受重力下坠（`subY` 0xC0 起、`y` $A8→$B8→$C6…），并在 f1010 回到 Resting。
   ⇒ 这是 `UpdateEnemyCommon_Decide` / 跳跃位移径的独立缺口，**不属于激活路径本身**。
3. 该探针 7/8 采集点失配（f940 注入前 0px，f970 起 98–160px），**不入 L4 矩阵**，
   已归档 `cases/_archive/case-enactivate-probe.txt`。

## 1. 注入设计

目标：把首房**已存在的敌槽**（槽 `$40`）在采集窗内改造成 Resting 的 Sidehopper 地面型
（`type=0`，`L977B[0]=$64` bit6 置位 ⇒ 走加速模型），`delay=1`、`data05` bit3 置位，
让下一帧执行激活。

## 2. 四处陷阱（每一处都让探针"通过"但什么都没测到）

| # | 陷阱 | 现象 | 判据 |
|---|---|---|---|
| 1 | **EnExtra 基址按槽步进** | 写 `$6AF8` 以为槽 4，实际是槽 0 | `EnsExtra` 基 `$6AF4` + 16×槽号；槽 4 ⇒ `$6B34` |
| 2 | **status 语义是 1 起** | 注入 `status=0` 后敌人完全不动 | asm `07:F351` 跳表：0=ExitSub（空槽）/1=Resting/2=Active/…/≥7 RemoveEnemy |
| 3 | **引擎未启动** | f420–760 区间注入零效果 | `SamusIntro ; 07:C9D7` 演出到 f~903，此后 `GameEngine`(MainRoutine=3) 才调 `UpdateWorld ; 07:CB29` ⇒ 敌环根本不跑 |
| 4 | **离屏双返** | 放到 `y=$28,x=$BC` 仍零效果 | `UpdateEnemy_CheckIfVisible ; 07:F37F` 不可见 ⇒ `pla/pla` 双返回，整槽跳过 |

教训：探针"全 0px 一致"必须同时证明**注入生效**（读回注入后的槽面）与**路径执行**
（某处状态确实改变），否则就是空对空。本次前两轮都是空对空。

## 3. 激活写入与模拟器逐字节对比（f970，注槽 `$40`）

| 字段 | Rust | 模拟器 |
|---|---|---|
| `EnsExtra.status`(+0) | 02 | 02 |
| `Ens.x`(+1) / `radiusX`(+2) | 08 / 0A | 08 / 0A |
| `animIndex`(+6) / `resetAnim`(+5) | 07 / 05 | 07 / 05 |
| `accelY`(+10) / `accelX`(+11) | 40 / 00 | 40 / 00 |
| `Ens.movementIndex`(+8) | 01 | 01 |
| `Ens.speedY`(+2) / `speedX`(+3) | FF / 02 | FF / 02 |
| `Ens.data05`(+5) | 1A | 1A |
| **`EnsExtra.subY`(+8)** | **00** | **C0**（已下坠 1 帧以上） |
| **`Ens.x`** | **98（未动）** | **A0（已移动 8px）** |

⇒ 激活路径写入表（movementIndex 播种 / 四表装载 / 朝向翻转 / `data05&$DF`）**全部一致**；
分歧只在"之后怎么动"。

## 4. 新发现的缺口（待独立修复）

注入后的 Sidehopper 在 Rust 侧：

- `subY`（`$6AF4+x+8`）恒 0，`y` 恒 `$A8`——重力/位移完全不发生；
- 同槽 `animFrame` 仍在步进（`3E→07→06→1A`），说明 `update_enemy_anim` 在跑，
  即 `update_enemy_common_anim_move` 被调用，但**位移没有落地**；
- 模拟器在 f1010 已把它转回 Resting（`status` 2→1，走 `enemy_active` 的 bit5 径）。

后续要查的点（按 asm）：`UpdateEnemyCommon_Decide` 的跳跃判定是否要求"在地面"
（本注入把敌人放在空中 `y=$A8`），以及 `enemy_move` 的速度/子像素积分是否被碰撞径回退。
**注意**：这只是"空中激活"这一非常规状态下暴露的差异，不代表常规 Sidehopper 行为；
但它是目前唯一能观察到该 AI 位移径的探针。

## 5. 重开条件

修掉 §4 的位移缺口后，把 `cases/_archive/case-enactivate-probe.txt` 移回 `cases/`，
预期 f940 0px、f970 起逐帧 0px；届时它同时充当激活路径与 Sidehopper 位移径的像素证据。
