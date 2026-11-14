# 输入窗相位重锚：440 帧开场演出吞掉按键（A 方案）

> 2026-11-14。起因：为 #3 电梯补 L4 采集点时发现「注入后 Samus 不动」，顺查得知
> 既有 14 案的动作输入**全部落在开场演出窗内**——矩阵的「动作覆盖」此前是空的。
> 用户裁定方案 A（m17683）：把输入窗整体后移到演出之后，属结构性重锚，走
> `baseline_update.py l4 --rebase` sanctioned 路径。

## 1. 现象与定谳

`analysis/traces/case-001-…frames.jsonl` 实录：`SamusX` 在 f359 起恒 `128`，直到
**f902**；f903 才变（129/130/131/133…，`status=1` Run）。而 case-001 的输入是
`hold 500-550 right` / `hold 600-650 left` / `press 700 a` / `press 800 b`——全在窗内，
两侧都「不动」，于是这些格子的 0px 一致只是**两侧同样没有反应**。

asm 依据（A 级）：

| 位置 | 内容 |
|---|---|
| `07:C8D1 SamusInit` | `lda #_id_SamusIntro / sta MainRoutine`；NTSC `lda #$2C / sta Timer3`；`jsr IntroMusic` |
| `07:C266 UpdateTimer` | `TimerDelay` 每 10 帧减 1 时把 `Timer3` 减 1 ⇒ `$2C`×10 = **440 帧** |
| `07:C9D7 SamusIntro` | `Timer3==0` ⇒ `MainRoutine←_id_GameEngine`；非 0 时 `cmp #$1F / bcs RTS`（前 310 帧连 Samus 都不画）+ `SamusFadeInTimeTbl`（`07:CA13` = `1E 14 0B 04 FF`）逐级换调色板 |
| `07:CA13 SamusFadeInTimeTbl` | 淡入档位（与上同址，表在 SamusIntro 尾） |

结论：这不是移植缺陷，是 ROM 的真实时序。f359 进游戏、f~799–903 才进 `GameEngine`。

## 2. 重锚做法

一次性脚本（不入库）对 12 个「游戏内动作」案：

- `KEEP_UNTIL=420`：`press 310/350 start`、`capture 420 game-in` 不动（标题屏段）。
- `S = 910 − 首动作帧`；把 >420 的 `press/hold/capture/endFrame/fixture` 与 `frames`
  一律平移 S；注释里的 `帧 N`、`fN`、`capture N` 同步换算（防 stale 注释）。
- 每文件首行加 `# [A-重锚] …` 标记（**必须带 `#`**，否则被调度解析器当未知 DSL 指令）。

| 案 | 平移 S | 重锚后 frames | 首动作 |
|---|---|---|---|
| case-001-game-start-samus-interaction | +410 | 1610 | 910 |
| case-002-door-blue-interaction | +460 | 1660 | 910 |
| case-collide-probe | +470 | 1370 | 910 |
| case-crawlprobe | +410 | 1370 | 910 |
| case-jumpledge-probe | +460 | 1460 | 910 |
| case-killenemy-probe | +460 | 1560 | 910 |
| case-leftwalk-probe | +470 | 1270 | 910 |
| case-leftwalk-probe2 | +470 | 1870 | 910 |
| case-morphball-probe | +460 | 2160 | 910 |
| case-scrollseam-probe | +480 | 1380 | 910 |
| case-shootprobe-probe | +460 | 1360 | 910 |
| case-walkfcprobe | +470 | 1870 | 910 |

不动的三案：`case-elev-norfair-transition`（本来就 post-intro）、`case-fcprobe`（相位
探针，只有 `capture 420`）、`case-password-justin`（标题/密码屏流）。

## 3. 重锚立刻暴露的掩盖分歧（22 格）

重锚前 122 格 / match 120 / diff 2；重锚后 **132 格 / match 108 / diff 24**。
新增 10 格里 2 格一致、8 格失配；另有既有格由「一致」翻成失配——合计 **22 格真实分歧**：

