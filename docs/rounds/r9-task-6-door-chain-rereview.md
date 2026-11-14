# r9 复评 · #6 出门链（复核修复 commit `ae6c982c`，父 `5ca9c3d7`）

verifier（证伪立场）· 2026-11-14 · 只读复核（未改文件，未跑 cargo test / gates / L4）

## verdict: pass

第一轮 fail 的两处实错（`Doors_RemoveIfOffScreen` 基址 / 循环边界）均已修对，且测试从「断言自实现」
改为裸绝对址 + 越界哨兵，具备独立抓错能力。其余 8 项经 diff 范围确认原样未动。
遗留 3 条 P2/P3 建议（不阻塞），主要集中在「门槽 0 缺正向断言」与「补点帧号未定位」。

## R1 基址等价 — PASS（A 级）

asm 事实（三方独立互证，不靠实现注释）：

- `constants_ram.asm:842` `.enum $0300 export` → `:844 Objects .instanceof Object $10 startfrom 0`
  ⇒ Objects 基址 = **$0300**；`:853 Doors instanceof Object 4 startfrom 0`（第 9 组，索引 8-11）
  ⇒ Doors = $0300 + 8*$10 = **$0380**；`:839` 注释「slot 8 to B is for doors」同向。
- ROM `07:ED67` = `BD 00 03`（`lda $0300,x`）、`07:ED6C` = `BD 0B 03`（`lda $030B,x`）
  —— **同一个 X 索引，两个绝对址**，非零页偏移。

Rust：`handler.rs:1260 const OBJECTS_BASE: u16 = 0x0300;`，
`handler.rs:1349 xu = OBJECTS_BASE + x as u16`、`xu + 0x0B`。
x = $B0/$A0/$90/$80 ⇒ 绝对址 $03B0/$03A0/$0390/$0380，onScreen 侧 $03BB/$03AB/$039B/$038B。

判定：**等价**。旧错（`DOORS_OFF=0x80` ⇒ $0130..$0100）已彻底清除，全仓 `grep DOORS_OFF` 为空。

## R2 循环语义 — PASS（逐字节推导，未采信注释）

ROM `07:ED65` 起 21 字节（python 读 rom，bank7 偏移 `0x1C000+(addr-$C000)`）：

```
07:ED65  A2 B0        ldx #$B0
07:ED67  BD 00 03     lda $0300,x        ; ← @loop，BMI 目标
07:ED6A  F0 08        beq +8  (→ED74)
07:ED6C  BD 0B 03     lda $030B,x
07:ED6F  D0 03        bne +3  (→ED74)
07:ED71  9D 00 03     sta $0300,x        ; A=0（bne 未取⇒A 为 0）
07:ED74  20 F4 F1     jsr Xminus16
07:ED77  30 EE        bmi @loop          ; ED79-18=ED67 ✓ 回跳
07:ED79  60           rts
```

`Xminus16`（`07:F1F4`）= `8A 38 E9 10 AA 60` = `txa / sec / sbc #$10 / tax / rts`。
**关键**：`rts` 不改标志，故 `bmi` 看到的是 **SBC 结果的 N**（SBC 覆盖了 TXA 的 N/Z）。

迭代集合（按 N 逐轮推）：$B0→$A0(N=1,回跳) →$A0→$90(1) →$90→$80(1) →$80→$70(N=0,退出)
⇒ **body 恰 4 次，地址 $03B0/$03A0/$0390/$0380**（即门槽 3/2/1/0 全覆盖）。

Rust（`handler.rs:1350-1359`）：
`loop{ body(x); x -= 0x10; if (x as i8) >= 0 { break } }`
→ x=0xB0 跑；0xA0(as i8 = -96) 继续；0x90(-112) 继续；0x80(-128) 继续；最后一次 body=$0380；
减到 0x70(as i8 = 112) 退出。**4 次，地址完全一致**。

等价性成立的前提：`x as i8` 截断 = bit7 = N，仅在 x ∈ $00..$FF 时成立（当前恒成立）。

**P2 建议**：加一行 `debug_assert!(x <= 0xFF)` 或注释「x 恒 ≤$FF，`as i8` 才等价于 N 标志」，
防止将来改初值时静默失效。

## R3 测试独立性 — PASS（两种旧错都会红），但有 1 处漏

反推（把实现分别改回旧错，看断言）：

| 注入的旧错 | 会红的断言 | 结论 |
|---|---|---|
| 基址退回 `0x80` | 断言 `0x03B0 == 0`：实现去 $0130 找不到门 ⇒ $03B0 仍是 1 ⇒ 红；同时哨兵 `0x0120` 被清 0 ⇒ 红（双重） | 能抓 |
| 循环退回 `while x >= 0`（12 轮） | `$0340` 布「存在+离屏」，12 轮版 x=$40 命中并清 0 ⇒ 红 | 能抓 |

