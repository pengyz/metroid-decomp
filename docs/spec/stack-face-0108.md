# `$0108-$01FF` 栈/杂项 248 B：写点普查与 typed 可行性（B2-c 取证）

> 目标：给 `$0108-$01FF`（现全部 Undeclared）做写点普查，判定哪些能 typed、哪些只能 ephemeral。
> 只写本文件；不改 `crates/`、`data/status/`、判据。允许否定性结论。

## 0. 结论摘要

| 段 | 地址 | 长度 | 结论 | 判据 | 级 |
|---|---|---|---|---|---|
| A：具名变量 | `$0108-$010F` | 8 B | **可 typed**（8 个具名字段，见 §2） | `.enum $0106 export` 里逐个具名 + 各自有显式 `sta`/`lda`；运行期两字节（EndTimer）每帧在动 | A |
| B：空档 | `$0110-$01CF` | 192 B | **不可 typed**（无游戏语义）；建议声明为 `Unmodeled` 并注明「栈页空档」，或保持 Undeclared | 反汇编零显式访问；10 009 份 trace 里 **192 B 全 0**；栈最深只到 `$01D7`（41 B） | A |
| C：栈真身 | `$01D0-$01FF` | 48 B | **只能 ephemeral（栈）**，禁止 typed | 只由 `php/pha/jsr/rts` 隐式写；运行期活跃；Rust 无 6502 栈，不能作真相所有者 | A |

三点要一起看：

1. **页 1 不是纯栈页**：`$0106-$010F` 是游戏自己划走的 10 B 变量区（`ClearSamusStats` 会在开局清 `$0100-$010F` 共 16 B）。本卡范围从 `$0108` 起，是因为 `$0106/$0107` 是 `Health`（已 typed，属 `samus_vitals_face`）。
2. **`$0110-$01CF` 是"没被用到的空档"而不是"未发现的数组"**：反汇编里对 `$0110-$01FF` 的显式读写是 0 处（唯一带索引的页 1 写是 `ClearSamusStats` 的 `sta $0100,y`，y 最大 `$0F`）。这条是**否定式断言**，已按 §3 的命令逐条 grep 过。
3. **栈的实际用量很小**：10 009 份 trace 的页 1 非零字节最低到 `$01D7` ⇒ 观测到的最深栈 = `$01FF-$01D7+1 = 41 B`。这与 NMI 每帧固定压 4 B（`php/pha/txa/pha/tya/pha`）+ jsr 返回地址的量级一致。

## 1. 页 1 布局（把范围放回上下文）

| 地址 | 内容 | 证据 |
|---|---|---|
| `$0100-$0105` | 从未被触碰（10 009 份 trace 全 0） | §3 命令 2 的输出：`0100` 行前 6 列全 0 |
| `$0106-$0107` | `Health`（`dw`，低位 nibble 血量/高位 nibble 坦克） | `constants_ram.asm:826-829`；已 typed（`samus/state.rs:979-991`） |
| `$0108-$010F` | 本卡段 A：8 个具名变量 | `constants_ram.asm:831-841` |
| `$0110-$01CF` | 本卡段 B：空档 | §3 命令 2/3 |
| `$01D0-$01FF` | 本卡段 C：栈 | §3 命令 2；NMI 入口 `php/pha/...`（`prg7_engine.asm:233-241`） |

开局相关代码（A 级）：

- `Startup 07:C01A`：`dex / txs` 把 S 置为 `$FF`（`prg7_engine.asm:72-73`），S 指向栈页顶。
- `ClearSamusStats 07:C578`：`ldy #$0F / lda #$00` 循环 `sta $0100,y`、`dey`、`bpl` ⇒ 清 `$0100-$010F` 共 **16 B**（`prg7_engine.asm:1427-1439`）。它是页 1 里唯一的显式带索引写，且上界就是 `$010F`。

## 2. 段 A：`$0108-$010F` 逐字节表（8 B，可 typed）

