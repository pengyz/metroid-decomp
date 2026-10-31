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

## 5. 跳墙对拍结果（2026-10-31 迭代 3-5——用户"跳不进去"专项）

**路线全解（emu+rust 双腿 matched-seam，input=sched 修正后）**：
- 走廊（y=193 地面）→ 墙位 x=4 → **全高跳（hold A 55 帧）**：y 193→110（83px）+
  左漂过墙顶（x 4→255 回绕）→ 落坑沿 (244,113)。**rust 现版能过墙**：f1131-1180
  跳跃轨迹与 emu 逐帧全同（含落点）。
- **残余实锤（=用户"顶部图块碰撞异常"）**：墙顶角 1px 碰撞偏差——f1164 emu x=3 vs
  rust x=2（rust 多进 1px）、f1172 emu 250 vs rust 249（同形），随后即回同步。
  角部=墙顶与天花板交接 tile 的横移碰撞判差 1px——上行/横移检在角部 tile 的
  边界条件分歧（c3315af 上行臂 x 全距修的同族残余），根因定位待专项。
- f1-f885 全同（走廊+卷轴过渡+球房入段）；f886 起 st 0/1 墙边微闪（emu 站立
  动画帧 vs rust 恒 1——次要）；X 分叉仅 f1164/f1172 两帧各 1px 且自愈。
- **坑内继续**：落坑沿后左推不动（x=244 卡死——坑沿左缘实体）、二段跳（f1240）
  越坑左落 (68,177)——球位 (104,152) 触摸未触发（拾取面=物品子系统未移植，
  §3 任务卡；触摸时序迭代待移植后）。

**工具坑（本轮实证，KB 候选）**：
1. sched 手工生成：l4.write_schedule 的 caps 参=**元组列表**；传 parse_scenario 的
   dict 列表（`for f,tag in caps` 解包 keys）⇒ capture 行写成 "capture frame tag"
   ⇒ rust 解析失败/静默用旧 sched——**A 键输入整段丢失**（跳墙"未触发"假象）。
   正解=`[(c['frame'],c['tag']) for c in plan['captures']]`。
2. cases 加用例漏 `sample_point spin-exit` ⇒ emu 腿 endFrame 缝（F1 缝错位重演）。
3. 对比脚本 None/0 判等：emu 缺行 None vs rust 0 会误报分歧——分叉判定须双侧非 None。

## 6. 墙顶角 1px 根因定位（2026-10-31——速度字节铁证）

watch 扩展（0309 SpeedX_I/0313 SpeedX_F）后双腿对比——**角部 1px=空中漂移速度模型缺失**：

| 帧 | emu speedX | rust speedX | x |
|---|---|---|---|
| f1150-1163（贴墙跳上，左推被墙挡） | **$FE.E0（−1.125px/f 保持）** | **$00.00（清零）** | 4 恒 |
| f1164（角部释放） | $FE.E0 ⇒ 走 1px（x=3） | $00.00 ⇒ 走 2px（x=2，自有步进） | 分歧 |
| f1165+（过墙顶） | $FE.E0 ⇒ 1px/f | $00.00 ⇒ 1px/f（偶 2px） | 回同步 |

- emu：跳跃横漂**保持速度字节**（被墙挡住不清零——asm 的速度清零属地面停走径
  StopHorizontalMovement；空中径保持），角部释放帧按保持速度走 1px。
- rust：跳跃横漂**不写速度字节**（$0309/$0313 恒 00），横移=自有固定步进（2px/f），
  释放帧步进相位与 emu 错 1px ⇒ 角部 1px 偏差（随即重同步——后续帧步进节奏巧合
  对齐，长时漂移面仍会累积）。
- **修法方向**：rust 跳跃横漂改写 \$0309/\$0313（空中漂移速度=asm 空中横控语义——
  按住方向⇒速度趋向空中漂移上限 \$FE.E0=−1.125px/f，被挡不清零）；角部/长漂移
  轨迹随之对齐。asm 锚：空中横控=MoveSamusLeft/Right 同径（E626/E6A2）+速度清零
  仅地面（check_stop_horz_mvmt 的 SAMUS_ACCEL_Y 早退 rust 已有——缺的是速度写入面）。

## 7. 修复模型定稿（2026-10-31——int_x 积分器语义破译）

rust **已有**完整积分器 `horz_integrate`/`int_x`（integrators.rs:147——speed_i/f+subpix
三态+accel 积分+limit 钳制+disp 输出），**跳跃径未接线**（自有 2px 固定步进）。

- **limit 编码破译**：max_src=$12 ⇒ lim=$0120=1.125px/f（空中漂移上限=$FE.E0 ✓实测）；
  max_src=$18 ⇒ $0180=1.5px/f（地面走跑上限）——与 SetSamusJump 的 $18/$12 同族数值
  （gr_HIGHJUMP 分支 ldy #$18/#$12——asm D002 区）。
- **accel 字节**：$D0（左）/？（右）——frac 每帧 −0x30（实测 D0→A0→70 递减序列）。
- **修复接线**（跳跃空中径）：D055 已写 $0309←方向（在）→ 接 `horz_integrate`
  （accel=$D0/max=$12 写 $0315/max 变量）→ 按 out.disp 位移（替换固定步进）→
  solid_entity_response 的 accelX 此时=$D0（负）⇒ 贴墙早退不清零（速度保持 ✓）→
  落地清零径已有。被挡帧积分照常（速度保持上限）✓。
- **验收**：case-morphball-probe 墙跳窗 f1131-1180 逐帧（速度字节+位置）+角部
  f1164 消歧+全程 matched-seam；crawlprobe 回归（地面径不动）。
