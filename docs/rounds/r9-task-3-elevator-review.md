# r9 复核 · #3 第二批（电梯十态 + SamusElevator + 碰撞段，commit `1235e25d`）

verifier（证伪立场）· 2026-11-14 · 只读复核（未改文件，未跑 cargo test / gates / L4）

## verdict: fail

**P0：滚屏触发条件两个方向全反**（`crates/game-core/src/frame/elevator.rs` 的 `samus_elevator`）。
其余 1a–1i 九项等价（其中 1c 有条件），P2 三条非阻塞，覆盖缺口声明属实。

## P0 详情（A 级：asm + ROM 字节）

`refs/m1disasm/SRC/prg7_engine.asm:4168-4183`（`SamusElevator ; 07:D423`）：

| 方向 | asm | 语义 |
|---|---|---|
| 下行 | `lda Samus.y / sec / sbc ScrollY / cmp #$84 / bcc Lx057` | `bcc` 跳到 `Lx057` = **跳过** `jsr ScrollDown` ⇒ **scr_y ≥ $84 才滚** |
| 上行 | `... / cmp #$64 / bcs Lx060` | `bcs` 跳到 `Lx060` = **跳过** `jsr ScrollUp` ⇒ **scr_y < $64 才滚** |

修复前 Rust 写成 `if scr_y < 0x84 { scroll_down }` / `if scr_y >= 0x64 { scroll_up }`——两个比较符都反。

> 【2026-11-14 自纠】初版此处写「回绕段写在同一分支体内」是**错的**：asm `Lx057/Lx058` 是
> `jsr ScrollDown` 之后的**合流点**，滚与不滚两条路径都执行回绕；Rust 侧 `if` 花括号同样在
> `let mut y = …` 之前闭合，修复前后结构一致。真正的后果是：条件反了 ⇒ **该滚时不滚**
> ⇒ `ScrollY` 不推进而 `Samus.y` 照推，两者脱钩、回绕发生在错误的视觉位置上。起始 `Samus.y=$70`（< $84）即一上车就反向滚。

为什么没被发现：23 条电梯测**没有一条断言 $84/$64 触发**（`grep 0x84` 只命中 etype 断言），
且 L4 无电梯采集点 ⇒ 620 passed 与「逐行对 asm 等价」的声明同时成立，正说明这是盲区。

## 逐条判定

- **1a `ScrollXToCenter` 的 `bmi`**：等价。`sec` 紧邻 `sbc ScrollX` ⇒ 结果 = (x−ScrollX) mod 256、N=bit7；
  Rust `wrapping_sub & 0x80 != 0` 同义（含 d=0 ⇒ ScrollLeft 一致）。`ScrollX==0` 分支三写等价。
- **1b 目的区映射**：等价（上行 0/1、下行 `etype&$0F`，末尾 `|$10` 传 `IsEngineRunning`）。
- **1c `PaletteToggle eor` 常量**：**有条件等价**——`eor #(_id_Palette00+1)~(_id_Palette05+1)` 只有
  在「Palette00..05 表序 id = 0..5」前提下才等于 7；asm 文本无法唯一反解 ⇒ 须注释该前提。
- **1d Tourian 判据**：等价，但 `cpy #$12 / bcc` 实际覆盖 **InArea $12–$14 三区**（Kraid/Tourian/Ridley），
  我原注释写「Tourian」语义偏窄。
- **1e 两次等 NMI**：等价（`WaitNMIPass_` 在 `SelectSamusPalette` 前；`ScreenOn` 自带一次）。
- **1f `ScreenOn` 用 deferred 旗**：等价（与仓库既有暗窗约定一致）。
- **1g 结局径五写**：等价（含提前 return）。
- **2 `ElevatorStop` 的 `bmi` + `sta MirrorCntrl`**：等价（ToggleScroll 只在 bit7 清后执行）。
- **3 `check_collision_of_slots`（$F1FA）**：等价。竖滚房 `(xh*240+xy)-(yh*240+yy)` 的 256→240 补偿
  与取负顺序对称；`dist_hi = $04 | (dy<0)<<1 | (dx<0)` 与 `asl` 后形状一致；`|dy|>=$100 ⇒ 无重叠` 等价
  （P2 边界：真值 ∈[256,495] 时 8 位幅值回卷，仅 radius∈(208,255] 才分叉，不可达）。
- **4 `is_samus_on_top_effect`（$D9BA）**：等价（`dist_y+1==radius` 且不写 isHit；否则
  `isHit=(old&$38)|dist_hi|$40`）。
- **5 `destroy_enemies`**：等价，且「原版 off-by-one 漏 $DF」声明**正确**
  （`SoundRoutineTablePtr dw ;$E0` ⇒ 常数 $48）。
- **6 分支覆盖**：等价（除 P0）；`Lx062` 的 `sty Samus.speedY` 在 `cmp #$05/$07` 之前。
- **7 测试独立性**：**弱**——状态机骨架与写点覆盖尚可，但阈值条件、y 回绕与 ToggleSamusHi 的耦合零断言。
- **8 覆盖声明**：属实（`cases/*.txt` grep `elevator|电梯` 为空）。

## P2 建议（不阻塞）

1. `const SCRN_VX: u8 = 0`＋`SCRN_VX/2+128` 值对（128）但极脆：任何人「修正」SCRN_VX=256 就溢出
   ⇒ 直接写 `0x80`。
2. `Palette00+1=1` / `PALETTE_TOGGLE_XOR=7` 的耦合前提未注明。
3. `in_area >= 0x12` 注释应写「Kraid/Tourian/Ridley 三区」。

## 覆盖缺口（verifier 独立认证）

`SamusDoor`（#6）与电梯十态（#3）**均无任何 L4 采集点走到**：唯一门案
`case-002-door-blue-interaction` 的 8 个采集点两侧 `$56/$59/$58/$4E` 全 0、`$0300` 序列
`14/16/16/17/18/FF/00/00` 从未为 5；电梯则连 `Elevator.status≠0` 都未出现。
故两条链当前**只有单测证据、零对拍证据**——单测只证明「函数按我读的 asm 办事」，
不证明「我读的 asm 与 ROM 行为一致」，而 #6 的基址错与本轮的比较符反，恰是这类错的两次实例。

## 处置（lead，commit 随附）

1. P0 已修：下行 `scr_y >= 0x84` ⇒ `scroll_down`、上行 `scr_y < 0x64` ⇒ `scroll_up`。
2. 新增 4 条阈值边界断言（`$84` 必滚 / `$83` 不滚 / `$63` 必滚 / `$64` 不滚），
   并做**变异验证**：把比较符改回反向 ⇒ 两条新测当场红（`scr_y==$84 ⇒ 必须 ScrollDown` /
   `scr_y<$64 ⇒ 必须 ScrollUp` 双双失败），恢复后 25/25 绿。
3. P2 三条全收（`SCRN_VX` 常量删除、xor 前提注释、三区语义注释）。
4. 覆盖缺口按 #6 三步形态写进 `docs/spec/subsystem-completeness-audit.md` 的 #3 更正段，
   #3 状态由「已实现」改为「**已实现 / 待差分验证**」。
