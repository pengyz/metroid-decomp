# f1301 修复规格：`$0454/$045E` 写点链与最小变更

> 状态：**取证完成**（task #130，2026-11-27）。前置：#120「f1301 定界」、#122「语义/写点/预测」。
> 本文件只给规格与预测，不改 `crates/`（写面边界）。

## 0. 结论（A 级）

- 分歧起因不是"多算了一次命中"这么笼统：**Rust 的碰撞判盒漏了实体高字节（nametable）`hi`**。
  f1301 时敌方在 `EnsExtra.5.hi=0`（世界格内），子弹在 `Objects.13.hi=1`（下一张 nametable），
  两者在**屏幕空间**相距 262 px；ROM 用 16 位位置比较判为不碰，Rust 只比低字节（6 px）判为碰。
- 因此 `$0454`（`Ens.5.isHit` bit5）与 `$045E`（`Ens.5.weaponAction`）在 f1301 被同时置位，
  下一帧 `EnemyReactToSamusProjectile` 把子弹 `status` 从 1 改成 4（爆），与 emu 分叉。
- 修复最小面：**碰撞判盒按 ROM 恢复"轴 + 高字节"比较**（横向房比 X 的 hi、纵向房比 Y 的 hi，
  并做 240 行补偿）；不动伤害/状态机/绘制。

## 1. 帧数据（A，`/tmp/d120`：`--range 0300-04FF`，1400 帧）

| 帧 | 侧 | 子弹 `$03D0`：status/y/x/hi | 敌 `$0450`：y/x | 敌 `$0454` isHit | 敌 `$045E` weaponAction |
|---|---|---|---|---|---|
| 1299 | emu | 1 / 169 / 126 / 1 | 154 / 120 | 00 | 00 |
| 1300 | emu | 1 / 165 / 126 / 1 | 154 / 120 | 00 | 00 |
| 1300 | rust | 1 / 165 / 126 / 1 | 154 / 120 | 00 | 00 |
| 1301 | emu | 1 / 161 / 126 / 1 | 154 / 120 | **00** | **00** |
| 1301 | rust | 1 / 161 / 126 / 1 | 154 / 120 | **20** | **01** |
| 1302 | emu | 1 / 157 / 126 / 1 | 154 / 120 | 00 | 00 |
| 1302 | rust | **4** / 157 / 126 / 1 | 154 / 120 | 20 | 01 |

- 两侧低字节位置逐帧相同（169→165→161→157、154/120），`hi` 也相同（子弹 1、敌 0），
  所以差异不在位置生成，而在**判盒是否读 `hi`**。
- `$03DC`=1（`Objects.hi`）、`$6B4B`=0（`EnsExtra.5.hi`）在 f1300/1301 都成立（rust 状态 dump；
  `EnsExtra` 槽在 #120 已逐字节比对两侧相等，见 `docs/findings/tas-divergence-f1301.md` §3）。
- 滚动状态：`ScrollDir($49)=3`（向右）、`ScrollX($FD)=F4→F5`、`ScrollY($FC)=0` ⇒ **横向房**。

## 2. ROM 写点链（A，逐跳）

1. `UpdateWorld 07:CB29`（`prg7_engine.asm:2494`）在 `UpdateSamusProjectiles` 之后 `jsr CollisionDetection`（`:2512`）。
2. `CollisionDetection 07:F034`（`:10060`）敌槽环：`status==0|3` 跳过；`==5` 只跳过"弹×敌"子环（`:10151`）。
3. 每弹槽（`X`=敌槽字节偏移、`Y`=弹槽）：弹型过滤 `<4 ∨ 7 ∨ 0A ∨ 0B`（`:10157-10165`）→
   `CollisionDetectionEnemy_CheckWithObjectYSlot 07:F140`（`:10243`）。
