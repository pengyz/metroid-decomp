# 按区（bank）资产：基址、表长与危险清单

> 起因：敌人 AI 派发改为按区表之后（`d2be3bf1`），复核发现**动画三表仍只从 bank1 提取**。
> 本文是「先取证」的产物：只列地址与长度，全部可用文末命令从 ROM 自证；不含实现。

## 0. 结论摘要

| 级 | 事实 | 后果 |
|---|---|---|
| P0 | `EnAnimTable` / `EnFramePtrTable1,2` / `EnPlacePtrTable` 的**基址每区不同**，而 `crates/game-data/src/extract.rs` 的 `EN_ANIM_TABLE=0x9D6A` / `EN_FRAME_PTRS=0x9DE0` / `EN_PLACE_PTRS=0x9F0E` 是 bank1 硬编码 | 非 Brinstar 敌人的 `animIndex` 去查了 bank1 的表：Ridley 的 Dessgeega 取 `0x42`，`bank1[0x42]=0xFF` ⇒ 16 次原地回绕，落 `unported!("bank07:E0BC-wrap")` |
| P0 | 五区 anim 表**内容差异极大**（与 bank1 同字节数：b2 26/118、b3 8/118、b4 77/118、b5 20/118） | 不是「基址错、内容同」——照抄 bank1 会得到完全不同的动画序列 |
| P1 | `crates/game-data/src/assets.rs:249` 的 `KEYS[(area & 0x0F) % 5][idx]` 与 `structures()` 的 `_ => RI_STRUCTS` | area≥5 时**静默回绕**到别区数据（不报错、不记账） |
| P1 | `crates/game-core/src/enemy/common.rs:214` 的 `ENEMY_ACTIVE_ANIM_INDEX` 是单常量（bank1 `$965B`） | 非 Brinstar 的激活动画索引错区 |
| P2 | 旧注「Brinstar place 表 14 项」**是错的**，实为 **16 项**（前 2 项为跨区共享） | 现有 `EN_PLACE_LEN=14` 少读 2 项 |

## 1. 五区基址与表长（A 级）

`refs/m1disasm/SRC/<area>/enemy_sprite_data.asm` 的标签地址，与 ROM 逐字节一致：

| area | bank | `EnAnimTable` | anim 长 | `EnFramePtrTable1` | `EnFramePtrTable2` | 帧指针项数 | `EnPlacePtrTable` | place 项数 |
|---|---|---|---|---|---|---|---|---|
| Brinstar | 1 | `01:9D6A` | 118 | `01:9DE0` | `01:9EE0` | 151 | `01:9F0E` | **16** |
| Norfair | 2 | `02:9BDA` | 138 | `02:9C64` | `02:9D64` | 138 | `02:9D78` | 14 |
| Tourian | 3 | `03:A406` | 38 | `03:A42C` | `03:A52C` | 138 | `03:A540` | 16 |
| Kraid | 4 | `04:9C86` | 113 | `04:9CF7` | `04:9DF7` | 151 | `04:9E25` | 16 |
| Ridley | 5 | `05:9B85` | 107 | `05:9BF0` | `05:9CF0` | 138 | `05:9D04` | 15 |

三条机械关系（五区全部成立，可用作提取侧的自检）：

- `anim 长 = EnFramePtrTable1 - EnAnimTable`；
- `EnFramePtrTable2 = EnFramePtrTable1 + 256`（**表 1 恒 128 项**）；
- `帧指针项数 = (EnPlacePtrTable - EnFramePtrTable1) / 2`、`place 项数 = (place 表首指针 - EnPlacePtrTable) / 2`。

## 2. 跨区共享项（这决定了索引空间，不能只看「每区多少项」）

两个指针表都不是「全区私有」，各以**共享项开头**：

- `EnFramePtrTable2` 的**前 2 项**是 `PtrTableEntryArea`：`EnFrame_MissilePickup` / `EnFrame_SmallEnergyPickup`（bank1 实测 `A1F7` / `A1FF`），其余为 `PtrTableEntry` 区内帧。
- `EnPlacePtrTable` 的**前 2 项**同为 `PtrTableEntryArea`：`EnPlace0` / `EnPlace1`，其后才是区内 place。

⇒ 索引空间统一为 `0 .. 项数-1`，其中 `0/1` 两槽是共享语义；**每区的项数不同，但前缀共享**。旧注把 Brinstar 记成 14 项，是因为只数了 `PtrTableEntry`、漏了 2 条 `PtrTableEntryArea`。

## 3. 内容差异（证明「照抄 bank1」不成立）

以 bank1 的 118 B anim 表为基准，与其余四区同长度前缀逐字节比较：

| bank | 相同字节 / 118 |
|---|---|
| 2 Norfair | 26 |
| 3 Tourian | 8 |
| 4 Kraid | 77 |
| 5 Ridley | 20 |

帧/place 表同理（表 1 前 128 项多为区内帧指针，几乎全不同）。因此按区提取是**必须**的，不是优化。

## 4. 危险资产清单（按地址区，不按标签名）

判据：**同一 CPU 地址在不同 bank 的内容不同**。按标签名扫描会漏项——bank4 把 `AreaMapPosX/Y` 改名成 `AreaSamusMapPosX/AreaSamusMapPosY`（`prg4_kraid.asm:136-138`），地址仍是 `04:95D7/$95D8`。

机械分母（地址区，五 bank 全覆盖）：

