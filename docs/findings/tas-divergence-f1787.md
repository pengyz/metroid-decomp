# TAS 100% 首个内存面分歧：f1787 槽 2 `$0429`（Skree 落地后 `Ens.delay` 不再推进）

> 证据分级按 `AGENTS.md` §3：A=字节/汇编/逐帧 trace 直证；B=代码语境 + 调用链推断；C=猜测（不据此改代码）。

## 0. 结论（一句话）

`update_enemy_common_no_move_no_anim` 被用在 Skree「落地爆体」分支上，但 ROM 那条分支**没有 rts**、
落回 `SkreeExit_Active` 后仍然 `jmp CommonJump_UpdateEnemyCommon`（动画 + **EnemyMove** + 尾件）——
EnemyMove 是 `Ens.delay(+9)` 的唯一推进者，所以 rust 侧该字节从落地那帧起恒为 `$01`，emu 逐帧递增。

## 1. 复现（A）

100% 影片 `movies/lordtom-metroid-100p.inputlog`（120540 帧）的分段回放产物：

> 目录代际：`analysis/tas-100/` 是**修前**产物（首分歧 f1787），`analysis/tas49-seg10k/`、`analysis/tas49-check/` 是**修后**产物（首分歧 f2084）。不要把两者的首分歧混看。

| 面 | 文件 | f1787 `$0429` |
|---|---|---|
| emu | `analysis/tas-100/emu/seg-000000-010000.jsonl` | `0x02` |
| rust | `analysis/tas-100/rust/full.state.jsonl` | `0x01` |

- 该字节 = 槽 2 的 `$0400 + 2*16 + 9` = `Ens.delay`（`crates/game-core/src/enemy/state.rs` 的 `ens::DELAY=9`；`EnsExtra.type` 读 `$6AF4+2*16+14 = $04` ⇒ Skree）。
- 逐帧：emu 自 f1785 起 `01,02,03,04,05,06,07,08`（f1787=02、f1799=08），rust 侧 f1785–f1799 恒 `01`。
- 复现命令（不需要 emu 在线，直接读上面两份产物）：

```bash
python3 - <<'PY'
import json
E={};R={}
for ln in open('analysis/tas-100/emu/seg-000000-010000.jsonl'):
    o=json.loads(ln); E[o['f']]=bytes.fromhex(o['w']['ens'])
for ln in open('analysis/tas-100/rust/full.state.jsonl'):
    o=json.loads(ln); R[o['f']]=bytes.fromhex(o['w']['iram'])[0x400-0x100:0x500-0x100]
for f in range(1780,1800):
    print(f, 'emu', E[f][0x29], 'rust', R[f][0x29])
PY
```

## 2. 定界

### 2.1 ROM（A 级，`refs/m1disasm/SRC/enemies/skree.asm`）

- `:16-17` `lda Ens.0.movementInstrIndex,x / cmp #$0F / bcc SkreeExit_Active`；
- `:19-20` `cmp #$11 / bcs SkreeBlowUpIntoProjectiles`；
- `:27-31` 爆体径：`dec EnsExtra.0.jumpDsplcmnt,x / bne SkreeExit_Active`；
- `:32-54` 清槽 + 生成 4 弹的 `@loop`，`bpl @loop` 结束后**没有 `rts`**；
- `:56-60` `SkreeExit_Active:` `lda #$02 / jmp CommonJump_UpdateEnemyCommon`。

⇒ 落地秒数走完前的每一帧，爆体径**先递减 jumpDsplcmnt，再照常走 UpdateEnemyCommon**（含 EnemyMove）。

### 2.2 rust（修复前，改前代码在 `crates/game-core/src/enemy/skree.rs` 的 `instr >= 0x11` 分支）

```rust
        update_enemy_anim(mem, x, 2);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
```

`no_move_no_anim` 不调用 `enemy_move` ⇒ `Ens.delay` 不再被推进，恒留在落地时的 `$01`。
（`update_enemy_common_anim_move` = `update_enemy_anim + enemy_move + update_enemy_common_no_move_no_anim`，
见 `crates/game-core/src/enemy/framework.rs:647-651`。）

### 2.3 为什么不是别的层（排除项）