字段名全部来自 `refs/m1disasm/SRC/constants_ram.asm:826-843`（`.enum $0106 export`）。「运行非零」= 10 009 份 `analysis/traces/*.ram.hex` 里该字节非零的帧数。

| 地址 | 字段 | 语义（原文要点） | 写点（asm） | 读点（asm） | Rust 现状 | 运行非零 | 级 |
|---|---|---|---|---|---|---|---|
| `$0108` | `MiniBossKillDelayFlag` | Kraid/Ridley 击杀后启动能力音乐与延迟 | `prg7:6051`（LDD75 结算）、`prg7:2044`（消费后清） | `prg7:2040` | `frame/tick.rs:676-678`（字节面） | 0 | A（语义）/ B（Rust 未 typed） |
| `$0109` | `PowerUpDelayFlag` | 拾取道具后启动能力音乐与延迟 | `prg7:5650`（拾取）、`prg7:2045`（清） | `prg7:2041` | `pickup/spawn.rs:28/289`、`frame/tick.rs:676-678` | 0 | A / B |
| `$010A` | `EndTimer`（低字节） | 逃生计时（两位 BCD） | `prg3:1421`（设置）、`prg3:1975`（递减） | `prg7` HUD 路径、`ui/hud.rs:47-53` | `world/room_draw.rs:1483`、`frame/tick.rs:545` | 9 831 | A |
| `$010B` | `EndTimer+1`（高字节） | 同上，`$FF` = 未启用 | `prg3:1422`、`prg3:1981` | `prg3` 判定、`ui/hud.rs:47` | `world/room_draw.rs:565/1483` | 9 831 | A |
| `$010C` | `EndTimerEnemyHi` | 逃生计时敌（高字节） | `prg3:1428`、`prg3:1517` | —（未见具名读） | `ram_map.rs:443` 常量 | 0 | A（写）/ C（读点未见） |
| `$010D` | `EndTimerEnemyIsEnabled` | `#00`=无 / `#01`=启用 TIME BOMB 敌 | `prg3:884`、`prg3:1425` | —（未见具名读） | `ram_map.rs:445` 常量 | 0 | A（写）/ C（读点未见） |
| `$010E` | `MissileToggle` | 0=弹 / 1=导弹 | `prg7:5483` | `prg7:2546/3836/4006` | `frame/tick.rs:649`、`frame/memory.rs:769` | 0 | A |
| `$010F` | `SamusHurt010F` | 原版注释 **「never read」**，取值随受击来源变化 | `prg7:10063/10521/10602/10621` | **0 处**（与注释一致） | `samus/handler.rs:289/586` **写它但标注为 `WalkSoundDelay`** | 9 717 | A（never read）/ B（Rust 命名可疑） |

两条要单独说的：

- **`$010F` 的 Rust 用法与本卡证据冲突（B 级，只登记不判错）**：反汇编里 `WalkSoundDelay` 在 `$53`（`constants_ram.asm:568`），`SetSamusRun 07:CC98` 写的是 `WalkSoundDelay`（`prg7:2824-2826`）；而 `$010F` 是只写不读的 `SamusHurt010F`。Rust `samus/handler.rs:289/586` 把 `0x010F` 当 `WalkSoundDelay` 读写。是否造成实际差需要 trace 对照（超出本卡边界）——但**命名与地址至少有一处是错的**，建议实现/清债卡核一次。
- `$010C/$010D` 的「读点未见」是**具名读**为空；它们可能被间接读（HUD 或逃生链），未穷举，标 C 级。

## 3. 段 B/C：`$0110-$01FF` 的写点普查（无显式写，只有栈）

### 3.1 显式访问：0 处（A 级否定式断言，附命令）

```bash
# 对 $0110-$01FF 的字面量访问：无输出
grep -rnE '\$01[1-9A-F][0-9A-F]' refs/m1disasm/SRC/*.asm
# 对 $0100 的索引访问：只有 ClearSamusStats 一处，y 上界 $0F
grep -rnE '\$0100[ ,][xXyY]|\$0100\b' refs/m1disasm/SRC/*.asm
```

