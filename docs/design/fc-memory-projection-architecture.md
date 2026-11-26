# FC 内存投影架构：现状取证、目标边界与迁移判断

> 状态：**分析稿（待复核）**。作者：承宇（架构）。落盘日期：2026-11-23。
> 上游：`docs/spec/entity-ownership-and-nesmap.md`（目标方案提案，状态=待审）、
> `docs/design/data-driven-architecture.md` 等 5 篇数据驱动文档、
> `docs/findings/typed-domain-migration.md`（迁移实录）。
> 本稿不推翻上游方案；它补的是上游缺的三块：**现状逐项取证**、**与数据驱动文档的冲突点**、
> **迁移判断与可机械检查的触发条件**。
> 纪律：本稿每条判断都给出可复跑的命令或 `文件:行` 指针；未执行的一律在 §6 显式声明。
> 版本：v2（2026-11-23 返工自审，修正清单见 §7）。

---

## 0. 证据基线（本稿读数的时间点）

| 项 | 值 | 取得方式 |
|---|---|---|
| git HEAD | `495426ae` | `git rev-parse --short HEAD` |
| 读数时间 | 2026-11-23T20:31+08:00 | `date -Iseconds` |
| 门禁总览 | 27 项：PASS 23 / FAIL 0 / SKIP 3 / TOLERATED 1，`STATUS: OK` | `python3 gates/cli.py status` |
| 测试数 | **1,193**（`cargo test --workspace` 汇总；ratchet 基线 1,189） | 同上，`rust-tests` 项 **[2026-11-26 更新]** |
| 例程分母 | 1,135，未移植 **603** | 同上，`routine-inventory` 项 |
| 字节比对面 | 2,272 B | 同上，`nes-map` 项 |
| 裸字节面 | 1,094 / 1,160 处 / 35 文件（可下调 66） | 同上，`raw-surface` 项 |
| 裸地址（生产面） | 518 / 524 处 / 23 文件；测试面另 175 处 | 同上，`no-raw-addr` 项 |
| L4 基线账 | 21 案 / 172 采集点（match 171 · diff 1） | 读 `shots_rust/matrix.json`（`head=b1e33a6d`、`at=2026-11-25T12:45:01Z`），**未跑 `gates/cli.py l4`** |
| L3 game-core 腿 | **未启用**（只有 oracle 腿） | `status` 的 `trace-diff` 项原文 |
| 依赖方向门 | G11 判据已按 m21400 移除，`arch-contract` 只留 `analysis/` 不入库 | `status` 的 `arch-contract` 项原文 |

> SKIP 3 项（`visual-diff` / `l4` / `proof-checks`）是慢层，本稿**没有**跑它们；
> 涉及像素与全量对拍的结论一律标注为「读账本」而非「已验」。

---

## 1. 现状架构清单（哪一层是 FC 内存镜像，哪一层已是 Rust 语义）

### 1.1 五类内存面在 `crates/` 的落点

| 内存面 | 落点 | 形态 | 证据 |
|---|---|---|---|
| RAM 2 KB `$0000-$07FF` | `state.rs::Manager.storage` → `memmap/storage.rs::NesStorage.ram:[u8;0x800]` | **字节镜像**（私有字段，只能经方法访问） | `crates/game-core/src/memmap/storage.rs:18`、`crates/game-core/src/state.rs:99-102` |
| WRAM 8 KB `$6000-$7FFF` | 同上，`NesStorage.wram:[u8;0x2000]` | **字节镜像** | `crates/game-core/src/memmap/storage.rs:18-24` |
| OAM `$0200-$02FF` | `render/oam_shadow.rs::OamShadow`（`page:[u8;0x100]` 为真相，`OamEntry` 是 4 B 视图） | **typed 真相 + 字节投影** | `crates/game-core/src/render/oam_shadow.rs:21-31,59-65`；路由见 `crates/game-core/src/memmap/raw.rs:60-63,99-102` |
| PPU 地址空间（CHR 8 KB / 名表 2 KB / 调色板 32 B） | `state.rs::RenderState{chr:Vec<u8>, nametable:Vec<u8>, palette:[u8;32]}` | **独立结构体，不在 RAM 里**（PPU 址空间不是 CPU RAM） | `crates/game-core/src/state.rs:196-221`；写入路径 `crates/game-core/src/render/ppu.rs:18,41,52` |
| PPU 寄存器 `$2000/$2001/$2005/$2006/$2007` | `Manager::write_io` 分派 → `RenderState{ppuctrl,ppumask,scroll_x,scroll_y,ppuaddr,latch_hi}` | **寄存器态独立建模，按语义字段存**（不落 RAM） | `crates/game-core/src/frame/memory.rs:14-32`；`crates/game-core/src/state.rs:196-221` |
| APU 寄存器 `$4000-$4017` | `Manager.apu_log: Vec<(u16,u8)>` 事件流 → 壳层 `AudioSink` | **事件流**（不是寄存器镜像） | `crates/game-core/src/frame/memory.rs:37-40`、`crates/game-core/src/state.rs:161-162`、`crates/game-platform/src/runtime.rs:38-46` |
| 手柄 `$4016/$4017` | `Manager.joy1/joy2: u8` + `read_joy_pads` 直接产出派生字节；壳层出 `PadState.bits` | **语义值直接装配，串行 strobe/shift 被等效短路** | `crates/game-core/src/frame/nmi.rs:304-322`（注释「输入字节直配（strobe/shift 等价）」）、`crates/game-platform/src/lib.rs:21-45` |
| MMC1 bank 窗口 | `frame/boot.rs::io` 常量（`$8000/$A000/$C000/$E000`）+ `mmc1_serial_write` | **编译期分派，运行期无 bank 状态**：串行写是 no-op | `crates/game-core/src/frame/boot.rs:19-40,464-467`（`let _ = v; // STA reg（MMC1 移位面——RAM watch 不可见）`）；`Manager` 无 bank 字段（`crates/game-core/src/state.rs:99-181`） |
| OAM DMA `$4014` | `frame/nmi.rs` 写事件 + `oam_shadow` 整页 | 事件（整页 DMA 语义） | `crates/game-core/src/frame/nmi.rs:24`、`crates/game-core/src/frame/boot.rs:39-40` |

**结论（A 级）**：CPU 侧的 `$0000-$07FF` 与 `$6000-$7FFF` 是**唯一的字节镜像面**；
PPU 址空间、PPU 寄存器、APU、手柄都已经是**独立结构体或事件流**，本来就「脱离了 FC 内存」。
所以"FC 内存投影"这个问题，精确地只剩**两块字节数组与它们上面的地址语义**。

### 1.2 已经是纯 Rust 语义层的清单

