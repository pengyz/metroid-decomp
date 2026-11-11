# R9 独立差分复核：卡 C JUSTIN BAILEY 密码进场 unported 全链端口（commit `87e48ca2`）

日期：2026-11-11 ｜ 角色：verifier（独立差分验证员，**证伪立场**）｜ 复核对象：单 commit `87e48ca2`（父 `560bbbac`，已 push origin/master）

**verdict = `needs_revision`**

理由（两条可复现证伪，均 A 级）：

- **F1**：`crates/game-core/src/frame/nmi.rs:724` 的 area→bank 映射与 ROM `BankTable(07:CA30)` 不符（Kraid/Tourian 互换）。该行本身是既有代码，但**本 commit 恰好改了同一函数**（守卫 `idx>=25` → `idx >= offs.len()`，并声称「贴 asm 无界查表」的 per-bank 忠实），改动把 Kraid/Tourian 区 `idx25..27` 从 **fail-loud `unported!` 爆口**变成**静默读错 bank 的调色板**——违反 AGENTS D7「缺口确定性爆在 bank:addr」的 fail-loud 纪律，且 L4 无 Kraid/Tourian 用例、门禁不可见。
- **F2**：`data/status/baselines/baseline_history.jsonl` 本 commit 新增条目 reason 含**事实错误**：「两个未接线孤儿图形件已移出工作树（防全新 clone 掉测）」——本 commit **零文件删除**，孤儿件 `crates/game-core/src/render/sprite.rs`（4 个 `#[test]`）**仍在树中且仍不编译**。ratchet 基线的变更理由必须可核（AGENTS §9 `data/status/` 改动需证据），该句不可核且与仓库状态矛盾。

其余 A/B/C/D 逐点结论见下；除 F1/F2 外，声称的 7 条主体（端口链、烘焙、锚定、findings 更正）**均证伪不成立（即属实）**，`gates/cli.py check` 实测 PASS=13 FAIL=0，`l4_ratchet_violations(default)` 实测 0 违规，ROM sha256 实测一致。

---

## A. asm/ROM 逐行对读

### A1 sound 静音径（$B3EB）—— ✓ 属实（1 处 B 级注释瑕疵）

asm 原文（`refs/m1disasm/SRC/sound_engine.asm`）：

```
349:SoundEngine_GameIsPaused: ; $B399
407:ClearInitFlags: ; $B3D6
417:SoundEngine_SilenceMusic: ; $B3EB
```

（commit Evidence 写 `:415`，实为 `:417`；`ClearInitFlags` 实为 `:407`（任务卡写 :405）——行号漂移 2~6 行，不影响结论。）

`:392` `lda SFXNoiseInitFlags / lsr / bcs SoundEngine_SilenceMusic` ↔ rust `crates/game-core/src/frame/nmi.rs:224` `if mem.read_ram(crate::ram_map::SFX_NOISE_INIT) & 1 != 0`（lsr+bcs ≡ bit0 检查）。

复用函数实读（非信注释）：`crates/game-core/src/sound/mod.rs:1259`

```rust
pub fn initialize_sound_addresses() -> (Vec<(u16, u8)>, Vec<(u16, u8)>) {
```

返回 14 条 RAM：`0x0653,0x0654,0x0655,0x0607,0x0688,0x0689,0x068A,0x068B,0x068C,0x068D` + `0x062D,0x0602,0x065D,0x062C`；5 条 APU：`0x4000←0x10,0x4004←0x10,0x400C←0x10,0x4008←0x00,0x4011←0x00`。