4. `GetRadiusSumsOfEnXSlotAndObjYSlot 07:F1BF`（`:10313`）= `EnsExtra.radiusY/X` + `Objects.radiusY/X`。
5. `CheckCollisionOfXSlotAndYSlot 07:F1FA`（`:10366`）：
   - 先 Y：`Temp03 = $02 & ScrollDir`；**为 0 时按 Y 的高字节差做 16 位比较**并 `-($100-$F0)= -16` 补偿 240 行，
     `Temp01_DiffHi != 0` 直接 `@RTS`（carry 置=不碰）；
   - 再 X：`ScrollDir & $02 != 0`（横向房）时**同样按 X 的 `hi` 做 16 位比较**，高字节非零即不碰；
   - 低字节差 `>=` 半径和 ⇒ 不碰，否则 carry 清=碰（`:10394-10430`）。
6. `CollisionDetectionEnemy_ReactToCollisionWithSamusProjectile 07:F2CA`（`:10561`，仅 carry 清进入）：
   `Objects.status,y → Ens.weaponAction,x`（`$040E,x`）、`jsr SetSamusProjectileIsHit`（`:10493`，置弹 `$03DA |= $20`）、
   `GetEnemyIsHitFlags 07:F332`（`:10645`）→ OR 进 `Ens.isHit`（`$0454,x`）。

f1301 的实际走法：Y 差 7 < 半径和 10（通过）；X 低字节差 6，但 `hi` 差 1 ⇒ ROM 的 16 位比较得 262（≥256）
⇒ carry 置 ⇒ **不进 F2CA**，两字节都不写。Rust 只比 6 ⇒ 进 F2CA。

## 3. Rust 侧对照（文件:行）

- `crates/game-core/src/weapon/projectile.rs:490 uw_collision_detection`；
  判盒在 `:529-541`：`let dy=(ey-py_y).unsigned_abs(); let dx=(ex-py_x).unsigned_abs();
  if dy >= ery+pry || dx >= erx+prx { continue; }` —— **只读 `$0400+x+0/1` 与 `py+0x0D/0x0E` 的低字节，
  没有读 `$6AF4+x+7`（EnsExtra.hi）与 `py+0x0C`（Objects.hi），也没有 `ScrollDir` 分支**。
- 命中写点 `:544`（`$040E,x ← pstat`）、`:545`（`py+0x0A |= 0x20`）、`:556`（`$0404+x |= flags`）与 ROM 一致；
  `enemy_is_hit_flags`（`:503`）=`(dist_hi^3)<<3` 与 `GetEnemyIsHitFlags` 一致。
- 下一帧 `crates/game-core/src/enemy/framework.rs:177 enemy_react_to_projectile` 读 `isHit` bit5 →
  `:220 set_status(Hurt)`、`:223 explode_enemy` → 子弹 f1302 `status=4`。整条链自洽，**唯一错处是判盒缺 hi**。

与 #122 的关系（写点无需修正，只补根因）：#122 的写点链结论（`$0454`=`Ens.isHit`、`$045E`=`weaponAction`、
`$045C`=`prevStatus`）与本文件一致，无差异需要改；#122 的 C4「环内读到 Resting」假设在本例上不必成立——
`prevStatus` 未被写（f1301 内外 `$045C` 相同），被写的是 `isHit/weaponAction`。#122 的 P1（预测下一处 f1308）
是按差异表顺序推的，**修复后作废**，原因见 §6：f1308 的槽 2 差异预期同属判盒链（同一 `ScrollDir`、`hi` 不同），
会随本修复一起消失；是否真的同因由修复卡复核，若不消失则按 §6 否证判据 2 单列。

与 #122 已修正口径的关系：#122 已定 `$045E`=`weaponAction`（`wa_RegularBeam=1`）、`$045C`=`prevStatus`；
本文件不改那两条，只是把"为什么会被写"落到 `hi` 上。

## 4. 最小变更（伪代码级，写面外，供修复卡照抄）

