# 敌移动指针表在资产包里只有一半（A 级，待修）

**结论**：`EnemyMovementPtrs`（每区 `$96DB` 起）在 ROM 里是 **36 个 `u16` = 72 B**
（18 个索引 × 右/左两个指针），而 `assets/metroid.pack` 的五个
`Area{Br,No,To,Kr,Ri}EnMovPtrs` 段只有 **36 B**（= 9 个索引）。
索引 9..17 的指针缺失 ⇒ `common::enemy_get_delta_xy` 走回落串。

发现于 Kraid 家族端口（`enemy/kraid.rs`）：Kraid 的 Lint/Nail 用
`EnemyMovementChoices` 索引 `0x0E..0x11`，全部落在缺失区，实测会回落到
Skree 串 `$97F5`（bank1）/ `$97EF`（bank4）——即「行为看起来在动，但动的是别人的串」。

## 1. ROM 侧证据（可复现）

读法：`prg` 偏移 = `bank*0x4000 + (addr-0x8000)`，`rom` 去掉前 16 B iNES 头。

```
python3 -c "
rom=open('rom/Metroid (USA).nes','rb').read()[16:][:131072]
def bo(b,a): return b*0x4000+(a-0x8000)
for bank,name in [(1,'Br'),(2,'No'),(3,'To'),(4,'Kr'),(5,'Ri')]:
    base=bo(bank,0x96DB)
    vals=[int.from_bytes(rom[base+i*2:base+i*2+2],'little') for i in range(40)]
    print(name, [hex(v) for v in vals[32:40]])
"
```

五个 bank 的 `idx 32..35` 全是合法指针、`idx 36..39` 全 `$0000`：

| 区 | bank | idx 32..35 |
|---|---|---|
| Brinstar | 1 | `9909 9920 9937 994e` |
| Norfair | 2 | `97ed 97ed 97ed 97ed` |
| Tourian | 3 | `97d5 97d5 97d5 97d5` |
| Kraid | 4 | `986d 9884 989b 98b2` |
| Ridley | 5 | `97f3 97f3 97f3 97f3` |

⇒ 表长 = 36 项（72 B），边界是紧接其后的 `$0000` 填充，不是 36 B。

## 2. 包侧证据

`pack-tool list` / 直接解包：`BrEnMovPtrs`、`NoEnMovPtrs`、`ToEnMovPtrs`、
`KrEnMovPtrs`、`RiEnMovPtrs` 五段长度全为 **36**。

`crates/game-data/src/extract.rs` 的对应段按 `EN_MOV_PTRS_LEN = 36` 提取
（历史注释记「EnemyMovementPtrs 真 36 B，旧读 138 项越界」——那句结论的
「36 B」是**字节数写成了项数**：真值是 36 **项** = 72 **字节**）。

## 3. 影响面（未测，需 L4 验）

`crates/game-core/src/enemy/common.rs::enemy_get_delta_xy` 用
`enemy_movement_ptrs(area)` 取表、按 `index*2 + facing` 索引 `u16`。
36 B ⇒ 只有索引 0..8 可解析；索引 9..17 取到越界/填充值，实现里回落到固定串。
Brinstar 的 `EnemyMovementChoices` 有 15 项（`AreaBrEnMovChoices` 30 B =
15 × `u16`？待核），其中有索引落在 9 以上的敌人（Skree/Crawler 家族的
高索引串）行为**当前就是错的**，只是矩阵里没有用例踩到可见像素差。

Kraid 家族不受影响：`enemy/kraid.rs` 用 asm 标签就地锚定的偏移量
（`KRAID_MOV_STR_OFFSETS`，值 = asm 标签 − 串区基址 `$97EF`，逐字节与 ROM
`$9839..$98C3` 核对），走的是与 `enemy_get_delta_xy` 相同的 SignMagSpeed 语义。

## 4. 修法（一张卡）

1. `crates/game-data/src/extract.rs`：`EN_MOV_PTRS` 长度 36 → 72（五段同长，
   加提取侧自检：段尾必须是 `0000`，否则拒出包）。
2. `pack-tool extract --force` + `verify`（162→183 段全过）。
3. `enemy/kraid.rs` 的 `KRAID_MOV_STR_OFFSETS` 可改为查表取值（行为应不变，
   改完用 23 条锚测对照）。
4. 跑 **L4 全矩阵**：这条会改所有高索引敌人的位移，预计有像素变化——
   若出现回退，说明原实现依赖了错误的回落串（补偿巧合），需逐格归因而不是回退修复。
5. `routine_inventory` / `nes_map` 不动（不是代码面变化）。

## 5. 已修（2026-11-20，同一轮）

`ENEMY_TABLE_LAYOUT` 的 `(0x96DB, 36)` 改为 `(0x96DB, 72)`，五个
`Area*EnMovPtrs` 段重建后 72 B；`pack-tool verify` 183 段全过；`kraid.rs` 新增
`hardcoded_offsets_agree_with_pack_pointer_table` 把就地锚定偏移与包内指针逐项钉住。
L4 全矩阵 **154/6/0 逐格不变**（无回归），说明现有 20 个用例没有走到
movIdx≥9 的可见路径——这是覆盖缺口，不是「改对了」的证明。

## 6. 原始判断：为什么当时不混进 Kraid 卡

Kraid 卡的范围是 AI 例程；修数据段会连带改变已有敌人的位移语义，必须有
L4 直裁（零容差）单独一轮。混在 AI 卡里提交等于把两类风险绑在一起。

Evidence: ROM 五 bank `$96DB` 直读 36×u16 表（`idx36+` 全 `$0000`）+ 包内五段
长度 36 B；`extract.rs` 历史注释把「36 项」写成「36 B」的误记已定位。
Confidence: 90（表长与截断是字节级事实；影响面的像素后果未实测）