与 asm 三子件逐址对上：
- ClearMusicAndSFXAddresses 十清 `SQ1UsedBySFX($0653)/SQ2UsedBySFX($0654)/TriUsedBySFX($0655)/LoadMusicSQ1SQ2PeriodsFlag($0607)/SFXNoiseContFlags($0688)/SFXSQ1($0689)/SFXSQ2($068A)/SFXTri($068B)/SFXMulti($068C)/MusicCont($068D)` ✓
- ClearSounds 五 APU `lda #$10 sta SQ1_VOL($4000)/SQ2_VOL($4004)/NOISE_VOL($400C); lda #$00 sta TRI_LINEAR($4008)/DMC_RAW($4011)` ✓
- ClearSpecialAddresses 四清 `MusicTriCounterCntrl($062D)/SFXPaused($0602)/MusicLoopFlags($065D)/MusicLoopsOnEnd($062C)` ✓
- ClearInitFlags 六清 `$0680-$0685` ✓（rust 抽出 `fn clear_init_flags`，正常径与静音径共用，与 asm「两入口共用」一致）

`beq ClearInitFlags` 恒走：ClearSpecialAddresses 尾 `lda #$00` 后全是 STA（不改标志）+ rts（不改标志）⇒ Z=1 ✓。

B 级瑕疵（非阻塞）：asm 执行序=十清→五 APU→四清→六清；rust 序=14 RAM→5 APU→六清（四清提前到 APU 之前）。跨域无标志依赖、无中途读标志，语义等价；但 `nmi.rs` 注释里 `SFXPaused` 被归到「ClearInitFlags 旗」属注释不准（`$0602` 实由 ClearSpecialAddresses 清）。

### A2 prepare_vram 两臂（$99E3 / $93B9）—— ✓ 属实（烘焙字节逐字节 = ROM）

bank0 file off = `16 + bank*0x4000 + (cpu-0x8000)`：

| 地址 | file off | ROM 字节 | 烘焙常量 | 结论 |
|---|---|---|---|---|
| 00:99E3 | 0x19F3 | `20 88 10 19 0A 1C 1C FF 20 18 1B 0D FF 19 15 0E 0A 1C 0E 23 C0 50 00 23 D0 48 55 23 D8 60 FF 23 DA 44 F0 00`（36B） | `PASSWORD_PLEASE_VRAM_STRUCT` | **逐字节一致** |
| 00:93B9 | 0x13C9 | `21 8C 09 10 0A 16 0E FF 18 1F 0E 1B 00`（13B） | `GAME_OVER_VRAM_STRUCT` | **逐字节一致** |
| 00:9984 | 0x1994 | `21 8C 05 1C 1D 0A 1B 1D 21 EC 08 0C 18 17 1D 12 17 1E 0E 00`（20B） | `START_CONTINUE_VRAM_STRUCT` | **逐字节一致**（既有） |

ctrl 解码 `0x50/0x48/0x60/0x44` = RLE+16/+8/+32/+4，与 asm 注释 `$23C0×$10 $00 / $23D0×$08 $55 / $23D8×$20 $FF / $23DA×$04 $F0` 吻合（`vram.rs` 解码器 `ctrl&0x3F`，0⇒256）。

`crates/game-core/src/frame/title_bridge.rs` 新增臂：

```
0x99E3 => super::vram::vram_struct_write(mem, &game_data::title::vram_structs::PASSWORD_PLEASE_VRAM_STRUCT),
0x93B9 => super::vram::vram_struct_write(mem, &game_data::title::vram_structs::GAME_OVER_VRAM_STRUCT),
other => crate::unported!(&format!("bank00:{other:04X}-PrepareVram")),
```

### A3 BankInitTable / InitBank2 / InitNorfairGFX / gfx 描述子 —— ✓ 属实

`prg7_engine.asm:1363` 9 项 ROM 实测 07:C51F = `$C531 $C552 $C583 $C590 $C5B6 $C5C3 $C45C $C45C $C45C`，与 asm 逐行一致；`frame/boot.rs` `2 => init_bank2(mem)` 只补 idx2（0/1 既有，`_ => unported!("bank00:C51F")` 兜底保留）✓。

