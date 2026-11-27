# f1301 判盒修复的 ROM 向量集（四组边界 + 16 位比较语义）

> 卡 #136（tsk_01M8DB1TG112MHV9N9R38A）。给 #137（`crates/game-core/src/weapon/projectile.rs` 判盒修复）准备可执行向量与 ROM 判据。
> 规格来源：`docs/spec/f1301-fix-plan.md` §4（本文逐条对齐，见 §4）；ROM 原文 `refs/m1disasm/SRC/prg7_engine.asm:10366-10491`。

## 0 术语与调用约定

- 判盒例程：`CheckCollisionOfXSlotAndYSlot ; 07:F1FA`。**X 槽 = 敌**（`GetEnemyXSlotPosition 07:F152`），**Y 槽 = 弹**（`GetObjectYSlotPosition 07:F186`）。
- 两个轴按房间滚动方向分工：入口 `lda #$02 / and ScrollDir / sta Temp03`，`Temp03 != 0`（`ScrollDir & 2`，即左/右滚）⇒ **横向房，主轴 X**；否则 **纵向房，主轴 Y**（`07:F207-07:F20C`、`07:F244-07:F249`）。
- 主轴比**高字节（nametable 位）**；副轴不比高字节，只用低位减法的借位当作 `dhi ∈ {0, -1}`（`07:F224-07:F229`、`07:F251-07:F255`）。
- `hi` 归一：`Temp0B = (XHi ^ PPUCTRL_ZP) & 1`、`Temp0A = (YHi ^ PPUCTRL_ZP) & 1`（`GetXSlotPosition_Common 07:F17F`、`GetYSlotPosition_Common 07:F193`）。`PPUCTRL_ZP = $00FF`（`constants_ram.asm:820`）。**不是 `$00FE`**（那是 `PPUMASK_ZP`，其 bit0 是 greyscale）。
- 半径和由调用方先算好：`GetRadiusSumsOfEnXSlotAndObjYSlot 07:F1BF`：`Temp04_YSlotRadiusY = EnsExtra.radiusY + Objects.radiusY`、`Temp05_YSlotRadiusX = EnsExtra.radiusX + Objects.radiusX`。
- 返回：**carry set = 不碰**，carry clear = 碰。两轴都通过后，最终判据是 `distX < radiusX`（`07:F25F-07:F261` 的 `cmp Temp05 / bcs @RTS`，相等亦算不碰）。

## 1 ROM 逐步（A 级）

```
Temp10_DistHi = $02                     ; 07:F1FC-07:F1FE（后续 asl 供 isHit 位用，非本卡范围）
Temp03 = $02 & ScrollDir
Diff = Yx - Yy (sec/sbc), borrow0       ; 07:F212-07:F217
if Temp03 == 0:                         ; 纵向房：主轴 Y 比 hi
    if (XHi^P) & 1 != (YHi^P) & 1:
        DiffHi = XHi16 - YHi16 - borrow0 ; 07:F262-07:F265（XHi16/YHi16 为 §0 归一后的 0/1）
        if DiffHi < 0: Negate16(Diff,DiffHi); Temp10++   ; 07:F268-07:F26E
        Diff = Diff - $10; if borrow: DiffHi--           ; 07:F21B-07:F221（240 行补偿）
    else:
        DiffHi = $00 - $00 - borrow0     ; 07:F224-07:F229
else:                                    ; 横向房：Y 为副轴
    DiffHi = $00 - $00 - borrow0
if DiffHi != 0: return no-collision      ; 07:F22E-07:F235
if Diff >= radiusY: return no-collision  ; 07:F237-07:F23B

Temp10 <<= 1
Diff = Xx - Xy (sec/sbc), borrow1        ; 07:F23F-07:F24A
if Temp03 != 0:                          ; 横向房：主轴 X 比 hi
    if (XHi^P)&1 != (YHi^P)&1:
        DiffHi = XHi16 - YHi16 - borrow1
        if DiffHi < 0: Negate16(Diff,DiffHi); Temp10++
    else:
        DiffHi = $00 - $00 - borrow1
else:                                    ; 纵向房：X 为副轴
    DiffHi = $00 - $00 - borrow1
if DiffHi != 0: return no-collision      ; 07:F256-07:F25C
if Diff >= radiusX: return no-collision  ; 07:F25D-07:F261
return collision
```

`NegateTemp00Temp01 ; 07:E449`：16 位取补——`lo = $00 - lo`、`hi = $00 - hi - borrow`。

## 2 修复实现契约（#137 用）

