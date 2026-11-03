# 走表双调 A=0 异常窗定谳（r14 阶段 0-1）

- 日期：2026-11-03
- 状态：**已证实**（DSH_SPRITE_TRACE 探针直证）
- 探针窗：f812-820（case-001 射击窗 + walkfcprobe 走表窗）

## 1. 定谳问题

r14 阶段 0-1：「走表双调 A=0 异常窗定谳——双调 A=0 执行径：regs.a 污染/旁路二选」。
即：f812-820 窗内观察到的 A=0 走表，是 ①set_samus_data 双调中 regs.a 被污染成 0，
还是 ②独立旁路（另径显式传 0）。

## 2. 探针实录（A 级——DSH_SPRITE_TRACE=1，探针窗移 812-820）

### 2.1 walkfcprobe f812（每帧 4 走表 3 draw 调用）

```
[ssd] n=812 FC=26 A=3                          ← set_samus_data 入口 regs.a=3
[uoa] n=812 ENTRY ai=2 ar=0 ad=0 a_in=3        ← 走表①（ssd 头 DC8F——a_in=3=regs.a）
[ado] n=812 pi=0 a_in=3                        ← draw①（ssd 尾——a_in=3=regs.a）
[uoa] n=812 ENTRY ai=3 ar=0 ad=3 a_in=3        ← 走表②（ado 内含 DC8F——双调第二走）
[ado] n=812 pi=64 a_in=0                       ← draw②（spawn 径——**a_in=0 显式**）
[uoa] n=812 ENTRY ai=3 ar=0 ad=2 a_in=0        ← 走表③（ado 内含——**A=0**）
[ado] n=812 pi=64 a_in=0                       ← draw③（spawn 第二 item 槽——a_in=0）
[uoa] n=812 ENTRY ai=3 ar=0 ad=1 a_in=0        ← 走表④（**A=0**）
```

### 2.2 case-001 f812-820（每帧 2 走表 1 draw——纯双调窗）

~~~
[ssd] n=812 FC=26 A=4
[uoa] n=812 ENTRY ai=8 ar=7 ad=1 a_in=4        ← 走表①（a_in=4=regs.a）
[atr] n=812 after Health A=4                   ← 链后 A 恒 4
[atr] n=812 after Lava A=4                     ← lava_and_move_check 后 A 恒 4
[ado] n=812 pi=0 a_in=4                        ← draw（a_in=4=regs.a）
[uoa] n=812 ENTRY ai=8 ar=7 ad=4 a_in=4        ← 走表②（双调第二走）
~~~

case-001 f812-820 全窗 **A=4 恒定、零 A=0**（18 走表/9 帧，a_in 全=4）。

## 3. 定谳：旁路（非 regs.a 污染）

**已证实（A 级）**：

1. **污染假说否定**：set_samus_data 双调两走的 a_in 恒=regs.a 传值（walk=3/case-001=4）；
   `[atr] after Lava A=` 恒定不变——lava_and_move_check（E269）污染点的 a_save/restore
   护 A 有效（physics.rs:841-846）。case-001 射击窗零 A=0 实录。
2. **旁路实锤**：A=0 走表全部带 `pi=64`（=0x40）标记——来源是
   **pickup/spawn.rs:172 `anim_draw_object(mem, 0x40, 0)` 显式 0**（每帧 2 条
   =双 item 槽 DB36 径），与 mainloop.rs:2417 弹径显式 0——与 set_samus_data
   的 regs.a 链完全无关。
3. **A=0 走表落 Samus 槽（错槽）**：update_obj_anim 的 base=0x300+**$4B**
   （PageIndex——physics.rs:914），spawn 径未设 $4B=0x40——asm 权威
   `lda #$40/sta PageIndex`（prg7_engine.asm:5505-5507）**漏端口**。探针走表③④的
   ai/ar/ad 序列与走表①②（Samus 槽）连续递变（ad 0→3→2→1→0）＝同一槽实锤。
4. **本不该走表**：asm powerup 显示径=`jsr ObjDrawFrame`（prg7_engine.asm:5561
   ——DE4A 纯绘、无走表）。rust 调 anim_draw_object（DE47=走表+绘）＝**幻影走表**。

## 4. 走表算术（A=0 双径叠加后果——walk 窗 anim 过速机制）

walkfcprobe 每帧 4 走表同槽（Samus）：walk1: ad=0⇒推进+ad←3；walk2: 3→2；
walk3: 2→1；walk4: 1→0⇒推进+ad←0（A=0 reload）⇒ **每帧 2 推进、ad 归 0 循环**
——animIndex 每帧两跳（实录 ai 序 2,1,3,2,1…）＝走表动画 ~3× 过速。
asm 单走 A=3 正确序：ad 3,2,1,0（3 帧周期推进 1 次）。

## 5. 修径指向（阶段 1 输入）

| rust 现场 | asm 对应物 | 判 | 修 |
|---|---|---|---|
| physics.rs:859 ssd 尾 anim_draw_object | :2989 `jmp ObjDrawFrame`（DE4A 纯绘） | 双调第二走 | 换纯绘形 |
| pickup/spawn.rs:172 anim_draw_object(0x40,0) | :5561 `jsr ObjDrawFrame`（DE4A 纯绘） | 幻影走表+错槽 | 换纯绘形 |
| mainloop.rs:2417 anim_draw_object(弹槽,0) | :12212 `jsr ObjDrawFrame`（DE4A 纯绘） | 幻影走表 | 换纯绘形 |
| mainloop.rs:240 update_obj_anim(0) 显式 | :2237 `jmp AnimDrawObject`（DE47 单走） | 双调第一走 | 删显式 |
| room.rs:258 update_obj_anim(6) 显式 | DrawDoor `jmp AnimDrawObject`（DE47 单走） | 双调第一走 | 删显式 |

Evidence: DSH_SPRITE_TRACE 探针实录（§2——walkfcprobe/case-001 各 f812-820 全窗）/
prg7_engine.asm :2236-2237/:2989/:5505-5507/:5561/:12212/areas_common.asm :1450-1471/
physics.rs :817-860/:890-914/spawn.rs :172/mainloop.rs :240-241/:2417。
Confidence: 92