| 结构 | 语义化程度 | 证据 |
|---|---|---|
| `SamusState` | 23 个字段私有，`$0300-$0316` 全覆盖，`get_byte/set_byte` 只是字节面适配器；`TYPED_ADDRS` 是投影唯一源 | `crates/game-core/src/samus/state.rs:182-221,334-363` |
| `SlotState`（敌槽） | `EnsExtra` 前 7 B × 12 槽 typed；`EnemyStatus` 用枚举保留未定义值（quirk 保真） | `crates/game-core/src/enemy/state.rs:79-95,939-975` |
| `OamShadow` | 整页 typed 真相，`OamEntry` 不复制真相 | `crates/game-core/src/render/oam_shadow.rs:59-70` |
| `RenderState` | CHR/名表/调色板/四寄存器字段化；名表镜像排布显式建模（`mirror`） | `crates/game-core/src/state.rs:196-221` |
| `HealthChange` / `SoundInitFlags` / `ObjectGlobals` / `ScrollBlocks` | 各 2–5 B typed，带 `ADDRS` 表 + `get_byte/set_byte` | `crates/game-core/src/state.rs:105-115` 及各域 `ADDRS` |
| `PadState` + `apu_log` | 输入是位形值、音频是写事件流 | `crates/game-platform/src/lib.rs:21-45`、`crates/game-core/src/state.rs:161-162` |
| 引擎时钟/确定性 | `frame/clock.rs`、`frame/tick.rs`、`rng.rs` 不依赖平台 | `crates/game-core/src/lib.rs:1-40`、`crates/game-platform/src/lib.rs:1-18` |

### 1.3 尚未语义化的字节面（这是全部问题的量）

| 四态 | 字节 | 构成 |
|---|---|---|
| Owner | **1,401** | 15 个 typed 面之和，逐面对照见下方「Owner 逐面清单」。旧口径 374 B（OAM 256 + Samus 23 + 敌额外面 84 + zp 9 + 声 init 2）已被 B1 批次的声明层追加取代 **[2026-11-26 更新，依据 `cargo run -p game-core --bin nesmap-dump` 按 entity 聚合]** |
| View（与 Owner 重叠） | 23 | 对象池 0 号槽对 Samus typed 面的只读视图（`crates/game-core/src/world/object.rs:244-263`：`impl NesMapped`，`SPANS` 的 `View(&["SamusState"])` 段在 `:248-251`）。**这 23 B 同时被 Samus 的 Owner 段覆盖**，gate 报的是「View 23 B（only 0）」——only-view=0 B，**不参与求和** |
| Unmodeled | **385**（**24 段**，逐段见 `--list`） **[2026-11-26 更新]** | 按大小：`$0317-$03FF` 233 B（对象池 `$0300-$03FF` 16 槽 × 16 B 扣掉 23 B View 后的尾部；`world/object.rs:241` 自述「其余 233 B 未 typed ⇒ `Unmodeled`」，声明在 `:252-258`）、`$0400-$04BF` 192 B（敌主面整脸，12 槽 × 16 B）、`$04C0-$04DF` 32 B、敌额外面尾 12 段 × 9 B（槽 k 起点 = `$6AFB+16k`，最后一段终到 `$6BB3`；每槽前 7 B 已 typed，这 9 B 是尾部）、`$6BB4-$6BD3` 32 B、其余 **10 段小空洞共 12 B** |
| Undeclared | **486**（**11 段**＝10 段账本 238 B + 1 段未登记 248 B） | 账本已登记 **10 段共 238 B** + **完全未登记 1 段 248 B**（`$0108-$01FF`，6502 栈区）。`$04E0-$0682` 那 419 B 已被 B1 批次的 `ObjectSlots`/`SoundRam` 声明吸收 **[2026-11-26 更新，依据 `python3 tools/nes_map_check.py --list`]** |

复跑：`python3 tools/nes_map_check.py --list`（输出逐段列出，含"未登记（无）"两行）。

**求和校验（复核人一定会做，这里先给出）**：Owner 1,401 + Unmodeled 385 + Undeclared 486 = **2,272 B** = 比对面 （旧口径 374 + 611 + 1,287 同样成立，只是三项数值已随声明层推进而变；**[2026-11-26 更新]**）
（`$0000-$07FF` 2,048 + `$6AF4-$6BD3` 224）。View 的 23 B 与 Owner 段重叠、only-view 为 0，**不在求和里**；
若照表格行数机械相加会得到 2,295，差的就是这 23 B。

**Owner 逐面清单（1,401 B，2026-11-26）**：复跑 `CARGO_TARGET_DIR=/tmp/t cargo run -q -p game-core --bin nesmap-dump | python3 -c "import json,sys,collections;d=json.load(sys.stdin);a=collections.defaultdict(int);[a.__setitem__(s['entity'],a[s['entity']]+s['len']) for s in d['spans'] if s['key']=='owner'];print(sorted(a.items(),key=lambda kv:-kv[1]))"`

| 面 | 字节 | span | 声明位置 |
|---|---|---|---|
| `OamShadow` | 256 | 1 | `crates/game-core/src/render/oam_shadow.rs` |
| `ObjectSlots`（敌槽/爆散/mother/mellow） | 256 | 18 | `crates/game-core/src/memmap/object_slot_face.rs`（#29） |
| `TileBlasts`（`$0500-$05CF`，13×16 B） | 208 | 13 | `crates/game-core/src/memmap/tile_blast_face.rs`（B1-a） |
| `SpareScratch`（`$068E-$06FF`+`$0787-$079F`+`$07F0-$07FF`） | 155 | 3 | `crates/game-core/src/memmap/spare_face.rs`（#52） |
| `SoundRam`（`$0600-$0682`+`$0685-$068D`） | 140 | 2 | `crates/game-core/src/memmap/sound_ram_face.rs`（#29） |
| `RoomSpriteTables`（`$0700-$077F`） | 128 | 18 | `crates/game-core/src/memmap/room_sprite_table_face.rs`（B1-b 第一批） |
| `VramBuffers`（`$0780-$0786`+`$07A0-$07EF`） | 87 | 4 | `crates/game-core/src/memmap/vram_buffer_face.rs` |
| `Enemy`（12 槽 × 7 B 额外面） | 84 | 12 | `crates/game-core/src/enemy/state.rs` |
| `ZeroPage`（`$0000-$0032`） | 51 | 1 | `crates/game-core/src/memmap/zero_page_face.rs`（#81） |
| `SamusState`（`$0300-$0316`） | 23 | 23 | `crates/game-core/src/samus/state.rs` |
| `ObjectGlobals` | 5 | 5 | `crates/game-core/src/world/object_globals.rs` |
| `HealthChange` / `SamusVitals` / `SoundInitFlags` / `ScrollBlocks` | 2+2+2+2 | 4 | `samus/health_change.rs`、`memmap/samus_vitals_face.rs`（#77）、`sound/`、`world/` |
| **合计** | **1,401** | 103 | — |

