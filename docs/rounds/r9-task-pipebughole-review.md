# R9 任务复核：PipeBugHole 全链（commit `ccb323ea`）

- 复核对象：`ccb323ea`（父 `d4e019f2`），`feat(core): 端口 LoadPipeBugHole / UpdatePipeBugHole 全链（bank07:EC57 / 07:FAF2 / 07:FAFF）`
- 复核方式：独立 verifier 子代理（`verifier-pipebughole`），**证伪立场**，只读手段（`git show/diff`、read/grep、`cargo test -p game-core --lib`、sidecar 只读重算），禁跑 l4/all/room_sweep。
- 复核时工作树非静止（另一路在跑整改与房扫），结论以 **commit 树**为准。

## ① 结论

**verdict = pass-with-findings（实现层通过；记录/证据层 4 项 P1 已在本轮修掉）**

- 实现层 claim 1–4 逐条 A 级通过，未发现行为偏差：碰撞 `bcc`↔`no_overlap` 方向未反、槽 0 永不更新 quirk 保留、恒吃 3 字节、`$0681 |= $08`、参数字节序均正确。
- 记录层不可信（P1-1..P1-4），已修：见 ③ 的处置列。
- 测试层有 3 个可存活变异（M2/M3/M4），已补 3 条杀伤测并**逐条实测判红**（见 ④）。

## ② 逐条判定（摘要）

| # | 主张 | 判定 | 关键锚 |
|---|---|---|---|
| 1 | `load_pipe_bug_hole` 逐行等价 `LoadPipeBugHole ; 07:EC57`，恒 3 字节 | PASS | `prg7_engine.asm:9184-9223`；`AddToPtr00 :9745-9753`；`room_draw.rs:723-731 i += 3` |
| 2 | 件序只跑槽 3/2/1，槽 0 永不更新 | PASS | `prg7_engine.asm:12179-12187`（`ldy #24` → `sbc #8` → `bne`）；`pipe_bug_hole.rs:76-80 (1..4).rev()` |
| 3 | 四步语义（含碰撞方向） | PASS | `prg7_engine.asm:12189-12264`；`samus/physics.rs:1604 no_overlap = dx_mag >= rx_sum` |
| 4 | `SFX_OutOfPipe` 净效果 `$0681 \|= $08` | PASS | `prg7_engine.asm:2593-2596`、`:2664-2667`；`constants_ram.asm:1145-1146`；`framework.rs:17,52` |
| 5a | 测试数 +13、无删除 | PASS（数量） | `#[test]` 文本计数 1038→1051；唯一"删"是 `memory.rs write_io_feeds_render_state` 去缩进重排 |
| 5b | 锚测能判红 | FAIL→已修 | M2/M3/M4 三个变异存活（见 ④） |
| 6 | 声明与事实一致 | FAIL→已修/已勘误 | 测试数算术、`Evidence:` 行号、房扫结论三处（见 ③） |

## ③ findings 与处置

