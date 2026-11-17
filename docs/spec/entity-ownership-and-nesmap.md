# 实体所有权 + NES 映射声明（架构裁定与迁移方案）

> 状态：**提案（待审）**。上游：`docs/spec/entity-scene-model.md`（D-01）、
> 用户裁定「不接受继承 NES 内存模型」「每个 struct 自带映射规则，映射单向只读」。
> 独立对抗复核：子代理只读取证（2026-11-17），结论已并入本文 §2、§4、§5。

## 1. 目标与非目标

**目标**：实现态由 Rust 语义类型拥有；NES 字节面降级为**只读投影**（仅服务对拍与 dump）。
业务代码不得以地址为 API。

**非目标（且不宣称）**：
- 不宣称"可达状态完备"（D7：行为空间无机械分母）。
- 不宣称"像素相等证明语义等价"（L4 只裁对不对）。
- 不追求"类型系统证明一切"；目标是**每个字节都有一个被声明的解释，假解释在门禁里红**。

## 2. 独立复核的关键否决（已核实，改方案）

| # | 复核发现 | 事实核对 | 对方案的修改 |
|---|---|---|---|
| R1 | ram.hex 尾段只到 `$6B73`（128 B = 敌槽 0–7），敌池有 12 槽（`SLOT_COUNT=12`，末尾 `$6BB3`），爆炸额外面从 `$6BB4` 起 | `crates/game-core/src/bin/rust_screenshot.rs:472-480` 实测：前段取 `snapshot_nes()`、尾段取 `read_wram()`，**两来源同文件**；`enemy/state.rs:122` SLOT_COUNT=12 | 比较面改 `$0000-$07FF + $6AF4-$6BD3`（2240 B），**整面单一来源**；先补 dump 尾段再谈覆盖度 |
| R2 | 「每字节恰一认领」在当前布局下**不可满足**：`$0307` 同时是 `SamusOnElevator`（`samus/state.rs`）与对象槽 `data07`／门槽 `DoorType`（`world/object.rs`） | 两处常量表并存，且门绘制按对象槽读同址 | 判据降级为 **「每字节恰一个 owner + N 个声明 view」**；`ObjectPool` 持 `$0300-$03FF`，Samus 是 slot0 的 typed overlay（与现状等价） |
| R3 | `restore_mss` 无条件拷回 `0x800`，typed 格在 `storage.ram` 留下副本 ⇒ 直接读 `ram()` 会拿到 stale，而测试仍绿 | `frame/memory.rs:154-176`；`memmap/storage.rs:80` 只在全新 Manager 上断言 | restore 末尾对 typed 地址清零 + 断言；导出只有一个出口（`snapshot_nes()`） |
| R4 | 「字段分借 + systems」不足以覆盖真正跨域例程（同一函数交错写 Samus/敌/弹/声旗标，且每敌循环重置 `$010F`） | `weapon/projectile.rs:369-395`、`samus/physics.rs:943` | systems **只换签名不改写入顺序**；跨槽一律 `CollisionCtx<'a>{ mem: &mut Manager }` + 槽索引，禁止 `(&mut samus, &mut enemies[i])` |
| R5 | 搬目录会被棘轮门挡：新文件接裸面即 FAIL，总量上涨即 FAIL；依赖方向门已按 m21400 裁掉 | `gates/cli.py:2274-2316`、`data/status/raw_surface.json` | **串行前置**：`raw-surface` 归零后才搬目录；迁移期只 `git mv`（不新增账本键）；搬完先立 G11′ |
| R6 | 「每个可达状态恰一认领」没有机械分母 | AGENTS.md D7 | 静态判据（并集=全域、交集=∅或白名单）+ 动态插桩（write 断言 owner 唯一），**不宣称**完备 |

## 3. 裁定

- **D-02（状态所有权）**：终局为**实体自持**（`Samus`/`Enemy`/`Object`/`Explosion`/`Room`/`Camera` 各持字段，`impl` 带动作，无镜头类型）。
  过渡期 Samus 仍按 D-01 作为「对象池 0 号槽的类型化别名」——**两套模型不同时持同一字节**，切换按域、按阶段（§5 P5）。
- **D-03（每字节归属）**：`$0000-$07FF` + `$6AF4-$6BD3` 每字节有且仅有**一个 owner**；其他解释必须声明为 **view（只读）** 并进 `views` 白名单。`owner` 冲突 = 门禁红。
- **D-04（判据来源）**：SPANS / 字段偏移 / 槽步长一律**从 `refs/m1disasm/out/M1_NES_NTSC.sym` 机械生成**，禁止 import Rust 侧常量（避免"两份手写清单互查"式永绿）。
- **D-05（顺序）**：先让字节面可归零（typed 化 + 门），再做目录重组（OOP 子系统）。理由：重组期没有门可守新结构。

## 4. 映射声明形态

```rust
pub struct NesSpan { pub start: u16, pub len: u16, pub kind: SpanKind }
pub enum SpanKind { Value, Bits(&'static str), Bcd(&'static str), Derived(&'static str) }
pub enum SpanKey { Owner, View(&'static [&'static str]), Case(KindValue), Mode(ModeValue),
                   Ephemeral(&'static str), Unmodeled(&'static str) }
pub trait NesMapped {
    const DISCRIMINATOR: Option<Discriminator>;      // 本类型的状态判别字段
    const SPANS: &'static [(NesSpan, SpanKey)];
    fn project(&self, out: &mut NesImage);           // 单向；拿不到反向入口
}
```