**注意盲区**：比对面 = `$0000-$07FF`（2,048 B）+ `$6AF4-$6BD3`（224 B），
定义在 `crates/game-core/src/memmap/nesmap.rs:245-251`，并有单测锚定（同文件 `:375-390`）。
WRAM 共 8,192 B，**其中 7,968 B 在比对面之外**（`$6000-$6AF3` 与 `$6BD4-$7FFF`）——
含房间图案数据的装载区。这些字节的正确性目前**只有 L4 像素与单测兜底**，没有字节级对拍腿。
`verify_shim` 已把"面外触碰"单独计数（`crates/game-core/src/memmap/verify_shim.rs:404-422`），
说明这个盲区是已知的，但**尚未进判据**。

### 1.4 数据流（进 → 改 → 出 → 回灌）

```
场景/录像(joy 计划) ──► frame/tick + engine（60Hz 定步）
                            │  ← 每 tick 进入（60Hz 定步）
                            ├─► 域模块改状态（samus/weapon/enemy/world/pickup/ui/sound/title）
                            │        │ 已 typed 的：直接改 typed 字段（真相）
                            │        │ 未 typed 的：mem.write_ram(addr, v) → storage.ram
                            │        └─► 寄存器面：mem.write_io(addr, v) → RenderState / apu_log
                            │  （同一帧内可多次）
                            ├─► 帧末：snapshot_nes()  ──单向投影──► NesImage{ram,wram}
                            │                                           │
                            │        ┌──────────────────────────────────┼────────────┐
                            │        ▼                                  ▼            ▼
                            │   ram.hex / ppu.hex              --state-out       capture_mss
                            │   （observe/dump.rs:39）          （bin 侧）        （frame/memory.rs:433）
                            │  ← 回灌前字节面已就位
                            └─► 反向：restore_mss(ram,wram,regs)（frame/memory.rs:341）
                                     先拷字节面 → 再按 typed 清单回灌 → 清零 typed 格 → debug_assert
```

### 1.5 五条失败路径（每条都有证据，不是假想）

| # | 失败路径 | 机制 | 现状防线 | 缺口 |
|---|---|---|---|---|
| F1 | **双真相** | 绕过 `read_ram/write_ram` 直写 `ram_mut()` 或直读 `ram()` 的 typed 格，typed 与字节各说各话 | `write_path_does_not_mirror_typed_bytes_into_storage`（`memmap/storage.rs:87`）、`mirrored_alias_does_not_bypass_typed`（`:101`） | 只对已 typed 的 1,401 B 有效 **[2026-11-26 更新]**；`ram()` 仍是 `pub`，`frame/memory.rs:29,416` 就直读 `ram()[0x00FF]` |
| F2 | **导出漏投影** | 新导出直读字节面而非 `snapshot_nes()`，typed 字段 dump 成 0（假绿） | `observe/dump.rs` 单一来源声明（`crates/game-core/src/memmap/observe/dump.rs:1-12,39`）；`docs/findings/typed-domain-migration.md` §3.5 列了 5 个必须走投影的出口 | 靠纪律与文档，无机械检查（无门禁断言"新导出不得直读 ram()"） |
| F3 | **回灌漏项** | `restore_mss` 少回灌一族，MSS 往返丢字段 | `frame/memory.rs:341` 末尾清零 + `debug_assert`；capture 走 `snapshot_nes`（`:433`） | 单份手写清单，新增 typed 域时靠人记；无"每个 typed 域必须被 restore"的机械断言 |
| F4 | **无主字节** | 486 B 无 owner；其中 248 B（`$0108-$01FF` 栈区）连指针都没有 **[2026-11-26 更新]** | `nes-map` 锁死 Undeclared 不许增长 | 无主 ≠ 没人用：2026-11-17 三场景实测（`docs/knowledge/decision-static-vs-dynamic-coverage.md`）面内触达 964 B，其中 Undeclared 343 B 被真实读写。**该读数本次未重跑** |
| F5 | **比对面盲区** | WRAM 7,968 B 不在字节对拍面（§1.3） | `verify_shim` 单独计面外触碰 | 房间装载/图案数据的字节级错误只能靠像素 diff 或单测发现 |

---

## 2. 目标架构方案：分界、所有权、依赖方向

### 2.1 三条线（本稿主张的分界口径）

```
真相线（唯一可写）        投影线（只读、可抛）           验收线（外部、有裁判权）
─────────────────        ────────────────────         ──────────────────────
Rust 语义类型              NesImage / ram.hex            Mesen2 执行 + ROM 字节
（typed 字段）             ppu.hex / --state-out          （AGENTS D3 最终真相）
                                 MSS bundle
```

- **真相线**：业务代码只能写 typed 字段（或经域句柄），不得以地址为 API。
- **投影线**：`snapshot_nes()` 是唯一出口，`restore_mss` 是唯一入口；投影**不是第二真相**
  （同 `docs/findings/typed-domain-migration.md` §4 第三条坑）。
- **验收线**：字节面**必须保留**。它是对拍接口，不是遗留包袱——删掉就没有差分裁判了。
  2026-10-28 的裁定原文是「双层=两次独立移植，互证不成立」（`docs/design/semantic-layer.md:8-11`），
  结论为「投影层整体退役，真值链必须外部」（同文件 `:4-12`）。同一逻辑推出：
  **单一实现 + 单一投影 + 外部裁判**。

**因此"脱离 FC 内存约束"的准确含义是**：业务代码不再认识地址（`write_ram(0x0071,1)` 这类消失），
而不是字节镜像消失。字节镜像退化为导出格式。

### 2.2 模块图与所有权

```
┌──────────────────────────── 平台层（独立 crate，零 FC 依赖） ────────────────────────────┐
│ game-platform：FrameClock / VideoSink / InputSource(PadState) / AudioSink($4000-$4017 流) │
│ game-shell-*（各平台）：实现上述 trait，持平台依赖与产物形态                                │
└───────────────▲────────────────────────────────────────────▲───────────────────────────┘
     256×240 RGB │                                             │ APU 写流
                 │                                             │
┌────────────────┴─────────────────────────────────────────────┴───────────────────────────┐
│ frame/ 编排层：engine / tick / clock / nmi / memory(Manager 方法) / fixture               │
│   Manager 是唯一内存模型持有者（storage + 7 个 typed 域 + regs + 渲染态 + 帧账）           │
└───────┬───────────────────┬───────────────────────┬──────────────────────────────────────┘
        │ 语义域句柄          │ 语义域句柄             │ 寄存器面唯一入口 write_io
        ▼                   ▼                       ▼
  samus/ weapon/       enemy/ world/          render/（RenderState：CHR/名表/调色板/
  pickup/ ui/          object/ scroll/         ppuctrl/ppumask/scroll）——不入 RAM
  sound/ title/        camera/ door/
        │                   │
        └─────────┬─────────┘
                  ▼
        memmap/：storage（私有字节容器）/ raw（read_ram·write_ram 分派）/ nesmap（声明+核算）
                  │ 单向投影 ↑ 反向 restore
                  ▼
        observe/：dump（ram.hex/ppu.hex）/ frame_face（32 B 帧面）
```

