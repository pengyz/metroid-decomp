# 第八轮：f1574 `$0406` 绘制侧 inc 修复（首分歧 f1574 → f1575 `$0410`）

> 任务 #152（tsk_01M8EAC406F3MN7TR75XKT，2026-11-27）。写面：`crates/game-core/src/render/`、
> `crates/game-core/src/enemy/`、本文。上游：#148（定界 `docs/findings/tas-divergence-f1575.md`）、
> #151（规格 `docs/spec/f1574-fix-plan.md`，实现判据 §2.2/§2.3/§2.4、验收 §5/§6）。
> 基线 HEAD：`28e81144`（`fix(enemy): #152 chunky 爆散计时移回绘制面`）；影片
> `movies/lordtom-metroid-100p.inputlog`（全片同一份，未重录）。

## 0. 结论

1. **目标字节达成（A 级）**：`$0406`（`Ens.0.data06`，slot0）在 **f1574 由 emu `01` / rust `00`
   变为两侧同为 `01`**；f1574 帧内 256 B 敌窗口（`$0400–$04FF`）**差异字节数 1 → 0**（改前后
   逐字节比对，见 §3）。首分歧离开 f1574。
2. **新首分歧 = f1575 `$0410`（4 B：`$0410/$0415/$0445/$0455`）**。这是**修前已存在**的残差：
   同一影片在 `28e81144~1` 树上复跑（`/tmp/r152pre`），f1575 已含这 4 B（外加当时的 `$0406`），
   逐字节数值与修后完全相同；`$0406` 修好后它们才暴露为「首个」分歧。故本卡**未引入新差异**，
   只把被掩盖的旧残差前移了 1 帧（f1574 → f1575）。
3. **规格 §5 的逐帧预言表需更正**（A 级读数顶替预测）：规格预测「f1574=1 / f1575=2 / 每帧 +1」，
   实测两侧 `$0406` 自 f1574 起**恒为 1 到 f1600**——本次事件中 `place==EnPlace1` 的绘制只发生
   f1574 那一帧，inc 因此只执行 1 次（ramp 只在 place1 绘制帧发生，B 级推断：此后 slot0 帧首
   ctrl 低 4 位不再是 1）。ROM 语义（`07:DDEA inc $0406,x` 只在 place1 分支）与实测一致，
   实现按 ROM 写，不做「每帧 +1」的特判。
4. **预测 P1 只部分命中**：`f1574 equal` 命中；`f1575–f1600 连续 equal` **未命中**，失败点是
   上面第 2 条的既有残差，不是本次 inc 顺序/位置错（P2/P3/P4 均不成立：rust 值不再为 0、
   inc 次数不是每帧 +2、离屏路径未被 visible 早退挡住——见 §2 表与 §3 对照）。
5. 未调阈值、未加白名单、未改判等面；`$0410` 族残差留后续卡（§4）。

## 1. 命令与产物

```bash
# 主跑（修后，HEAD 28e81144）
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 4000 --range 0400-04FF --out /tmp/r152 --allow-rust-missing

# 对照跑（修前，临时 git checkout 28e81144~1 的三个源码文件，跑完 checkout HEAD 恢复）
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 1700 --range 0400-04FF --out /tmp/r152pre --allow-rust-missing
```

读数（`/tmp/r152/report.json` → `compare`）：

| 项 | 值 |
|---|---|
| `compared` | 3967 / 4000 |
| `coverage_note` | 判等覆盖 3967/4000，rust 缺 33 帧（前 5: 2,4,5,6,11；含 1572/1573）；缺帧退出比较 |
| `emu_missing_frames` | `[]`（本跑 emu 4000 帧齐全，无右移盲区） |
| `first_divergence` | `{frame:1575, field:"$0410", offset:16, emu:155, rust:154, diff_bytes:4}` |

对照跑（`/tmp/r152pre`，1700 帧）：`compared=1673`，
`first_divergence={frame:1574, field:"$0406", offset:6, emu:1, rust:0, diff_bytes:1}`
——复现 #148 的原始读数，说明对照树确为修前状态。

## 2. 逐帧 `$0406` 表（slot0，`$0400+6`；emu 取值 `frames.jsonl` 的 `w.ens[6]`，
rust 取值 `rust.state.jsonl` 的 `w.iram[(0x0406-0x0100)]`）

| frame | emu `$0406` | rust `$0406` | emu `$0410` | rust `$0410` | 备注 |
|---|---|---|---|---|---|
| 1570 | 0 | 0 | 157 | 157 | |
| 1571 | 0 | 0 | 156 | 156 | |
| 1572 | 0 | — | 156 | — | rust 未 tick（span 跳帧），不可比 |
| 1573 | 0 | — | 156 | — | 同上 |
| **1574** | **1** | **1** | 155 | 155 | **本卡目标；整帧 256 B 零差异** |
| 1575 | 1 | 1 | 155 | 154 | 首分歧（`$0410` 族 4 B） |
| 1576 | 1 | 1 | 154 | 153 | `$0410` 族延续 |
| 1577 | 1 | 1 | 153 | 152 | `$0410` 族延续 |
| 1578–1600 | 1 | 1 | 152 | 152 | `$0410` 族收敛，两侧一致 |

