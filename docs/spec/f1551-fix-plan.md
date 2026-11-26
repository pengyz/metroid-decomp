# f1551 `$040D` 修复规格：掉落门链的最小变更

> 卡 #142（tsk_01M8DGK5EJ5JMD3S9HYK33）。只写本文件，`crates/` 零改动。
> 定界来源：`docs/findings/tas-divergence-f1551.md`（#140，已验收）。
> 同套路参照：`docs/spec/f1301-fix-plan.md`（#130）。

## 0 结论

f1551 的差不是 `$040D` 的算术错，而是 **`EnemyCompleteDeath` 的掉落门链只实现了一道**：

- ROM 在 `status←Pickup`、`data0D←$60` 之后依次过三门：1/16 无掉落 → 导弹/能量配额 → 大能量资格（`L968B[type]&1`），任一失败走 `LDD5B ⇒ RemoveEnemy`（非 Tourian）。
- Rust 只做了 1/16 那道（`framework.rs:585-589`），配额与 `L968B` 两道缺 ⇒ 本应 clean 0 的槽保持 Pickup，f1551（1551 mod 4 = 3）开始 `data0D 60→5F`。
- 最小变更：在 `enemy_complete_death` 写完 `animFrame` 后补「配额 + 大能量资格」两组判定，失败清槽；配额变量与随机初始化**已经端口**（门与开机两处，见 §3），不需要新增状态。

## 1 `$040D` 写点链（#140 所列的核对与修正）

| # | 位置 | asm 行 | 触发 | 写值 |
|---|---|---|---|---|
| 1 | `07:DD2A` | `prg7_engine.asm:5976` | 敌死亡（`EnemyCompleteDeath 07:DD0D`） | `data0D←$60`（×4 = 384 帧） |
| 2 | `07:F4D9` | `:10960` | Pickup 态，`FrameCount&3==0` | `dec`，到 0 `RemoveEnemy` |
| 3 | `07:F458` | `:10861` | Frozen 态，`FrameCount&7==0` | `dec` |
| 4 | `07:F46A` | `:10874` | 解冻 | `←EnemyForceSpeedTowardsSamusDelayTbl[type]` |
| 5 | `07:F860` | `:11712` | 敌初始化 | 同表 |
| 6 | `07:F6C9-F6D0` | `:11345/11352` | `InitEnActiveAnimIndex` 径 | 同表 |
| 7 | `07:F562` | `:11064` | 冰冻 | `←$40` |

读点：`07:F471`（`:10877`，冻结闪烁阈值 `<$0B`）、`:10960` 的 `dec/bne`、`:10975`（消失判定）、`:11348`。

**与 #140 的差异**：#140 §2 表把「强制朝 Samus 加速延迟」与「冻结/拾取」并列列出，没写清**同一字节的三种语义由 `status` 决定**；本表按写入点重排（死亡入口 → Pickup 递减 → Frozen 递减/解冻 → 初始化/冰冻），语义分组见 `constants_ram.asm:118-121` 的 asm 注释。其余 7 条写点与 #140 一致。

## 2 写前依赖状态（ROM 顺序，A 级）

`EnemyCompleteDeath ; 07:DD0D`（`prg7_engine.asm:5957-6047`）：

```
Lx135 (07:DD0D):
  lda Ens.prevStatus,x / asl / bmi LDD75     ; bit6=小 Boss ⇒ 另路（已实装）
  jsr ReadTableAt968B                         ; ldy EnsExtra.type,x / lda L968B,y → Temp00
  jsr LoadTableAt977B / and #$20              ; type←(L977B[type]&$20)，覆盖 type
  status ← enemyStatus_Pickup                 ; 07:DD20
  data0D ← $60                                ; 07:DD2A
  r ← RandomNumber1
  if r < $10: goto LDD5B                      ; 1/16 无掉落
LDD30 (07:DD30):
  y ← r & 7 ; frame ← PickupTypeTable[y]      ; $DE35 = 80 81 89 80 81 89 81 89
  if frame == EnFrame_MissilePickup($81):
      if MissilePickupQtyMax == MissilePickupQtyCur: goto LDD5B
      if MaxMissiles == 0: goto LDD5B
      MissilePickupQtyCur += 1 ; rts           ; 保留 Pickup
  else:
      if EnergyPickupQtyMax == EnergyPickupQtyCur: goto LDD5B
      EnergyPickupQtyCur += 1
      if frame != EnFrame_BigEnergyPickup($89): rts
      lsr Temp00 ; bcs rts                     ; bit0=1 才允许大能量
LDD5B:
  if InArea != $13(Tourian): RemoveEnemy
  else: 重置配额（cur=0、max=1）后 goto LDD30（同一 r 重试）
```

要点（均 A 级，字节已核）：