| 案 | 失配格（px） |
|---|---|
| case-002-door-blue-interaction | 1110=252、1160=10236、1250=10474、1310=10474、1460=11015、1610=11159 |
| case-jumpledge-probe | 1150=368、1165=126、1190=9087、1215=10018、1240=10197、1260=10702 |
| case-scrollseam-probe | 1100=642、1130=252、1160=252、1180=184、1280=126、1320=184、1360=126 |
| case-morphball-probe | 2080=8、2110=8、2160=8 |
| case-password-justin | f700=23、f1100=42（既有挂账 `oracle-present-pipeline-lag`） |

全 0px 的案（重锚后仍全绿，说明其动作真的被执行了）：case-001 13/13、collide 7/7、
crawl 6/6、elev 10/10、fcprobe 2/2、killenemy 7/7、leftwalk 6/6、leftwalk2 7/7、
shoot 4/4、walkfc 5/5。

## 4. 由重锚暴露并已修的两处真错（commit `a000dc09`）

1. **`uw_all_doors` 是空壳**：`frame/room.rs::update_all_doors`（`07:8B79` 四门环）实现与
   单测齐备，但 `frame/mainloop.rs::uw_all_doors` 空实现、**从无 `jsr` 点**。asm
   `prg7_engine.asm:2510 jsr UpdateAllDoors`。后果：门对象从不初始化/更新（`case-002`
   f1110 emu 门 status=2/animFrame=`$31` vs rust 永停 status=1 全零）。
2. **`update_door` 缺 `stx PageIndex`**（`07:8B87`）：`DrawDoor→AnimDrawObject→
   UpdateObjAnim`（`07:DC8F`）用 `ldx PageIndex`（`$004B`）取槽；缺写则动画推进打到
   上一个 `PageIndex` 的槽，门自己 `animFrame/animDelay` 不动、半径取错。补后门槽
   16 字节与 emu 逐字节相等（`02 18 08 31 04 30 31 01 00 00 00 00 00 68 f0 01`）。
   两条既有单测原本把 bug 钉成期望（`animIndex=$30`/`delay=0`），已按 asm 更正为
   `$31`/`$06`/`animFrame=ObjectAnimIndexTbl[$30]`。

修完后像素分歧**未减少**（24 格不变）⇒ 残余分歧与门链无关。

## 5. 残余分歧的初判（未修）

以 `case-jumpledge-probe` f1150（368px，bbox `x[23,96] y[124,207]`）为例：

- 滚动/位置面**全等**：`SCROLL_X=4C`、`SCROLL_Y=00`、`MAP_POS=(05,0E)`、`SamusY=C1`、
  `SamusScrY=C1`、`SCROLL_DIR=03`。
- 差在敌人：En 槽 2（`$0420`）emu `y=C4 x=A2 data05=16` vs rust `y=88 x=64 data05=1B`
  —— rust 侧这只爬行敌**自出生起位置不变**（悬在半空），且 `data05` 位不同。
- `EnsExtra`（`$6AF4+16i`）：`slot4/slot5` emu `status=00` vs rust `02`；`slot2`
  f1190 emu `00`（已消失）vs rust `02` ⇒ **rust 敌生命周期/清除面缺失**。
- `case-002` f1160 起的万像素级分歧 bbox 覆盖全屏、按 16×16 块统计第 13 行整行 ~160/块
  ⇒ 疑整屏/滚动面，尚未定位。

初步分类：①敌移动 AI（`En` 位置/朝向不动）②敌生命周期（不 despawn）③某整屏面
④morphball 8px 小面。

## 6. 口径与后续

- 本重锚是 sanctioned rebase（`kind=rebase` + evidence 双留痕），理由：采集面结构性
  变更是**判据改进**，不是行为回退；既有 110 格无一回退，2 格（password-justin）仍
  是已挂账的呈现管线滞后。
- 后续每修一类分歧：单案 L4 → 全矩阵 → 修一格也是真收获；不得以「反正 24 格」为由
  调阈值或加白名单。
- 教训：**0px 一致只证明「两侧同一种走法」，不证明「这段代码被走到」**。本次之前，
  14 案的动作证据全为空对空——与 `shootprobe`/`scrollseam` 同类问题第四次复发。今后
  新 case 的输入窗必须避开 `MainRoutine==8` 区间（`Timer3` 耗尽前）。