实测：第一条 **无输出**；第二条命中 3 类、全部不是对 `$0110-$01FF` 的 RAM 访问：

- `prg7_engine.asm:1434 sta $0100,y` —— `ClearSamusStats` 的清零写，y 上界 `$0F`；
- `prg2_norfair.asm:123 .word $0100` —— 伤害表数据（`;Base damage caused by area enemies`），是立即数不是地址；
- `sound_engine.asm:2268…` 若干 `.word $0100,$0300,$0500,…` —— 音效指针表数据。

### 3.2 隐式写：栈（C 段唯一来源）

| 机制 | 位置 | 每次写几个字节 | 说明 |
|---|---|---|---|
| `txs` | `prg7_engine.asm:73`（Startup） | 只设 S，不写内存 | S=`$FF`，栈从 `$01FF` 向下 |
| NMI 入口保存 | `prg7_engine.asm:233-241`（`php/pha/txa/pha/tya/pha`） | **4 B/帧** | 服务例程入口保险，固定占 `$01FC-$01FF` 附近 |
| `jsr`/`rts` | 全仓（示例：`prg7:1434` 后的 `jsr BankSwitch`、`jsr InitBrinstarGFX`） | 2 B/次 | 返回地址两字节 |
| BRK/IRQ | 未使用（向量指向 RTS 桩） | — | 若有中断会再压 3 B，本项目未见 |
| `pha/pla/php/plp` | 例程内临时保存（如 `prg7:233-241`） | 1 B/次 | 成对、回弹 |

因此 `$0180-$01FF`（段 C 的高半）是**每帧都在写的活栈区**；`$0110-$017F`（段 B 高半）**没有任何写者**——除非栈深度超过 `$01FF-$017F = 128 B`。

### 3.3 运行期分布（A 级）

```python
# 输出见 §3.3 表
import glob, pathlib, collections
nz=collections.Counter(); tot=0
for p in sorted(glob.glob('analysis/traces/*.ram.hex')):
    b=bytes.fromhex(pathlib.Path(p).read_text().strip())
    if len(b) < 2272: continue
    tot += 1
    for i,v in enumerate(b[0x100:0x200]):
        if v: nz[i]+=1
print('files', tot)
for start in range(0,0x100,0x10):
    print(f'{0x100+start:04X}', ' '.join(f'{nz.get(start+o,0):5d}' for o in range(16)))
```

实测（`files 10009`）：

```
0100     0     0     0     0     0     0  5037  9674     0     0  9831  9831     0     0     0  9717
0110     0     0     0     0     0     0     0     0     0     0     0     0     0     0     0     0
0120     0  ...（$0110-$01CF 全部 0，共 192 B）
01D0     0     0     0     0     0     0     0     2     2   539   539   645   645   645   645   856
01E0   856  2510  2524  8353  8019  8544  8561  9651  9612  9654  9422  9914  9720  9553  9767 10000
01F0  9752  9991  9893  9991 10003  9982 10006 10002 10008  9637  9994  9690 10009 10009 10009 10009
```

读法：

- `$0110-$01CF` **192 B 全 0**（10 009 帧 × 192 B 无一次非零）⇒ 观测意义上「从未被写」。
- `$01D7` 起开始出现非零（该地址有 2/10 009 份），`$01F4/$01FC/$01FD/$01FE/$01FF` 接近 100% 活跃 ⇒ 栈顶固定活跃区。
- 最深的单次观测：逐份扫页 1 的**最低非零地址**（排除变量区 `$0106-$010F`），最小值 **`$01D7`**（2 份：`analysis/traces/tmp-anyp-13k_f09000_c9000.ram.hex` 等）⇒ 深度 = `$01FF-$01D7+1` = **41 B**。

> 结论的强度：这是「观测到的最大深度 41 B」，不是「证明不可能更深」。静态上 NMI + 主循环 + 最深调用链的 jsr 层数没有穷举，所以段 B 的「永不写」标 **A（观测）/ B（静态推断）** 两档。