1. **`Temp00` 用的是覆盖前的 type**（`ReadTableAt968B` 在 `type` 被 `LoadTableAt977B` 覆盖之前执行）。实现时必须先取 `L968B[type_orig]`，再写 `EnsExtra.type`。
2. `PickupTypeTable` 的三个语义值：`$80` 小能量、`$81` 导弹、`$89` 大能量；本房 f1550 的 `r&7=5` ⇒ `$89`。
3. 配额比较是 `Max == Cur ⇒ 失败`（不是 `>=`），且导弹**额外**要求 `MaxMissiles != 0`。
4. Tourian 分支会**用同一个 `RandomNumber1` 重试**（`bne LDD30` 恒成立）。若失败原因是「大能量资格」而非配额，这会再次失败并再次重置 —— 原版存在潜在死循环（C 级，见 §7）。

## 3 Rust 侧对照：为什么是 95 而 ROM 是 96

现实现（`crates/game-core/src/enemy/framework.rs:565-605`）：

| ROM 步 | Rust 现状 | 结论 |
|---|---|---|
| 小 Boss 支路 | `:576-582` `prev_status()&0x40` | 已一致（#112 落） |
| `ReadTableAt968B` | `:591` 注释「本径后续不读」，**未取值** | 缺（本卡要用） |
| `LoadTableAt977B/and#$20→type` | `:597-603`（`enemy_facing(... )&0x20`） | 已一致 |
| `status←Pickup` / `data0D←$60` | `:583-584` | 已一致 |
| 1/16 无掉落 | `:585-589` | 已一致 |
| `PickupTypeTable[r&7]→animFrame` | `:602-604` | 已一致 |
| 导弹/能量配额 | **无** | 缺 |
| 大能量资格 `L968B&1` | **无** | 缺 |
| Tourian 重置重试 | **无**（仅 `DIVERGENCES 0x0DD5B`） | 缺 |

配额变量本身已端口、且写点与 ROM 一致（这是本卡能只改一处的前提）：

- `$0093 MissilePickupQtyMax`、`$0094 EnergyPickupQtyMax`、`$0095 MissilePickupQtyCur`、`$0096 EnergyPickupQtyCur`（`constants_ram.asm:697-703`，注意 **Max 在 Cur 前**，与 #140 正文的泛指顺序不同）。
- 门进入：`crates/game-core/src/samus/physics.rs:1436-1441`（`samus_enter_door`，Cur←0、Max←`Rnd&0x0F`、EnergyMax←`(r<<1)|$40`），测试 `:592-595` 已断言四字节。
- 开机/换区：`crates/game-core/src/frame/tick.rs:576-577`（`MissilePickupQtyMax←$27` 或 `$2F`，EnergyMax 同值）。
- `L968B` 全 128 项访问器已存在：`game_data::assets::rom_table_968b(area_bank(area), type)`（`crates/game-data/src/assets.rs:585`）；现有调用例 `framework.rs:115`。

⇒ 95 vs 96 不是初值或相位，而是**调用次数**：ROM 在 f1550 的同一帧内 `RemoveEnemy`（status→0，`data0D` 停在 `$60`），Rust 停在 Pickup，f1551 的递减条件成立。两侧 RNG 在该帧相同（`$E5`），`EnergyPickupQtyCur` 在 emu 已 0→1、Rust 仍 0（#140 实测）。

## 4 修复规格（最小变更，伪代码级）

改动落点：`crates/game-core/src/enemy/framework.rs`，`enemy_complete_death` 的 `let frame = tbl[...]` 之后（现 `:602-604`），**不新增文件、不改函数签名**。

```rust
// 值域常量（本文件局部，避免 ram_map 行号门禁；语义见 constants_ram.asm:697-703）
const MISSILE_QTY_MAX: u16 = 0x0093;
const ENERGY_QTY_MAX:  u16 = 0x0094;
const MISSILE_QTY_CUR: u16 = 0x0095;
const ENERGY_QTY_CUR:  u16 = 0x0096;

// 注意：l968b 必须在 type 被覆盖前取（ROM 在 07:DD11 读表、07:DD1A 才覆盖 type）
let l968b = game_data::assets::rom_table_968b(
    crate::frame::area_bank(crate::frame::cur_area(mem)) as usize,
    orig_type, // = 覆盖前 mem.enemy(slot_of(px)).enemy_type()
);
let area = crate::frame::cur_area(mem);
let drop_ok = |mem: &mut Manager| -> bool {
    if frame == 0x81 {
        if mem.read_ram(MISSILE_QTY_MAX) == mem.read_ram(MISSILE_QTY_CUR) { return false; }
        if mem.max_missiles() == 0 { return false; }
        let cur = mem.read_ram(MISSILE_QTY_CUR);
        mem.write_ram(MISSILE_QTY_CUR, cur + 1);
        true
    } else {
        if mem.read_ram(ENERGY_QTY_MAX) == mem.read_ram(ENERGY_QTY_CUR) { return false; }
        let cur = mem.read_ram(ENERGY_QTY_CUR);
        mem.write_ram(ENERGY_QTY_CUR, cur + 1);
        if frame != 0x89 { return true; }      // 小能量
        l968b & 1 != 0                          // 大能量资格
    }
};
if !drop_ok(mem) {
    if area == 0x03 {
        // Tourian（07:DD66 Lx140）：cur←0、max←1，用同一 RandomNumber1 重试（最多一次有效）
        loop {
            mem.write_ram(ENERGY_QTY_CUR, 0);
            mem.write_ram(MISSILE_QTY_CUR, 0);
            mem.write_ram(MISSILE_QTY_MAX, 1);
            mem.write_ram(ENERGY_QTY_MAX, 1);
            if drop_ok(mem) { return; }
            // ROM 在「配额失败」时会重试成功；「大能量资格」失败会再次落到 LDD5B。
            // 原版在此可能死循环（C1）；实现按 ROM 原形循环，测试只用 Tourian 可满足的类型。
            crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x0DD5B));
            break; // 见 §7 C1：不在 Stage A 引入原版可能 hang 的循环
        }
    }
    mem.enemy(slot_of(x)).set_status(EnemyStatus::NoEnemy);
}
```

