# findings：samus-move3 双腿场景包（task-28，task-11 前置）

> 状态：**场景包就绪**（2026-10-16）；移动版 B 跑判据=「porter 移动链填够即开」。
> 复跑：`python3 tools/scenario_run.py run samus-move3` ×2 +
> `cargo run -p game-oracle --bin l3-dump -- --scenario samus-move3`。

## 1. 交付四件

| 件 | 状态 |
|---|---|
| ① 夹具 v11 状态设置→DSL | **机制就绪**：`fixture <帧> <addr> <值> [invariant\|evolving]`（设计 §3 F_a 注入模型）——scenario_run DSL + scenario_capture F_a 写入（采样前注入，断言 (b) F_a+1 读回口径）+ run.json 记录。**本场景不需要**：解锁序列（start/a/start G4 A1）自然达态；夹具备用于密码链覆盖不到的态（位置/中间态） |
| ② oracle 腿 capture | **spin-exit 采样**（task-24 同源）；manifest 案+换锚；run1/run2 对称 |
| ③ game-core 腿接口 | l3-dump 就绪+**引爆面实证**：`unported @ bank07:B31B`（nmi.rs:184——移动链首缺口）=task-11 前置证据 |
| ④ 恒 1 断言/键集白名单 | 随案声明（legs.*.sample_point=spin-exit ⇒ 恒 1 门控激活；frame_keys 白名单在 manifest） |

## 2. 双跑对称（判据①，输出原文）

```
run1 sha256: 06bf0d3562721be98bd657ec9a5d79eadc37179b886fad0e903adf8c367cf474
run2 sha256: 06bf0d3562721be98bd657ec9a5d79eadc37179b886fad0e903adf8c367cf474
双跑一致: ✓   行数: 1383 缺帧数: 17
```

**行集事实**（manifest legs.oracle.missing 全录）：
`[2,4,5,6, 361-363, 542-548, 1002-1003, 1400]`——boot 相 + **解锁/转场相位**
（start/a/start 转屏期间主循环未过 spin 出口）=如实行集；
head_skip=1（首行 FC=00，boot-title 同款）。

## 3. 双腿采样同源（判据②）

oracle=spin-exit（$C0D3）/game-core=spin 出口缝（tick_frame_with，NMI 后/帧工作前）
——task-24 迁移面同源；恒 1 断言门控随 legs.*.sample_point 激活。

## 4. 引爆面（判据③，task-11 前置证据）

```
unported @ bank07:B31B（crates/game-core/src/frame/nmi.rs:184）
```

移动链首缺口=NMI 路径 bank7:B31B 例行——**task-11（移动面移植）的排程坐标**；
fail-loud 不落盘语义保持（无半成品绿面）。

## 5. 遗留

1. 移动版 B 跑待 porter 移动链过 B31B 后开跑（对齐全比较+缺口地图同 boot-title 机制）；
2. fixture 三条件断言（(a)(b)(c)）在 diff_traces 的消费面待首个用夹具的场景落地；
3. 转场缺帧行集（361-363/542-548 等）语义=主循环未过 spin——若任务需要全行集，
   采样点扩展（转场面）随 task-11 需求定。
