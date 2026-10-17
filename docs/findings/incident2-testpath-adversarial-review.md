# findings：事故②对抗调查——crates 测试执行路径视角报告（porter 线）

> 2026-10-17；全员复合调查令（事故②根因——子 agent 两次卡死工作流）。
> **证伪立场**：假设 fix（task-38 watchdog）无效——找出它为什么没防住第二次。
> 独立重导（未读他人结论——port2 层叠线报告落盘后交叉对质）。

## ③ 独立重导：全部阻塞原语枚举（crates/ 代码面）

| # | 原语 | 位置 | 挂死机理 | 复现命令 | 坏例实证 |
|---|---|---|---|---|---|
| P1 | **走带环无终止符** | frame/nmi.rs:125（vram_struct_write `loop{}`） | 缓冲区无 $00 终止符⇒环扫（**debug=溢出 panic fail-loud / release=溢出包装⇒无限环=真挂**） | `timeout 15 cargo test -p game-core --release <repro>` | **exit 124**（击杀=挂死实证）；debug 态=「attempt to add with overflow」panic（exit 101 快败） |
| P2 | **stdin 裸读环** | game-oracle/bin/int-x-bridge.rs:10（`stdin.lock().lines()`） | stdin 未闭合⇒**永久阻塞读** | `timeout 5 cargo run -q -p game-oracle --bin int-x-bridge`（无重定向） | stdin 开着=5s 不返（击杀 124）；`</dev/null` 立即退出 |
| P3 | 2^32 穷举环 | game-oracle/bin/int-y-proof.rs:56（`loop{}`） | 有界但 **~4 分钟级**（proof-checks 门 SKIP 守护——显式触发才跑） | `cargo run -p game-oracle --bin int-y-proof` | 长时占用（超时面非死挂——sub-agent tool 超时上限下=「卡死」体感） |
| P4 | **pre-commit hook 无超时** | .git/hooks/pre-commit（`exec python3 gates/cli.py check` **裸 exec**） | 门禁任一环挂⇒**commit 永久阻塞**⇒sub-agent bash call 不返⇒工作流卡死 | 插入挂死测→`git commit` | 见 P1 机理（cargo test 环节挂⇒hook 挂） |
| P5 | 幽灵环审计（阴性） | title/sparkle.rs:270/273/338 | 表界 panic 终止/`t0>>=1` 有界——**非挂死** | — | 审计阴性（排除） |

**测试点挂死面清单（①）**：135 个 #[test] **零阻塞原语**（全有界 for/断言——grep+逐段扫描
阴性）；挂死面**不在测试代码**在**被测运行时环**（P1）与**执行基座**（P2/P4）。

## ② 门禁执行路径时长预算

`time gates/cli.py check` = **10.0s 全 battery**（rom/seed/assets/naming/knowledge/
docs-links/rust-tests/subroutine-diff/trace-diff/visual-diff/ratchet/tree/doc-sync）——
**无单环挂死**；但：**rust-tests 环=cargo test 无超时**（P1 release 挂死经此传导）；
**hook=裸 exec 无超时**（P4 传导终点）。

## 证伪立场：fix 为什么没防住第二次

task-38 watchdog 覆盖面=**Mesen/xvfb 侧**（tools/scenario_run.py：Mesen `--timeout=N`+
wait_for_x 10s 上限）——**三层真空未盖**：
1. **cargo test/gates 环无超时**（P1 经 rust-tests 门→hook 传导——watchdog 盲区）；
2. **hook 裸 exec**（P4——传导终点无界）；
3. **sub-agent turn 无 deadline**（tool 早返无触发点——顶层无界）。

⇒ fix 无效判词：**watchdog 盖了数据源（Mesen）没盖执行基座（cargo/hook/turn）**——
第二次卡死走执行基座路（P1/P2/P4 任一），watchdog 对该路零覆盖。

## 两线互补（port2 交叉对质注）

port2 层叠线（runner:776 裸 subprocess.run/MCP connection.ts:308 裸 await 不对称/
subagent turn 无 deadline）=**层叠真空链**；本线=**机理面**（VRAM walk 无终止符=其 0x18
分析的 WritePPUByte 溢出 quirk 族同源：缓冲满⇒0 终止+双 PLA 断链——无终止符走环=无限环
=挂死机理互证）。两线合围：**机理（无终止符环）× 层叠（无界基座）= 卡死通路全图**。

## 修复建议（按防线深度）

1. **机理层**（本线）：vram_struct_write 环加**界护栏**（off>0x4F 族上限⇒unported/
   panic——同 sound_dispatch_chain depth>12 形）；int-x-bridge stdin 改超时读或文档钉
   「必须重定向」。
2. **执行基座层**（port2 线）：hook/gates/cargo test 加超时包裹（timeout <N>s）；
   runner subprocess.run 加 timeout 参数。
3. **顶层**（harness）：sub-agent turn deadline（port2 线层叠面）。

## 复现配方（P1 坏例——临时件已清，配方留证）

```rust
// frame/nmi.rs 临时测（演示后移除——suite 须绿）
#[test]
fn hang_repro_vram_walk_no_terminator() {
    let mut m = FrameMemory::default();
    for a in 0..0x800u16 { m.write_ram(a, 0x01); } // 全 RAM 非零
    m.write_ram(0x001B, 1);
    check_vram_struct_write(&mut m); // ← debug=溢出 panic（101）/release=挂（124）
}
```

实证输出：debug「attempt to add with overflow」（nmi.rs:146）exit 101 快败；
release `timeout 15 …` → **exit 124**（击杀=真挂死）。

**护栏纪律注**（lead 裁定）：repro 测自身必须**守卫限时下打回而非挂死**——本演示全程
timeout 包裹（15s 击杀=护栏形）；若 repro 无护栏挂死=三层真空的自证（自指反例）。
临时件已清（suite 103 绿）——配方留证即本报告。
