# `$6AFB-$6BD3` 敌额外面尾段语义规格（B2-b 取证）

> 目标：给 `ensmap-dump` 里没有 note 的 13 段 / 140 B 做取证规格，判断哪些能 typed、哪些只能 `_todo_`。
> 范围：`$6AFB-$6B03`、`$6B0B-$6B13`、…、`$6BAB-$6BB3`（12 段 × 9 B）与 `$6BB4-$6BD3`（32 B）。
> 本卡只取证，不改 `crates/`。结论先说（§0）。

## 0. 结论摘要

| # | 结论 | 级 |
|---|---|---|
| 1 | 12 段 × 9 B 是 **`EnsExtra` / `EnProjectilesExtra` 各 6 槽的「尾 9 B」**：每槽 16 B 里前 7 B（`status`…`animIndex`，偏移 0–6）在 Rust 已是 typed，剩偏移 7–15 落在字节面。**12 段不是独立结构，是同一结构的尾巴**。 | A |
| 2 | 这 9 个字节**在反汇编里全部有具名**（不是「语义未知」）：`hi`、`subPixelY`、`subPixelX`、`accelY`、`accelX`、`spare1C`、`jumpDsplcmnt`、`type`、`data1F`（`constants_ram.asm:125-160` 的 `.struct EnExtra`）。所谓 Unmodeled 是**表示层债**（Rust 侧仍走 `read_wram(0x6AF4+x+off)` 字节面），不是 ROM 语义空白。 | A |
| 3 | 第 13 段 `$6BB4-$6BD3`（32 B）**不是尾巴**，而是 `EnExplosionsExtra`（4 槽 × 8 B 的 `.struct EnExplosionExtra`，`constants_ram.asm:161-170`）——它的 8 个字段全部已具名，尾字节根本不存在。把这一段与 12 段并列会误导。 | A |
| 4 | 运行期活跃度（10 009 份 trace）：EnsExtra 6 槽的活字节集中在 `hi`、`subPixelY/X`、`accelY/X`、`jumpDsplcmnt`、`type`、`data1F`；**`spare1C` 在全部 trace 里恒为 0**；EnProjectilesExtra 6 槽只活得前 8 B（`status`…`hi`），尾 9 B 全 0；`EnExplosionsExtra` 只活 `animFrame/animDelay/resetAnimIndex/animIndex` 4 B。 | A（统计见 §3） |
| 5 | 「per-area extra」假设**否证**：这些基址是全局唯一的一份（符号表各一条），寻址形状是 `基址 + 槽*16 + 偏移`，没有「区号 × 基址」；跨区 trace 里同一槽地址都在用（§4 给命令）。 | A |
| 6 | Rust 侧**已具名**这些字节（`enemy/state.rs` 有 `accel_y/accel_x/jump_dsplcmnt/subpixel_y/subpixel_x/hi/enemy_type/data1f` 访问器），但**表示仍是字节面**；`NesMapped` 声明把每槽 7 B 记 `Owner`、9 B 记 `Unmodeled`（`state.rs:942-970`）。 | A |
| 7 | 可直接 typed 的字节：`hi`、`subPixelY/X`、`accelY/X`、`jumpDsplcmnt`、`type`、`data1F`；只能 `_todo_` 的：`spare1C`（原版注释「Unused」+ 运行恒 0）；不能动的：无（本段不涉及硬件影子）。 | B |

## 1. 分段表（与 `nes_map_check --list` 逐条对齐）

`python3 tools/nes_map_check.py --list | grep 6A` 实测输出（原样）：

```
[Unmodeled] $6AFB-$6B03 (9 B): （无）
[Unmodeled] $6B0B-$6B13 (9 B): （无）
[Unmodeled] $6B1B-$6B23 (9 B): （无）
[Unmodeled] $6B2B-$6B33 (9 B): （无）
[Unmodeled] $6B3B-$6B43 (9 B): （无）
[Unmodeled] $6B4B-$6B53 (9 B): （无）
[Unmodeled] $6B5B-$6B63 (9 B): （无）
[Unmodeled] $6B6B-$6B73 (9 B): （无）
[Unmodeled] $6B7B-$6B83 (9 B): （无）
[Unmodeled] $6B8B-$6B93 (9 B): （无）
[Unmodeled] $6B9B-$6BA3 (9 B): （无）
[Unmodeled] $6BAB-$6BB3 (9 B): （无）
[Unmodeled] $6BB4-$6BD3 (32 B): （无）
```