```rust
// crates/game-core/src/weapon/projectile.rs —— 用 ROM 的 16 位轴比较替换现有 dy/dx 判盒。
// 忠实 CheckCollisionOfXSlotAndYSlot（07:F1FA）：
//   (lo, borrow) = a - b; dhi = hi_a - hi_b - borrow（比 hi 且 hi 不等时）
//   dhi = -borrow（比低字节，或 hi 相等时）
//   dhi < 0 ⇒ NegateTemp00Temp01 对 (lo,dhi) 取补
//   纵向房 Y 轴在 hi 不等时再补偿 240 行：lo -= (256-240)=16，借位则 dhi -= 1
//   dhi != 0 ⇒ 不碰（ROM 的 `lda Temp01_DiffHi / bne @RTS`）
fn rom_axis_delta(a: u8, ha: u8, b: u8, hb: u8, use_hi: bool, compensate_240: bool) -> Option<u8> {
    let (mut lo, borrow0) = a.overflowing_sub(b);
    let mut dhi: i16 = if use_hi && ha != hb {
        ha as i16 - hb as i16 - borrow0 as i16 // sbc 的借位必须进 dhi
    } else {
        -(borrow0 as i16) // ROM 的 `lda #$00 / sbc #$00`
    };
    if dhi < 0 {
        let (nlo, borrow1) = 0u8.overflowing_sub(lo); // NegateTemp00Temp01
        lo = nlo;
        dhi = -dhi - borrow1 as i16;
    }
    if compensate_240 && use_hi && ha != hb {
        let (nlo, borrow2) = lo.overflowing_sub(0x100 - 0xF0);
        lo = nlo;
        if borrow2 { dhi -= 1; }
    }
    if dhi != 0 { None } else { Some(lo) }
}

let scroll_dir = mem.read_ram(crate::ram_map::SCROLL_DIR); // $49
// hi 比较前与当前 nametable 基址归一（asm：GetX/YSlotPosition_Common 做 `eor PPUCTRL_ZP; and #$01`）
let base = mem.read_ram(crate::ram_map::PPUCTRL_ZP) & 0x01; // $FF 影子（$00FE 是 PPUMASK，别用）
let ehi = mem.read_wram(0x6AF4 + x + 7) ^ base;  // EnsExtra.0.hi
let phi = mem.read_ram(py + 0x0C) ^ base;        // Objects.hi

if scroll_dir & 0x02 == 0 {
    // 纵向房：Y 主轴比 hi 且做 240 行补偿；X 只比低字节
    if rom_axis_delta(ey, ehi, py_y, phi, true, true).unwrap_or(0xFF) >= ery + pry { continue; }
    if rom_axis_delta(ex, ehi, py_x, phi, false, false).unwrap_or(0xFF) >= erx + prx { continue; }
} else {
    // 横向房：X 主轴比 hi；Y 只比低字节
    if rom_axis_delta(ex, ehi, py_x, phi, true, false).unwrap_or(0xFF) >= erx + prx { continue; }
    if rom_axis_delta(ey, ehi, py_y, phi, false, false).unwrap_or(0xFF) >= ery + pry { continue; }
}
```

要点：①`hi` 必须按槽读（敌 `$6AF4+x+7`、弹 `py+0x0C`）；②轴由 `ScrollDir & 2` 决定，
不是两边都加 hi（否则原版"同屏但 hi 不同"的合法擦边会被误杀）；③`>=256` 一律不碰（ROM 的 `DiffHi!=0` 早退）；
④不改半径和、不改 `enemy_is_hit_flags`、不改 F2CA 的写顺序。⑤伪代码只表达"主轴带 hi、另一轴仍低字节"；两轴的边界/借位细节以 `07:F1FA` 原文为准，落地时优先抽成 helper 供拾取碰撞（`07:F1A7`）复用。

**§4 的四组向量（验收用，同 hi/跨 hi 都要覆盖）**

| a(hi) | b(hi) | `rom_axis_delta(...,use_hi=true)` | 说明 |
|---|---|---|---|
| 120(0) | 126(1) | `None`（dhi≠0） | 本卡 f1301 场景：262 px，不碰 |
| 120(0) | 130(0) | `Some(10)` | 同 hi、目标在左：借位进 dhi→取补成 10 |
| 130(0) | 120(0) | `Some(10)` | 同 hi、目标在右，正差原样 |
| 0(0) | 0(0) | `Some(0)` | 重合 |

（首版伪代码漏了「同 hi 但低字节为负」的借位，会把 10 算成 246 ⇒ 永远不碰；已按上表修正。）

**注意**：hi 归一只用 `PPUCTRL_ZP($00FF)` 的 bit0。`$00FE` 是 `PPUMASK_ZP`，其 bit0 是 greyscale 开关；误用它会让跨 nametable 场景（正是 f1301）静默失效。

## 5. 与 #129「门径窗 3 vs 2」的先后

f1301 < f1572，本修复**必须先落**：`hi` 判盒修好后，f1301–f1349 这一串 `$0454/$045E/$0424/$042E…`
预期一起消失；#129 管的 f1572 之后的分歧是另一条链（门/滚动窗），两者不能互相掩盖——
若先改门径窗，f1301 仍在，`replay` 的首分歧仍报 f1301，会把 #129 的效果掩盖掉。
该顺序约束也应由 #129 卡 note 记录（我无该卡写权限，请在验收/派卡时补一条「不得先 pick #129」）。

## 6. 可证伪预测（成对）

- **主预测**：修好后 `tas_replay_diff.py replay --frames 2200 --range 0400-04FF` 的 `first` 不再是 f1301；
  f1301/f1301–f1349 段内 `$0454/$045E/$0424/$042E/$0420/$0421/$0429/$042B/$042C/$042F` 全部消失。
- **次预测**：首分歧预期后移到 **f1551 `$040D`**——注意它只是 #122 记录里的下一处新地址，不是本卡推导出来的（C1）；
  若 f1551 也属同链（`$040D`=槽5 `delay`），则首分歧继续后移到 **f1572–f1581 簇**（`$0401/$0405/$0406/$0409/$040B/$040F/$0410/$0415/$0445/$0455/$041A/$0411`）。
- **否证判据**：
  1. 修后 `first` 仍是 f1301 `$0454`（或前移到更早）⇒ 根因不是 `hi` 判盒，回滚并重查；
  2. 修后 f1301 消失但 f1308 `$0424` 仍在 ⇒ 只修掉了槽5，槽2 另有同因/异因，需单列；
  3. 任何"修后新增格"或 run-all 新增 diff ⇒ 修复越界。

## 7. 复跑命令（B/A）

```bash
# 1) 2200 帧定界（比 #122 的 2200 帧口径一致）
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 2200 \
  --range 0400-04FF --out /tmp/f1301-fixcheck --allow-rust-missing

