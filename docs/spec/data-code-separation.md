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

每波顺序固定（W4 第一性复盘后修订为四步，见 §5.1）：
**写 extract → `pack-tool verify`（包 == ROM 重算）→ 波次边界跑全矩阵 → 才删旧件 + 删账本条（同一 commit）**。

### 5.1 退役登记表（实测波次，按落地顺序）

| 批 | 旧件 → 包段 | 提取函数 | commit | 验收 |
|---|---|---|---|---|
| W1 | `title/gfx_data.rs`（GFX_INFO/GFX_DATA）→ `gfx.info`/`gfx.data` | `extract_gfx_info`/`extract_gfx_data` | `27f821a1` + 门禁 `0a244270` | 三方式字节等 + L4 |
| W2 | `area/enemy_gfx.rs`（1.47 MB）→ `en.anim`/`en.fptr*`/`en.pptr`/`en.frames`/`en.places`/`en.*offs` | `extract_enemy_gfx` | `3ddd7702` | 表长 118/151×2/14×2 机械定谳 |
| W2b | 世界地图两份重复常量 → `world.map`；包首加 `meta.json` | `sections()` 内联 + `build_meta_json` | `c2309d00` | 两副本同源实证 |
| W3 | `area/{graphics,norfair_graphics}.rs` 房流五件套 → `area.br.*`/`area.no.*` | `extract_area(&AREA_BR/&AREA_NO)` | `b61d7bdb` | br 47/50/260 + no 46/49/248 |
| W4 | `area/graphics.rs` 敌九表 → `area.br.en_*` | `BR_ENEMY_TABLES` | `84ee8d92` | 九表逐项等 |
| W4b | `rom_tables.rs` → `rom.tbl968b`/`rom.vec95ce`/`rom.nt_hi` | `extract_rom_tables` | `54b7dfc3` | 8×128 / 8×2 / 4 B |
| W4c | `sound/music_tables.rs` → `sound.notes`/`sound.note_len*` | `extract_music_tables` | `82614dd2` | 128 + 11/12/12 |
| W4d | `sound/music_streams.rs`（19.4 KB）→ `sound.streams`/`stream_offs`/`stream_bases`/`env_*`/`sfx_data` | `extract_music_streams` | `07c71f85` | 48 流 2092 B 逐项等（含负索引 quirk） |
| W4e | `title/palette_data.rs`（18 常量）→ `pal.ptrs`/`pal.data`/`pal.offs` | `extract_palette_data` | `b46dec3b` | 六 bank 159 项 |
| W4f | `area/sprite_data.rs` → `sprite.data` | `extract_sprite_data` | `c3e0adf4` | 1457 B + 三表界 |
| W5a | `game-core/frame/sound_hook.rs::SOUND_REGION`（game-core 持 ROM 字节=架构违规）→ `sound.region` | `extract_sound_region` | `c9ddefeb` | 2688 B = `prg[0x3289..0x3D09]` |
| W5b | `area/items.rs` 五区 `ITEMS_*`（十进制结构体切片，旧判据 0 命中=隐身）→ `area.{br,no,kr,to,ri}.items`（6 B/条记录） | `extract_items` | `0dc10b35` | 15/33/9/35/6=98 条逐字节等 |
| W5c | `sound/sound_data.rs`（11 常量）→ 拆三路：SFX 4B 记录窗 `sound.sfx_tables`（120 B）+ 曲头表 `song.headers`（156 B）+ 偏移表读 `sound.region`（`$BBFA` 在窗内）；`SONG_INTRO_HEADER` 证伪为非 ROM 行（全表无此字节序）⇒ 降为测试内 fixture；`SFX_REG_INIT_TBL`/`EN_*_B7` 三常量零消费者 ⇒ 删 | `extract_sfx_tables`/`extract_song_headers` | 本批 | W2/W3/W4 25 处记录址逐一与旧常量等 + 156 B 逐行等 |
| W5d | `title/vram_structs.rs`（4 blob 1066/20/36/13 B，含终止）→ `title.vram_intro`/`vram_start`/`vram_pass`/`vram_over` | `extract_title_vram` | 本批 | 四段各自界且逐字节等 ROM（`$82F4`/`$9984`/`$99E3`/`$93B9`，bank0 窗）|
| W5e | `samus/samus_data.rs`（三表）→ `samus.actions`(4B)/`samus.pnt_up`(3B)/`samus.accel`(2B)；**更正**：旧 `RUN_ACCEL_TBL [u8;256]` 自 `$CCC0` 起吞了后续代码，已按 asm 真长 2 B 收口 | `extract_samus_tables` | 本批 | 三表逐字节等 ROM；`$CCC0+2` 即 `SetSamusRunAccel` 之后代码 |
| W5f | `render/render_data.rs`：`INTRO_STAR_PALETTES`（8×10B）→ `title.star_pal`；`NES_PALETTE` **不迁**（PPU 硬件主调色板，ROM 中不存在 ⇒ 平台常量随代码） | `extract_intro_star_palettes` | 本批 | 80 B 与 ROM `$8B0F` 窗逐字节等 + 八块互不重复（asm：non-repeating） |
| W5g | `title/title_data.rs` 整文件（20 常量）→ 17 段 `title.*`；`BOTTOM_SPARKLE_DATA` 真长 **98 B**（旧 54 B 截断=烘焙漂移）；`START_CONTINUE_BLOB` 与 `title.vram_start` 同源 ⇒ 删；`PW_NOERR_MSG`（全 `$FF`）与 `INIT_SPARKLE_DATA/_TBL` 重复对 ⇒ 代码侧收口 | `extract_title_tables` | 本批 | 11 表「旧==包==ROM」逐字节等；`BOTTOM` 98 B == ROM `$89E9`（旧 54 B 不等）；`pack-tool verify` 86 段 OK |
| W5h | `game-core/enemy/common.rs` 四件：`ENEMY_MOVEMENT_PTRS` 真表 **36 B**（旧 138 项=过度读）、`MOVEMENT_STRINGS` 512 B 窗、`L977B`/`EN_ANIM_TABLE` 复用已迁段（去重）| `BR_ENEMY_TABLES` | 本批 | 指针前 36 B == 段 == ROM `$96DB`；串区 == ROM `$97EF`；`L977B` 64 B == `BrEnemyFacing`；verify 88 段 OK |

**判据（每批三条机械可复跑）**：①`pack-tool verify` 全段 OK（包 == 从 ROM 重算，不再手搓脚本）
②账本 `data-boundary` v2 里该文件的 `data` 条目当次删除 ③波次边界 `l4_diff_run.py run-all` 全矩阵不变。

**meta 段硬约束（本批新增）**：`build_meta_json` 对"有段无 meta 条目"直接硬错——此前音乐/精灵/调色板/声引擎
16 个段在 meta 里被静默跳过（自描述缺段=漂移检测盲区）。

## 6. 门禁

1. **`data-boundary` v2**：仓库内不得出现"ROM 派生数据"——
   - 体量 = `max(const/static 数组顶层元素数, 0xNN 字节字面量数)`，阈值 200；
   - 账本 `data/status/data_boundary_allowlist.json` 三分：`data`（只减不增、归零即红、必须进包）/
     `fixture`（验证期望数据，禁进包、不 ratchet）/ `symbol`（地址表类，不 ratchet）；
   - `assets/*.pack` 一律 gitignore，且不得有 tracked 包。
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