`prg7_engine.asm:1441 InitBank2` = `lda #$00 / sta GameMode / jsr ScreenNmiOff($C45D) / jsr InitNorfairGFX($C622) / jmp NMIOn($C487)` ↔ rust `init_bank2`：`GAME_MODE=0`、`NMI_STATUS=1`、`iter_wait()`、`init_norfair_gfx(mem)`、`iter_bogged(2)` —— 与既有 `init_bank1` 同构建模（NMI 域形/GFX 重活计数为仓库既定折算）✓。

`prg7_engine.asm:1542 InitNorfairGFX` 七件序 = CREBG1, CREBG2, NorfBG1, NorfBG2, NorfairSprites, Solid_0FC0, `jmp Solid_1FC0` ↔ rust `[CREBG1, CREBG2, NORF_BG1, NORF_BG2, NORFAIR_SPRITES, SOLID_0FC0, SOLID_1FC0]`（尾跳建模为同序循环末项）✓。

gfx 描述子 ROM 07:C6E0 逐 7B（id0/id4/id5/id7/id8/id9/id29 全 dump）：

```
id4 = 05 60 8D 00 12 50 04
id5 = 06 F0 9E 00 18 00 08
id7 = 06 F0 A6 00 10 60 02
id8 = 06 50 A9 00 17 70 00
id9 = 02 60 8D 00 0C 00 04
```

与 `frame/gfx.rs` `gfx_info` 新增五条**全等**；`gfx_block` 新增 `4/5/7/8/9 => gb::*_GFX` 五臂 + `init_norfair_gfx` 消费 ✓。

### A4 SamusInit $C902 lsr —— ✓ 属实

ROM bank7 `$C8F0` dump：

```
38 07 8E 0A 01 8E 0B 01 86 8B 86 8E A0 27 A5 74 29 0F F0 04 46 49 A0 2F 84 FA
84 93 84 94 AD D9 95 8D 0D 03 A9 80 8D 0E 03 A5 FF 29 01 8D 0C 03 A9 00 8D
06 01 A9 03 8D 07 01 60
```

`A5 74 29 0F F0 04 46 49 A0 2F` = `lda InArea / and #$0F / beq Lx002(+4) / lsr $49 / ldy #$2F` ✓；`46 49`=lsr $49（ScrollDir，constants_ram.asm `ScrollDir db ;$49`）、`F0 04`=beq +4 ✓。lsr 的 carry 出被随后 `A0 2F`（ldy 不改标志… 实为 ldy **改** N/Z）覆盖，且下一次读标志前无人消费 carry ⇒ rust `mem.write_ram(SCROLL_DIR, sd >> 1)`（丢 carry）忠实 ✓。

`frame/mainloop.rs` diff：`crate::unported!("bank07:C902-ScrollDir")` → `let sd = ...; write_ram(SCROLL_DIR, sd >> 1); 0x2F`，Brinstar 分支仍 `0x27` ✓。TDD `crates/game-core/tests/samus_init_scrolldir.rs` 两测（非 Brinstar `2→1`+三处 `0x2F`；Brinstar 保持 `0x2`+`0x27`）实跑 `2 passed` ✓。

### A5 CheckPaletteWrite $C1FF + 各 bank 项数独立 dump —— ✓ 属实（但见 F1）

`prg7_engine.asm:536 @paletteDataIsPending: ; 07:C1FF`，ROM 实测 `88 dey / 98 tya / 0A asl / A8 tay / BE 60 95 / B9 61 95 / … / 85 1C / 4C 0C C3` —— **asm 确无范围检查** ✓（rust 保留 fail-loud 守卫、界改由烘焙表项数决定，属「贴 asm + 保留缺口爆口」的折中）。

