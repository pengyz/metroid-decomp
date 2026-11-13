# 声链 handler 触达面：28 件已接臂中 24 件在 L4 矩阵里零触达

> 2026-11-13 · 止血件 1 第 ③④ 件 · 证据级别 A（实测打点 + asm 逐行标签）
> 相关 commit：`a880789f`（sidecar 落盘 + 未接臂清单）、`95913336`（增长即红门）、
> `eb178e08`（L4 移出 hook）、本次（DSH_SOUND_TRACE + 本节）

## 1. 问题：已接 ≠ 已验证

声链在 `crates/game-core/src/frame/sound_hook.rs` 有两条出口：

| 出口 | 现状 | 账上可见性 |
|---|---|---|
| **未接臂** `_ => None`（`sound_hook.rs:389-391`） | 22+ 件未知 handler 落 `DIVERGENCES` | ✅ 可见（sidecar → `data/status/handler_touchlist.json`，8 址） |
| **已接臂** 28 件 handler match（`sound_hook.rs:280-380`） | 有实现，但从无触达观测 | ❌ **不可见**（"已接且高频" 与 "已接但从未跑到" 是同一行） |

因此 `a880789f` 固化的触达清单只覆盖未接臂 —— 缺口演化的另一半（已接臂是否真的被走到）完全没数据。

## 2. 打点：`DSH_SOUND_TRACE`

`sound_hook.rs` 新增：

```rust
fn sound_trace(mem: &FrameMemory, ptr: u16, depth: u8, kind: &str) {
    if std::env::var("DSH_SOUND_TRACE").is_ok() {
        eprintln!("[sound] fc={:02X} ptr={:04X} kind={} depth={}", mem.read_ram(0x002D), ptr, kind, depth);
    }
}
```

两个调用点（一处一个 kind，同帧同 ptr 不会双计）：

- `if let Some(out) = out { sound_trace(mem, ptr, depth, "handler"); ... }`（已接臂真正执行处）
- `sound_trace(mem, ptr, depth, "unhandled");` 紧跟 `DIVERGENCES.push(ptr)`（未接臂）

不开环境变量时只有一次 `getenv` + 分支，零分配零输出。

> **实施中自查出来的错**：第一版把 handler 打点放在 `if let Some(out)` **之前**，导致
> `ptr=B7E7` 同帧既打 `handler` 又打 `unhandled`（`B7E7` 其实走 `_ => None`，out=None）。
> 修正后复核：`awk '$2,$3' | uniq -d` 为空 —— 同帧同 ptr 只有一个 kind。

## 3. 实测覆盖面（15 个 sched 全跑）

`DSH_SOUND_TRACE=1` 逐案跑 `shots_rust/*.sched`（15 个，含 `bundle-ff`），聚合：

| 指标 | 值 |
|---|---|
| 已接 handler 臂总数 | **28** |
| L4 矩阵触达的 | **4** |
| **零触达的** | **24（85.7%）** |

### 3.1 触达的 4 件

| addr | asm 标签 | asm 锚 | 事件数 | 首帧 | 案数 |
|---|---|---|---|---|---|
| `0xB7D9` | `BeepSFXInit` | `sound_engine.asm:1227` | 130 | fc=02 | 1（password） |
| `0xB878` | `BombLaunchSFXInit` | `sound_engine.asm:1336` | 24 | fc=30 | 1（password） |
| `0xBC7A` | `GotoMusic01Init` | `sound_engine.asm:2165` | 28 | fc=01 | **14（全部）** |
| `0xBC80` | `GotoMusic03Init` | `sound_engine.asm:2171` | 14 | fc=13 | **14（全部）** |

只有两件音乐 init 组合臂跨全案覆盖；两件 SFX init（Beep / BombLaunch）**只在密码用例**出现。

### 3.2 零触达的 24 件

| addr | asm 标签 | asm 锚 |
|---|---|---|
| `0xB50F` | `SpitFlameSFXInit` | `sound_engine.asm:626` |
| `0xB52B` | `ScrewAttackSFXInit` | `sound_engine.asm:650` |
| `0xB56E` | `MissileLaunchSFXInit` | `sound_engine.asm:705` |
| `0xB583` | `BombExplodeSFXInit` | `sound_engine.asm:726` |
| `0xB598` | `SamusWalkSFXInit` | `sound_engine.asm:752` |
| `0xB5EC` | `BossHitSFXInit` | `sound_engine.asm:806` |
| `0xB650` | `SamusHitSFXCont` | `sound_engine.asm:869` |
| `0xB673` | `SamusHitSFXInit` | `sound_engine.asm:896` |
| `0xB695` | `IncorrectPasswordSFXInit` | `sound_engine.asm:924` |
| `0xB6CD` | `MissilePickupSFXInit` | `sound_engine.asm:971` |
| `0xB6E7` | `EnergyPickupSFXInit` | `sound_engine.asm:994` |
| `0xB703` | `SamusJumpSFXInit` | `sound_engine.asm:1024` |
| `0xB710` | `EnemyHitSFXInit` | `sound_engine.asm:1037` |
| `0xB716` | `BulletFireSFXInit` | `sound_engine.asm:1045` |
| `0xB7AC` | `DoorOpenCloseSFXInit` | `sound_engine.asm:1195` |
| `0xB7EF` | `BigEnemyHitSFXInit` | `sound_engine.asm:1247` |
| `0xB834` | `SamusToBallSFXInit` | `sound_engine.asm:1290` |
| `0xB8A7` | `MetroidHitSFXInit` | `sound_engine.asm:1378` |
| `0xB8D2` | `SamusDieSFXInit` | `sound_engine.asm:1414` |
| `0xB921` | `StatueRaiseSFXInit` | `sound_engine.asm:1463` |
| `0xBC77` | `GotoMusic00Init` | `sound_engine.asm:2162` |
| `0xBC7D` | `GotoMusic02Init` | `sound_engine.asm:2168` |
| `0xBC83` | `GotoMusic04Init` | `sound_engine.asm:2174` |
| `0xBC86` | `GotoMusic05Init` | `sound_engine.asm:2177` |