| 候选 | 结论 |
|---|---|
| 敌槽名册/装载面（`EnemyInitDelayTbl`→`Ens.delay`） | 与分歧无关：两侧 `$0428`(movementIndex)=`07`、`$0426`(movementInstrIndex)=`0C→0F→11` 完全同步，分歧只出现在 `$0429` |
| `movement string` 取数（`enemy_get_delta_xy`） | 同上，指令下标两侧一致；且爆体径在 ROM 里根本不经 AI 的取数分支，只经 EnemyMove |
| emu 侧采集相位 | 同一 trace 里 f1785–f1786 两侧全 256 字节相等，只有 f1787 起 `$0429` 单字节差，不是整帧错位 |

## 3. 修复（`crates/game-core/src/enemy/skree.rs`，本卡写面内）

```rust
        // ROM 的 Explode 分支没有 rts：spawn 循环 `bpl @loop` 后直接落到 SkreeExit_Active
        // （skree.asm:41-60），仍然 `lda #$02 / jmp UpdateEnemyCommon` ⇒ EnemyMove 照跑。
        update_enemy_common_anim_move(mem, x, 2);
        return;
```

新增回归测试 `enemy::skree::tests::landing_branch_still_runs_enemy_move_and_advances_delay`
（`movementInstrIndex=0x11` + `jumpDsplcmnt=10` + `delay=0` ⇒ 调 `skree_ai` 后断言 `delay==1`）。
反向验证：把分支改回旧形，该测试报
`assertion left == right failed: 落地径仍走 UpdateEnemyCommon：EnemyMove 每帧推进 Ens.delay（旧形恒 0）` ⇒ 是真回归守卫。

## 4. 回归实测（A）

- 修后 10000 帧回放（`analysis/tas49-seg10k/coverage.json`）：**0–2083 帧全等**，首个分歧后移到
  **f2084 `$0415` emu=0x1E / rust=0x1A**（槽 1 的 `data05` bit2，1 字节）。
- 修后 2400 帧回放（`analysis/tas49-check/coverage.json`）给出同一首分歧 f2084 ⇒ 与段长无关。
- `cargo test -q -p game-core --lib` → **1042 passed / 0 failed**（原 1041 + 新增 1）。
- `python3 gates/cli.py check` → `SUMMARY: PASS=24 FAIL=0 SKIP=3 TOLERATED=1` / `CHECK: PASS`。
  （首版测试用 `m.write_ram(0x0081, 0)` 这类字面量地址直接搭台，被 `arch-coupling` 与 `no-raw-addr`
  两道门同时判红；改用 typed 面 `set_enemy_status_pre_ai / set_data06 / set_jump_dsplcmnt / set_enemy_type`
  后转绿——门禁在这里起的是正确作用。）

## 5. 未解决（登记，不在本卡）

- **f2084 `$0415`**：槽 1 `data05` bit2（`$04`，ROM 侧「垂直运动且朝下」标记位）在 emu 置位、rust 未置。
- 下一步该看的（B 级线索，未验证）：
  - ROM：`refs/m1disasm/SRC/prg7_engine.asm:11483` `UpdateEnemy_EnData05DistanceToSamusThreshold`（07:F75B）之外，
    真正**异或 bit2** 的位置是 `refs/m1disasm/SRC/areas_common.asm:239-248` 的 `@movementStrings`：
    `lda L977B,y / lsr / lsr / bcc @abortLoop / lda #$04 / jsr XorEnData05` —— Up/Down 两条失败径都汇到这里。
  - rust 侧**已实现**同样的异或：`crates/game-core/src/enemy/motion.rs:139-190`（`enemy_if_move_failed_vertical`：
    data05 bit7 清零走 movement-string 时 `if raw & 0x02 != 0 { xor_en_data05(mem, x, 0x04) }`）。
  - ⇒ 所以 f2084 更像是「这一帧 emu 的竖直移动失败、rust 的成功」或 `L977B` 表项位（`raw & 0x02`）取值不同，
    而不是「xor 没写」。取证入口：给 `EnemyIfMoveFailedUp/Down` 的入口加计数（`crates/game-core/src/enemy/` 内可插桩），
    与 emu 侧 `$0415` 的翻转帧对照。
- 整片 120540 帧未重跑（36 分钟）；本卡只回归到 10000 帧，剩余窗口的分歧未纳入本次定界。

## 6. 复算清单

```bash
# 修复后 10000 帧、单段
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 10000 --segments 10000 --out analysis/tas49-seg10k
python3 -c "import json;d=json.load(open('analysis/tas49-seg10k/coverage.json'));print(d['first_divergence'])"
```