**所有权（按代码实测）**：

| 层 | 谁持有状态 | 谁能写 |
|---|---|---|
| `storage` | `Manager`（字段私有，`crates/game-core/src/state.rs:102`） | 只能经 `ram_mut()`（`pub(crate)`，`:32`）或 `write_ram`/`write_wram` |
| typed 域 | `Manager` 的 7 个 `pub(crate)` 字段（`crates/game-core/src/state.rs:104-120`） | 域方法；字节面经 `set_byte` 转投 |
| 寄存器/PPU | `Manager.render`（`crates/game-core/src/state.rs:180`） | 仅 `write_io`（`frame/memory.rs:14`） |
| 平台面 | 壳层 | game-core 只出 trait 需要的数据（RGB/APU流/joy 位） |

**依赖方向（现状 vs 目标）**：目标方向见 `docs/spec/entity-ownership-and-nesmap.md:95-97`
（`platform → shell → systems → {world,entities} → memmap → state`；`memmap` 不得依赖 `entities`）。
**现状该方向没有任何门守着**：`arch-contract` 的 G11 判据已按 m21400 移除
（`python3 gates/cli.py status` 的 `arch-contract` 行原文），所以今天"分层"是**约定**，不是判据。
这是 P6 目录重组前必须先补的东西（spec R5 同结论，见该文件 `:25`）。

### 2.3 与 `docs/design/data-driven-*.md` 的关系

数据驱动系列（5 篇）讲的是**资产轴**：ROM 里的代码 vs 数据如何分家
（`data-driven-architecture.md:7-11`：代码→game-core、数据→game-data）。
本稿讲的是**状态轴**：运行期 NES 字节 vs Rust 语义类型如何分家。
两轴正交，共用同一条判据：**真相唯一、投影可再生、判据机械可导出**。
不冲突的部分：`pack`（`.pack` 197 段逐段 == ROM 重算，`asset-pack` 门禁）已经是
"数据侧投影"的成功先例，状态侧可以照抄这个形状（声明 + 逐段校验 + 只减不增）。

### 2.4 冲突点（必须显式解决的 5 条）

| # | 冲突 | 证据 | 影响与建议 |
|---|---|---|---|
| C1 | **地址定义有两处**：`game-data/src/constants/ram_layout.rs` 声明 `SAMUS_Y/X/HEALTH` 三个地址常量，与 `game-core/src/ram_map.rs` 重复 | `crates/game-data/src/constants/ram_layout.rs:6,8,10`（`0x0300/0x0302/0x0304`）；`grep -rn "ram_layout" --include=*.rs crates/` 只有 2 条命中，**都不是消费**：`crates/game-data/src/constants/mod.rs:5`（`pub mod ram_layout;`）与该文件自身的文档注释。生产用的是 `crates/game-core/src/ram_map.rs` 的 139 条 `pub const`（`grep -c "pub const" crates/game-core/src/ram_map.rs` = 139） | 与 `crates/game-core/src/lib.rs:3` 注释「映射层（memmap/ram_map，地址只此一层）」冲突。建议：删或标注 `ram_layout.rs` 为 deprecated，地址唯一源收敛到 `ram_map.rs`（typed 后进一步收敛到各域 `ADDRS` 表） |
| C2 | **文档列了不存在的模块** | `data-driven-architecture.md:27` 列 `constants/zero_page.rs`、`:38` 列 `kernel/`；实际 `crates/game-data/src/constants/` 只有 `mod.rs`+`ram_layout.rs`，`crates/game-core/src/` 无 `kernel/`（`ls` 实测） | 新读者会被引到空地。建议按本稿 §1 重写该两行（`frame/` 已在同文件 `:39` 自我更正过） |
| C3 | **"bank 窗口"被当成运行期状态** | `data-driven-architecture.md:38` 把 bank 窗口列进 `kernel/`；实际 MMC1 串行写是 no-op（`frame/boot.rs:464-467`），`Manager` 无 bank 字段 | 不是错误，是**已完成的简化**（编译期分派）。文档应写明"bank 窗口在移植中已被编译期化"，否则 P6 会有人去"补"一个不需要的状态 |
| C4 | **房间数据只有静态验证** | `data-driven-architecture.md:28` 说 RoomRAM `$6000-$67FF` 归 game-data；但运行期装载进 WRAM 的字节在比对面之外（§1.3，7,968 B 盲区） | 数据侧的"可独立测试"成立（`asset-pack` 197 段）；运行期装载正确性没有字节腿。建议把"WRAM 装载区是否进比对面"立为独立决策（见 §4.3 触发条件 T4） |
| C5 | **终局目录有两个版本** | `docs/spec/entity-ownership-and-nesmap.md:89-96` 是 `entities/ world/ systems/ render/`；`data-driven-architecture.md:37-52` 是 `kernel(input/render/world/samus/weapon/…)` | 两者都在"未来态"，今天不冲突；但 P6 只能选一个。建议以 spec §7 为准（建议理由是它带依赖方向 `:95-97`，而 data-driven 版无依赖约定） |

---

## 3. 迁移评估（按模块，含工作量分级）

### 3.1 门禁现状对照（判断的前提）

| 门禁 | 读数 | 对"大改"的含义 |
|---|---|---|
| `rust-tests` | 1,193（ratchet 基线 1,189） **[2026-11-26 更新]** | 每个迁移批次必须不降；现有测试是唯一快速回归网 |
| `nes-map` | 面 2,272 / Owner 1,401 / Unmodeled 385 / Undeclared 486（View 23 only 0 不计入求和），四态恒等式成立 **[2026-11-26 更新]** | 静态腿已就位，**可用**（补声明能被机械检出） |
| `raw-surface` | 1,094/1,160 / 35 文件 | 新文件接裸面即红，**可用** |
| `no-raw-addr` | 518/524 / 23 文件；具名 151/151 | 只监控 5 个族，覆盖窄 |
| `routine-inventory` | 1,135 / 未移植 603 | **53% 的行为还没搬**：目标形状仍在生长 |
| `trace-diff`（L3） | game-core 腿**未启用** | **没有逐帧跨机裁判**——这是"不能大改"的最硬理由 |
| `l4` | 21 案 / 172 格 / 171 match / 1 diff（读 `shots_rust/matrix.json`，未跑） | 采集点少、慢层不进 hook；大改的回归信号弱 |
| `visual-diff` | SKIP | 渲染面变化无法在本轮被机械看见 |
| `arch-contract` | G11 已裁 | 重构期没有依赖方向门 |
| `completeness-ratchet` | 57 格（基线 57） | 行为面分母是声明的，不是机械的 |