槽归属（`refs/m1disasm/out/M1_NES_NTSC.sym`）：

| 段 | 槽 | 结构 | 槽首 | 尾 9 B 的绝对地址 |
|---|---|---|---|---|
| 1 | EnsExtra[0] | `EnExtra` 16 B | `$6AF4` | `$6AFB-$6B03` |
| 2 | EnsExtra[1] | 同上 | `$6B04` | `$6B0B-$6B13` |
| 3 | EnsExtra[2] | 同上 | `$6B14` | `$6B1B-$6B23` |
| 4 | EnsExtra[3] | 同上 | `$6B24` | `$6B2B-$6B33` |
| 5 | EnsExtra[4] | 同上 | `$6B34` | `$6B3B-$6B43` |
| 6 | EnsExtra[5] | 同上 | `$6B44` | `$6B4B-$6B53` |
| 7 | EnProjectilesExtra[0] | `EnExtra` 16 B | `$6B54` | `$6B5B-$6B63` |
| 8 | EnProjectilesExtra[1] | 同上 | `$6B64` | `$6B6B-$6B73` |
| 9 | EnProjectilesExtra[2] | 同上 | `$6B74` | `$6B7B-$6B83` |
| 10 | EnProjectilesExtra[3] | 同上 | `$6B84` | `$6B8B-$6B93` |
| 11 | EnProjectilesExtra[4] | 同上 | `$6B94` | `$6B9B-$6BA3` |
| 12 | EnProjectilesExtra[5] | 同上 | `$6BA4` | `$6BAB-$6BB3` |
| 13 | EnExplosionsExtra[0..3] | `EnExplosionExtra` 8 B ×4 | `$6BB4` | —（整段就是 4 槽，无尾） |

槽首公式（Rust 已用同一式）：`$6AF4 + slot*16`（`state.rs:936` `ensx_slot()`）。

## 2. 逐字节字段表（12 段共用，偏移 7–15）

`hi` 以下的字段名与注释全部来自 `refs/m1disasm/SRC/constants_ram.asm:125-160`（`.struct EnExtra`）。「写点/读点」给的是**具名引用**（asm 里以 `EnsExtra.0.<field>,x` 形式出现，x 已含 `槽*16+偏移`）。

| 偏移 | 绝对地址（槽 0） | 字段 | 语义（原文要点） | 写点（bank:asm 行） | 读点（bank:asm 行） | 级 |
|---|---|---|---|---|---|---|
| 7 | `$6AFB` | `hi` | 名表号（0 = NT0，1 = NT3） | `prg7_engine.asm:8951`（`GetEnemyType` 出生时写） | `:6105`、`:8137`、`:10258`、`:10266`、`:10710`（绘制/切表） | A |
| 8 | `$6AFC` | `subPixelY` | 竖直亚像素累加器 | `areas_common.asm:902`（`sta EnsExtra.0.subPixelY,x`） | `areas_common.asm:899` | A |
| 9 | `$6AFD` | `subPixelX` | 水平亚像素累加器 | `areas_common.asm:976`（`sta EnsExtra.0.subPixelX,x`） | `areas_common.asm:973` | A |
| 10 | `$6AFE` | `accelY` | 竖直加速度增量 | `areas_common.asm:449/528` | `areas_common.asm:526/848` | A |
| 11 | `$6AFF` | `accelX` | 水平加速度增量 | `areas_common.asm:462/481` | `areas_common.asm:479/932` | A |
| 12 | `$6B00` | `spare1C` | 原版注释 **「Unused」** | 无（全反汇编零具名引用） | 无 | A（运行恒 0）+ B（语义=预留） |
| 13 | `$6B01` | `jumpDsplcmnt` | 跳/位移像素数；Skree 起爆延迟 | `:10754`（`sta EnsExtra.0.jumpDsplcmnt,x`） | `:11743`、`:12276` | A |
| 14 | `$6B02` | `type` | 敌型 = 敌参数表下标 | `:8934`（`GetEnemyType` 存 type） | `:5951`、`:8953`、`:10519`、`:10782`、`:10872` | A |
| 15 | `$6B03` | `data1F` | 两用：`EnemyFlipAfterDisplacement` 的符号位（`$00`=绝对值、`$40`=正、`$80/$C0`=负）；`EnemyIfMoveFailed` 的落点选择 | `:11281`（`sta EnsExtra.0.data1F,x`）、`:12278/12279`（`inc/dec` 读改写）、`areas_common.asm:725` | `areas_common.asm:216/282/344/404/704/752`（`lda EnsExtra.0.data1F,x`）；`:12278/12279` 的 inc/dec 本身也先读 | A |

