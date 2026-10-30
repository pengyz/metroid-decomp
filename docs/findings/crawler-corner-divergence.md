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
> **部分达成**：slot4 f815-825 x 对齐（190→200=emu 同值）；slot5 与 f950+ 角检
> 仍分歧——**残余面=转向决策的更深层**（内角/外角/翻转链的输入差异），定位需
> f735-f820 窗逐帧 trace（crawlprobe：watch_range 0440-044F+data0A/data05/FC
> 双侧逐帧）——待续考证。门禁 check PASS=13 FAIL=0，L4 76 格零缩水。

## 5. 证据指针

- prg7_engine.asm:7991/7999/8007/8218/8303/8310（五路由+两共享核）
- areas_common.asm:98-107（CrawlerMovementRoutinesTable——四方向映射与 rust idx 一致）
- enemies/crawler.asm（内角/外角/翻转/Reorient 全文——rust 结构已忠实）
- mainloop.rs:865-925（crawler_ai）/1154-1236（移动+碰撞检）
- case-001 双腿 ram.hex（f810-f1200 敌槽值——本档 §1）