独立逐 bank dump（边界判据=表后第一项非 `3f` 开头 VRAMStruct）：**bank0=19、bank1=28、bank2=28、bank3=28、bank4=28、bank5=28**，与 commit 声称 `N_ENTRIES = (19, 28, 28, 28, 28, 28)` 一致 ✓（`tools/bake_palette_data.py:26`）。bank2 末四项 `$A1FB/$A203/$A20B/$A213`，末 blob `3F 10 04 0F 15 34 29 00` ✓。asm 侧：`prg0_title.asm:3201` 列 Palette00..Palette12=19 项；`prg2_norfair.asm:53-80` 列 Palette00..Palette1B=28 项 ✓。

title 路径 pending 上界：prg0_title 内已知最大 `lda #_id_Palette12+1.b` ⇒ pending=19 ⇒ idx=18 ≤ bank0 上界 18 ✓，守卫不会挡合法标题值。（`inc PaletteDataPending` 两处 :3722/:3951 属 EndGame 路径，走 `EndGamePaletteWrite: ; 00:9F54` 的 `EndGamePalettePtrTable`，不经 `PalettePtrTable`。）

TDD `crates/game-core/tests/palette_ptr_table_suitless.rs` 两测：pending=26→idx25→Palette19 `$3F10←0F 15 34 19`、pending=28→idx27→Palette1B `$3F10←0F 15 34 29`，与 ROM bank2 末两条流逐字节一致；实跑 `2 passed` ✓。测试用 `InArea=0x11`（Norfair）——恰好绕开 F1。

### F1（A 级，needs_revision 主因之一）area→bank 映射与 ROM BankTable 不符

ROM 07:CA23 `SwitchBank`：

```
85 74   sta InArea
29 0F   and #$0F
A8      tay
B9 30 CA lda $CA30,y        ; BankTable
85 24   sta BankSwitchPending
4C DE C4 jmp $C4DE
02 03 05 04 06             ; BankTable @07:CA30
```

asm `prg7_engine.asm:2274-2279`：

```
BankTable: ; 07:CA30
    .byte $01+1   ;Brinstar.
    .byte $02+1   ;Norfair.
    .byte $04+1   ;Kraid hideout.
    .byte $03+1   ;Tourian.
    .byte $05+1   ;Ridley hideout.
```

⇒ 真映射（InArea&0xF → bank）：`0→1, 1→2, 2→4(Kraid), 3→3(Tourian), 4→5`。

仓库既有正确烘焙：`crates/game-core/src/frame/mainloop.rs:2901`

```rust
const BANK_TABLE: [u8; 16] = [0x02, 0x03, 0x05, 0x04, 0x06, 0x48, ...];
```

而**本 commit 所改函数** `crates/game-core/src/frame/nmi.rs:724`：

```rust
((mem.read_ram(crate::ram_map::IN_AREA) & 0x0F) as usize) + 1 // 区 bank（1-5）
```

⇒ `[1,2,3,4,5]`：**Kraid（$12）读 bank3=Tourian 表、Tourian（$13）读 bank4=Kraid 表**（bank 文件序经 `prg3_tourian.asm` 271 处 `; 03:` / `prg4_kraid.asm` 115 处 `; 04:` 独立确认）。

影响与本 commit 的关系：
- 该行是**未改上下文**（`git show 560bbbac:...nmi.rs` 第 655 行同式）——映射错误本身**非本 commit 引入**。
- 但本 commit 改了同一函数的守卫：改前 `if idx >= 25 { unported!("bank07:C1FF-idx") }` 对**所有** bank 生效 ⇒ Kraid/Tourian 的 idx25..27 是 fail-loud 爆口；改后 `idx >= offs.len()`（区域 bank=28）⇒ **Kraid/Tourian 的 idx25..27 静默读错 bank 的调色板**。这是把「确定性爆 bank:addr」换成「静默错数据」，与 AGENTS D7 缺口左移机制② 直接冲突，且 L4 无 Kraid/Tourian 用例、门禁不可见。
- commit 声称「消费端守卫改数据驱动 offs.len()（贴 asm 无界查表）」——asm 无界查的是**当前映射 bank 的表**，rust 查的是**按 InArea 线性推的 bank**；两者在 Kraid/Tourian 不等价，故该「贴 asm」声明不完整。