| id | 级别 | 问题 | 处置 |
|---|---|---|---|
| P1-1 | data 层 | `handler_touchlist.json` 三处 `stale_note` 把"推点已接线"误记成"无用例覆盖"（真实成因：`sound/hook.rs:207`、`:218`、`weapon/bomb.rs:35` 已接线） | **已修**：三条改为「推点已接线…不是覆盖率缺口，也不代表行为回退」 |
| P1-2 | 台账/消息 | 测试数算术自相矛盾（`+13+2` vs 实际 +11/+2；`baseline_history` 记 `1038→1049` 却写 +13） | **已修**：`baseline_history.jsonl` 该行 reason 加 2026-11-22 更正（真实 +11）；消息错项以本节留痕（commit message 为历史载体，不回改） |
| P1-3 | 证据指针 | `Evidence:` 的 `:10205-10260` 指向弹×敌碰撞环（`07:F0F7-F112`），真值是 `:12179-12187` / `:12189-12249` | **已勘误**（本节即真值锚；后续提交引用本节） |
| P1-4 | 证据载体 | "房扫 f60 64→0px" 无提交载体，且 HEAD 的 `room_sweep_0.json`（12:05:24）仍记 mismatch | **已修**：随后提交 post-fix 房扫产物（`--note bit6fix-area0`），message 引用其 `generated_at` 与 tag |
| P2-1 | 测试 | M2（data05 ScrollDir 项）、M3（strong 双血 / data0D）、M4（占用径 isHit）三个变异存活 | **已修**：补 `load_hi_follows_ppuctrl_xor_scroll_dir`、`spawn_strong_hole_doubles_health_and_keeps_scroll_bit`、占用测试加 `is_hit` 载荷断言；1051→1054 |
| P2-2 | 测试设计 | `update_all_never_touches_slot_zero` 用共享 `enemySlot` 间接观测 | 登记（未改）：`(1..4)`→`(0..4)` 变异已判红，仅覆盖强度不足 |
| P2-3 | 提交卫生 | `memory.rs`/`room_draw.rs`/`update.rs` 含 rustfmt 噪声，淹没语义 diff | 登记（未改）：回退格式会重写已推 commit 的树，收益低于风险 |
| P3-1 | 注释 | `room_draw.rs` 占位臂注释仍把 `$EC57` 列为未端口，且"像/虫=3"错（像=1） | **已修**：注释改为「已端口/仅剩 type 0=LoadElevator」+「像=1/虫=3」 |
| P3-2 | 表述 | `GetEnemyType` 的 bit6 分支未移植，"逐行等价"应限定为非 bit6 径 | 登记：真实房流未观测到 `type=7 且 b1 bit6` |
| P3-3 | 注释 | `pbh_status` 注释写 bit0-3，生敌处掩 `&$3F` | **已修**：改为 bit0-5（原 asm 第三方注只写 bit0-3） |

## ④ 变异验证（本轮实测，非声明）

对 `crates/game-core/src/enemy/pipe_bug_hole.rs` 逐一注入变异、跑 `cargo test -p game-core --lib enemy::pipe_bug_hole`：

| 变异 | 结果 |
|---|---|
| `d5` 去掉 `mem.scroll_dir() & 0x01` 项 | **FAILED 12/13**（`spawn_strong_hole...` 判红） |
| 删掉 `special_attribs & 0x80 ⇒ hp <<= 1` | **FAILED 12/13**（同上判红） |
| 占用径提前 `set_is_hit(0)` | **FAILED 12/13**（`is_hit` 载荷断言判红） |
| 复原 | 13/13 绿 |

（第一次注入 M4 时替换串不匹配导致静默 no-op、测试仍绿——已在命令里加 `assert old in s` 后重做，属"换前必须断言"的旧教训复发一次。）

## ⑤ 未覆盖点（只读手段无法判定）

1. L4 实跑数字（`match166/diff6/blocked0/172 格`）只能核载体自洽，执行需 `gates/cli.py l4`（本轮提交前已跑过 `run-all`）。
2. `gates/cli.py check` 全闸需实跑（本轮已跑：PASS=23 FAIL=0）。
3. 像素主张（f60 64→0、f75 起 sprite 残差）由随后的房扫载体承担（P1-4）。
4. L1–L3 trace：移除 `DIVERGENCES(0xEC57)` 后 trace 面是否对齐需 `gates/cli.py trace`。
5. 真实房数据是否触达 `type=7 且 b1 bit6`、各区 `b1` 是否含 bit7（双血分支）——需全房流扫描。
6. NMI 序下 `UpdateAllPipeBugHoles` 与 `SpritePagePos` 的先后需 oracle trace。

## ⑥ Confidence

**88 / 100**（实现层 95+：全部落到 `bank:addr` 与 rust 行号双向核对；扣分项=运行时主张与真实房数据可达性未证）

## ⑦ 与实施方结论的差异

实施方宣布"完成"时，实现正确但**账实不符**（P1-1/P1-4 最危险：会把"已接线"当成"覆盖率缺口"，把未提交的房扫结果当成已入库证据）。本轮已按上表处置；后续同类提交必须先落载体再写 message。
