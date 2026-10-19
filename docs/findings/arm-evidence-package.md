# findings：臂深化卡包预备（移动臂域——卡包骨架）

> 状态：**臂深化证据预备仓就位**（2026-10-20）。a 系形（同 g3 包）。
> 烤案源=a1-022+（a 系 preg 延续——簇首×4 LCG→ROM golden）。

## 1. 卡包骨架（臂深化域——待簇清单勘）

- **域面**：移动臂（Samus 移动/碰撞/状态机——触达战役素材接面：ToggleSamusHi/
  碰撞机制/存活窗/房 23 几何）；
- **烤案源**：a1-022+（a 系 preg 延续）——簇首×4 LCG→ROM golden（同 a1 形）；
- **消化口径**：同 a 系 F13（三级标签——名注优先）；
- **待勘**：臂域簇清单（refs asm 先读纪律——移动例程族/碰撞族/状态机族）。

## 1.5 臂域簇清单（refs asm 先读——GoSamusHandler 派发表全形）

| 簇 | 员 | 成员（addr+名） |
|---|---|---|
| ARM-00 派发 | 2 | UpdateSamus($CC0D)/GoSamusHandler($CC1A) |
| ARM-01 站立 | 1 | SamusStand($CC36) |
| ARM-02 奔跑 | 2 | SamusRun($CCC2)/SetSamusRunAccel($CCB7) |
| ARM-03 跳跃 | 1 | SamusJump($D002)（**共享×2**——跳/指上跳同员） |
| ARM-04 球态 | 2 | SamusRoll($D0E1)/SetSamusRoll($D0B5) |
| ARM-05 指上 | 1 | SamusPntUp($D198) |
| ARM-06 门内 | 1 | SamusDoor($D3A8)（滚动中） |
| ARM-07 死亡 | 2 | SamusDead($D41A)/SamusDead2($D41F) |
| ARM-08 电梯 | 1 | SamusElevator($D423) |

- **派发表**（A 级：CC1A-JumpEngine）：Samus.status 索引→10 状态（Stand/Run/Jump/
  Roll/PntUp/Door/Jump×2/Dead/Dead2/Elevator）；
- **触达素材接面**：SamusRoll/SetSamusRoll（BUG-001 楔交互）+SamusDoor（触达战役
  门过渡）+SamusElevator（电梯家族）——**素材接面全**。

## 2. 行数核自查条

- 本包=骨架态（簇清单待勘）——自查随簇清单落。