复现命令：

```bash
python3 -c "
d=open('rom/Metroid (USA).nes','rb').read(); o=7*0x4000+(0xCA30-0xC000)
print(' '.join(f'{b:02X}' for b in d[o+16:o+21]))"   # → 02 03 05 04 06
grep -n "IN_AREA) & 0x0F" crates/game-core/src/frame/nmi.rs   # → 724: … + 1
```

**处方（供 lead 裁定，本复核不改码）**：① 改为查 `BANK_TABLE`（或等价 match `[1,2,4,3,5]`）；或 ② 若留待后续波，须在 `docs/bug-ledger.md` 登记 `BUG-xxx`（D4：Stage A 不修但必须登记）并在 findings 原地注明 Kraid/Tourian 面未覆盖——**不得以现状静默通过**。

---

## B. 防弱化审计

| 项 | 结论 | 实测原文 |
|---|---|---|
| `gates/` 零改动 | ✓ | `git diff --name-only 560bbbac 87e48ca2 -- gates/ \| wc -l` → `0` |
| 无白名单/阈值改动 | ✓ | 19 文件清单内无 `gates/visual_whitelist.json`、无 `tools/l4_diff_run.py`、无 `cases/` 已有用例改动（新增仅 `cases/case-password-justin.txt`） |
| 无删用例/删场景 | ✓ | `git diff --name-status 560bbbac 87e48ca2` 全 `A/M`，`git diff --diff-filter=D --name-only … \| wc -l` → `0` |
| `git log --oneline -1` | ✓ | `87e48ca2 feat(core): 卡C JUSTIN BAILEY 密码进场 unported 全链端口 + 新用例首次锚定` |
| ROM sha256 | ✓ | `sha256sum "rom/Metroid (USA).nes" \| cut -c1-16` → `e6e6b7014685adae` |
| l4 锚 12 格 2 match+10 diff（diff 未锚成 match） | ✓ | `data/status/l4_ratchet_baseline.json → cases/case-password-justin.states`：`320:title`/`420:menu` = match(diff_pixels 0)，其余 10 格 = `diff`（1294/1365/1294/1275/1384/1294/769/14272/14272/14434 /61440）——与我复跑的 l4report **逐格相同** |
| history 末条 kind=anchor | ✓ | `l4_ratchet_baseline.json.history[-1].kind = "anchor"`，`match:100, diff:10, blocked:0, cells_total:110` |
| `rust_test_count` 524→529 | ✓（数字对，理由错，见 F2） | `git diff … baseline.json`：`- "rust_test_count": 524` / `+ 529`；静态复算（gates 口径 `crates/**/*.rs` 数 `#[test]`）= 529 |
| 既有 13 用例逐格回归 | ✓ 零真回归 | 13 个 `shots_rust/<case>.l4report.json` 逐格 vs 基线：**全部 OK**，唯一差异是 10 个格 `NOT_IN_BASELINE`（见下「flaky 摘锚」）；palette 相关格（morphball 21、scrollseam 14、001/002 等）全 `match` |
| `l4_ratchet_violations(baseline, matrix)` default | ✓ 0 | 实测 `DEFAULT violations: 0`、`for_update violations: 0`（matrix=`shots_rust/matrix.json`，`generated_by="tools/l4_diff_run.py run-all"`） |

### flaky 摘锚（10 格 match 被移出锚）—— 判定：sanctioned，非「逃逸删格」

事实：父基线 13 用例 108 格**全 match**；本 commit 后 14 用例 110 格 = 100 match + 10 diff，**少了 10 格**：

- `case-jumpledge-probe` 移除 7 格：`550:walk-mid, 690:pre-jump, 705:takeoff, 730:rise, 755:peak, 780:land, 800:land-settled`（原均 match）
- `case-walkfcprobe` 移除 3 格：`820:l380, 1020:l580, 1400:end`（原均 match）

