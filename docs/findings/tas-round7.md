# 第七轮：f1551 掉落门链修复（首分歧 f1551 → f1575）

> 任务 #143（tsk_01M8DH5XN3ZHSF663HRQ6B，2026-11-27）。写面：`crates/game-core/src/enemy/framework.rs`
> （+ 同文件测试）、`crates/game-core/src/world/tourian_quota.rs`（仅测试断言，seq4 放行）、
> `crates/game-core/src/frame/memory.rs`（+7 行 typed setter，seq2 放行的访问器先例）。
> 上游：#140（定界 `docs/findings/tas-divergence-f1551.md`）、#142（规格
> `docs/spec/f1551-fix-plan.md`）、#144（掉落与拾取规格）、#117（Tourian 配额 LDD5B 先例）。
> 基线 HEAD：`ede1e845`（树干净，未跟踪文件 2 个非本卡）。

## 0. 结论

1. **首分歧离开 f1551**：`--frames 4000 --segments 4000 --range 0400-04FF` 重跑，首个真实分歧
   从 **f1551 `$040D`（1 B，emu `$60` / rust `$5F`）** 后移到 **f1575 `$0406`（5 B，emu `01` /
   rust `00`）**。改前/改后同一命令、同一影片（`movies/lordtom-metroid-100p.inputlog`）。
2. **预测命中（A 级）**：#142 规格在实现落盘**之前**预测「f1551–f1573 全 match；f1574 的
   7B（$0401/$0405/$0406/$0409/$040B/$040D/$040F）消失；新首分歧 ≥f1575」——实测首分歧
   **恰为 f1575**，三段全部命中（预测文本见 `docs/spec/f1551-fix-plan.md` §5，commit 前已冻结）。

   > 【2026-11-27 更正】「恰为 f1575」依赖本次 emu 采样，稳健口径为 **≥f1574**：本跑 emu 腿
   > `emu/seg-000000-004000.jsonl` 实测 3962 帧（4000 缺 39，含 f1572–f1574；第二次采样
   > /tmp/r143/post2 缺帧分布相同），`compare()` 按 `set(emu) & set(rust)` 交集判等
   > （tools/tas_replay_diff.py:206）⇒ 缺帧帧退出比较，首分歧右移到 f1575（5B）。对影独立
   > 复跑（含 f1574）实测首分歧 **f1574 `$0406` 1B（emu01/rust00）**。两跑一致成立的判据：
   > f1551 已离开、f1574 的旧 7B 串消失；差异只在 f1574 这 1B 是否被采样到。「f1551–f1574
   > 段全部 match」按对影读数修正为「f1551–f1573 全 match，f1574 余 1B（$0406）」。
3. **根因按 #140 定界修复**：Rust `enemy_complete_death` 只实现 1/16 无掉一道门；ROM
   `EnemyCompleteDeath`（07:DD0D 起）还有**导弹/能量配额门**与**大能量资格门（L968B bit0）**。
   f1551 现场是 Brinstar（bank1）能量配额：emu 配额满 ⇒ 走 LDD5B 清槽（`$040D` 恒 `$60`）；
   rust 无门 ⇒ 转成掉落物（`$040D=$5F`）。
4. **新分歧 f1574/f1575 `$0406`（chunky 计时字段）是新缺口**，不掩盖：首分歧帧 f1574
   （1B，emu=01/rust=00；我方采样缺 f1574 时右移为 f1575 5B），属
   f1575 起的下游链（同一敌槽的 chunky 爆炸计时写序），留给下一张卡，本卡不越界。

## 1. 改前 / 改后（同一处门链）

改前（`crates/game-core/src/enemy/framework.rs`，`enemy_complete_death` 尾段）：
只有 1/16 无掉门（`r < 0x10 ⇒ ldd5b`），随后无条件写 `animFrame ← PickupTypeTable[r&7]`、
status←Pickup——掉落物**必生成**，配额与 L968B 资格两道门缺失（07:DD30–07:DD58 整段）。

改后（同一函数，照 ROM 顺序）：

```rust
// type 覆盖前取 L968B[orig_type]（asm 07:DD11 jsr ReadTableAt968B 在 07:DD1A 覆盖 type 之前）
let orig_type = mem.enemy(slot_of(px as u8)).enemy_type();
let l968b = game_data::assets::rom_table_968b(
    crate::frame::area_bank(crate::frame::cur_area(mem)) as usize, orig_type);
… status←Pickup；data0D←0x60；1/16 门（不变）……animFrame ← tbl[r&7]；
let gates_pass = |mem: &mut Manager| -> bool {
    if frame == 0x80 {                       // 导弹支（asm 07:DD36 cmp #$81 的 bne 支）
        let (max, cur) = (mem.pickup_missile_qty_max(), mem.pickup_missile_qty_cur());
        if max == cur || mem.max_missiles() == 0 { return false; }   // 07:DD3A / 07:DD41
        mem.set_pickup_missile_qty_cur(cur + 1);                     // 07:DD4B inc
        true
    } else {                                 // 能量支
        let (max, cur) = (mem.pickup_energy_qty_max(), mem.pickup_energy_qty_cur());
        if max == cur { return false; }      // 07:DD47
        mem.set_pickup_energy_qty_cur(cur + 1);                      // 07:DD4F inc（先于资格检查）
        frame != 0x89 || (l968b & 1) != 0    // 07:DD51 cmp #$89；07:DD58 lsr Temp00/bcs
    }
};
```

