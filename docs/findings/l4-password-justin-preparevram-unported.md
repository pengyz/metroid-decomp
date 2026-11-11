# L4 case-password-justin 双侧对拍归因——密码屏 VRAM 装载径 unported 缺口阻塞（卡 C）

日期：2026-11-10 ｜ 角色：porter（改写工程师）｜ 状态：~~归因落档（不修复，报回 lead 派修）~~ → 【2026-11-11 更正】已修复并端口完（见文末更正节）

> 【2026-11-11 更正（AGENTS §3.6 原地标注，不删史）】本文初稿结论「10 点全 blocked、
> 不入 ratchet、报回 lead 派修」是**卡 C 起步时（2026-11-10）的快照**，已被后续修复推翻：
> `prepare_vram` 补 `0x99E3`/`0x93B9` 两臂解锁 6 格；随后 unported 链
> `SoundEngine_SilenceMusic($B3EB)` → `BankInitTable idx2($C51F)` →
> `gfx_block/C6E0` → `SamusInit ScrollDir($C902)` → `PalettePtrTable($C1FF-idx)`
> 逐层端口完，blocked **清零**（现态 2 match / 10 diff / 0 blocked，110 格分母）。
> 剩余 10 格 diff 是**真像素差**（原被 blocked 掩盖），非缺口——归因见本档续记与
> `/tmp/card-c-wip.patch` 大件（竖镜三修/LoadSamusGFX/淡入修/per-area 图形表，
> 属卡 C 后续波，逐件重生收敛）。本档历史量表保留作证据，不作现状判据。

## 结论（TL;DR）

新用例 `cases/case-password-justin.txt`（JUSTIN BAILEY 密码进场，frames 2600，12 显式采集点
+capture_every 30 game）双腿对拍：**title/menu 2 点 0px 全同；其余 10 点被 Rust 腿 unported
缺口阻塞（非像素 diff）**——密码屏 VRAM 装载地址 `0x99E3` 在
`crates/game-core/src/frame/title_bridge.rs:217` `prepare_vram` 的 match 表缺失，
命中 `unported!` fail-loud 桩，帧 504 起渲染链断裂。**不入 ratchet**（blocked 集合只减不增，
本用例 10 个 blocked 采集点不能入库）；归因落档等修复后重跑再议 13→14。

## 逐采集点量表（2026-11-10 run，锚 scenario=b2146fc8… sched=2e8c1e8f… rom=e6e6b701…）

| 帧 | tag | 状态 | diff px |
|---|---|---|---|
| 320 | title | ✅ match | 0 / 61440 |
| 420 | menu | ✅ match | 0 / 61440 |
| 530 | pwd-screen | ⛔ blocked（unported gap @f504） | — |
| 700/850/950/1100 | mid-1..4 | ⛔ blocked（同上） | — |
| 1150 | pw-done | ⛔ blocked | — |
| 1190 | submit | ⛔ blocked | — |
| 1300 | post-submit | ⛔ blocked | — |
| 1430 | spawn-t4s | ⛔ blocked | — |
| 2390 | spawn-t20s | ⛔ blocked | — |

汇总：一致 2 · 失配 0 · 缺口阻塞 10（共 12）。emu 腿本身完整（2600 帧/97 采集/wall 54.7s）。

## 根因（B 级代码语境 + A 级地址锚）

- asm：`LoadPasswordScreen` 00:911A（refs/m1disasm/SRC/prg0_title.asm:2394）：
  `ldx #<L99E3 / ldy #>L99E3 / jsr PreparePPUProcess_`（:2399-2401）；
  L99E3 定义在 prg0_title.asm:3560 = `VRAMStructData $2088, "PASS WORD PLEASE"`（NES_NTSC 支）
  + `VRAMStructDataRepeat $23C0, $10` 属性表清零。
- Rust：`prepare_vram`（crates/game-core/src/frame/title_bridge.rs:210-218）match 仅
  `0x82F4`（INTRO_BG）/`0x9984`（START_CONTINUE，r25 补）两臂；`0x99E3` 落 `other` 臂
  `crate::unported!(&format!("bank00:{other:04X}-PrepareVram"))`（:217）。
- 触发帧 504：密码屏装载（标题相位推进至密码入口）——与卡 A 帧号契约吻合
  （0x18@f501→0x18@f505 屏稳 f516，装载先于屏稳）。
- 层定位（动手前三问）：问题在 **Rust 移植面缺口**（game-data 未烘焙 L99E3 VRAM struct
  + title_bridge match 未补臂），非 oracle/门禁/asm 投影问题。

## emu 腿出生态契约复核（A 级 RAM 实测，与卡 A 契约全数一致）

analysis/traces/case-password-justin.frames.jsonl：

| 帧 | $6878 Energy | $6879 Missiles | $69B3 JustInBailey | $74 InArea | $6988 PWByte00 |
|---|---|---|---|---|---|
| 1430 | 127(7F) | 255(FF) | 1 | 17(0x11 Norfair) | 166(0xA6 'J') |
| 1631 | 127 | 255 | 1 | 17 | 166 |
| 2390 | 127 | 255 | 1 | 17 | 166 |

## 处置

1. **不修**（porter 写面限定：crates/game-core/ 逻辑禁碰）——报回 lead 派修：
   修复面=game-data 烘焙 L99E3 struct（tools/bake_vram_structs.py 扩）+
   title_bridge.rs prepare_vram 补 `0x99E3` 臂（可能连带 InitPasswordFontGFX/
   DisplayInputCharacters 深件，见 title_bridge.rs:200-204 注释「未烘焙面随件爆」）。
2. 修复后重跑 `python3 tools/l4_diff_run.py run case-password-justin`：
   - 若 12 点全 0px → sanctioned 上调（`python3 tools/baseline_update.py l4 --reason …`）13→14；
   - 若仍有像素差 → 本档续记归因（diff 点不进 match 集）。
3. run-all 矩阵口径：本用例记 12 分母（2 match + 10 blocked，blocked 计分母不计 pass）。

Evidence: 00:911A/00:99E3 asm 锚（prg0_title.asm:2394/2399-2401/3560）；
title_bridge.rs:217 unported 桩；l4report 锚 scenario=b2146fc8…/sched=2e8c1e8f…；
emu trace f1430/f1631/f2390 RAM 实测。
Confidence: 90