## 4. typed 可行性结论（给实现卡）

### 4.1 段 A（`$0108-$010F`，8 B）：可 typed

- 建议切分：`MiniBossKillDelayFlag`(u8)、`PowerUpDelayFlag`(u8)、`EndTimer`(u16 BCD，高低字节分别可读)、`EndTimerEnemyHi`(u8)、`EndTimerEnemyIsEnabled`(u8)、`MissileToggle`(u8，0/1)、`SamusHurt010F`(u8，write-only 标记)。
- 命名来源：全部来自 `constants_ram.asm` 的具名，**不需要 `_todo_`**。
- 账目 delta（用 `nes_map_check --write` 之前预期）：`delta_owner = +8`、`delta_view = 0`、`delta_unmodeled = 0`、`delta_undeclared = -8` ⇒ 恒等式 `delta_owner + delta_view == -(delta_unmodeled + delta_undeclared)` 即 `8 == 8` 成立。
- 注意：`$0108/$0109/$010C/$010D/$010E` 在本批 trace 里非零帧数为 0（不出现战斗/拾取/导弹切换的采集），**不能因此判「死字节」**——它们有明确写点。

### 4.2 段 B（`$0110-$01CF`，192 B）：不可 typed，建议 `Unmodeled` 声明

- 没有游戏语义（零显式访问），也没有稳定内容（全 0）。
- typed 需要「谁是真相」：这里没有任何 Rust 侧唯一真相可投影，所以不能 `Owner`。
- 建议：若要让账目更诚实，把 `$0110-$01FF` 声明成 `Unmodeled`，note 写「6502 栈页：段 B 为未用空档，段 C 为活栈；Rust 无栈，只保证不写」。delta：`delta_unmodeled = +240`、`delta_undeclared = -240`，恒等式同为成立。
- **不建议**把它按 16 B 切块硬命名（会造伪语义）。

### 4.3 段 C（`$01D0-$01FF`，48 B）：ephemeral，禁止 typed

- 内容每帧变（NMI 保存区 + jsr 返回地址），语义是「CPU 调用栈」，不是游戏状态。
- Rust 侧没有 6502 栈；把这段声明成 `Owner` 会要求 Rust 维护一个它根本不需要的栈，且必然与 oracle trace 的字节不同（字节面 diff 会爆）。
- 结论：typed 不可行；`Unmodeled` + note 即可。

## 5. Rust 侧现状对照

| 地址 | Rust | 状态 |
|---|---|---|
| `$0106/$0107` | `samus/state.rs:979-991` 的 `set_health/health`（经 `samus_vitals_face`） | 已 typed（本卡范围外，仅作边界说明） |
| `$0108/$0109` | `frame/tick.rs:676-678`（读非零即清）、`pickup/spawn.rs:28/289` | 字节面，**未 typed**（有具名常量缺失：`ram_map` 只有 `MISSILE_TOGGLE`/`END_TIMER*`） |
| `$010A/$010B` | `ram_map.rs:100 END_TIMER`；`ui/hud.rs:47-53`、`world/room_draw.rs:565/1483`、`frame/tick.rs:545` | 字节面 + 具名常量 |
| `$010C/$010D` | `ram_map.rs:443/445` 常量 | 常量存在、未见使用点（与 asm「读点未见」一致） |
| `$010E` | `ram_map.rs:142`、`frame/tick.rs:649`、`frame/memory.rs:769-772` | 字节面 + 具名常量 |
| `$010F` | `ram_map.rs:203 SAMUS_HURT_010F` 常量已存在；但 `samus/handler.rs:289/586` 当 `WalkSoundDelay` 读写 | **地址/命名冲突**（见 §2 末），B 级 |
| `$0110-$01FF` | 无任何读写；`nesmap` 里是 `Undeclared` | 与「未用 + 栈」一致 |

## 6. 复现命令（逐条本机实跑）

