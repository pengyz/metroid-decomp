# 动态执行证据：字节面读写实测（P3 动态腿）

> 证据级别：**A**（测试内插桩实测 + 账本机械对账，2026-11-17）。
> 复现：`python3 tools/raw_execution_report.py`（内部现跑 `cargo test ... --nocapture`，不读缓存、不读环境变量）。

## 0. 结论（全部可由本页表格/命令重算）

- 账本比对面 **2272 B**；本次三场景实测触达 **964 B**（42%），未触达 **1308 B**。
- 触达中：读 351 B、写 921 B、读写双触 308 B。
- 面外（WRAM `$6000-$7FFF` 未进账本 face 的部分）另测得 **2226 B** 被读/写——账本**未建模**该区域（不是探针错误，见 §7）。
- 账本核对：Undeclared 段 **15/18** 段被实测使用（343 B）；Unmodeled 段 **8/27** 段在窗口内零触达（88 B）。
- Owner 段实测触达 **313/374 B**；owner 冲突 **0 处**（测试内 `assert_owner_unique` 全量遍历 + 合成双 Owner 用例）。

## 1. 方法

### 1.1 插桩形态（只在测试内）

- 探针：`crates/game-core/src/memmap/verify_shim.rs`，模块声明为 `#[cfg(test)] mod`，生产构建**不编译**；位图用 `thread_local!`，`arm()`/`disarm()` 显式开关，不读 `std::env`、不写文件。
- 挂点：`memmap/raw.rs` 的 `read_ram`/`write_ram`/`read_wram`/`write_wram` 四入口 + `samus/state.rs` 的 `Samus::get`/`Samus::set` **未命中 typed 时的字节面回退**。
- 口径：记录的是**字节面访问请求**（含 typed 域经兼容面读到的请求），不是物理存储写；地址归一与运行时一致（`$6000-$7FFF`→WRAM，其余 `& 0x7FF` 2KB 镜像）。

### 1.2 录制窗口

- 每个场景新建 `FrameEngine`；**tick1（boot）不计**——boot 的 `clear_pages` 经 `write_ram` 无条件零填 `$0200-$07FF`/`$6000-$7EFF`，计入会把整个面部误判为「写过」。
- 窗口为 tick2..=末帧；场景表见 §3。

### 1.3 已知盲区（未观测 ≠ 未使用）

- `frame/memory.rs` 写 PPUDATA 时 `self.ram()[0x00FF]` 为**直读**，不经 `read_ram`——`$00FF` 的这一次读不可见（同址其他路径的读写仍可见）。
- `restore_mss` / typed 清零直接 `ram_mut()`/`wram_mut()`：测试装配径，非游戏循环。
- 三场景（标题/起步行走/蓝门）覆盖有限：未触达**不等于**死字节，只说明「本窗口没用到」。

## 2. 面部总覆盖（按账本四态）

| 类 | 面部字节 | 实测触达 | 读 | 写 | 未触达 | 触达率 |
|---|---:|---:|---:|---:|---:|---:|
| Owner（typed 归属） | 374 | 313 | 54 | 313 | 61 | 83% |
| Unmodeled（已声明、字节面是真相） | 611 | 308 | 105 | 298 | 303 | 50% |
| Undeclared（面部无任何声明） | 1287 | 343 | 192 | 310 | 944 | 26% |
| **合计** | **2272** | **964** | **351** | **921** | **1308** | **42%** |

## 3. 场景矩阵与分场景触达

| 场景 | 帧窗 | 输入（帧号） | 面内触达 | 其中读 | 其中写 | WRAM 读区间数 | WRAM 写区间数 |
|---|---|---|---:|---:|---:|---:|---:|
| `title` | 2–400 | 见 `crates/game-core/src/memmap/verify_shim.rs` | 240 | 112 | 228 | 1 | 1 |
| `walk` | 2–1300 | 见 `crates/game-core/src/memmap/verify_shim.rs` | 950 | 310 | 906 | 56 | 14 |
| `door` | 2–1500 | 见 `crates/game-core/src/memmap/verify_shim.rs` | 961 | 349 | 917 | 53 | 12 |