```rust
// 忠实 07:F1FA 的单轴差。a = X 槽（敌），b = Y 槽（弹）。
// compare_hi: 该轴是否为主轴（主轴 true，副轴 false）。
// compensate: 仅当「纵向房 + 主轴 Y + 归一后 hi 不相等」为 true。
// 返回 None = 该轴不碰（ROM 的 dhi != 0），Some(d) = 低字节距离，交给半径比较。
fn rom_axis_delta(a_pos: u8, a_hi: u8, b_pos: u8, b_hi: u8,
                  compare_hi: bool, compensate: bool, ppuctrl: u8) -> Option<u8> {
    let (lo, borrow0) = a_pos.overflowing_sub(b_pos);
    let na = (a_hi ^ ppuctrl) & 1;
    let nb = (b_hi ^ ppuctrl) & 1;
    let mut lo = lo;
    let mut dhi: i16 = if compare_hi { na as i16 - nb as i16 - borrow0 as i16 }
                       else { -(borrow0 as i16) };
    if dhi < 0 {
        let v = -(dhi * 256 + lo as i16);   // 16 位取补
        lo = (v & 0xFF) as u8;
        dhi = v >> 8;
    }
    if compensate {
        if lo < 0x10 { lo = lo.wrapping_sub(0x10); dhi -= 1; }
        else { lo -= 0x10; }
    }
    if dhi != 0 { None } else { Some(lo) }
}
```

判碰：

```rust
let dy = rom_axis_delta(e_y, e_hi, p_y, p_hi, v_room, v_room && hi_diff, ppuctrl);
let dx = rom_axis_delta(e_x, e_hi, p_x, p_hi, h_room, false, ppuctrl);
let hit = matches!((dx, dy), (Some(x), Some(y)) if x < radius_x && y < radius_y);
```

`v_room = ScrollDir & 2 == 0`，`h_room = !v_room`；`hi_diff = ((e_hi ^ ppuctrl) & 1) != ((p_hi ^ ppuctrl) & 1)`。**注意 `ppuctrl` 取 `$00FF`**。

## 3 向量集（可复算字节值）

输入为 `敌(pos,hi)` 与 `弹(pos,hi)`，`PPUCTRL_ZP` 取 0。半径和按 `radiusX = radiusY = 16` 判最终 hit（ROM 本身只用调用方传入的和，见 §0）。

| # | 场景 | 敌(a) | 弹(b) | 轴/compare_hi | lo | borrow | dhi_raw | 取补后(lo,hi) | 补偿 | 差结果 | 半径16 ⇒ hit |
|---|---|---|---|---|---|---|---|---|---|---|---|
| G1 | 横房主轴 X，跨 nametable | x=120 hi=0 | x=126 hi=1 | X / true | `$FA` | 1 | `$FE`(−2) | (`$06`,`$01`) | 无 | **None** | 不碰 |
| G2 | 横房主轴 X，同 hi，右边 10px | x=120 hi=0 | x=130 hi=0 | X / true | `$F6` | 1 | `$FF`(−1) | (`$0A`,`$00`) | 无 | **Some(10)** | 碰 |
| G3 | 反向（弹在左 10px） | x=130 hi=0 | x=120 hi=0 | X / true | `$0A` | 0 | `$00` | — | 无 | **Some(10)** | 碰 |
| G4 | 完全重合 | x=120 hi=0 | x=120 hi=0 | X / true | `$00` | 0 | `$00` | — | 无 | **Some(0)** | 碰（相等<16） |
| S1 | 副轴 Y，敌下 10px | y=110 hi=0 | y=100 hi=0 | Y / false | `$0A` | 0 | `$00` | — | 无 | **Some(10)** | 碰 |
| S2 | 副轴 Y，弹下 10px（借位） | y=100 hi=0 | y=110 hi=0 | Y / false | `$F6` | 1 | `$FF`(−1) | (`$0A`,`$00`) | 无 | **Some(10)** | 碰 |
| V1 | 纵房主轴 Y，跨 nametable | y=120 hi=0 | y=126 hi=1 | Y / true | `$FA` | 1 | `$FE`(−2) | (`$06`,`$01`) | `$06→$F6`,dhi 1→0 | **Some(246)** | 不碰（246≥16） |
| V2 | 纵房主轴 Y，跨屏回绕 | y=224 hi=0 | y=0 hi=1 | Y / true | `$E0` | 0 | `$FF`(−1) | (`$20`,`$00`) | `$20→$10` | **Some(16)** | 不碰（相等，`bcs`） |
| V3 | 纵房主轴 Y，跨 nametable 16px | y=120 hi=0 | y=136 hi=1 | Y / true | `$F0` | 1 | `$FE`(−2) | (`$10`,`$01`) | `$10→$00`,dhi 1 | **None** | 不碰 |

说明：