- area header：`$95C3-$95E5`
- 敌表区：`$960B-$97D5`
- 各 bank 的 `EnAnimTable` / `EnFramePtrTable1,2` / `EnPlacePtrTable` 及其后的帧/place 数据（见 §1）

在该分母下共 **43 项危险资产**，代表性条目：

`AreaMusicFlag`、`AreaEnemyDamage`、`AreaItemRoomNumbers`、`AreaSamusY`、`AreaPaletteToggle`、`AreaMellowAnimIndex`、`AreaRoutine`、`ChooseEnemyAIRoutine`、`EnemyDeathAnimIndex`、`EnemyHealthTbl`、`EnemyRestingAnimIndex`、`EnemyActiveAnimIndex`、`L968B`、`EnemyInitDelayTbl`、`EnDistanceToSamusThreshold`、`EnSpeedXTable`、`EnSpeedYTable`、`EnAccelXTable`、`EnAccelYTable`、`L977B`、`EnProjectile*`、`EnemyMovementChoices`、`EnAnimTable`、`EnFramePtrTable1`、`EnFramePtrTable2`、`EnPlacePtrTable`。

另有 703 组跨区 `include` 的表**内容相同**（`enemies/*.asm`、`areas_common.asm` 等），不需要按区复制——这也是为什么段键用 `(area, key)` 而不是 `(bank, offset)`。

## 5. 访问器现状（迁移前基线）

| 资产 | 现状 | 位置 |
|---|---|---|
| 敌九表（health/force/facing/…） | 已按区（`area.*.en_*`，各 5 段） | `crates/game-data/src/assets.rs` |
| 敌死亡动画 | 已按区（`area.*.en_death_anim`，`680507b9`） | 同上 |
| AI 派发表 | 已按区（`enemy/ai_table.rs`，`d2be3bf1`） | `crates/game-core/src/enemy/ai_table.rs` |
| **anim / 帧指针 / place** | **单表，全部 bank1** | `extract.rs::EN_*` + `assets::enemy_anim_table()` |
| 激活动画索引 | 单常量（bank1 `$965B`） | `enemy/common.rs:214` |
| 区索引取值 | `(area & 0x0F) % 5` 静默回绕 | `assets.rs:249` |

## 6. 迁移顺序（每步可机械验收）

1. `assets.rs` 的 `% 5` 回绕与 `_ => RI_STRUCTS` 改 fail-loud（**先修放大器**，否则后面每步都被它掩盖）。
2. anim / 帧指针 / place 按区入包（5 区 × 3 键，长度取 §1）；`extract.rs` 用一张 `(bank, anim_base, anim_len, fptr_base, fptr_len, place_base, place_cnt)` 参数表驱动。
3. `crates/game-core/src/enemy/anim.rs` 改为经按区访问器取表。
4. `enemy/common.rs` 的 `ENEMY_ACTIVE_ANIM_INDEX` 换按区。
5. 值层 newtype：`AnimIdx{area, off}`（`resolve()` 是唯一读表入口，**不提供 `From<u8>`**），防止「裸 u8 跨区使用」这种错误再次静默通过编译。
6. 新门 `bankvars`：按 §4 的地址区生成基线（地址 + 各 bank 字节 sha），校验差异项只减不增，并要求每个差异项在 Rust 有按区访问器、不存在单常量副本。

## 7. 自证命令（任何人可复跑）

```bash
cd /home/peng/workspace/metroid-decomp && python3 - <<'PY'
d = open('rom/Metroid (USA).nes','rb').read()[16:]          # 去掉 16 B iNES 头
def w(bank, addr):                                          # bank0-6 落 $8000 窗口
    o = bank*0x4000 + (addr-0x8000); return d[o] | (d[o+1] << 8)
areas = [('Br',1,0x9D6A,0x9DE0,0x9EE0,0x9F0E), ('No',2,0x9BDA,0x9C64,0x9D64,0x9D78),
         ('To',3,0xA406,0xA42C,0xA52C,0xA540), ('Kr',4,0x9C86,0x9CF7,0x9DF7,0x9E25),
         ('Ri',5,0x9B85,0x9BF0,0x9CF0,0x9D04)]
for n,b,an,f1,f2,pl in areas:
    cnt = (w(b,pl) - pl)//2                                  # place 项数 = (首指针 - 表基址)/2
    print(n, 'anim', f1-an, 'fptr', (pl-f1)//2, 'place', cnt, 'first_place_ptr', hex(w(b,pl)))
PY
```

期望输出：

```
Br anim 118 fptr 151 place 16 first_place_ptr 0x9f2e
No anim 138 fptr 138 place 14 first_place_ptr 0x9d94
To anim 38 fptr 138 place 16 first_place_ptr 0xa560
Kr anim 113 fptr 151 place 16 first_place_ptr 0x9e45
Ri anim 107 fptr 138 place 15 first_place_ptr 0x9d22
```

## 8. 边界（本文不主张的）

- 本文只证「地址与长度」，**不证**这些表在 Rust 侧应该怎么消费（消费规则仍以各 AI 例程的 asm 为准）。
- 危险资产清单的 43 项来自地址区扫描 + 逐项字节比对，**未逐项核对其 Rust 消费点**；迁移时应逐项复核（清单可能多列或漏列）。
- 「共享 703 组」的判定基于内容相同，不代表语义相同；若将来发现某区对共享表有差异使用（如越界读进相邻区数据），须单独取证。