要点：`$0406` 在 f1574 上一帧（1571，可比的最后 0 值帧）为 0，f1574 起两侧同为 1 且不再变化；
`$0410`（slot1 的 `Ens+0`，即 y）在 f1575–1577 差 1 后于 f1578 收敛。

## 3. 差异字节明细与「修前已存在」对照（A 级）

修后（`/tmp/r152`）：

| frame | 差异字节数 | 明细（地址: emu / rust） |
|---|---|---|
| 1574 | **0** | — |
| 1575 | 4 | `$0410` 155/154、`$0415` 30/94、`$0445` 88/24、`$0455` 24/88 |
| 1576 | 4 | `$0410` 154/153、`$0415` 94/30、`$0445` 24/88、`$0455` 88/24 |
| 1577 | 4 | `$0410` 153/152、`$0415` 30/94、`$0445` 88/24、`$0455` 24/88 |

修前（`/tmp/r152pre`）：

| frame | 差异字节数 | 明细 |
|---|---|---|
| 1574 | 1 | `$0406` 1/0 |
| 1575 | 5 | `$0406` 1/0 + 上表 1575 的 4 B（数值逐字节相同） |
| 1576 | 5 | `$0406` 1/0 + 上表 1576 的 4 B |
| 1577 | 5 | `$0406` 1/0 + 上表 1577 的 4 B |

⇒ 两组读数相减，本卡净效果 = **删去 `$0406` 这 1 B**，其余 4 B 一字未动。字段归属（`state.rs`
访问器）：`$0410`=slot1 `y()`（main+0）、`$0415`=slot1 `data05()`（main+5）、`$0445`=slot4
`data05()`、`$0455`=slot5 `data05()`。这 4 B 的写序/值互换（30↔94、88↔24）与 slot1 的 y
早一帧递减，同属「多槽字段在绘制/更新同帧的写序差」，不在本卡写面结论内，交 §4。

## 4. 后续（建议单开一张卡，写面 `crates/game-core/src/enemy/` 或 `render/`）

- 现象：f1575–1577 共 4 B，`$0410` y 差 1（1575 才暴露，1578 收敛）＋ `$0415/$0445/$0455`
  的 `data05` 值在两槽间互换（bit6 位差、以及 slot4/5 的 88/24 对调）。
- 复现：§1 主跑命令 + §3 的逐字节比对脚本口径（`w.ens` vs `w.iram[(addr-0x100)]`）。
- 判据：修后 f1575 起 256 B 零差异且不发生新地址分歧；`$0406` 保持 1。

## 5. 边界与未覆盖

1. **旋转混入未实现**：ROM place1 分支会把 `ExplodeRotationTbl`（`00 80 C0 40`）混进 ctrl
   （`prg7_engine.asm:6144-6150`），Rust `render/oam.rs` 仅对 Samus 爆炸实现
   （`samus_explode_draw`），敌侧仍缺——属既有缺口，本卡未扩大。
2. **爆散池 `x≥0xC0` 折中**：`enemy_complete_death` 用 `slot_of(x)=x/16`，只对敌池
   `x=slot*16` 成立；爆散池 `x=0xC0+slot*8` 会算成敌槽 12/13。规格 §2.4 要求「通用写法照常
   inc，只保护死亡调用」，实现按此加 `if x < 0xC0`（代码注释已写明是折中、非 ROM 语义；
   「池从未 place==1」仍是 C 级未证实）。
3. **rust 缺 33 帧**：span/载房窗一跳多帧属既有记账（`coverage_note`），本卡未改动；本次
   emu 腿 4000 帧齐全，无「首分歧右移」盲区。
4. **门禁**：`cargo test --workspace` 1295 passed / 0 failed（基线 1289 + 本卡净增 6 条）；
   `python3 gates/cli.py check` PASS=24 FAIL=0 SKIP=3 TOLERATED=1；L4 矩阵见 #152 提交说明。

## 6. 证据指针

- 代码：`crates/game-core/src/render/enemy_sprite.rs`（`draw_at`：`animFrame==0xFF` 首步早退 →
  帧表/ctrl/place_idx → `place_idx==1` 时 `data06 += 1`、`==0x19` 死亡并中止 → 半径 → 可见性）；
  `crates/game-core/src/enemy/framework.rs`（`update_enemy_explode` 只保留 anim + common，
  timer 自增整段删除）；`crates/game-core/src/enemy/state.rs`（`EnsFace::data06/set_data06`）。
- ROM：`refs/m1disasm/SRC/prg7_engine.asm:6134-6156`（place1 分支）、`:6065-6068`（空白帧早退）、
  `:6158-6186`（半径→可见性→绘制）；字节 `$DDEA=FE 06 04`、`$DDED=BD 06 04`。
- 读数：`/tmp/r152/report.json`、`/tmp/r152pre/report.json`、两侧 `*.state.jsonl` /
  `emu-traces/*.frames.jsonl`（窗口 `$0400–$04FF`，emu 键 `w.ens`、rust 键 `w.iram`）。