- G1 与 V1 是同一组坐标、不同房间方向：横房主轴 X 得 `dhi=1 ⇒ None`；纵房主轴 Y 得补偿后 `dhi=0, d=246`，最终由半径判据拒绝。**两者都不碰，但返回值不同**——#137 不能只看「是不是 None」。
- V2 的 16 恰好等于 radius16：ROM 用 `cmp / bcs`，**相等算不碰**，所以 hit=false；这是边界，测试必须盖。
- 副轴（S1/S2）只用借位，借位经 16 位取补后得到小正数；不会因为「弹的位置更大」就误判远。
- G3/G4 用无借位路径，验证 `dhi=0` 时返回低字节本身。

## 4 与 `docs/spec/f1301-fix-plan.md` §4 逐条对齐

| fix-plan §4 的表述 | 本文字节级依据 | 一致？ |
|---|---|---|
| `(lo,borrow0)=a.overflowing_sub(b)`；`dhi = ha-hb-borrow0`，其余 `-borrow0` | `07:F212-07:F217` + `07:F224-07:F229` / `07:F262-07:F265` | 一致 |
| `dhi<0` 走 16 位取补 `NegateTemp00Temp01` | `07:F268-07:F26E`、`07:E449` | 一致 |
| 240 行补偿只在「纵向房且 hi 不等」：`lo-=0x10`、借位 `dhi-=1` | `07:F21B-07:F221`（位于 hi 不等的分支内） | 一致 |
| `hi` 归一用 `PPUCTRL_ZP`，`$00FE` 是 PPUMASK | `constants_ram.asm:819-820`、`07:F17F`/`07:F193` | 一致（fix-plan 已是改后版本） |
| 主轴按 `ScrollDir & 2` 选 Y/X | `07:F207-07:F20C`、`07:F244-07:F249` | 一致 |
| 表：`120(0)/126(1)⇒None`、`120(0)/130(0)⇒Some(10)`、`130(0)/120(0)⇒Some(10)`、`0/0⇒Some(0)` | 本文件 §3 G1–G4 逐行复算 | 一致 |

**一处补充（不是推翻）**：fix-plan 的伪代码把 `compensate_240` 当入参；实现时必须只在实际走「主轴 + hi 不等」分支时传 `true`。若对同 hi 也补 240，`Some(10)` 会变成 `None`（同一房内 10px 不该不碰）。本文件 §2 用 `compensate` 显式表达这条。

## 5 复现

```bash
cd /home/peng/workspace/metroid-decomp
python3 - <<'PY'
def axis(apos, ahi, bpos, bhi, compare_hi, compensate, ppuctrl=0):
    lo=(apos-bpos)&0xFF; borrow=1 if apos<bpos else 0
    na=(ahi^ppuctrl)&1; nb=(bhi^ppuctrl)&1
    dhi=(na-nb-borrow) if compare_hi else (-borrow)
    if dhi<0:
        v=-(dhi*256+lo); lo=v&0xFF; dhi=v>>8
    if compensate:
        if lo<0x10: lo=(lo-0x10)&0xFF; dhi-=1
        else: lo-=0x10
    return (None,dhi) if dhi else (lo,dhi)
cases=[("G1",120,0,126,1,True,False),("G2",120,0,130,0,True,False),
       ("G3",130,0,120,0,True,False),("G4",120,0,120,0,True,False),
       ("S1",110,0,100,0,False,False),("S2",100,0,110,0,False,False),
       ("V1",120,0,126,1,True,True),("V2",224,0,0,1,True,True),
       ("V3",120,0,136,1,True,True)]
for n,a,ah,b,bh,ch,co in cases:
    lo,hi=axis(a,ah,b,bh,ch,co)
    print(f"{n}: delta={'None' if lo is None else lo} dhi={hi}")
PY
```

期望输出（本机实测）：

```
G1: delta=None dhi=1
G2: delta=10 dhi=0
G3: delta=10 dhi=0
G4: delta=0 dhi=0
S1: delta=10 dhi=0
S2: delta=10 dhi=0
V1: delta=246 dhi=0
V2: delta=16 dhi=0
V3: delta=None dhi=1
```

## 6 边界

- 本文件只给判盒差与半径判据；`Temp10_DistHi` 的 `isHit` 位（`asl` 与 `GetEnemyIsHitFlags 07:F332`）不在本卡范围，由 #137 按现有写点处理。
- 半径和由调用方从槽读取，本文的 `radius=16` 只是判最终 hit 的示例；真实值见 `EnsExtra.radiusX/Y` 与 `Objects.radiusX/Y`。
- 未覆盖：Samus 与敌、门与 Samus 的判盒（同一例程的不同调用者），其槽位与半径和来源不同，不能直接套用本表。