### 3.2 分域评估

工作量分级（**按现状体量机械判定，三档互斥**）：**S** = 文件数 ≤ 1 **且** LoC ≤ 200；**M** = 文件数 ≤ 5 **且** LoC ≤ 1,000；**L** = 其余（文件数 > 5 **或** LoC > 1,000）。
判定只看「现状」列的两个数，任何人都能复算；**S/M/L 表示"整域迁移是多大一摊"，不表示本稿主张现在做**（本稿现在主张的 A/B/C 三件见 §4.1，它们本身不是域级迁移）。
数据来源（全部**递归**，含子目录）：LoC=`find crates/game-core/src/<dir> -name '*.rs' -print0 | xargs -0 cat | wc -l`；文件数=`find crates/game-core/src/<dir> -name '*.rs' | wc -l`；测试=`grep -rn "#\[test\]" crates/game-core/src/<dir> | wc -l`；
裸面=`data/status/raw_surface.json` 账本口径（非实时重算）。
测试数分解校验（递归口径）：`grep -rn "#\[test\]" crates --include=*.rs | wc -l` = **1,193**，与 `cargo test --workspace` 的 passed 汇总同数。
分解：`game-core/src/`（非 bin）**1,082** + `game-core/src/bin/` **7** + `game-core/tests/` **20** = game-core **1,109**；加 `game-data` **65** + `game-platform` **14** + `game-shell`（`game-shell-pc` lib 3 + `apu_dump` 2）**5** = **1,193**。 **[2026-11-26 更新] 上一版把 game-shell 记作「计 0」，实测 `grep -rn "#\[test\]" crates/game-shell --include=*.rs | wc -l` = 5，故 1,082+7+20+65+14+5=1,193。**
旧分解「970+6+7 / +20+65+14+5 = 1,087」是 2026-11-25 读数，已随 #77/#81 等新增单测变化 **[2026-11-26 更新，依据上面两条命令]**。

**先给结论**：按现状体量，**除 `pickup/`（M）、`ui/`（M）、`input/`（S）、`ending/`（S）外，每个域都是 L**。
这正是结论选「分阶段重构」而不是「现在整体重构」的量化理由——任何一域的整体迁移都是跨多文件、上千行的改动，
而唯一的跨机裁判（L3 game-core 腿）还没启（§3.1）。

| 域 | 现状（LoC / 文件 / 模块内测试 / 裸面账本） | 迁 typed 收益 | 风险 | 工作量 | 门禁影响 |
|---|---|---|---|---|---|
| `samus/` | 6,628 / 7 / 163 / 82 | 已 100% typed（23 B），只剩域内便利面 | 低（已完成，作为模板） | L（7 文件） | 无 |
| `render/`（OAM） | 2,453 / 7 / 59 / 36 | OAM 已 typed；剩余是渲染态喂入 | 低 | L（7 文件、2,453 行） | `visual-diff` 若启，须看像素 |
| `sound/` | 3,832 / 4 / 62 / 31 | `$0683-$0684` 已 typed；`$0685-$07FF` 379 B 虽有账本指针却无 owner（是 Undeclared 单段最大） | 中：声音引擎状态机多、与 NMI 时序耦合 | L（3,832 行） | `nes-map` Undeclared 下降；测试数上升 |
| `world/` | 6,212 / 15 / 107 / 370（**裸面最多**） | 对象池视图化、房间装载、电梯、门 | 中高：`room_load`/`elevator` 是跨帧任务机，行为敏感 | L（15 文件） | 触碰 `raw-surface` 最大项，须逐批 `--write` 下调 |
| `enemy/` | 13,042 / 34 / 279 / 261（**代码量最大**） | 敌主面 192 B 整脸 typed、额外面尾 108 B | 高：34 文件、12 槽 × 16 B 字段语义、同址多义（`$0406` 三名） | L（34 文件） | 测试面 279 条，回归信号好；但迁移易引入时序差 |
| `weapon/` | 1,631 / 4 / 27 / 112（**密度最高**：112 处/文件） | 弹槽自身是对象池槽，随对象池一起动 | 中：`projectile.rs` 单文件 112 处裸面 | L（1,631 行） | `raw-surface` 单文件最大项之一 |
| `frame/` | 6,766 / 11 / 167 / 127 | `$00-$0F` 临时量、`$0100-$01FF` 栈页（**栈不可 typed**） | 中：`nmi`/`tick` 是全局时序 | L（11 文件） | 触碰 `frame/memory.rs` 边界文件（豁免），改动须单独审 |
| `title/` | 4,647 / 8 / 55 / 52 | `title/state.rs` 已是"事件层"（`write_event_*`），可收敛为相位 typed | 中：密码/相位机 | L（8 文件） | 无 |
| `pickup/` | 580 / 3 / 10 / 53 | 随对象池一起动 | 低 | M | 无 |
| `ui/` | 499 / 3 / 12 / 36 | HUD 值读出，已接近语义 | 低 | M | 无 |
| `input/` `ending/` | 1+1 行 / 各 1 文件 / 0 测试 / 0 裸面 | 输入是位形值、ending 是单帧 | 低 | S | 无 |
| `memmap/` | 1,933 / 9 / 29 / 0（豁免） | 声明与核算层，本身就是要保留的那层 | 低 | L（9 文件） | 改动直接影响 `nes-map`/`raw-surface`，须单 commit |
| `game-data/` | 5,861 / 25（`crates/game-data/src/`，含 `constants/`、`enemy/` 等子目录） | 只涉及 C1（地址常量重复） | 低 | L（25 文件） | `data-boundary` 阈值 200 附近，注意 |

### 3.3 为什么"立即整体重构"不成立（逐条对门禁）

1. **没有跨机裁判**：L3 game-core 腿未启用（§3.1），49k 行改动无法证明行为不变。
   现有唯一强信号是 1,193 条单测 + 21 案像素账 **[2026-11-26 更新]**——不足以兜住一次目录级重排。
2. **目标形状仍在生长**：53% 例程未移植；现在定死的所有权/目录，会在剩下 603 个例程搬迁时被反复推翻。
3. **搬迁被棘轮门挡**：新文件接裸面即 FAIL、总量上涨即 FAIL（`raw-surface` 行），
   而 `raw-surface` 现在有 1,094 处——**先归零才能搬**（spec R5 同结论）。
