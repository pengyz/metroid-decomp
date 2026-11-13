# 数据与代码分离（ROM 派生数据外置为可提取资产包）

> 用户裁定（2026-11-13）：**仓库与发行二进制都不含 ROM 数据**。用户自备对应版本的
> ROM，用我们提供的工具（开发期独立工具）提取游戏数据（图形/CHR/音频/表），
> 产出资产包，运行时读取资产包。
>
> 参考：`~/workspace/super-metroid-decomp` 的 `docs/plan/runtime-pack-integration.md`
> （运行时改读注入 pack + `data-boundary` 门禁 + `tools-boundary` 门禁）与
> `docs/plan/data-materialization.md`（"解压/解码只有一份实现"纪律）。

## 1. 目标形态

```text
仓库：         纯代码 + 版式/常量/算法（无 ROM 字节、无派生二进制）
发行二进制：    纯代码，启动时要求 --pack（或环境变量 METROID_PACK）
用户机器：      rom.nes ──pack-tool extract──▶ metroid.pack ──▶ 游戏读取
开发机：        同上（CI/本地 L4 用同一 pack）
```

三条硬约束：

1. **代码里不许出现 ROM 字节派生的大常量数组**（现在 `crates/game-data/src` 有
   ~2.0 MB / 19 747 行这类数据，必须迁出）。
2. **地址/布局知识留在代码**（"ROM 的哪个偏移对应哪个段"是代码，不是数据）——
   这样提取器与运行期访问器共用同一份段清单，不会漂移。
3. **缺包必须 fail-loud**，打印如何生成，不静默回落到任何内置副本。

## 2. 资产包格式（`metroid.pack`）

单文件、自描述、按段切分。所有整数小端。

```text
Header（48 B）
  0x00  magic       "MTPK"             4 B
  0x04  version     u16 = 1            2 B
  0x06  flags       u16 = 0            2 B
  0x08  rom_sha256  [u8;32]           32 B  ← 提取时 ROM 的 sha256（校验用户 ROM 版本）
  0x28  rom_size    u32                4 B
  0x2C  section_count u16              2 B
  0x2E  reserved    u16                2 B
  0x30  SectionIndex[section_count]    每项 20 B
  0x..  payload

SectionIndex entry（20 B）
  0x00  key_id   u16    段 id（**名字在代码里**，见 PackKey；包里只存数值）
  0x02  flags    u16    bit0=reserved（压缩，暂不用）
  0x04  off      u32    相对 payload 起点
  0x08  len      u32    段长
  0x0C  crc32    u32    段内容 crc32（防截断/拼接）
  0x10  reserved u32
```

设计取舍：

- **段名不进包**：段 id 由代码里的 `PackKey` 枚举定义。这样包不含任何可读的语义信息，
  也不给"用别人的包跑我们的游戏"留口子（rom_sha256 已校验）。
- **不压缩**：包体量 ~2 MB 级，压缩只增加一条代码路径与一类失败；未来需要再加 flag。
- **段粒度按消费点切**（一个 `pub const` 一个段，或一组强相关常量共用一个段），
  不按 ROM bank 切——避免消费方为了拿 8 字节拖进 16 KB。

## 3. 工具 `pack-tool`（crates/pack-tool）

```text
pack-tool extract --rom <rom 路径> --out <pack 路径>   # 提取并写包（打印逐段 sha256 摘要）
pack-tool verify  --rom <rom 路径> --pack <pack 路径>  # 逐段 crc32 与 ROM 重算比对
pack-tool list    --pack <pack 路径>                  # 列出段 id/名称/长度/crc（调试用）
```

- 提取逻辑放在 `game-data` 的 **`extract` feature**（`crates/game-data/src/extract.rs`），
  与运行期访问器共用同一份 `PackKey` 段清单：**一个段只有一处定义、一处提取、一处读取**。
- ROM 身份校验：`pack-tool extract` 先算 sha256，与本仓库锁定的 ROM 锚点
  （`e6e6b701…65b6`，AGENTS §0）比对，不符直接拒绝并打印期望值。
- `pack-tool` 只依赖 `game-data`，不链接游戏引擎；发行版**不打包它**也行（开发期工具），
  但它是唯一官方提取入口。

## 4. 运行期契约

```rust
// crates/game-data/src/pack.rs
pub fn init_from_bytes(bytes: Vec<u8>) -> Result<(), PackError>;   // 校验 magic/version/rom_sha256
pub fn init_from_env() -> Result<(), PackError>;                   // METROID_PACK → 文件；默认 assets/metroid.pack
pub fn get(key: PackKey) -> &'static [u8];                         // 缺段/未初始化 ⇒ panic（打印生成指引）
pub fn try_get(key: PackKey) -> Option<&'static [u8]>;
pub fn is_ready() -> bool;
```

