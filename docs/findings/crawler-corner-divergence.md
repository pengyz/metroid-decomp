# 爬行转向/撞角分歧考证——敌移动碰撞检的单点判缺口（r9 后续）

- 日期：2026-10-29
- 状态：**根因面已钉（asm A 级+rust 代码读证）**；修= porter 轮（本档不修）
- 前置：FC 冻结修（findings/fc-phase-residual.md §5，db745dbd）落地后爬行轨迹仍分歧
  （case-001 f810 起：rust 190/190/190/191 vs emu 196/200/200/200；f950 起路径分叉
  扩大——FC 判据一已达成，本档为**残余分歧的独立面**）。

## 1. 现象

case-001 双侧爬虫（slot4/5）FC 对齐后仍分歧：f735 前双侧同静（188/40）；f735-f810
无采集窗内路径分叉；f810+ emu 右移+下行（y 40→48→141），rust 慢速（190→191）后
右逃（236）。转角时机与方向选择系统性不同。

## 2. 根因面（asm A 级 + rust 读证）

**asm：敌移动碰撞检全部经共享全距核**（与萨姆斯侧 CheckMoveVertical $E7BD 同族）：

| 方向 | 例程 | 路由 | 走环 |
|---|---|---|---|
| 上 | EnemyCheckMoveUp（07:E770，prg7_engine.asm:7991） | DistToCenterY=ry+8 → EnemyCheckMoveVertical（07:E783:8007）→ CheckMoveVertical | **x 全距** |
| 下 | EnemyCheckMoveDown（07:E77B:7999） | DistToCenterY=−ry → 同上 | **x 全距** |
| 右 | EnemyCheckMoveRight（07:E8FC:8303） | DistToCenterX=−rx → EnemyCheckMoveHorizontalBranch（07:E904:8310）→ CheckMoveHorizontal（07:E89B:8218） | **y 全距**（顶界=y−ry 余块+1/GetNumBlocksToCheck/LE7DE 共享环，YOffset=8） |
| 左 | （对称） | 同上 | **y 全距** |

**rust：四方向全为单点判**（mainloop.rs）：
- `enemy_check_move_right`（:1229）单点 `(y, x+rx+8)`；
- `enemy_check_move_up`（:1199）单点 `(y−ry+8, x)`；
- down/left 同族单点。

爬虫 radius 跨块（帧记字 radiusY/X 非 0）时单点判漏探相邻行/列块 ⇒ 碰撞/探针
结果分歧 ⇒ 内角（orientation−1）/外角（orientation+1 探移）转角决策分歧 ⇒ 轨迹
分叉。**与萨姆斯侧问题 B（c3315af 上行臂）/①（2e7e5435 下行臂）完全同族——敌人侧
尚未迁移全距核**。

## 3. 附带疑点（假设区，修时一并核）

1. `enemy_check_move_up` 公式 `y−ry+8` vs asm 探点 `y−(ry+8)=y−ry−8`——差 16px
   （rust 疑把 DistToCenterY 减法方向写反；对照 down 变体定裁）。
2. `enemy_move_one_pixel_right` 边缘路径：asm x==0 名表切换流（ScrollDir/ScrollX/
   SwitchEnemyNameTable）rust 缺；asm 右界==\$FF 检 rust 条件差一像素（wrap==0）；
   成功径 jumpDsplcmnt 递减（data05 bit7）缺——case-001 房间触不到，随件补。
3. `crawler_ai` gate==0 分支漏 `update_enemy_anim(3)`（asm @exit 两支共享 3 帧动画
   门）——动画面非轨迹面。
4. \$1C GameOver 臂/0x02 fade_delay 臂单帧差——见 fc-phase-residual.md §5 假设区。

## 4. 修（porter 轮处方）

四方向碰撞检迁移全距核：纵臂复用 `vertical_span_cols`（x 走环）、横臂复用
`horizontal_span_rows`（y 走环，physics.rs 已有萨姆斯侧同构件）；判点公式对照
asm（DistToCenterY/X 符号、+8 块距）；mutation 测=跨块 radius 单点漏探场景
（侧块/上下块碰撞单点形必红）。修后判据=case-001 敌槽轨迹双侧同值（f810-f950
窗）+L4 零缩水。

