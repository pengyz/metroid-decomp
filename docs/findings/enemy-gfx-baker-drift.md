# 敌帧/位数据面：烘焙器与提交件已漂移（W2 暂缓迁移）

> 状态：**未定谳**。W2（数据与代码分离第二波）原计划迁移 `crates/game-data/src/area/enemy_gfx.rs`
> （1.4 MB，仓内最大 ROM 派生数据件），实测后发现「脚本口径」与「提交件口径」不一致，
> 迁移会把运行真值换掉 ⇒ **暂停迁移**，先取证。

## 1. 现象（A 级，可复现）

```text
$ python3 tools/bake_enemy_gfx.py --check
[FAIL] enemy_gfx 与 ROM 烘焙一致（--check）
```

即：**当前提交的 `area/enemy_gfx.rs` 不是当前 `tools/bake_enemy_gfx.py` 生成的产物**
（脚本在 r26 之后被改过，而文件没有重新生成；或当时用一次性脚本生成）。

同类漂移清单（同一命令族，2026-11-13 实测）：

| 烘焙器 | `--check` |
|---|---|
| `bake_area_graphics.py` | FAIL（per-area 参数化后产物改为 `brinstar_graphics.rs`/`norfair_graphics.rs`，属预期漂移） |
| `bake_enemy_gfx.py` | **FAIL（本档）** |
| `bake_gfx.py` | FAIL（W1 已把 `title/gfx_data.rs` 改为包访问器，属预期） |
| `bake_area_map.py` / `bake_music_streams.py` / `bake_music_tables.py` / `bake_palette_data.py` / `bake_sprite_data.py` / `bake_vram_structs.py` | PASS |

## 2. 差异的量化

以 ROM（`rom/Metroid (USA).nes`，131 088 B，整文件 crc32 `A2C89CB9`）为准，
按当前脚本口径重新提取，与提交件常量逐项对比：

| 段 | 提交件常量 | 脚本口径提取 | 判定 |
|---|---|---|---|
| `en.anim`（`EnemyAnimIndexTbl $9D6A`，128 B） | 128 B | 128 B | 一致 |
| `en.fptr1`（`EnFramePtrTable1 $9DE0`，256 B） | 256 B | 256 B | 一致 |
| `en.fptr2`（`EnFramePtrTable2 $9EE0`，256 B） | 256 B | 256 B | 一致 |
| `en.pptr`（`EnPlacePtrTable $9F0E`，256 B） | 256 B | 256 B | 一致 |
| `en.frames`（帧数据体） | **13 640 B** | **13 640 B** | **一致** |
| `en.foffs`（256 项） | 256 项 | 256 项 | 一致 |
| `en.places`（位件数据体） | **204 448 B** | **539 457 B** | **不一致（2.64×）** |
| `en.poffs`（128 项） | 128 项 | 128 项（含 len=63496 这类声明长度） | 不一致 |

⇒ **只有「位件（placement）数据体」不一致；帧面完全一致。**

## 3. 结构性成因（A 级）

`AreaPointers_ROM` 的 7 个字给出四张表的 CPU 地址
（`refs/m1disasm/SRC/prg1_brinstar.asm:92-95`）：

```text
EnFramePtrTable1: ($9DE0)   ; 两张表以容纳全部条目
EnFramePtrTable2: ($9EE0)
EnPlacePtrTable:  ($9F0E)
EnAnimTable:      ($9D6A)
```

**地址算术不自洽**：`$9DE0 + 256 = $9EE0` ✓（表 1 恰好 128 项），
但 `$9EE0 + 256 = $9FE0 ≠ $9F0E`（差 0x2E=46，即表 2 若按脚本假设的 128 项会**越过**
place 表起点 46 字节）。实测两张表里确实混入了**非指针值**：

| 表 | 低址（`< $8000`，非合法 ROM 指针）项数 | 零项 | 最小非零值 |
|---|---|---|---|
| `fptr1` | 0 | 0 | `$9FC2` |
| `fptr2` | **34** | 4 | `$0004` |
| `pptr` | **44** | 4 | `$0004` |

这些 `< $8000` 的值（`$0004/$00F4/$00FC/$0400/$04F0/$04F8/$0408…`）不是 ROM 指针形态，
而是**表长口径错误导致的跨表/跨数据错位读**。