输入回放与 `cases/case-001-game-start-samus-interaction.txt` / `cases/case-002-door-blue-interaction.txt` 同帧号；帧号即 `rust_screenshot` 的 `frames` 轴。

## 4. Undeclared 段实测（账本 18 段）

| # | 段 | 长 | 触达 | 读 | 写 | 未触达 | refs |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | `$0000` | 51 | 38 | 26 | 37 | 13 | ram_map.rs:70、ram_map.rs:73、ram_map.rs:26… |
| 2 | `$0034` | 3 | 1 | 0 | 1 | 2 | world/room_load.rs:240 |
| 3 | `$003B` | 14 | 14 | 0 | 14 | 0 | world/room_load.rs:167、world/room_load.rs:172、world/room_load.rs:173 |
| 4 | `$004A` | 1 | 1 | 1 | 1 | 0 | world/room_draw.rs:726、world/camera/scroll.rs:63 |
| 5 | `$004C` | 3 | 1 | 1 | 1 | 2 | samus/handler.rs:161、samus/physics.rs:236、samus/handler.rs:274… |
| 6 | `$0053` | 7 | 4 | 2 | 4 | 3 | asm:refs/m1disasm/SRC/constants_ram.asm:568、samus/handler.rs:84、world/room_draw.rs:511 |
| 7 | `$005C` | 9 | 1 | 1 | 1 | 8 | world/door.rs:12-13 |
| 8 | `$0066` | 5 | 3 | 1 | 3 | 2 | world/room_load.rs:247 |
| 9 | `$0070` | 140 | 124 | 35 | 123 | 16 | world/elevator.rs:424、world/update.rs:75、world/elevator.rs:159-160 |
| 10 | `$00FE` | 8 | 2 | 2 | 2 | 6 | world/room_load.rs:32、render/vram.rs:35、frame/memory.rs:195 |
| 11 | `$0108` | 2 | 2 | 2 | 0 | 0 | frame/tick.rs:637 |
| 12 | `$010A` | 246 | 4 | 3 | 3 | 242 | ui/hud.rs:47、frame/memory.rs:195 |
| 13 | `$04E0` | 16 | 0 | 0 | 0 | 16 | asm:refs/m1disasm/SRC/constants_ram.asm:975 |
| 14 | `$04F0` | 16 | 0 | 0 | 0 | 16 | asm:refs/m1disasm/SRC/constants_ram.asm:976 |
| 15 | `$0500` | 208 | 26 | 13 | 13 | 182 | asm:refs/m1disasm/SRC/constants_ram.asm:986、asm:refs/m1disasm/out/M1_NES_NTSC.sym:11708、world/statue.rs:30… |
| 16 | `$05D0` | 48 | 0 | 0 | 0 | 48 | asm:refs/m1disasm/out/M1_NES_NTSC.sym:11847、asm:refs/m1disasm/out/M1_NES_NTSC.sym:11995 |
| 17 | `$0600` | 131 | 67 | 62 | 63 | 64 | sound/mod.rs:25、sound/mod.rs:33、asm:refs/m1disasm/SRC/constants_ram.asm:1001 |
| 18 | `$0685` | 379 | 55 | 43 | 44 | 324 | ram_map.rs:102、frame/tick.rs:600、frame/nmi.rs:191… |

零触达的 Undeclared 段（**本窗口**未使用，不代表死字节）：

- `$04E0` × 16 B（refs：asm:refs/m1disasm/SRC/constants_ram.asm:975）
- `$04F0` × 16 B（refs：asm:refs/m1disasm/SRC/constants_ram.asm:976）
- `$05D0` × 48 B（refs：asm:refs/m1disasm/out/M1_NES_NTSC.sym:11847、asm:refs/m1disasm/out/M1_NES_NTSC.sym:11995）

## 5. Unmodeled 段实测（账本 27 段）