4. **没有依赖方向门**：G11 已裁，搬完无人守（§2.2）。
5. **纪律优先级**：AGENTS §1「逻辑一致性 > 代码优雅」。为整洁牺牲可验证性 = 踩红线。

### 3.4 为什么"暂缓"也不成立

`nes-map` 的 Undeclared 有 **248 B 连指针都没有**（§1.3） **[2026-11-26 更新]**，且业务代码每天在加新的裸地址写法
（`raw-surface` 基线 1,160、现 1,094，只降了 66）。**每多移植一个例程，就把一处地址写法固化进新代码**：
今天补声明是"加声明"，等 603 个例程搬完再补就是"全仓改"。所以纯暂缓会让成本单调上升。

---

## 4. 结论：**分阶段重构**（明确三选一）

**判定：分阶段重构。不立即整体重构，也不暂缓。**

理由一句话：**方向已对、地基已浇筑 61.7%（Owner 1,401 B / 比对面 2,272 B） **[2026-11-26 更新]**，缺的是机械判据与覆盖率，不是再来一次设计**；
而"再加一次大设计"的代价（失去裁判、目标漂移、门槛撞车）远高于收益。

### 4.1 现在做（本稿主张立卡的三件事，均不动行为）

| # | 动作 | 判据（机械可查） | 风险 | 建议 owner 面 |
|---|---|---|---|---|
| A | **补 Undeclared 声明**：486 B → 0（先补 248 B 无指针段 $ **[2026-11-26 更新]** 余 238 B 有账本指针） | `python3 tools/nes_map_check.py --list` 不再出现"未登记"；`nes-map` PASS；`cargo test` 1,193 不降 | 零行为变化（只加声明/指针） | `memmap/nesmap.rs` + 各域 `SPANS`（写路径分散，按域切分） |
| B | **动态腿扩面**：现有 3 个手搭场景（title/walk/door，`memmap/verify_shim.rs:388`）→ 21 个 L4 案采集点 | 每案 `assert_owner_unique` 零冲突；`tools/raw_execution_report.py` 覆盖 21 案 | 测试面改动，不碰生产逻辑 | `memmap/verify_shim.rs` + `data/status/` 账本 |
| C | **新代码 typed-first 入规**：`no-raw-addr` 监控族从 5 族扩到"本批移植涉及的族"；`raw-surface` 加"每批净下降"约束 | 门禁红即停；`raw-surface` 每批总量 ≤ 上批 | 会拖慢移植节奏——需负责人拍板 | `gates/`（单独 commit） |

优先级：**A 与 B 并行**（写路径不重叠：A 改 `memmap`+域 `SPANS`，B 改 `verify_shim`+账本），
C 作为政策项拍板后进门禁。

### 4.2 推迟（按 spec 的 P4–P7，逐条给触发条件）

| 阶段 | 内容 | 启动触发条件（全部满足才动） |
|---|---|---|
| P4 | 剩余族 typed 化（`$00-$0F` 临时量、`$0317-$03FF`、敌主面 192 B、对象池 233 B、`$04E0-$0682`、`$0685-$07FF`） | ①A 完成（Undeclared=0）②该族动态腿零冲突 ③该族有对应的 L4 案（否则迁移无法被裁） |
| P5 | 实体自持：Explosion → Enemy → Object → Samus（最后） | ①P4 该域完成 ②`nes-map` 该域 Owner=全脸 ③capture/restore 往返断言在册 |
| P6 | 目录重组（`entities/ world/ systems/ render/`） | ①`raw-surface` 生产面 = 0 ②**先立 G11′ 依赖方向门** ③C5 目录版本冲突已裁定 |
| P7 | `read_ram/write_ram` 降为 `memmap` 内部件 | spec §5 五条硬条件（`docs/spec/entity-ownership-and-nesmap.md:72-73`）全满足 |
| 删 `NesStorage` | 字节容器退场 | 同上五条；**注意：即便删了容器，字节投影面也必须保留**（验收线，§2.1） |

### 4.3 本稿新增的两个待决问题（不在 spec 里，需负责人裁定）

- **T4 比对面是否扩到 WRAM 装载区**：现状 7,968 B 盲区（§1.3）。扩 = 对拍覆盖提升但 diff 噪声上升；
  不扩 = 房间装载错误只能靠像素。建议先做一次"面外触碰"实测（`verify_shim` 已埋点），按实测频次决定。
- **T5 地址定义收敛（C1）**：删 `game-data/src/constants/ram_layout.rs` 还是把它当导出面？
  建议删（无消费者），并在 `ram_map.rs` 头部注明"地址唯一源"。

---

## 5. 复核指引（怎么只用本稿就复现每条判断）

| 判断 | 复现命令/指针 |
|---|---|
| 门禁读数 | `python3 gates/cli.py status` |
| 四态与逐段空洞 | `python3 tools/nes_map_check.py --list` |
| Owner 构成 | `python3 -c "import json;d=json.load(open('data/status/nes_map.json'));print(d['owner_bytes'],len(d['owner_spans']));print([(s['start'],s['len']) for s in d['owner_spans']])"`（`owner_bytes=[290,84]`=RAM 面 290 + WRAM 面 84；44 段；RAM 290 = OAM 256 + `$0300-$0316` 23 + zp 9 + `$0683-$0684` 2） |
| 比对面定义 | `crates/game-core/src/memmap/nesmap.rs:245-251` + 单测 `:375-390` |
| 字节镜像私有 | `crates/game-core/src/memmap/storage.rs:18-24`、`crates/game-core/src/state.rs:99-102` |
| 投影出口唯一 | `crates/game-core/src/memmap/observe/dump.rs:1-12,39` |
| 反向入口 | `crates/game-core/src/frame/memory.rs:341-380`（清 typed 格 + 断言） |
| 寄存器不入 RAM | `crates/game-core/src/frame/memory.rs:14-40` |
| bank 窗口无运行态 | `crates/game-core/src/frame/boot.rs:464-467` + `crates/game-core/src/state.rs:99-181`（无 bank 字段） |
| 输入已语义化 | `crates/game-core/src/frame/nmi.rs:304-322`、`crates/game-platform/src/lib.rs:21-45` |
| 对象池视图声明 | `crates/game-core/src/world/object.rs:244-263`（`impl NesMapped` 244、`SPANS` 247：`View(&["SamusState"])` 段 248-251 共 23 B、`Unmodeled` 段 252-258 共 233 B） |
| 地址常量重复（C1） | `crates/game-data/src/constants/ram_layout.rs:6-10` vs `crates/game-core/src/ram_map.rs`（139 条） |
| 文档漂移（C2/C3） | `docs/design/data-driven-architecture.md:27,38` vs `ls` 实测 |
| 依赖方向无门 | `status` 的 `arch-contract` 行 |