## 4. 这 24 件意味着什么（不要读成实现错）

**不是实现 bug，是验证覆盖缺口。** 已接臂的代码路径很可能正确（多数有 asm 逐行对照与纯函数
单测），但 **L4 矩阵的 14 个场景从未把游戏驱到那些状态**：

- 射击类（`BulletFire` / `MissileLaunch` / `ScrewAttack` / `SpitFlame` / `BombExplode`）——
  用例里有 `shootprobe` 但只跑基础走路帧，没到武器发射的 SFX 分支；
- 受击类（`SamusHit` / `EnemyHit` / `BossHit` / `MetroidHit` / `BigEnemyHit`）——除了
  `killenemy-probe` 的击杀帧，缺受击路径；
- 移动类（`SamusWalk` / `SamusJump` / `SamusToBall`）——walk/jump/morph 用例存在，但
  SFX init 的触发条件（例如 `SamusWalkSFXInit` 需要 `$0688` 特定形态）未被命中；
- 拾取类（`MissilePickup` / `EnergyPickup`）、死亡（`SamusDie`）、雕像（`StatueRaise`）、
  门（`DoorOpenClose`）——完全无场景。

结论：**L4 的 110/112 match 只证明「矩阵走过的那条窄路」是对的**（AGENTS §0 D7 早已声明：
对拍只裁对不对、不证全不全）。本清单首次把「已接臂的未验证面」量化成 24/28。

## 5. 增长即红（已落地，`95913336`）

`gates/cli.py` 的 `l4_touchlist_violations()` 在 `gate_l4` 里跑：

- **未接臂面**：sidecar 出现 `data/status/handler_touchlist.json` 清单外的 addr ⇒ FAIL；
- fail-closed 四态：清单缺失/解析失败/entries 空 / sidecar 件数 < `cases_total` / sidecar 解析失败。

> 注意：本门禁目前只管**未接臂**（`entries`，8 址）。已接臂的 `handlers` 段（28 件）
> 是本次新增的**观测基线**，暂不进门禁 —— 因为「某臂从触达 → 零触达」在语义上可能是
> 合法的场景变更（用例改了），需要人判。若要机械化，应做成「新增零触达臂必须显式登记」。

## 6. 复现命令

```bash
# 单案打点
DSH_SOUND_TRACE=1 cargo run -q -p game-core --bin rust_screenshot \
  -- --schedule shots_rust/case-password-justin.sched --out /tmp/rsnd 2>/tmp/snd.log

# 分 kind 计数
grep '^\[sound\]' /tmp/snd.log | sed 's/.*kind=\([a-z]*\).*/\1/' | sort | uniq -c

# 全案（15 sched）
for f in shots_rust/*.sched; do n=$(basename "$f" .sched)
  DSH_SOUND_TRACE=1 cargo run -q -p game-core --bin rust_screenshot \
    -- --schedule "$f" --out /tmp/rsnd_$n 2>/tmp/snd_$n.log >/dev/null
done
```

## 7. 遗留（未做，登记不臆造）

1. **已接臂单测覆盖**：24 件零触达臂中哪些有纯函数单测（`sound::*_sfx_init` 直调）？
   有单测 ≠ 有集成验证，但至少能区分「实现正确性有证据」与「完全无证据」两级。
2. **`B7E7 BeepSFXCont` / `B87F BombLaunchSFXCont` 仍未接**（`_ => None` 却高频：
   526 / 109 事件）：asm 全形已读（`sound_engine.asm:1240` = `jsr IncrementSFXFrame /
   bne LoadSQ1PeriodLow@RTS / jmp EndTriSFX`；`:1345` = 4 帧后 `inc SFXTriVar0` 到 2 再
   `jmp EndTriSFX`，`IncrementSFXFrame: ; $B4A9` 是 `inc SFXNoiseFrame,x / cmp SFXNoiseLength,x`）。
   接线需要 `EndTriSFX`（`:1363`）、`LoadSQ1PeriodLow`（`:1176`）与 `NoiseUsedBySFX($0652)`
   的完整语义 —— 属独立卡，不在止血件 1 范围。
3. **场景扩面**：要真正验证那 24 件，需要新用例（射击/受击/拾取/死亡）。
   按用户裁定的路线丙，止血期不开新场景卡。
