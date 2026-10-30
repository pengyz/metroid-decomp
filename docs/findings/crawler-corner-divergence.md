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
> 【2026-10-30 归因闭合（源码阅读解）——用户指正：读 refs 源码即可，无需新仪器】
> T3 初值装载点=SamusInit（07:C8D1）：`lda #\$2C / sta Timer3`——**44×10=440 帧淡入**
> （m1disasm 原注「440 frames to fade in Samus (7.3 seconds)」）。crawlprobe 对照：
> T3←44 双侧仅差 1 帧（emu f358-work/rust f359-work），但首递减差 3 帧（f366/f363）
> ⇒差值三层 contributor：①boot T3=8 装载差 1 帧（rust f6/f7 vs emu f7/f8——boot 序）；
> ②\$29（TimerDelay）prescaler 相位在 T3←44 时刻已累积 2 帧（title 相各臂
> WaitNMIPass_ 内旋帧的 prelude 跳过 pattern 差累积——load→首递减距离 emu 7 帧/
> rust 5 帧实录）；③game-start 房建相结构差：emu MR=1(AreaInit) 占 f353-356 多帧
> +MR=2 f357-358，rust 0→2 单帧跳过（AreaInit/MoreInit 未建模为多帧相）。
> 修=按 asm 逐臂对齐内旋帧数与房建相帧数（source-reading 可完成，Lua 仪器备而
> 不用）——下一轮 porter 逐臂对齐。
> 【2026-10-30 转换臂 un-fuse 实验——证伪回滚】由 dec-gap 推演「emu 转换臂=3 无
> prelude 帧」假设，将 \$15|\$1B 臂拆回三段（去融合）——**门禁证伪**：case-001
> 锚定 match 集 6 点缩水（f550+ 282px 漂移，L4 FAIL 41 违规）+crawlprobe 像素
> 4/2→1/5。行距推演不足以定 emu prelude 帧集（采样沿语义干扰）；**既定仪器=
> oracle 侧 UpdateTimer（\$C266）调用迹 Lua hook**（逐帧 FC 记录）取真 prelude
> 帧集后再整形。已回滚融合形（r16 形），L4/PASS 恢复；T3 纪元 +3（boot +1+
> 转换窗 +2）留档为残余。另：BUG-091 落账（IsObjectVisible 竖滚径 radiusX/ScreenY
> 轴错配——用户裁后续可优化）。
> 【2026-10-30 完整帧图（TD 仪器测绘，逐帧会计工单基座）】emu 逐帧调用/自增表
> （watch 0029+FC+MR/TR，row=pre-work 语义）：
> boot: [f1 pre+0x00work][f2 wait(TD 恒)+inc+TR→1][f3 pre+0x01cursor0][f4 wait]
> [f5 seg TR→2][f6 seg TR→3+inc][f7 pre+fade...每帧 call+inc]
> 转换: [f310 pre+hit+seg0 冻][f311 冻][f312 seg+TR→0x16 冻][f313 pre+inc]
> game-start: [f350 pre+select][f351 InitializeStats 冻][f352 pre+AreaInit(MR→1)]
> [f353-358 MoreInit 内旋 冻][f359 pre+SamusInit(T3←44,MR→8)][f360+ intro 每帧]
> intro: 44 decs×10 帧（SamusInit T3←44 起）→T3==0⇒GameEngine。
> rust 对照（现态）：boot 少 0x01 内旋帧（0x00 spin 错位 f3 vs emu f2）；转换臂 2 tick
> vs emu 4；game-start span 冻 AreaInit（emu 带 prelude 调用）+MoreInit 内旋未建模
> （一帧直通）。**修=按此帧图重构 boot/转换/game-start 三窗的臂与 span 结构**——
> 已知约束：GameEngine 进入帧移动会平移 gameplay 时间线⇒锚定像素集需重立+基线
> 重锚（顺序依赖，需用户确认重锚）。
> 【2026-10-30 干净对拍（当前 HEAD 融合形，机械提取）】分歧地图（951 可比帧）：
> ①**TD(\$29) 相位差自 f2 起**（949 帧）——根=boot 首迭代定位：emu 首迭代跨 f1-f2
> （prelude@f1+内旋@f2+inc@f2 末），rust 合并唤醒 tick=f2 一帧完（prelude+work+inc），
> 0x00 SpinWait(1) 落 f3（emu 无此帧）——**boot ±1 帧**；
> ②**MR 差 3 帧**（f793-795）——GameEngine 进入 rust f793 vs emu f796=①boot ±1+
> 转换窗 ±2（0x1B 臂内旋 2 vs 3）+game-start 窗结构差累积；
> ③**SS（fade status）差 15 帧**（f493+/593+/683+/753+/793+ 簇）——fade 步进由 T3
> 阈值驱动，T3 纪元偏移⇒步进帧偏移——**②的下游投影**；
> ④**FC 差仅 f916+**（尾部 45 帧）——f916/917 emu 跳帧（尾部 0x?? 内旋）rust 未建模
> ——次要面。
> 修序（依赖序）：①boot 首迭代定位（0x00 内旋帧移到 inc 前）→②转换臂内旋帧数
> →③game-start span 拆分——每步以 TD/FC/MR 逐帧对拍验收。**方法论定谳：手工帧
> 算术在本域不可靠（采样沿语义三次干扰），一切以机械对拍脚本输出为准。**
> 【2026-10-30 新鲜数据复核（重生成 rust 产物后）】分歧地图**确认为真**（非过期
> 产物）：逐帧原始行实证——**rust 整个 MainLoop 流晚 1 帧启动**（emu 首个 prelude
> 调用@f1，rust@f2——boot tick1=PreSpin 占帧、首迭代在 tick2），全流程逐帧平移
> +1（TD 相位差自 f2、949 帧实测）；叠加 title 各臂内旋帧数差（0x00 spin 1 帧、
> 0x01 内旋 2 帧、0x1B 内旋 2-3 帧 vs emu 对应值）=GameEngine 进入 +3。**修=boot
> 首迭代前移 1 帧（tick1 即 NMI+boot_post+首迭代，消 PreSpin 独占帧——r14 merged
> 唤醒语义保持）+各臂内旋帧数按 emu 帧表对齐**——动 boot 时序风险高（r14 调优史），
> 须逐步机械对拍验收。
> 【实施工单（下一会话直开工，按序每步机械对拍验收）】
> S1 boot 合帧：engine.rs PreSpin 臂合并——tick1=boot_pre_spin+NMI#1(nmi_handler)+
>    boot_post_spin+main_loop_iter（=emu f1 全帧：pre+work），消 PreSpin 独占帧；
>    AwaitNmi 相位删（restore 路径 skip_next_nmi 语义核验保留）。
> S2 桥旗规则：0x00 入冻结族（emu f1 无 inc 实录）；spin 分支末尾 spin==0（最后
>    一旋）⇒no_return=false（完成帧带 inc——emu 0x00 f2 末 inc 实录）。
> S3 0x01 臂 4 tick 重构：cursor0(f3 冻)+SpinWait(1)(f4 冻)+cursor1(f5 冻 TR→2)
>    +cursor2(f6 TR→3+inc 不冻=完成帧)——现 2 段融合拆分。
> S4 0x1B 臂 3 段全冻（un-fuse 恢复+族旗全冻——S1/S2 落地后 un-fuse 的 282px
>    失败因素即消）。
> S5 game-start span 拆分：InitializeStats 冻帧+AreaInit 带 prelude tick+MoreInit
>    内旋 span(6)+SamusInit 带 prelude——span 触发点后移 1 tick。
> S6 验收链（每步）：crawlprobe 机械对拍（TD/FC/MR/TR/SS 逐帧）→S 全部后：
>    case-001 敌槽轨迹全窗同值→L4 全量重采+锚定集重立（**需用户确认重锚**）→
>    基线重锚→复核档。
> 【2026-10-30 S1 首次试验记录（已回滚）】boot 合帧首版（PreSpin 分支内联 NMI+
> boot_post+首迭代）实测：tick1 即跑 area_init+more_init（GameMode=0 时 GAME 路径
> 分派早于 title 建立——boot 相位/GameMode/flag 三者交互比模型复杂，\$0001=0x67
> 断言红），421 测中 1 红——**已回滚恢复绿**（421/0，CHECK PASS）。教训：S1 需要
> boot 相位与 GameMode 建立顺序的先行梳理，非简单分支内联；S2-S6 同理须在完整
> 帧图指导下整轮实施（新会话）——工单与全部实测数据已备齐（本文档）。