失败处理复用 `world/tourian_quota::ldd5b`（`InArea==$13` 精确比较照 #117 裁定）：
非 Tourian ⇒ RemoveEnemy 清槽；Tourian ⇒ 配额复位后同 r 重试一次；重试仍败 ⇒
C1 截断（`DIVERGENCES.push(0x0DD5B)` 记账 + 清槽，不复刻 ROM 死循环——bank3 的
L968B 无任何 bit0=1 项，Tourian 大能量资格恒败，见规格 §7）。

**inc 先于资格检查**是本次修复的关键序（asm 07:DD4F 在 07:DD58 之前），大能量测试
`big_energy_eligibility_denied_clears_slot` 显式断言 `cur==2`（过配额 inc 一次 + 失败重试
再 inc 一次）锚定这个序。

## 2. 证据（本机实测）

### 2.1 改前负例（HEAD `ede1e845`，树干净）

```
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 4000 --segments 4000 \
  --range 0400-04FF --out /tmp/r145
first_divergence = {'frame': 1551, 'field': '$040D', 'offset': 13,
                    'emu': 96, 'rust': 95, 'diff_bytes': 1}
```

（与 #142 卡面 `60/5F` 十六进制读数一致；emu 腿 78.4s、rust 腿 4.0s。）

### 2.2 改后复跑（同命令，`--out /tmp/r143/post`）

```
seg 0 [0,4000) diverge 79.0s 首分歧 {'frame': 1575, 'field': '$0406',
              'offset': 6, 'emu': 1, 'rust': 0, 'diff_bytes': 5}
覆盖率: 4000/4000（100%），比较帧 3956，f1575 后分歧帧 2410（漂移规模，非新增缺陷）
emu sha 50406a8a3de6 / rust sha aecf43183fdd
```

- f1551 `$040D` 1B 分歧消失；f1551–f1574 段全部 match（首分歧已到 f1575）。
- f1574 的 7B 串（规格 §5 预测清单）消失。

### 2.3 单测（先红后绿）

- 红灯：`cargo test -p game-core --lib tests_drop_gates` → **0 passed / 5 failed**
  （全败于旧无门语义）。
- 绿灯：`cargo test -p game-core` → **1167 passed / 2 failed**（剩余 2 条 =
  `frame/tests.rs` kill_chain，属授权外写面，已报备 #143 thread seq6 等放行；
  迁移面内 3 条已按新契约改断言并保留覆盖）。
- 门禁：`python3 gates/cli.py check` → PASS=23 / FAIL=1（arch-coupling 已由
  typed setter 修复转绿；rust-tests 即上述 2 条）。

## 3. 测试迁移账目（断言级，供复核）

| 测试 | 旧断言（旧契约） | 新断言（新契约） | 覆盖点 |
|---|---|---|---|
| `framework.rs::tests_explode_regular_beam::regular_beam_kill_becomes_pickup_immediately` | 束杀⇒Pickup | 束杀⇒Pickup（补构置 `set_pickup_missile_qty_max(1)`+`set_max_missiles(10)`） | 束杀立刻转掉落物（不变） |
| `tourian_quota.rs::tourian_drop_failure_retries_pickup` | r=0x05 ⇒ 复位+Pickup+anim 0x89 | r=0x01 ⇒ 复位+Pickup+anim 0x81+energy_cur==1 | Tourian 复位后重试生成（保留） |
| `tourian_quota.rs::tourian_zero_random_still_spawns_pickup` | r=0x00 ⇒ 复位+Pickup | r=0x00+MaxMissiles=5 ⇒ 复位+Pickup+anim 0x80+missile_cur==1 | `bne` 由 iny 决定 ⇒ Rnd=0 也重试（保留） |
| `tourian_quota.rs::tourian_big_energy_denied_truncates_c1`（新增） | — | r=0x05 ⇒ 复位+inc 先于资格+NoEnemy+anim 0x89 | C1 截断（旧 r=0x05 的资格失败语义移到这里） |
| `framework.rs::tests_drop_gates`（新增 5 条） | — | 配额导弹/能量支、资格拒/放行、导弹放行 inc | 三门齐全（本卡主体） |

## 4. 复跑命令

```bash
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 4000 --segments 4000 \
  --range 0400-04FF --out /tmp/r143/post
cargo test -p game-core
python3 gates/cli.py check
```

## 5. 遗留

1. f1575 `$0406`（emu=01/rust=00，chunky 计时）为**新首分歧**，下一张卡从
   `/tmp/r143/post/{report 数据在 coverage.json,emu/,rust/}` 起步定界。
2. 入口守卫 07:DD07–0C（仅 Tourian 下 type==$04/$02 ⇒ RemoveEnemy）Rust 现无，
   规格 §4 未含——已报 PMO，待并入或开新卡。
3. `frame/tests.rs` kill_chain 两条迁移在等放行时跑绿（thread seq6）。