| # | 段 | 长 | 触达 | 读 | 写 | 未触达 | refs |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | `$0033` | 1 | 1 | 0 | 1 | 0 | crates/game-core/src/world/room_ram.rs:156 |
| 2 | `$0037` | 2 | 0 | 0 | 0 | 2 | crates/game-core/src/world/room_ram.rs:156 |
| 3 | `$0039` | 2 | 2 | 2 | 2 | 0 | crates/game-core/src/world/room_ram.rs:156 |
| 4 | `$0049` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 5 | `$004F` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 6 | `$0050` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 7 | `$0051` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 8 | `$0052` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 9 | `$00FC` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 10 | `$00FD` | 1 | 1 | 1 | 1 | 0 | crates/game-core/src/world/camera/mod.rs:96 |
| 11 | `$0106` | 2 | 2 | 2 | 2 | 0 | crates/game-core/src/samus/state.rs:615 |
| 12 | `$0317` | 233 | 232 | 44 | 232 | 1 | crates/game-core/src/world/object.rs:11 |
| 13 | `$0400` | 192 | 44 | 32 | 42 | 148 | crates/game-core/src/enemy/state.rs:283 |
| 14 | `$04C0` | 32 | 0 | 0 | 0 | 32 | crates/game-core/src/enemy/explosion.rs:285 |
| 15 | `$6AFB` | 9 | 1 | 1 | 0 | 8 | crates/game-core/src/enemy/state.rs:814 |
| 16 | `$6B0B` | 9 | 1 | 1 | 0 | 8 | crates/game-core/src/enemy/state.rs:814 |
| 17 | `$6B1B` | 9 | 5 | 5 | 4 | 4 | crates/game-core/src/enemy/state.rs:814 |
| 18 | `$6B2B` | 9 | 1 | 1 | 0 | 8 | crates/game-core/src/enemy/state.rs:814 |
| 19 | `$6B3B` | 9 | 4 | 3 | 4 | 5 | crates/game-core/src/enemy/state.rs:814 |
| 20 | `$6B4B` | 9 | 4 | 3 | 4 | 5 | crates/game-core/src/enemy/state.rs:814 |
| 21 | `$6B5B` | 9 | 0 | 0 | 0 | 9 | crates/game-core/src/enemy/state.rs:814 |
| 22 | `$6B6B` | 9 | 0 | 0 | 0 | 9 | crates/game-core/src/enemy/state.rs:814 |
| 23 | `$6B7B` | 9 | 0 | 0 | 0 | 9 | crates/game-core/src/enemy/state.rs:814 |
| 24 | `$6B8B` | 9 | 0 | 0 | 0 | 9 | crates/game-core/src/enemy/state.rs:814 |
| 25 | `$6B9B` | 9 | 0 | 0 | 0 | 9 | crates/game-core/src/enemy/state.rs:814 |
| 26 | `$6BAB` | 9 | 0 | 0 | 0 | 9 | crates/game-core/src/enemy/state.rs:814 |
| 27 | `$6BB4` | 32 | 4 | 4 | 0 | 28 | crates/game-core/src/enemy/explosion.rs:288 |

窗口内**零触达**的 Unmodeled 段（账本声称「字节面是真相」，实测未被执行）：

- `$0037` × 2 B（refs：crates/game-core/src/world/room_ram.rs:156）
- `$04C0` × 32 B（refs：crates/game-core/src/enemy/explosion.rs:285）
- `$6B5B` × 9 B（refs：crates/game-core/src/enemy/state.rs:814）
- `$6B6B` × 9 B（refs：crates/game-core/src/enemy/state.rs:814）
- `$6B7B` × 9 B（refs：crates/game-core/src/enemy/state.rs:814）
- `$6B8B` × 9 B（refs：crates/game-core/src/enemy/state.rs:814）
- `$6B9B` × 9 B（refs：crates/game-core/src/enemy/state.rs:814）
- `$6BAB` × 9 B（refs：crates/game-core/src/enemy/state.rs:814）

## 6. 缺口清单（面部未触达字节）

未触达 **1308 B**，按连续区间（最多列 40 段；完整集合由脚本可重算）：