## 5. 证据指针

- prg7_engine.asm:7991/7999/8007/8218/8303/8310（五路由+两共享核）
- areas_common.asm:98-107（CrawlerMovementRoutinesTable——四方向映射与 rust idx 一致）
- enemies/crawler.asm（内角/外角/翻转/Reorient 全文——rust 结构已忠实）
- mainloop.rs:865-925（crawler_ai）/1154-1236（移动+碰撞检）
- case-001 双腿 ram.hex（f810-f1200 敌槽值——本档 §1）

> 【2026-10-30 帧骨架对齐后终判（frame-alignment-plan Steps 0-4 落地——cf7b999b）】
> **首分叉判据闭合**：crawlprobe matched-seam 下 slot4/5 激活 d5 序列双侧逐帧全同——
> 42@f797（bit1 可见）/1A@f798（bit3 距离阈）/5A@f799 起交替，GameEngine 进入差
> （MR f793-795 窗）消失，激活时机**同帧**。上游因果链（F5 单因果：TD 相位差 3=
> 0x01 窗 +1+0x1B 窗 +2）由帧骨架对齐根治，本档 §2-§3 的激活时机面不再有独立缺陷。
> fcprobe 5 字段×445 帧全同；crawlprobe FC/TD/MR/TR/SS 主线全同（残余仅 f916-917
> emu 冻结投影——frame-alignment-map-v2.md §7）。
> **新残差（激活下游，另案）**：行走路径分叉——emu slot4 沿天花板纯 x 步进（y=28 恒，
> 6/8 门控 x+1/2帧：BC→BD@f799→…→C3 hold@f807-809→C6…）；rust 自 f797 首步即
> **双轴对角移动**（y+x 同帧 +1：28BC→29BD→2ABE）随后围 2-3 块小回路（y 29→2B→29、
> x BD→BE→BC 振荡）——疑似转向核/碰撞检在 rust 把天花板行走误判为转角下壁（对角
> 双步非合法爬步）。d5 状态面双侧全同（1A/5A 交替节奏一致）——分歧在位移核不在
> 状态机。卷面：case-crawlprobe 敌槽 watch（$0440-$045F）逐帧可复现。
> 【勘误预告】本档 S2 工单（0x00 spin 1 帧）与 S1 合帧首版均被 frame-alignment-plan.md
> §2 对抗分析推翻/重定义——以该档 F1-F10 与 map-v2 图谱为准。
> 【2026-10-30 更正（verifier 复核 C.3 证伪——原地标注）】上条终判补注中"rust 无死
> ⇒无爆炸⇒无冻"句**作废**：em u 爬虫未被击杀（d5 1A/5A 交替持续；被误读的 B6→B3
> 倒计数实为 EnsExtra slot4+$0D jumpDsplcmnt），f916-917 冻结与**首次横滚**精确同窗
> （SX 双侧 f915=0→f918 非零）——本档 :91 原假设（首次横滚 ScrollRight VRAM）恢复
> 为首选。激活终判闭合与行走路径残差两面不变。
