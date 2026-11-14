# 敌出生 data05：一处补偿性错误（未收口，回退留档）

> 2026-11-14。起因：A 重锚后 `case-jumpledge-probe` 剩余 6 格失配，
> 逐帧取证把首分歧定位到「敌出生帧 f928」的 `data05`。本档记录
> **A 级取证结论**、**改动为何不能单独落地**，以及**收口条件**。

## 1. A 级事实（ROM 逐字节 + 逐帧取证）

`GetEnemyData ; 07:EB06` 全径（ROM `$EB06-$EB99`）的 `sta abs,x` 只有五处：

| 目标 | 含义 | ROM |
|---|---|---|
| `$040F` | `specialAttribs`（bit7 tough / bit6 mini boss） | `EB2B: 9D 0F 04` |
| `$0400` / `$0401` | `y` / `x` | `EB52` / `EB5B` |
| `$0404` | `isHit`（清 0） | `EB65` |
| `$6AF4` / `$6AFB` | `EnsExtra.status` / `EnsExtra.hi` | `EB60` / `EB6B` |

**没有 `$0408`** ⇒ 出生径不写 `movementIndex`（该字段只在
`UpdateEnemy_Resting_TryBecomingActive ; 07:F7BA` 经
`EnemyMovementChoiceOffset` 随机取）。

`data05` 的出生形是 `asl $0405,x`（`EB7E: 1E 05 04`）紧跟
`jsr InitEnemyFacingDirectionAxisAndDelay ; 07:FB7B`
（`20 B0 80` / `7E 05 04`：`ror $0405,x`）。`LoadTableAt977B ; 00:80B0`
= `lda L977B,y / asl` ⇒ `ror` 搬进 bit7 的是 **L977B 原字节的 bit7**。
净效果：

```
data05' = (L977B[type].bit7 << 7) | (data05 & 0x7F)
```

即 **bit0 保持旧值**，出生径没有任何「按滚动方向置 bit0」的写点。

Brinstar 的 `L977B`（`01:977B`，bank1 平铺 `prg[0x577B]`）实测
`64 6C 21 01 04 00 4C 40 04 00 00 40 40 00 00 00` —— 前 16 项 **bit7 全 0**。

## 2. 现场：首分歧帧 f928（`case-jumpledge-probe`）

| | `data05` | `+8` |
|---|---|---|
| 金腿（emu） | `0x00` | `0x00` |
| 旧 rust | `0x01` | `0x07` |

旧 rust 的两处来源：
- `crates/game-core/src/frame/room.rs::load_enemy` 里
  `let facing_bit = mem.read_wram(ex) & mem.read_ram(SCROLL_DIR) & 0x01;`
  再 `| facing_bit` 进 **bit0** —— 出生帧 `status=1`、`ScrollDir=3` ⇒ 恒 1；
- `mov_init = [0x01,0x03,0x05,0x06,0x07,…]` 播种写 `en + 0x08`。

## 3. 按 asm 改之后：赢了 3 格、输了 3 格（ratchet FAIL）

改动（已实施并逐案验证，随后回退）：`data05' = (旧值 & 0x7F) | (L977B[type].bit7<<7)`、
删除 `+8` 播种。

| 用例 | 基线 | 改动后 |
|---|---|---|
| `case-jumpledge-probe` 1240:land | diff 208 | **match** |
| `case-jumpledge-probe` 1260:land-settled | diff 312 | **match** |
| `case-scrollseam-probe` 1360:end | diff 126 | **match** |
| `case-002-door-blue-interaction` 1080:at-door | match | **diff 252** |
| `case-scrollseam-probe` 1070:scroll-r150 | match | **diff 181** |
| `case-scrollseam-probe` 1240:scroll-l40 | match | **diff 184** |

`python3 gates/cli.py l4` ⇒ `L4 逐采集点 ratchet 回退 3 处`（FAIL）。

## 4. 为什么输掉那 3 格：这是补偿，不是等价

`case-002` f1080 的 `En2` 实测：

| | `x` | `data05` |
|---|---|---|
| 金腿 | `0x7E` | `0x1B`（bit0=1，朝左） |
| 改动后 rust | `0x9A` | `0x1A`（bit0=0，朝右） |

金腿的 bit0 是**运行期**被置上的——`EnemyForceSpeedTowardsSamus` 的
**x 轴力**分支（`07:F6DF`）按 `Samus.x` 与 `En.x` 的比较把朝向写进 bit0。
而 rust 侧该分支是**占位臂**：

```rust
if attribs & 0x80 != 0 {
    // 【占位臂记账】x 轴力（bit7=1 型）F6DF 深件
    crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0xF6DF));
    return;
}
```

`crates/game-core/src/frame/mainloop.rs::enemy_force_speed`。

于是旧 rust 的「出生置 bit0 = status & ScrollDir & 1」在
`ScrollDir` 为奇数（横滚房绝大多数情形）时**恰好**与「朝左」同形，
把缺失的 x 轴力掩盖了——与 RFC 历史上 `+6 FC 补偿`「截取缝补偿」
「走表双调补偿」同型：**补偿巧合假合**。

## 5. 收口条件（按此顺序）

1. 端口 `EnemyForceSpeedTowardsSamus` 的 **x 轴力**分支（`07:F6DF` 起，
   含 `data05` bit0 的写面），使其在 `attribs & $80` 时也真正施力；
2. 同时删 `load_enemy` 里的 `facing_bit` 注入口、并删 `+8` 播种；
3. 验收：`case-002` / `case-scrollseam-probe` / `case-jumpledge-probe`
   三案逐格不劣于基线（ratchet `match` 只增不减）；
4. 只有在 3 通过后才 `baseline_update.py l4` 上调。

在此之前，**保留补偿**是有意选择：它是当前唯一能让上述三格 match 的
行为，但**不是** asm 等价——不得在文档/台账里把它写成「已实现」。

## 6. 复现命令

```
python3 tools/l4_diff_run.py align case-jumpledge-probe   # 需 case 内有 watch 列
python3 tools/l4_diff_run.py run  case-jumpledge-probe
python3 gates/cli.py l4
```

`ram.hex` 布局：单行扁平十六进制 2176 B = `$0000-$07FF`(2048) + `$6AF4-$6B73`(128)。
L977B 原表：`python3 -c "d=open('rom/Metroid (USA).nes','rb').read()[16:];
print(' '.join('%02X'%b for b in d[0x4000+0x177B:0x4000+0x177B+16]))"`。