补充两条以免误读：

- `type` 与 `hi` 的写法在出生径里是**成对**的（`:8934` 存 type、`:8951` 存 hi）；跨 bank 的 `LoadTableAt977B` 直接 `ldy EnsExtra.0.type,x` 取表下标，所以 `type` 是「每区参数表的索引」，**不是**区号。
- `data1F` 的四个取值不是位标志，而是**两个例程各自的枚举**（同一字节两种解释）。typed 时若要枚举化，必须标「解释随调用例程变化」，不能合成一个 bitflags。

## 3. 运行期分布：哪些字节真的在动

统计脚本（`analysis/traces/*.ram.hex` 每份前 2048 B 是 `$0000-$07FF`，其后 224 B 是 `$6AF4-$6BD3`；只取长度 ≥ 2272 的文件）：

```python
import glob, pathlib, collections
nz=collections.Counter(); tot=0
for p in sorted(glob.glob('analysis/traces/*.ram.hex')):
    b=bytes.fromhex(pathlib.Path(p).read_text().strip())
    if len(b) < 2272: continue
    tot += 1
    ex=b[2048:2048+224]
    for i,v in enumerate(ex):
        if v: nz[i]+=1
print('files', tot)
print('explosion 6BB4..6BD3:', ' '.join(str(nz.get(0xC0+o,0)) for o in range(32)))
for slot in range(12):
    print(f'{0x6AF4+slot*16:04X}', ' '.join(str(nz.get(slot*16+o,0)) for o in range(16)))
```

实测（`files 10009`，按「槽首 + 偏移 0..15」列出非零帧数）：

```
6AF4 3011 2221 2221 2196 2196 2196 2196 2745 232  34 196  40   0  961 2956  378
6B04 2735 2228 2228 2100 2004 2100 2101 2758 185  16 131  16   0  942 2796  118
6B14 2873 2125 2125 2082 2094 2082 2094 3083 206  22 197  28   0  614 3002  611
6B24 3158 2363 2363 2159 2178 2159 2178 3242 320 101 341 110   0 1050 3047  434
6B34 6103 6081 6081 6070 6079 6070 6079 1455 214  79 269 110   0 5128 6085  291
6B44 5962 6077 6077 5673 6069 5673 6075 1427 146  56 266  66   0 4938 6125  312
6B54   94  134  134  134  134  134  134  133   0   0   0   0   0    0    0    0
6B64   81   87   87   87   87   87   87   84   0   0   0   0   0    0    0    0
6B74   72   75   75   75   75   75   75   74   0   0   0   0   0    0    0    0
6B84   66   71   71   71   71   71   71   66   0   0   0   0   0   0    0    0
6B94   27   29   29   29   29   29   29   27   0   0   0   0   0   0    0    0
6BA4    0    1    1    1    1    1    1    0   0   0   0   0   0    0    0    0
explosion 6BB4..6BD3: 0 0 0 345 345 345 345 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
```

读法与结论：

- **EnsExtra（6AF4–6B53）**：偏移 8–11（`subPixelY/X`、`accelY/X`）与 13–15（`jumpDsplcmnt/type/data1F`）都活；偏移 7（`hi`）在绘制帧高、出生帧低，符合「只在上屏/出生时写」。
- **偏移 12（`spare1C`）全 12 槽、10 009 帧全为 0** ⇒ 「预留」有实测支撑，不是只看注释。
- **EnProjectilesExtra（6B54–6BB3）**：尾 9 B **全 0**。这是重要区别——`EnProjectilesExtra` 是 `EnExtra` 同型结构（16 B），但弹体从不使用亚像素/加速度/type 尾段 ⇒ 这些字节对弹体槽是「结构上存在、运行时不用」。
- **EnExplosionsExtra（6BB4–6BD3）**：只有偏移 3–6（`animFrame/animDelay/resetAnimIndex/animIndex`）在动；`status` 与 `hi` 在采样帧都是 0（爆炸在 4 帧内结束，采点未命中）。

## 4. 「per-area extra」假设：否证（A）

假设：这 13 段是按区域（Brinstar/Norfair/…）各一份的额外参数面。

三条反证：