```bash
# 1) 具名变量表与字段边界
sed -n '824,843p' refs/m1disasm/SRC/constants_ram.asm
# 2) 开局清内存与 S 初始化
sed -n '66,75p;1427,1439p' refs/m1disasm/SRC/prg7_engine.asm
# 3) 显式写点（段 A）
grep -rn 'sta \(MiniBossKillDelayFlag\|PowerUpDelayFlag\|EndTimer\|EndTimerEnemyHi\|EndTimerEnemyIsEnabled\|MissileToggle\|SamusHurt010F\)' refs/m1disasm/SRC/*.asm
# 4) 否定式断言：$0110-$01FF 零显式访问 + 唯一页 1 索引写
grep -rnE '\$01[1-9A-F][0-9A-F]' refs/m1disasm/SRC/*.asm
grep -rnE '\$0100[ ,][xXyY]|\$0100\b' refs/m1disasm/SRC/*.asm
# 5) NMI 每帧压栈
sed -n '233,244p' refs/m1disasm/SRC/prg7_engine.asm
```

`analysis/traces` 的页 1 统计见 §3.3 的 python 片段（10 009 份，可重跑）。

## 7. 假设区（C 级）与推翻判据

- **C1**：`$010C EndTimerEnemyHi` 与 `$010D EndTimerEnemyIsEnabled` 的读点未见，假设它们只被 HUD/逃生链间接读。
  推翻判据：在 asm 里找到除具名写外的 `lda/bit/cmp` 通过指针/索引读到 `$010C/$010D` 的路径，或运行期用读地址 hook 抓到读点。
- **C2**：`$010F` 在 Rust 被当 `WalkSoundDelay` 是历史误名。
  推翻判据：若 `handler.rs` 那条路径的实际效果与 asm 的 `$53` 路径逐帧等价（trace 对照 0 差），则说明地址错但行为被别的字段抵消，需要另找真因。
- **C3**：段 B 的「永不写」只在 41 B 深度内成立；要压到 `$0110` 需要栈深 ≥ 240 B（`$01FF-$0110+1`），当前观测差 199 B。
  推翻判据：构造/找到 jsr 嵌套 >128 B 的场景，或直接用 SP 读数（需 emu 支持 `getState()["cpu.sp"]`）测最大深度。

## 8. 未覆盖

- 未取 SP 寄存器值（当前 trace 格式无 `sp` 字段），栈深由「内存非零最低地址」间接估算。
- 未穷举静态调用链深度（只给了 NMI 固定 4 B + jsr 2 B/次的量级）。
- 未核对 `$010C/$010D` 的所有间接读者（只核了具名引用）。
- `$0100-$0105`（本卡范围外）恒 0 的事实已记录，但未判断原版是否曾打算把它当变量区。

## 9. 证据指针

| 事实 | 证据 |
|---|---|
| 页 1 变量枚举 `$0106-$010F` | `refs/m1disasm/SRC/constants_ram.asm:826-843` |
| `WalkSoundDelay=$53` | `constants_ram.asm:568`；写点 `prg7_engine.asm:2826` |
| S 初始化 `$FF`、开局清 `$0100-$010F` | `prg7_engine.asm:72-73`、`:1427-1439` |
| 段 A 写点 | `prg7:2044/2045/5483/5650/6051/10063/10521/10602/10621`；`prg3:884/1421/1422/1425/1428/1517/1975/1981` |
| NMI 每帧压 4 B | `prg7_engine.asm:233-241` |
| 运行期页 1 分布（10 009 份） | §3.3 python 片段；`analysis/traces/*.ram.hex` |
| `$0110-$01FF` 零显式访问 | §6 命令 4（两条 grep） |
| Rust 落点 | `frame/tick.rs:545/649/676-678`、`ui/hud.rs:47-53`、`world/room_draw.rs:565/1483`、`ram_map.rs:100/142/443/445`、`samus/handler.rs:289/586`、`pickup/spawn.rs:28/289` |