`tools/bake_enemy_gfx.py` 对此没有防护：`off = BASE + (addr - 0x8000)`，`addr < 0x8000`
时得到**负偏移**，Python 列表负索引会从 ROM **尾部**回绕取字节；且 `prg[o:o+len]` 越界时
Python **静默截断**而不报错。于是：

1. 相同脚本重跑得到的字节取决于「回绕读到的尾部内容」；
2. 项表里记录的 `len`（声明长度）与数据体实际字节数会脱钩（如 `len=63496`）；
3. 一旦把这段逻辑直译成 Rust，负偏移会 `panic`（subtract with overflow）或取到不同字节
   ⇒ **天然不可移植**。

## 4. 结论与处置

**处置：W2 的敌帧/位数据面迁移暂停**（`extract.rs` 里 `extract_enemy_gfx` 保留实现但
`out.extend(...)` 已注释，附本档指针），原因不是工程量，而是**证据不足**：

- 提交件（`EN_PLACES` 204 448 B）是**当前运行真值**（13/14 语义基线 + L4 110/112 建立在它之上）；
- 脚本口径（539 457 B）是**当前脚本的产物**，但没有证据表明它更接近原版行为；
- 两者相差 335 KB，涉及全部敌位件（敌人外形摆放），迁移等于**无声替换 1.4 MB 敌动画数据**；
- L4 14 个用例的敌覆盖很窄（`docs/findings/sound-handler-exposure.md` 同族结论：已实现≠已验证），
  「L4 不变」不足以证明敌行为不变。

## 5. 复现命令

```bash
# 脚本与提交件是否漂移
python3 tools/bake_enemy_gfx.py --check      # → FAIL

# 指针表里的非指针值（低址项）
python3 - <<'PY'
prg = open("rom/Metroid (USA).nes","rb").read()[16:]
BASE = 0x4000
def tbl(addr, n=256):
    o = BASE + (addr - 0x8000)
    return [prg[o+i] | (prg[o+i+1] << 8) for i in range(0, n, 2)]
for name, addr in [("fptr1",0x9DE0),("fptr2",0x9EE0),("pptr",0x9F0E)]:
    a = tbl(addr)
    bad = [(i, hex(v)) for i, v in enumerate(a) if 0 < v < 0x8000]
    print(name, "低址项", len(bad), "零项", sum(1 for v in a if v == 0), "min非零", hex(min(v for v in a if v)))
PY

# 地址算术：$9DE0+256=$9EE0 ✓，$9EE0+256=$9FE0 ≠ $9F0E（差 46）
```

## 6. 待办（重新打开本卡的前提）

1. **定谳表长**：以 `PtrTableEntry` 宏（`refs/m1disasm/SRC/macros.asm:76-96`）与
   `brinstar/enemy_sprite_data.asm` 的实际条目数为准，确定 `EnFramePtrTable1/2` 与
   `EnPlacePtrTable` 各自的**真实项数**（脚本假设 128/128/128 与地址算术矛盾）。
2. **定谳位件长度规则**：`place_end` 的真实语义（帧面用 `$FF` 自界；位件是否也是自界流？
   脚本用的是「至下一指针」+ `max(4,…)`，该规则在真实项数下是否仍成立）。
3. **对齐运行真值**：以提交件（204 448 B）为参照，找出产生它的规则；若提交件本身是旧 bug
   的产物，则按 asm 重算并**单独 commit**（附 L4 + 语义基线证据），不与本波混合。
4. 三者齐备后，再把 `en.*` 八段并入资产包并删掉 1.4 MB 常量。

## 7. 教训（写进方法论）

- **「搬运」不是「重算」**：数据迁移的验收判据必须是「包内字节 == 提交件字节」，
  而不是「脚本能跑通」。本波正是被这条拦住。
- **`--check` 是漂移探针**：11 个烘焙器里有 2 个真实漂移（`enemy_gfx`、`area_graphics`），
  长期没人跑 `--check`，导致「脚本改过、数据没重生成」的状态无人察觉。
  W6 应把「全部烘焙器 `--check` PASS」升级为门禁（或直接删脚本，改由 `pack-tool` 单一实现）。
- **Python 负索引/切片截断不可直译**：任何从 Python 搬到 Rust 的数据提取，都要检查
  越界路径；正例见 `extract.rs` 的 `bank_source_offset()`（显式 `Err`），反例即本档。