七种复用形态的落地：
1. 同义多名 → 一个字段 + 只读别名访问器（`data06()/movement_instr_index()`），span 记 `View(&[…别名…])`。
2. 按状态重解释 → `DISCRIMINATOR` + `Case(..)`；取用侧 `debug_assert!(kind)`。
3. 按模式重解释 → `Mode(..)`（`GameMode`/`TitleRoutine`）；要求每个模式都被完整认领。
4. 池重叠 → `EnsFace` 共享 face（敌 16 / 爆炸 8 步长），`$6AF4+$C0==$6BB4` 写进**门**而非注释。
5. 例程临时量 → `Ephemeral(reason)`，不 project，不参与判据（低址 Temp 差与像素无关）。
6. 镜像别名 → 主地址 + `View`；`NesImage` 的 `&0x7FF` 归一口径负责展开。
7. 位域 → `Bits(..)`；按字节计一次 owner。

## 5. 迁移方案（P0→P7）

| 阶段 | 内容 | 验收 | 回退 |
|---|---|---|---|
| **P0** | dump 尾段补 `$6B74-$6BD3`；ram.hex 整面单一来源（投影） | `run-all` 的 diff ≤ 6，且旧 6 格 px 不变 | 纯增 |
| **P1** | `restore_mss` 清 typed 格 + 断言；导出唯一出口 | 新增 roundtrip 断言；矩阵不变 | 单 commit |
| **P2** | 四域 `impl NesMapped`（Samus/Enemy/Explosion/Object），**不动存储** | `nes-map` 静态腿（并集/交集/views 白名单） | 声明层可整体删 |
| **P3** | `nes-map` 动态腿（测试内插桩断言 write 的 owner 唯一） | 132 格 + 录像 f0–f4017 零冲突 | 关插桩 |
| **P4** | 剩余族 typed 化：`$006E/$006F`、`$0683/$0684`、OAM 影子、zp 各族 | `raw_surface` 逐批 `--write` 下压；`no_raw_addr` 受监控族递减 | 逐族 revert |
| **P5** | 实体自持，顺序 **Explosion → Enemy → Object → Samus（最后）** | 每步 `cargo test` + 矩阵不退化 + capture/restore 往返 | 保留 `NesStorage` 兜底 |
| **P6** | 目录重组（`entities/ world/ systems/ render/ …`） | 新 **G11′** 依赖方向门 + 账本路径重映射（总量不变） | move 可 revert |
| **P7** | `read_ram/write_ram` 降为 `memmap` 内部件（`raw-surface` 生产面 = 0） | 门读数归零 | 保留 shim 一版 |

**删 `NesStorage` 的五条硬条件**：①`raw_surface` 生产面=0 且 files 空；②`no_raw_addr` 受监控族=0；
③`nes-map` 静态并集=2240 B、动态腿零冲突；④capture/restore 全状态字节同；⑤`storage.ram|wram` 只在 `memmap` 内出现。

## 6. 测试清单（每条都写明"防自证"手段）

| 层 | 断言 | 位置 |
|---|---|---|
| 静态 | SPANS 并集 = `$0000-$07FF ∪ $6AF4-$6BD3`；两两交 = ∅（除 views 白名单） | `tools/nes_map_check.py`（偏移从 `.sym` 生成） |
| 契约 | `read_ram(a) == snapshot_nes().ram[a]` 全 0x800 逐字节 | `memmap/storage.rs` |
| 契约 | 任意字节 `set_byte→get_byte` 往返（固定 seed 随机），不用常量表 | `samus/state.rs`、`enemy/state.rs` |
| 对拍 | 新增 case 覆盖爆炸槽 1–3 与敌槽 8–11（当前 dump 盲区） | `cases/` + `run-all` |
| 对拍 | `$6AF4+$C0 == $6BB4` 恒等式（值取自 `.sym`） | `tools/nes_map_check.py` |
| 快照 | `restore(capture(x)) == x`（typed + 字节双面）；restore 后 `ram()[typed_addr]` 全 0 | `frame/memory.rs` tests |

## 7. 子系统终局（OOP）

```
entities/{samus,enemy,object,explosion,projectile}   # 自持态 + 动作；slot 只是索引
world/{room,camera,door,elevator}
systems/{damage,collision,pickup,scroll,anim}        # 只持 CollisionCtx{ mem: &mut Manager }
render/  input/  audio/  title/  platform/
```

依赖方向（单向、由 G11′ 守）：
`platform → shell → systems → {world, entities} → memmap → state(类型)`；
`memmap` **不得**依赖 `entities`（否则投影成环）；`entities` 不得 `use systems`。

## 8. 未取证项（不得当结论）

1. R3 的 restore 污染：读码推断，**未跑断言**。
2. `_id` 相关 hover 面：`enemy/explosion.rs:5` 的文档仍写「额外面 `$6BB4 + i*16`」与 `EXTRA_STRIDE=8` 自相矛盾——文字漂移，待改。
3. `matrix.json` 的 rows 无 per-cell status 键，6 个 diff 格对应的具体 3 案未逐格取证。