- 包在启动时读一次，`Box::leak` 成 `&'static`，访问器返回 `&'static [u8]`——
  这样 183 个常量访问点只需把 `CONST` 换成 `fn()` 调用，不必把 `&Pack` 穿到每一层。
  取舍：引入一个进程级单例（游戏本来就是单引擎实例）；换来的是迁移面大幅缩小。
- 二进制入口（`rust_screenshot` / PC 壳 / oracle 腿）启动时调 `init_from_env()`；
  测试用 `init_from_bytes(合成包)` 或显式 `SKIP`。

## 5. 迁移波浪（每波独立 commit、独立验收）

| 波 | 范围 | 体量 | 验收 |
|---|---|---|---|
| **W1** | `title/gfx_data.rs`（`GFX_INFO`/`GFX_DATA`；`GFX_OFFS` 改为按 info 现算） | ~24 KB | L4 112 格不变 + rust_screenshot 产物逐字节一致 |
| W2 | `area/{graphics,norfair_graphics,sprite_data,items}.rs`、`enemy/enemy_data.rs`、`area/enemy_gfx.rs` | **~1.47 MB** | 同上（图形/房流/敌表） |
| W3 | `title/{palette_data,world_map,vram_structs,title_data,area_tables}.rs`、`world/world_map.rs` | ~45 KB | 同上（标题/调色板/密码屏） |
| W4 | `sound/{music_streams,music_tables,sound_data}.rs` | ~24 KB | 同上（音频数据面） |
| W5 | `rom_tables.rs`、`samus/samus_data.rs`、`render/render_data.rs`、`room_data/room_ram.rs`、`constants/ram_layout.rs` | ~14 KB | 同上 |
| W6 | 删除 `tools/bake_*.py`（11 个，功能并入 `extract`），加门禁，清仓 | — | `data-boundary` 绿 + 全仓 grep 无 ROM 派生数组 |

每波顺序固定：**先加段与提取器 → 改访问器 → 改调用点 → 删旧常量 → 跑 L4 → commit**。

## 6. 门禁

1. **`data-boundary`**（新）：仓库内不得出现"ROM 派生数据"——
   - `game-data/src` 下任何 `.rs` 里 `const NAME: [u8; N]` 且 `N > 512` 且非纯代码常量 ⇒ FAIL；
   - 不得存在 `include_bytes!` 指向 `assets/` 之外或指向被 gitignore 的内容；
   - `assets/*.pack` 一律 gitignore。
2. **`tools-boundary`**（新）：只有 `crates/pack-tool` 与 `game-data`（`extract` feature）
   允许接收 `rom: &[u8]` 形参；其它 crate 出现 ROM 形参 ⇒ FAIL。
3. 既有 `rom-integrity` 保留（开发期本地 ROM 的 sha256 锚）。

## 7. 验收判据（W1 起每波都跑）

1. `cargo test --workspace` 全绿；数据相关测试在无包时**显式 SKIP**（打印原因），不静默通过。
2. `python3 gates/cli.py check` 快层 PASS。
3. **L4 run-all 112 格逐项不变**（包内容与旧常量逐字节相等 ⇒ 像素必然不变）。
4. `rust_screenshot` 单案 14/14 `ppu.hex` 与迁移前逐字节相同。
5. `pack-tool verify --rom rom/Metroid\ \(USA\).nes --pack assets/metroid.pack` 全部段 crc32 通过。

## 8. 风险与对策

| 风险 | 对策 |
|---|---|
| 迁移期两套读取路径并存（旧常量 + 包）→ 假绿 | 每波必须在**同一 commit 内删旧常量**（"解压只有一份实现"纪律） |
| 测试依赖真实数据 → 无 ROM 环境红 | 数据相关测试显式 SKIP；逻辑测试用合成包；真实 ROM 用例留 `#[ignore]` |
| 全局单例与并发测试冲突 | `OnceLock`，写一次后只读；测试用独立进程（`cargo test` 每 test 同进程 ⇒ 提供 `pack::reset_for_test()`） |
| 包与代码版本漂移 | 包头 `version` + 每段 `crc32` + `rom_sha256`；不匹配即 fail-loud |
| 大段（敌图 1.4 MB）拖慢启动 | 只用 `read`（不 mmap）、一次性泄漏；实测启动 <50 ms 可接受 |