链路核验（A 级）：
1. `shots_rust/matrix.json.flaky_cells` 恰为这 10 格，带多 run 观测：jumpledge 4 runs `verdicts=["diff","match","match","match"]`（`20261109-224706/225356/230934/231922`，`code_state=b282c41ce591+7a2a584d68cb`）；walkfcprobe 3 runs `["blocked","match","blocked"]`（`20261110-110806/111843/112545`，`code_state=b4e96705a213+81313c311a7b`）⇒ 满足 `classify_flaky(min_runs=3, verdict 不一致)`。
2. 唯一写入口 `tools/baseline_update.py:79-83`：`【B 径 flaky 不锚值（2026-11-03）】… if states[case][key].get("state") == "flaky": del …` —— 该过滤器**父 commit 已存在**（`git show 560bbbac:tools/baseline_update.py` 同行）。
3. `gates/cli.py:1132-1136`（①豁免）与 `:1159-1160`（③豁免）为 2026-11-03 lead 裁定的既有规则，注释明写「flaky 格不锚值…处置记录=matrix.flaky_cells 留痕（不静默豁免）」；`tools/tests/test_l4_flaky.py` 钉死该面。
4. 观测时间早于父 commit（父 `560bbbac` = 2026-11-10 22:49；观测 11-09 22:47~23:19 / 11-10 11:08~11:25）⇒ 这 10 格在父 commit 时**已处于 flaky 豁免态**（锚为 inert），本次摘锚是把基线与既有机制对齐，**门禁判定结果无变化**。

**残余风险（B 级，非本 commit 引入，登记为遗留假设）**：`tools/l4_diff_run.py:824 classify_flaky` 分组键含 `code_state`，但返回键是格三元组（`out[cell] = …`，注释「同码态多态组取格名」）⇒ **历史任一码态的 flaky 组会让该格在当前码态继续判 flaky**（`build_matrix` → `state="flaky"` → ①③ 双豁免）。若未来这些格出现真回退，可能被历史 flaky 记录掩盖。此为 gate 既有设计（2026-11-04/05 立），非本 commit 弱化；建议单独立项核（判据：`l4_run_observations.jsonl` 是否按码态归档/清理）。

---

## C. 独立复跑（实测原文）

1. **`cargo test --workspace`** → `525 passed; 0 failed; 0 ignored`（分项：game-core lib `479 passed`、`rust_screenshot` bin `5 passed`、game-core 集成 10、game-data lib `27 passed`、+4 其他 = 525）。
   - **与 commit message「cargo test --workspace 全绿 529」不符**：529 是**静态 `#[test]` 计数**（gates `current_metrics` 口径，`gates/cli.py:537` `metrics["rust_test_count"] = count`），`gate_rust_tests`（`gates/cli.py:775-799`）= 跑 cargo test（过）+ 报静态数（529=基线 529）。
   - 差额 4 = `crates/game-core/src/render/sprite.rs:51/58/84/91`（`test_sprite_schedule_basic/boundary`、`test_sprite_init_basic/boundary`）——**不在模块树**（`grep "mod sprite"` 只命中 `frame/mod.rs:34 pub mod sprite`），实测 `cargo test -p game-core --lib sprite_schedule` → `0 passed … 479 filtered out`。**该孤儿在父 commit 同样存在**（`git show 560bbbac:…sprite.rs | grep -c '#\[test\]'` → `4`），非本 commit 引入。
   - 4 条新 TDD 实跑通过：`palette_ptr_table_suitless` `2 passed`、`samus_init_scrolldir` `2 passed`；nmi.rs 新增 `sound_engine_silence_music_path_clears_all` 计入 lib 479。

