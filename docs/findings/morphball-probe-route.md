# 变形球探针路线真值+移植任务卡（round r9 收尾轮——能力获取/变形滚动前置）

- 日期：2026-10-31
- 性质：emu 腿实证（Mesen2 地面真值）+移植缺口盘点；rust 侧对照**未成立**（物品子系统未移植——见 §3）
- 探针：`cases/case-morphball-probe.txt`（草案入库——扫掠迭代形；matched-seam 待移植后启用）

## 1. 路线真值（emu 实录，case-morphball-probe frames.jsonl）

- 起点（图 3,14）→ 球房（图 2,14）= **连续卷轴无门**：左走到 ScrollX 回绕（0→255）即
  `MapPosX--` + 载房（f812，RoomNumber←0x23，载房窗 bogged(2) 触发）——与右向首像素
  同构（ScrollLeft 的 ScrollX==0 特殊径）。
- **Maru Mari 生成真值**（PowerUps.0=$0748，f812）：`type=04 y=98 x=68 hi=01`——球在
  球房 (x=104, y=152)（地面 y=177 上方 25px，跳跃可达）、滚入侧名表（hi=1）。
  type=04=pu_MARUMARI；PowerUps 双槽 $0748-$0757（8B/槽：type/y/x/hi）；
  PowerUpDelayFlag=$0109。
- Samus 轨迹：intro 止于 f796-797（st 24→FF→1）→左行起步（加速曲线 ≈0.35px/f 均值）
  →f812 过渡（x=112）→左墙 x=4 停（f1120 附近）。
- 跳跃扫掠：x≈86 跳=2px 弹回（平台底 y≈175 实心）；x≈26-36 跳=16px 升（10 帧 press
  的部分跳高）；开口几何待精扫（全高 hold 跳+分位迭代）。
- 贴墙垂直跳/下压均无效（gear 恒 00——未触球）。

## 2. 用户三问的现状裁定

1. **左侧球位跳入**：未复现未判定——开口 x 位置待全高跳精扫；"顶部图块碰撞异常"若为
   rust 侧图块数据/上行检分歧，需 rust 腿先能生成球后再对拍。
2. **能力获取**：rust **不可能触发**——物品子系统未移植（§3），rust 腿无球。
3. **变形滚动**：依赖 2（无 gear 无法形态）；sa_Roll 状态面 rust 有常数（physics.rs
   "SAMUS_STATUS==4 sa_Roll 出"）但形态转换/滚行物理未对拍过。

## 3. 移植任务卡（asm A 级锚齐全——下一轮实施）

| 件 | asm 锚 | 内容 |
|---|---|---|
| ScanForItems | 07:ED98（prg7:9255） | setup_room 链首件之一（:8588 `jsr ScanForItems`）：ItemData 链扫（MapPosY/X 匹配）→SpawnPowerUp |
| SpawnPowerUp | 07:EDFE | PowerUps 槽填（type/y/x/hi；pos=(y&$F0)\|8/(x<<4)\|8 形——**以实测 04/98/68/01 反推字节序**）；CheckForItem（已有⇒不生成） |
| UpdateAllPowerUps | 07:DB37（:5503） | 槽环×2（PageIndex←$40）：buried 判（RoomRAM tile<$A0⇒不绘不触）→ObjDrawFrame（调色板 2 帧轮换）→触摸判 |
| 触摸→获取 | 07:DBA5-$DBF1 | AreObjectsTouching（07:DC7F——obj0 萨姆斯 vs obj1 物品半径和判）⇒PowerUpMusic+gear\|=MakeBitMask（beam 替换规则）+PowerUpDelayFlag←$FF（冻结 240 帧——game_engine 的 0x83/0x84 消费径已移植）+type←$FF+SelectSamusPalette |
| ItemData 烘焙 | 00:9029（prg0:2237） | 16b 词=IIIIIIXX XXXYYYYY（Maru Mari 词=ui_MARUMARI<<10\|(2<<5)\|14）——**节点链布局按 ROM 原字节烘焙**（词内 pos 读出 0x96 形——以 SpawnPowerUp 读径为准，禁手工推演） |
| 物品绘制 | ObjFrame_MaruMariItem（common_sprite_data:1461） | ObjDrawFrame 对象精灵面——**状态面外**（对象绘制子系统随件；watch 不含 OAM 时状态对拍可先行） |

## 4. 复现命令

```bash
python3 tools/scenario_run.py run cases/case-morphball-probe.txt --timeout 300
# 读数：analysis/traces/case-morphball-probe.frames.jsonl
# 关键字段：PowerUps(0748-0757)/PowerUpDelayFlag(0109)/SamusStatus(0300)/SamusX/Y(030E/030D)/
#          ScrollX(FD)/RoomNumber(5A)/gear(6878)
```