**本稿全部 `文件:行` 指针的机械校验**（一次性脚本，不进仓库；改完文档跑一遍）：

```python
import re,os
doc='docs/design/fc-memory-projection-architecture.md'
idx={}
for root,ds,fs in os.walk('.'):
    if '.git' in root or 'target' in root: continue
    for fn in fs: idx.setdefault(fn,[]).append(os.path.join(root,fn))
txt=open(doc).read(); ok=bad=0
for f,l in set((m.group(1),int(m.group(2))) for m in re.finditer(r'([A-Za-z0-9_./-]+\.(?:rs|md|py|lua|json|toml)):(\d+)',txt)):
    cand=[f] if os.path.exists(f) else idx.get(os.path.basename(f),[])
    cand=[c for c in cand if c.endswith(f)] or cand
    if len(cand)!=1: print('UNRESOLVED',f,len(cand)); bad+=1; continue
    L=open(cand[0],errors='replace').read().splitlines()
    if l>len(L) or not L[l-1].strip(): print('BAD-LINE',f,l); bad+=1; continue
    ok+=1
print(f'pointers ok={ok} bad={bad}')
```

当前输出：`pointers ok=56 bad=0`（N 个唯一指针全部落在存在的文件与**非空**行上；按 basename 在仓库内解析，
同名前缀唯一者直接命中，重名时用 `endswith` 收窄——所以本文的简写指针（如 `crates/game-core/src/state.rs:99-181`）能解析，
但**仍建议写全路径**）。

---

## 6. 未执行项（不得当结论）

1. **未跑全量 L4**：166/172 是账本读数，不是本轮实测；6 个 diff 格的具体案未逐格取证。
2. **未跑 `visual-diff`**（SKIP）：任何"渲染面不受影响"的说法本稿都不作。
3. **未重跑动态腿**：§1.5 F4 的"964 B 触达 / Undeclared 343 B"是 `docs/knowledge/decision-static-vs-dynamic-coverage.md` 的 2026-11-17 记录，仓内现态（Undeclared 已从 18 段变 13 段）需要重跑才能引用。
4. **未测新代码**：本稿只落文档，未改任何 `.rs`；因此**没有**新增测试或门禁读数变化。
5. **未评审 spec 本身**：`docs/spec/entity-ownership-and-nesmap.md` 仍是"提案待审"；本稿引用其结论时按"提案"对待，不当作已裁定。
6. **工作量分级是计数口径**，不是工时承诺：S/M/L 由文件数与行数机械划分，未做人力估算。

---

## 7. 返工自审记录（v2）

### 7.1 返工理由的可得性

- `team` 的 Thread 到 `headSeq=3` 为止只有 3 条讨论（2 条我的进度、1 条我的提交说明），**没有 requestChanges 的 note 条目**；
- `team inbox` 为空；工作树里也没有复核人留下的审阅文件（`git status --porcelain` 只有本稿与一份他人在途的 HANDOFF）；
- 已在本卡 Thread 请求补发理由（seq=4）。因此 **v2 不是"按已知返工理由修"，而是按 resultDefinition + 验收方式自查**。

### 7.2 自查修掉的 13 处（都是复核人一跑就会对不上的点）

| # | v1 的问题 | v2 的修法 | 自查命令 |
|---|---|---|---|
| 1 | 文首写「未执行的一律在 §8 声明」，但本文只有 §0–§6 | 改为 §6 | `grep -n "^## " docs/design/fc-memory-projection-architecture.md` |
| 2 | §1.5 标题写「四条」，表里有 F1–F5 五条 | 改为五条 | 同上 |
| 3 | §1.3 Unmodeled 行把 233 B 写了两次（一次是 `$0317-$03FF`，一次叫「对象池尾」） | 合并为逐段归因，引原自述 `crates/game-core/src/world/object.rs:241` 与声明 `:252-258` | `python3 tools/nes_map_check.py --list` 逐段对照 |
| 4 | View 行指针 `object.rs:244-259` 与实现位置不符 | 改 `:244-263`（`impl NesMapped` 起于 244、`SPANS` 起于 247、`View` 段 248-251、`Unmodeled` 段 252-258） | `grep -n "impl<'a> crate::memmap::nesmap::NesMapped\|const SPANS" crates/game-core/src/world/object.rs` |
| 5 | C1 说「全仓无任何消费点（grep 结果为空）」——实际 grep 有 2 条命中 | 写明两条命中分别是 `constants/mod.rs:5` 的模块声明与文件自身注释，**都不是消费**；并补 `crates/game-core/src/lib.rs:3` 注释出处 | `grep -rn "ram_layout" --include=*.rs crates/` |
| 6 | C5 引 spec 目录块为 `:88-93` | 改 `:89-96`（实体目录 `:89-93`、依赖方向 `:95-97`） | `sed -n '86,99p' docs/spec/entity-ownership-and-nesmap.md` |
| 7 | §2.1 把转述「第二份独立移植不能当 oracle」写成了引文 | 换成原文「双层=两次独立移植，互证不成立」+ 行号 `semantic-layer.md:8-11` | `sed -n '1,16p' docs/design/semantic-layer.md` |
| 8 | §3.2 测试数用的是**非递归** `grep -c`（漏子目录），world 记 106、memmap 记 25 | 改递归 `grep -rn`：world 107、memmap 29 | `for d in enemy world frame title samus render memmap weapon sound ui pickup; do grep -rn "#\[test\]" crates/game-core/src/$d | wc -l; done` |
| 9 | §3.2 写「模块内测试合计 971」，但递归实数 970，且差额未解释 | 改 970，并给全仓 1,087 的完整分解（970+6 根文件+7 bin+20 集成+65 game-data+14 game-platform+5 game-shell） | `grep -rn "#\[test\]" crates --include=*.rs | wc -l` + `cargo test --workspace` 的 `test result` 汇总 |
| 10 | sound 行说 `$0685-$07FF` 是「未登记段最大」——它其实是**账本已登记指针**的 Undeclared | 改为「有账本指针但无 owner，是 Undeclared 单段最大」 | `python3 tools/nes_map_check.py --list`（该段显示 `[账本] $0685-$07FF (379 B)`） |
| 11 | pickup/ui/input/ending 组写「10 文件」，实际 8 文件（3+3+1+1）；LoC 未给合计 | 改 8 文件、合计 1,081 | `ls crates/game-core/src/{pickup,ui,input,ending}/*.rs | wc -l` |
| 12 | §5 的 Owner 复现写成「`python3 -c` 读…」伪代码，复核人无法照抄 | 给可复制的完整一行命令，并写明 `owner_bytes=[290,84]` 两分量含义 | 直接执行该行 |
| 13 | §1.3 把 View 行标成「View(only) 23 B」并放进四态表，机械相加得 2,295 ≠ 比对面 2,272；gate 原文是「View 23 B（only 0）」——only-view=0，该 23 B 被 Samus 的 Owner 段覆盖 | 行名改「View（与 Owner 重叠）」，写明不参与求和，并补一行求和校验：374+611+1,287=2,272 | `python3 tools/nes_map_check.py`（读 `View(only 0)` 与 `四态恒等式成立`）+ `python3 -c` 相加 |