2. **`python3 tools/l4_diff_run.py run case-password-justin`** → `shots_rust/case-password-justin.l4report.json` 12 格：
   - match（0/61440）：`320:title`、`420:menu`
   - diff：`530:pwd-screen 1294`、`700:mid-1 1365`、`850:mid-2 1294`、`950:mid-3 1275`、`1100:mid-4 1384`、`1150:pw-done 1294`、`1190:submit 769`、`1300:post-submit 14272`、`1430:spawn-t4s 14272`、`2390:spawn-t20s 14434`
   - **2 match / 10 diff / 0 blocked / 0 error**；锚 `scenario=b2146fc8c53f0cab… rom=e6e6b7014685adae…`，frames 2600 / trace 2600 / captures 97 / shots 97 / wall 55.2s。

3. **既有 13 用例逐格回归**：13 份 l4report 全 `match`（合计 108 格），与基线逐格比对 **0 处状态回退**；仅 10 格因 flaky 摘锚显示 `NOT_IN_BASELINE`（见 B）。

4. **`python3 gates/cli.py check`** → `SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2`、`CHECK: PASS`，关键行：
   - `[PASS] rom-integrity … rom/Metroid (USA).nes=e6e6b7014685adae…`
   - `[PASS] rust-tests … cargo test --workspace 通过；测试数 529（基线 529）`
   - `[PASS] l4 … 用例 14 · 分母 110（blocked 0 计分母不计 pass）｜match 100 diff 10 blocked 0｜白名单条目 0`
   - `[PASS] tree-coherence … 140 个提交树 .rs 文件 mod 引用全部解析`（注：该门禁只验「mod 引用可解析」，不验「每个 .rs 都被引用」，故抓不到 sprite.rs 孤儿）
   - TOLERATED 2 项均为既有登记（trace-diff `l3-samus-move3` 空迹、visual-diff 采集面覆盖缺口），非本 commit 引入。

5. **`l4_ratchet_violations(baseline, matrix)` default** → `DEFAULT violations: 0`、`for_update violations: 0`（rom 锚 `e6e6b7014685adae` 双侧一致）。

---

## D. 遗留声明核对

1. **10 格 diff 确为非零真差、非「新代码引入的既有回归」** ✓：这 10 格在父 commit 的 findings 量表中是 `blocked`（unported 缺口），**历史上从未 match**，故不存在「match→diff 回退」；本次全部 `diff_pixels > 0`（最小 769，最大 14434），非 0 像素抖动。
2. **`docs/findings/l4-password-justin-preparevram-unported.md` 符合 AGENTS §3.6** ✓：新增文件，状态行 `状态：~~归因落档…~~ → 【2026-11-11 更正】…`，正文首段 `> 【2026-11-11 更正（AGENTS §3.6 原地标注，不删史）】…`，**原「10 点全 blocked、不入 ratchet」的量表与处置原文全部保留**，并注明「本档历史量表保留作证据，不作现状判据」。
3. **commit Evidence 行号漂移（不改结论，登记）**：`sound_engine.asm:415→实417`、`ClearInitFlags :405→实407`、`sound/mod.rs:1253→实1259`、`EndGamePaletteWrite` 实为 `prg0_title.asm:4438`（非 :4440）。Evidence 中 `PalettePtrTable bank2 file off 0x9560` 用的是 **PRG-only（去 16B iNES 头）口径**（与 `bake_palette_data.py` 文档串 `ROM.read_bytes()[0x10:]` 一致），含头为 `0x9570`——非错误，但建议写明口径。

### F2（A 级，needs_revision 主因之二）baseline_history 理由含不可核的假陈述

`data/status/baselines/baseline_history.jsonl` 末条（本 commit 新增）reason 逐字：

```
卡C新增 4 条 TDD（samus_init_scrolldir×2 + palette_ptr_table_suitless×2）+ 既有测试并入，
静态 #[test] 现值 529；两个未接线孤儿图形件已移出工作树（防全新 clone 掉测）。
cargo test --workspace 全绿。
```