实现注意：

- `frame` 取 `PickupTypeTable` 的**输出值**（`$80/$81/$89`），不是 `r&7`；表已硬编码在 `:602`。
- `orig_type` 必须在 `:597-603` 的 `type` 覆盖之前捕获；`enemy_force_attribs`（16 项）**不能用**——`ReadTableAt968B` 索引是 `type` 全 8 位、表 128 项，`type=$20` 已越界 16 项表。
- Tourian 分支先按 §7 C1 处理：记账 + 清槽。若验收要求逐字节复刻「重置后重试」，把 `break` 换成 `continue` 并加 `#[cfg(test)]` 次数上限断言。

**与 #137 的关系**：不同链、无先后依赖。#137 修的是 `07:F1FA` 判盒（`$0454/$045E` 由 `07:F2CA` 写），本卡修的是 `07:DD0D` 死亡掉落（`$040D`）。两者都在 `UpdateWorld` 帧内，但判盒在 `CollisionDetection`、死亡在敌 AI 尾，互不调用。**#137 已修**是本卡能以 f1551 为首分歧的前提。

## 5 可证伪预测

**主预测**：修复后 `--frames 4000` 的 `first_divergence` 不再等于 f1551 `$040D`，且 f1550 的 `EnsExtra.status/animFrame` 两侧一致（0/`$37`）。

**范围预测**（依据本次 4000 帧数据：f1551–f1573 全部是 `$040D` 单字节差；f1574 首次出现 7 字节差，恰是 slot0 被这枚不存在的拾取物占住后的复用差）：

- f1551–f1573 **全部变 match**；
- f1574 的 7 B（`$0401 $0405 $0406 $0409 $040B $040D $040F`）也应变 match（两槽状态一致，载房时同样被清）；
- 新的首分歧 **≥ f1575**。

**否证判据**（预测错就标否证，不改判据）：

1. 修后首分歧仍是 f1551 `$040D` ⇒ 根因判错（门链不是主因）。
2. f1551–f1573 出现 `$040D` 之外的差 ⇒ 新门引入了新差异（多半是 `l968b` 取值时机或配额初值）。
3. f1574 仍是这 7 B 同地址 ⇒ 门链执行时机不对（必须在死亡同帧清槽，不能延到下一帧的 Pickup 更新里）。
4. 若 f1574 正常而 f1575–f1900 出现 `$04xx` 差 ⇒ 属下一处未端口路径，记新卡，不回改本卡判据。

## 6 复跑

```bash
cd /home/peng/workspace/metroid-decomp
python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 4000 --range 0400-04FF --out /tmp/f1551-fix --allow-rust-missing
# 期望：first_divergence 不再是 1551/$040D；1551–1574 区间 equal
python3 - <<'PY'
import json
r=json.load(open('/tmp/f1551-fix/report.json'))
bad=[f for f in r['compare']['frames'] if 1551<=f['frame']<=1574 and not f['equal']]
print('1551-1574 unequal:', bad[:5])
PY
```

## 7 分级与边界

| 级 | 内容 |
|---|---|
| A | §2 的 ROM 顺序与三门、`PickupTypeTable` 值、`L968B[0x20]=00`、`MissilePickupQtyMax==Cur ⇒ 失败`、Tourian `cur=0/max=1` |
| A | §3 的「配额变量与写点已端口」：`physics.rs:1436-1441`、`tick.rs:576-577`、`rom_table_968b` 存在 |
| B | 95 vs 96 归因「缺门 ⇒ 保持 Pickup ⇒ f1551 递减」——机制链已由 #140 帧表支撑，但没有在 Rust 侧单步运行到 `frame=F4D9` 的探针 |
| C1 | Tourian 分支用同一 `RandomNumber1` 重试「大能量资格」失败时，原版可能死循环；本卡建议按 ROM 原形实现但 Stage A 暂用「记账 + 清槽」避免 hang，需验收确认 |
| C2 | 原版 `L968B` 表项在 0x10–0xFF 段的语义注释多为 unused，实际运行中 `type>=0x10` 是否可达未经穷举 |

未覆盖：导弹/能量配额的随机初值在门进入时由 `RandomNumber1` 决定，Rust 已端口但本卡未对「门进入后第一枚掉落」做端到端复跑；若修后出现配额相关新差，按 §5 判据 2 处理。