# 2) f1300/1301 槽字节核对（本文件 §1 表）
python3 - <<'PY'
import json
for line in open('/tmp/d120/emu/seg-000000-001400.jsonl'):
    r=json.loads(line)
    if r['f'] in (1300,1301):
        b=bytes.fromhex(r['w']['ens'])
        print('emu',r['f'],'Ens5',b[0x150:0x160].hex(),'obj3D0',b[0xD0:0xE0].hex())
PY

# 3) 两个零页影子一起打印，防止地址混淆（本节的 $00FF vs $00FE 教训）
python3 - <<'PY'
import json
r=json.loads(open('/tmp/f1301-fixcheck/rust.jsonl').readline())['w']['zp']
print('PPUCTRL_ZP($FF)=', r[0xFF], ' PPUMASK_ZP($FE)=', r[0xFE])
PY
```

期望：修复前命令 1 输出 `first={f1301,$0454,offset84,emu0,rust32,diff2}`；修复后 `first.frame > 1301`
（按 §6 预测到 f1551 或 f1572）。命令 2 输出与 §1 表逐字节一致。

## 8. 假设区（C）

- **C1**：f1551 `$040D`（槽5 `delay`）是否同链未定；若修后它成为首分歧，应按"新首分歧"另立卡，不在本卡硬判。
- **C2**：纵向房的 240 行补偿在 Rust 侧是否已有等价实现（`ScrollY` 相关路径）未核；本文件给的是 ROM 语义，
  落地时若发现既有 helper 已做补偿，复用而不是重写。
- **C3**：`Objects.hi` 的置位时机（子弹过页）未在本卡核；以 f1300 实读=1 为准。

## 9. 未覆盖

- 修后的完整 30000 帧回归（预期 `diverge` 从 28659 下降）不在本卡；由修复卡跑。
- 8×8/16×16 绘制面与 OAM 面不动，本卡不涉及。