- `$0007-$0009`（3 B；账本类：undeclared）
- `$000E-$0011`（4 B；账本类：undeclared）
- `$0020-$0022`（3 B；账本类：undeclared）
- `$0026-$0026`（1 B；账本类：undeclared）
- `$0031-$0032`（2 B；账本类：undeclared）
- `$0035-$0038`（4 B；账本类：undeclared/unmodeled）
- `$004C-$004C`（1 B；账本类：undeclared）
- `$004E-$004E`（1 B；账本类：undeclared）
- `$0053-$0054`（2 B；账本类：undeclared）
- `$0059-$005A`（2 B；账本类：owner/undeclared）
- `$005C-$0063`（8 B；账本类：undeclared）
- `$0066-$0067`（2 B；账本类：undeclared）
- `$0078-$0078`（1 B；账本类：undeclared）
- `$00DF-$00DF`（1 B；账本类：undeclared）
- `$00E8-$00E9`（2 B；账本类：undeclared）
- `$00EE-$00EF`（2 B；账本类：undeclared）
- `$00F1-$00F9`（9 B；账本类：undeclared）
- `$00FB-$00FB`（1 B；账本类：undeclared）
- `$0100-$0105`（6 B；账本类：undeclared）
- `$010C-$010D`（2 B；账本类：undeclared）
- `$0110-$01FF`（240 B；账本类：undeclared）
- `$03FF-$0404`（6 B；账本类：unmodeled）
- `$0406-$0414`（15 B；账本类：unmodeled）
- `$0416-$041F`（10 B；账本类：unmodeled）
- `$042C-$042C`（1 B；账本类：unmodeled）
- `$0430-$0434`（5 B；账本类：unmodeled）
- `$0436-$043F`（10 B；账本类：unmodeled）
- `$0443-$0443`（1 B；账本类：unmodeled）
- `$0446-$0446`（1 B；账本类：unmodeled）
- `$044C-$044C`（1 B；账本类：unmodeled）
- `$0453-$0453`（1 B；账本类：unmodeled）
- `$0456-$0456`（1 B；账本类：unmodeled）
- `$045C-$045C`（1 B；账本类：unmodeled）
- `$0460-$04FF`（160 B；账本类：undeclared/unmodeled）
- `$0501-$0508`（8 B；账本类：undeclared）
- `$050A-$050F`（6 B；账本类：undeclared）
- `$0511-$0518`（8 B；账本类：undeclared）
- `$051A-$051F`（6 B；账本类：undeclared）
- `$0521-$0528`（8 B；账本类：undeclared）
- `$052A-$052F`（6 B；账本类：undeclared）
- …共 113 段（其余略）

## 7. 与账本的差异（本页唯一的「冲突」口径）

1. **Undeclared 被实测使用**：15/18 段、343 B。账本口径 = 「面部无任何声明」（refs 是「谁碰过它」的线索，不是归属）——实测确认这些字节**不是死字节**，是**待归属的债**。
2. **Unmodeled 窗口内零触达**：8/27 段、88 B。账本称「已声明、字节面是真相」；实测在标题/起步/蓝门三场景内未执行——要么场景未覆盖（更可能），要么该字段当前版本走的是别的路径。**不据此改账本**。
3. **面外 WRAM**：2226 B 被读写但不在 `face`（`$6000-$7FFF` 仅 `$6AF4-$6BD3` 进账本）。这是账本的**已知面部边界**，不是探针越界；若将来要把 WRAM 全量纳入归属，分母需先扩。
4. **owner 冲突 0 处**：测试内对全部 SPANS 逐字节 `assert_owner_unique`；另有合成双 Owner 用例证明冲突会 panic 且文本含地址（`$0300`）。

## 8. 限制与后续

- 本报告是**执行证据**，不是完备性证明：三场景、约 3200 帧，覆盖的是「起步房间 + 蓝门」路径。
- 若要提高分母，按 `cases/` 增加场景即可（同一探针，无需改生产代码）；新增场景后重跑本脚本，数字会自动更新。
- 静态腿（`gates/cli.py check` 的 nes-map 门）仍守「每字节唯一 Owner / 账本只减不增」；动态腿不替代门禁，只把「没人用」从自陈变成实测。