### 7.3 仍未解决（复核人仍可能驳回）

- 驳回原话已由复核人（Thread seq=8）给出（v1 三条：233 B 重复、四条/五条不一致、10 文件），分别对应 §7.2 的 ③②⑪；note 未落进 Thread 属流程问题，复核人已另行上报。
- §3.2 的 LoC 与裸面账本是**计数口径**；`raw_surface.json` 的 per-file 计数可能略旧于工作树（账本更新需 `--write`，本稿未动）。

### 7.4 复核退回后修正（v3，仅动文档）

复核人在 Thread seq=8 判 v2.1 退回，未过判据两处（resultDefinition 第 3 条「工作量分级」+ 验收方式「仅凭文档内指针可复现」），另给三条建议。本版全部采纳：

| # | 复核意见 | 本版改法 | 复现命令 |
|---|---|---|---|
| 1 | §3.2 声称「全部递归」，但 world/memmap 的 LoC 与文件数是非递归（只数顶层），与同行的递归测试数混了口径 | 全部行按递归重算：world 5,729/13 → **6,212/15**、memmap 1,696/6 → **1,933/9**；其余行一并复核（enemy 13,042/34、frame 6,766/11、title 4,647/8、samus 6,628/7、render 2,453/7、weapon 1,631/4、sound 3,832/4、pickup 580/3、ui 499/3、input 与 ending 各 1 行 1 文件、game-data 5,861/25） | `for d in enemy world frame title samus render memmap weapon sound ui pickup input ending; do echo -n "$d "; find crates/game-core/src/$d -name '*.rs' | wc -l; find crates/game-core/src/$d -name '*.rs' -print0 | xargs -0 cat | wc -l; done` |
| 2 | S/M/L 规则「或」的两侧会同时命中、无法机械判定；且表中八个标签与规则对不上 | 规则改为**互斥**：S = 文件 ≤1 **且** LoC ≤200；M = 文件 ≤5 **且** LoC ≤1,000；L = 其余。全部标签按新规则重标（samus/render/world/enemy/weapon/frame/title/sound/memmap/game-data 均 L；pickup、ui 为 M；input、ending 为 S），并在表前写明「按现状体量机械判定，S/M/L 表示整域迁移的规模，不表示本稿主张现在做」 | 同上表 + §3.2 规则句 |
| 3 | 建议：「地基已浇筑 17%」需给口径 | 改「16.5%（Owner 374 B / 比对面 2,272 B）」 | `python3 -c "print(374/2272)"` → `0.1646…` |
| 4 | 建议：`$6AFB-$6BB3` 写成「每槽 7 B typed 之后的 9 B」易误读为 185 B 全非 typed | 改「12 段 × 9 B，槽 k 起点 = `$6AFB+16k`，最后一段终到 `$6BB3`；每槽前 7 B 已 typed」 | `python3 tools/nes_map_check.py --list` |
| 5 | 建议：提交证据的「361 行」与盘上不符 | 本版按实际行数重报：**410 行**（`wc -l`）；§7.2 记 13 条自审、§7.4 记本轮 5 项 | `wc -l docs/design/fc-memory-projection-architecture.md` |

另把 §5 的「指针机械校验」脚本换成能解析本文简写指针的版本，当前输出见 §5（`pointers ok=…`）。
复核人复现的那次 `-p game-data --lib` 瞬时失败与本方一致，双方均判并发/构建竞态，不作为退回理由。

## 状态同步（2026-11-26）

写作时 HEAD `495426ae`（2026-11-23）的读数，与当前 HEAD `4e7ce4de`（2026-11-26）已不同。下面只列变化与复跑命令，**不改本文章节结论**；正文里被改到的数字都已就地标注。

| 项 | 写作时（2026-11-23） | 现在（2026-11-26） | 取数命令 |
|---|---|---|---|
| Owner | 374 B | **1,401 B** | `python3 tools/nes_map_check.py --list` |
| View（only 0，不计入求和） | 23 B | 23 B | 同上 |
| Unmodeled | 611 B（27 段） | **385 B（24 段）** | 同上 |
| Undeclared | 1,287 B（13 段） | **486 B（11 段）** | 同上 |
| 合计 | 2,272 B | 2,272 B（1,401+385+486） | 同上 |
| `impl NesMapped` 文件数 | 3 | **两个口径，分别给清单**：①`grep -rl "impl NesMapped" crates --include=*.rs \| wc -l` = **13** = `memmap/{object_slot,room_sprite_table,samus_vitals,sound_ram,spare,tile_blast,vram_buffer,zero_page}_face.rs`（8 个面）+ `render/oam_shadow.rs` + `samus/health_change.rs` + `sound/state.rs` + `world/object_globals.rs` + `world/scroll_blocks.rs`；②`grep -rln "NesMapped for" crates --include=*.rs \| wc -l` = **19**（上面 13 个里除 5 个面用全限定写法外，另含 `enemy/explosion.rs`、`enemy/state.rs`、`samus/state.rs`、`world/camera/mod.rs`、`world/object.rs`、`world/room_ram.rs`） **[2026-11-26 更新：上一版把 `world_map.rs`/`nesmap.rs` 也列进来，它俩其实没有实现；此处按两条命令的逐字输出改正]** | 两条 `grep` 命令 |
| 单测 | 1,087 | **1,193**（`--workspace`；ratchet 基线 1,189） | `cargo test --workspace` |

**这批声明层对应的卡**（都已验收，可 `git log --oneline` 复核）：`ObjectSlots`+`SoundRam`（#29，`2bf74902`）、`SpareScratch`（#52）、`TileBlasts`/`RoomSpriteTables`/`VramBuffers`（#22 一带）、`SamusVitals`（#77，`bdba1b0f`，**唯一做过权威翻转的 2 B**）、`ZeroPage`（#81，`d2070cb7`）。

**还剩什么**（与 §4 的候选一致，未变）：Undeclared 486 B 中 238 B 有账本指针（10 段）、248 B 是 6502 栈区（`$0108-$01FF`，只能 Ephemeral，不参与降幅）；Unmodeled 385 B 里最大两块是 `$0317-$03FF` 233 B（对象池尾部）与 `$6BB4-$6BD3` 32 B。