1. **符号表只有一份**：`EnsExtra`/`EnProjectilesExtra`/`EnExplosionsExtra` 各一个基址（`$6AF4/$6B54/$6BB4`），没有 `.enum` 里按区再开一段。
2. **寻址形状固定**：代码全是 `EnsExtra.0.<field>,x`，x 由调用方给「槽偏移」（0、16、32…），没有区号参与；跨 bank 的共用例程（`areas_common.asm`）用同一公式。
3. **运行分布**：同一槽地址在五个区的 trace 里都在动（统计里 6AF4 槽 0 在 3 011 帧非零，覆盖 Brinstar/Norfair/Kraid/Tourian/Ridley 五区采集）；若按区分块，应出现「某区活跃、别的区恒 0」的分段，实测没有。

结论：这 140 B 的语义完全由「结构 + 槽号 + 偏移」决定，与区号无关。

## 5. 与 Rust 现有实现的逐项对照

| 项 | 位置 | 状态 |
|---|---|---|
| 基址/步长 | `enemy/state.rs:274/277`（`ENS_EXTRA_BASE=$6AF4`、`ENEMY_STRIDE=16`） | 一致 |
| 已 typed 的 7 B | `state.rs:133-152`（`mod ensx`：`STATUS/RADIUS_Y/RADIUS_X/ANIM_FRAME/ANIM_DELAY/RESET_ANIM_INDEX/ANIM_INDEX`，`TYPED_MAX=ANIM_INDEX=6`） | 一致（偏移 0–6） |
| 尾 9 B 的表示 | `state.rs:309` `extra(off) = 0x6AF4 + x + off`，经 `getx_/putx` 字节面 | **语义已具名、表示未 typed** |
| 尾字段访问器 | `state.rs:443 accel_y`、`:452 accel_x`、`:569 jump_dsplcmnt`、`:706 subpixel_y`、`:713 subpixel_x`、`:719 hi`、`:725 enemy_type`、`:731 data1f` | 存在（最终仍落字节面） |
| `NesMapped` 声明 | `state.rs:942-970`：每槽「7 B Owner + 9 B Unmodeled」×12 | 与 §1 分段完全对应 |
| 尾段债说明 | `state.rs:934` `ENSX_TAIL` 文案「尾 9 B 未 typed：accelY/jumpDsplcmnt/type/data1F 等」 | 一致；建议补 `hi/subPixelY/subPixelX/accelX/spare1C` 六个名字，避免读者以为只有 4 个字段 |
| `spare1C` | 无任何常量/访问器 | **缺失但不该加语义**：建议 typed 成 `_reserved` |
| `EnExplosionsExtra` | `enemy/explosion.rs`：`EXTRA_BASE=0x6BB4`(`:21`)、`EXTRA_STRIDE=8`(`:23`)、字段偏移模块 `explx`(`:40-56`)、句柄 `Explosion`+`Manager::explosion()`(`:58/:65`)、测试 `explosion_stride_differs_from_enemy_stride`(`:285`) 与 `$6BB4/$6BBC` 落址断言 | **已有字段偏移视图与句柄访问器**，但 `NesMapped` 声明为 `Unmodeled`（`:309-311`，0 owner），真相仍在字节面；span 在 `explosion.rs`，不在 `state.rs` |

## 6. typed 化建议（供实现卡，不是本卡结论）

| 字节 | 建议类型 | 理由 / 风险 |
|---|---|---|
| `hi`（偏移 7） | `u8`（0/1）或 `NameTable` 枚举 | 只有两个取值；写入点唯一（`GetEnemyType`），读点分散在绘制 |
| `subPixelY/X`（8/9） | `u8` 累加器 | 只做 8 位加法回绕，别做成 i8 |
| `accelY/X`（10/11） | **有符号**量 | 与 `adc` 累加进亚像素；符号性是 B 级（asm 未标 `.byte` 有符号），typed 时加一条单测固定「负加速度递减亚像素」 |
| `spare1C`（12） | `_reserved: u8`，断言恒 0（仅在测试里断言） | 原版注释 Unused + 运行恒 0；生产路径**不要**把它清 0（原版不清，Stage A 保真） |
| `jumpDsplcmnt`（13） | `u8`（幅值） | 符号由 `data1F` 决定，本身是幅值 |
| `type`（14） | `u8` + 每区参数表索引 | 是「表下标」不是区号；不同区同号不同义 |
| `data1F`（15） | 枚举（`0x00/0x40/0x80/0xC0`）+ 注释「解释随例程」 | 两个例程两套语义，别合成 bitflags |
| `EnExplosionsExtra`（`$6BB4-$6BD3`） | 8 B 结构，`status..hi` 全字段 | 已经是完整结构（无尾），可整体 typed |