证伪：
- `git diff --diff-filter=D --name-only 560bbbac 87e48ca2 | wc -l` → **0**（零删除）。
- 逐文件 `#[test]` 计数（两 commit 同口径复算）：`parent total: 524 → head total: 529`，差异仅 `frame/nmi.rs 18→19`、新增 `tests/palette_ptr_table_suitless.rs 0→2`、`tests/samus_init_scrolldir.rs 0→2`；`files only in parent: set()` —— **没有任何文件被移出**。
- `crates/game-core/src/render/sprite.rs` 在父/HEAD 均存在且各含 4 个 `#[test]`，实测**仍不编译**（`479 filtered out`）⇒ 「已移出（防全新 clone 掉测）」既无动作也无效果。
- 同句「+ 既有测试并入」也算不平：524+4=528≠529，实际 +1 来自 `nmi.rs` **新增** `sound_engine_silence_music_path_clears_all`（非「既有」）。

**处方**：按 §3.6 在该 history 条目（或 findings/台账）**原地追加日期更正**，写明真实增量构成与「孤儿件仍在树中」的现状；不得删史。

---

## 防弱化结论（逐项）

| 面 | 结论 |
|---|---|
| 阈值 / 白名单 | **未弱化**：`gates/` 零 diff，`visual_whitelist.json` 未触碰，l4 报告 `白名单条目 0` |
| 用例 / 场景 | **未删改**：`cases/` 仅新增 1 个；`--diff-filter=D` = 0 |
| 基线只升不降 | **未人为下调**：`match 100/diff 10/blocked 0` 中的 10 格摘锚由 `baseline_update.py:79-83` 既有 flaky 过滤器产生，观测记录留痕于 `matrix.flaky_cells`（≥3 run 不一致），且这 10 格在父 commit 时已处于豁免态；`l4_ratchet_violations(default)=0` |
| 门禁判定权 | **未绕过**：`gates/cli.py check` 实测 PASS=13 FAIL=0；commit 未改 `l4_diff_run.py`/`baseline_update.py`/`gates/*` |
| fail-loud 缺口 | **本 commit 有局部弱化（F1）**：Kraid/Tourian 区 `idx25..27` 由 `unported!` 爆口转为静默错 bank 读取（映射错误为既有，但暴露面由本 commit 扩大） |
| 证据真实性 | **本 commit 有一处假陈述（F2）**：baseline_history「孤儿图形件已移出工作树」与仓库状态矛盾 |

---

## 遗留假设清单

1. **F1 待裁定**：Kraid/Tourian 调色板 bank 互换是既有 bug，本 commit 扩大其静默面；修正或 `BUG-xxx` 登记二选一，需 lead 决定（本复核只读，未改码）。
2. **F2 待原地更正**：baseline_history 假陈述需按 §3.6 补更正（不删史）。
3. **flaky 跨码态粘性**（B 级）：`classify_flaky` 返回键=格三元组 ⇒ 历史码态的 flaky 会持续豁免当前码态；10 格未来回退可能被掩盖。属 gate 既有设计，建议单独立项核。
4. **4 个孤儿 `#[test]`**（`render/sprite.rs`）+ 静态 529 / 实跑 525 的口径差：既有问题，非本 commit 引入；`tree-coherence` 门禁结构上抓不到「未被 mod 引用的文件」。
5. **`initialize_sound_addresses` 写序微差**（asm 十清→五 APU→四清 vs rust 14 RAM→5 APU）：跨域无标志依赖，判为等价（B 级），未做逐帧 APU trace 复验。
6. **blocked 10→0**：父态 10 blocked 只有 findings 文档单方记录（`shots_rust/` 未入库，无法在不改码的前提下重放父态复验），本复核记 B 级。
7. **10 格 diff 的逐像素归因**（竖镜/LoadSamusGFX/淡入/per-area 图形表等 `/tmp/card-c-wip.patch` 大件）属卡 C 后续波，本次仅确认其非零、非既有 match 回退，未逐像素归因。

Confidence: 88