> 【2026-10-29 落地补注（b72b8f90）】修已落地：四检改走 enemy::movement 既有
> staging（DistToCenter 公式逐字节忠实——该 L2 端口本就存在但未接线）+
> check_move_boundary+span 构件复用（physics.rs 两 helper 转 pub(crate)）。判据
> **部分达成**：slot4 f815-825 x 对齐；slot5 与 f950+ 角检仍分歧（见下注）。
> 【2026-10-30 逐帧定位补注（case-crawlprobe）】全距核修后残余面已用逐帧 trace 钉到
> **激活时机**：crawlprobe（case-001 同相至 f960，watch_range 0440-045F+FC）双侧 952
> 可比帧——首分叉 **f794**：rust 双爬虫激活爬行（s4 d5=1A=bit1 可见+bit3 距离阈同帧，
> dA=01 下壁+外角探移复合位移），emu 静止至 f796、f797 行 bit1 置（d5=42）、f798 行
> bit3 置（d5=1A）、f799 行起同速右移（6/8 门同拍）。即 **rust 早 3-5 帧激活**，激活后
> 速率/门一致。激活链（asm）：UpdateEnemy_CheckIfVisible（07:F37F——fresh 不可见⇒
> pla pla 整体跳过）→UpdateEnemy_Resting（07:F3BE——bit6 30FPS 半率门=(slot/16 xor
> FC)&0→体跑）→OrEnData05 $18→距离阈 F75B（不可见⇒清 $18）→TryBecomingActive
> （07:F7BA——delay 递减归零 **且** bit3⇒status++）。rust 激活帧 d5=1A=bit1/bit3 同帧、
> emu 分两帧且晚 3 帧——可见闩（draw_enemy bit1）/延迟倒数节拍/距离阈判的时序交织
> 待下一轮专项定位（本档不修）。产件：cases/case-crawlprobe.txt+双侧逐帧
> jsonl/state.jsonl（analysis/traces+shots_rust）。
> 【2026-10-30 续挖（watch 扩展 00FC-030E+6B34+002C/001E）】激活门解体为**因果四层**：
> ①敌槽 status=Resting(01) 全程在位、delay=00 就绪（非空槽非延迟面）；
> ②UpdateEnemy 被 CheckIfVisible fresh 不可见整体弹回（d5 bit6 不翻转实证）；
> ③可见翻转=**GameEngine 进入时机**：rust MainRoutine→3 @f793、emu @f796（+3 帧）；
> ④intro（MR=8，f360 起）完成条件=Timer3==0，而 T3 由 UpdateTimer（07:C266）
> **TimerDelay 10 帧 prescale** 递减（asm `dec TimerDelay/bpl`+归零重置 9）——
> 实测递减纪元：首递减 emu f16 vs rust f15（**差 1 帧**）、T3 曾于 f86-f216 归零
> 闲置（更新器零值跳过+重载）、至 intro 末偏移长到 3 帧（递减计数 f360-800 双侧
> 各 44 次相同——纯相位非速率）。逐帧会计（title 相 MR=1/2 窗的 update_timer
> 执行差）=下轮定位面。
> 【2026-10-30 转换臂 un-fuse 实验——证伪回滚】由 dec-gap 推演「emu 转换臂=3 无
> prelude 帧」假设，将 \$15|\$1B 臂拆回三段（去融合）——**门禁证伪**：case-001
> 锚定 match 集 6 点缩水（f550+ 282px 漂移，L4 FAIL 41 违规）+crawlprobe 像素
> 4/2→1/5。行距推演不足以定 emu prelude 帧集（采样沿语义干扰）；**既定仪器=
> oracle 侧 UpdateTimer（\$C266）调用迹 Lua hook**（逐帧 FC 记录）取真 prelude
> 帧集后再整形。已回滚融合形（r16 形），L4/PASS 恢复；T3 纪元 +3（boot +1+
> 转换窗 +2）留档为残余。另：BUG-091 落账（IsObjectVisible 竖滚径 radiusX/ScreenY
> 轴错配——用户裁后续可优化）。

## 5. 证据指针

- prg7_engine.asm:7991/7999/8007/8218/8303/8310（五路由+两共享核）
- areas_common.asm:98-107（CrawlerMovementRoutinesTable——四方向映射与 rust idx 一致）
- enemies/crawler.asm（内角/外角/翻转/Reorient 全文——rust 结构已忠实）
- mainloop.rs:865-925（crawler_ai）/1154-1236（移动+碰撞检）
- case-001 双腿 ram.hex（f810-f1200 敌槽值——本档 §1）