## 7. 复现命令（逐条本机实跑）

```bash
# 1) 分段与槽归属（输出见 §1）
python3 tools/nes_map_check.py --list | grep -E '\$6A|\$6B'
grep -E " (EnsExtra|EnProjectilesExtra|EnExplosionsExtra|EnMotherBrainExtra|EnMellowHandlerExtra)$" \
  refs/m1disasm/out/M1_NES_NTSC.sym
```

```bash
# 2) 字段名与注释（输出见 §2；EnExtra 在 125-160 行，EnExplosionExtra 在 161-170 行）
sed -n '125,170p' refs/m1disasm/SRC/constants_ram.asm
```

```bash
# 3) 具名写点/读点（输出见 §2 各行）
grep -rn 'EnsExtra\.0\.\(hi\|subPixelY\|subPixelX\|accelY\|accelX\|spare1C\|jumpDsplcmnt\|type\|data1F\)' \
  refs/m1disasm/SRC/*.asm refs/m1disasm/SRC/*/*.asm | head -40
grep -rn 'spare1C' refs/m1disasm/SRC   # 只有 constants_ram.asm:139 的定义，零引用
```

```python
# 4) 运行期逐字节活跃度（输出见 §3）
import glob, pathlib, collections
nz=collections.Counter(); tot=0
for p in sorted(glob.glob('analysis/traces/*.ram.hex')):
    b=bytes.fromhex(pathlib.Path(p).read_text().strip())
    if len(b) < 2272: continue
    tot += 1
    ex=b[2048:2048+224]
    for i,v in enumerate(ex):
        if v: nz[i]+=1
print('files', tot)
print('explosion 6BB4..6BD3:', ' '.join(str(nz.get(0xC0+o,0)) for o in range(32)))
for slot in range(12):
    print(f'{0x6AF4+slot*16:04X}', ' '.join(str(nz.get(slot*16+o,0)) for o in range(16)))
```

```bash
# 5) Rust 对照（输出见 §5）
sed -n '133,152p;274,312p;934,975p' crates/game-core/src/enemy/state.rs
grep -rn "EnExplosionsExtra\|0x6BB4" crates/game-core/src   # 4 处，全在 enemy/explosion.rs
grep -n "EXTRA_BASE\|EXTRA_STRIDE\|mod explx\|Unmodeled" crates/game-core/src/enemy/explosion.rs
```

## 8. 假设区（C 级）与未覆盖

- **C1**：`accelY/accelX` 的有符号性由「与亚像素 `adc` 累加」推断（B 级偏 C）；未找 `EnemyIfMoveFailed` 的负值用例直证。
- **C2**：`subPixelY/X` 是否存在「超 8 位被丢弃的高位」语义未核（只有 8 位存储，看不出是否有意截断）。
- **未覆盖**：
  - EnExplosionsExtra 的 `status/hi` 采样未命中（爆炸 4 帧内结束），其取值域未实测。
  - EnMotherBrainExtra（`$6BD4`）/EnMellowHandlerExtra（`$6BE4`）在本卡窗口外，未取证。
  - 「弹体尾 9 B 恒 0」是 10 009 份 trace 的结论；未做「能否被某条罕见路径写入」的穷举证明。

## 9. 证据指针

| 事实 | 证据 |
|---|---|
| 13 段地址与槽归属 | `python3 tools/nes_map_check.py --list`；`refs/m1disasm/out/M1_NES_NTSC.sym` |
| `EnExtra` 9 个尾字段名与注释 | `refs/m1disasm/SRC/constants_ram.asm:125-160` |
| `EnExplosionExtra` 8 B 结构 | 同上 `:161-170`；`instanceof EnExplosionExtra 4`（`:1296`） |
| 尾字段读写点 | `prg7_engine.asm:8951/8934/10754/11281/12276-12279`；`areas_common.asm:449/462/479/481/526/528/848/899/902/927/932/973` |
| `spare1C` 零引用 | `grep -rn spare1C refs/m1disasm/SRC` → 仅定义处 |
| 运行期活跃度 | §3 脚本，10 009 份 `analysis/traces/*.ram.hex` |
| Rust typed 7 B 与尾债声明 | `enemy/state.rs:133-152`、`:274-312`、`:934-975` |