哨兵位址选择说明：`$0130/$0120/$0100/$0080/$00A0/$013B` 只对「旧基址」这一种错有效（正确实现下
它们不会被触碰是自然的）；真正钉住**边界**的是 `doors_remove_touches_exactly_four_slots`。

**P2 缺口**：门槽 0（`$0380` = `Doors+0`）目前**只有负向保护**——把它设为 0 并断言 0，
所以「少跑最后一轮（漏掉 $0380）」这类退化**两测仍绿**。改一行即可闭合：
把 `m.write_ram(0x0380, 0x00)` 改成 `0x01` + `m.write_ram(0x038B, 0x00)`，断言 `0x0380 == 0`。
这样四个槽的首（$03B0）尾（$0380）都有正向删除断言。

## R4 其余 8 项未被破坏 — PASS

`git show ae6c982c -- handler.rs` 只有 **3 个 hunk**（常量 / `doors_remove_if_offscreen` / 两个测试），
`samus_door`(1372)、`samus_door_tail`(1266)、`move_out_door`(1275)、
`door_delete_offscreen_enemies`(1301)、`goto_clear_all_metroid_latches`(1332) 全部原样。
`unported!` 残留只剩 `handler.rs:97`（status 表外索引，与本链无关）与 `handler.rs:1430`
（`samus_elevator` 的 `bank07:D42E`，另一 handler）——门链本身无桩。

故第一轮 1a（入门相不移位）、1b（bit7 旗）、1c（Lx052 落 MoveOutDoor）、1d（asl 的 A=$56）、
1e（敌环 $60/$40/$20/$00 + BUG 保真）、1f（虫洞环槽 0 不访问）、1h（BankTable=bank+1 / Tourian=4）、
1i（音乐面 pha/pla/bpl 与 `$80` 绕行）的「等价」判定**保持不变**（依据：diff 未触及 + 上一轮 ROM 字节）。

未复跑测试。commit message 的「589 passed / 静态 593 / 基线 592→593」我无法验证（禁跑），
但 +1 与本次只新增 1 个 `#[test]` 一致，无矛盾。

## R5 覆盖缺口声明 — PASS（声明准确），但补点形态**尚不可执行**（P2）

准确：`docs/spec/subsystem-completeness-audit.md:178` 已把 #6 由「已修」改为
「已实现 / 待差分验证」，并新增段更正说明「14 案 112 格无采集点会调用 SamusDoor
（case-002 全程 DoorEntryStatus=0、Samus.status 从未 5）」——与第一轮独立导出结论一致。

不可执行之处：建议只有「结项前须补一条门退出窗口内的采样点」，**没有帧号**。而已知现有采集点
把窗口**夹在 f790 与 f850 之间且两侧 $0056 都是 0**，随手加一个帧号很可能仍采到 0，白跑一次 L4。
建议改成三步可执行形：先逐帧记录 `$0056` 定位第一个 `$56≠0` 的帧 F；在 F..F+40 内加 2 个 capture；
断言该段 `$56 ∈ {1,2,3,4,5,$80..$84}` 且 `$59` 曾非 0、`$0300` 曾为 5。

**P3**：同表末列仍写「中（已完成）」，与状态列的「待差分验证」自相矛盾，建议改
「中（实现完成 / 差分待补）」。

## 附：本次复核使用的证据命令（可复现）

- `git show ae6c982c -- crates/game-core/src/samus/handler.rs`（3 hunk）
- `git show ae6c982c -- docs/spec/subsystem-completeness-audit.md`
- `sed -n '838,862p' refs/m1disasm/SRC/constants_ram.asm`（`.enum $0300` / Objects / Doors）
- `sed -n '9190,9225p' refs/m1disasm/SRC/prg7_engine.asm`（Doors_RemoveIfOffScreen 全文）
- python 只读 ROM：`0xED65` 21B、`0xF1F4` 8B（Xminus16）
- `grep -rn DOORS_OFF crates/`（空）、`grep -n unported! crates/game-core/src/samus/handler.rs`

## 复查后处置（lead 执行）

- P2 测试缺口已闭合：`$0380` 改为「存在+离屏」并断言被清；变异验证——把 break 条件改成
  `== -96`（少跑最后两轮）时该测当场红（`离屏门槽 8 应被清（末轮——漏跑即红）`）。
- R5 三步补点形已写进 `docs/spec/subsystem-completeness-audit.md` 的 #6 更正段。
- P3 表格末列已改「中（实现完成 / 差分待补）」。
