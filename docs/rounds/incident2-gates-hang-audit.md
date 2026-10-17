# 事故②根因调查——gates 总闸对抗审（verifier 视角）

日期：2026-10-17 ｜ 判据：复现命令+时长实测 ｜ 立场：证伪（假设 task-38 修复无效逐条打）

## ① 门禁执行路径时长预算+挂死面审计

pre-commit hook=薄封装（`exec python3 gates/cli.py check`=全闸串行），总闸总时长
=commit 阻塞面。逐门挂死面：

| 调用点 | 执行 | timeout | 挂死面 |
|---|---|---|---|
| `git_staged_files`(:179) | `git diff --cached` | **无界** | index.lock 竞争阻塞 |
| `gate_rust_tests`(:740) | `cargo test --workspace` | **无界** | **artifact 锁等待**（见③）+挂死测 |
| `_gate_runner_dispatch`(:776) | `python3 tools/diff_*.py` | **无界** | runner 自身挂死（内部子进程有界≠外层有界） |
| `gate_proof_checks`(:1013-) | SMT/穷举/构建 | 180s/900s/×4(720s/3600s) | 有界（DEFERRED 机制在案） |
| diff_subroutines 内部 | subprocess | 600s（task-38 ④） | 有界 |

既有界项：proof-checks 全族（超时→DEFERRED 欠账，不计 PASS）/diff_subroutines 内部 600s。
**无界三处=挂死面全集**。全链时长：红态快败 1.1s（rust-tests 快败）；绿态全闸
≈分钟级（rust-tests 全量+L2 441 案+L3 14 案+L4 7 帧+SMT 触发时 ~43s）。

## ② task-38 修复覆盖缺口复盘（证伪立场）

task-38 四面实修：L2 步数上限/watchdog 测（坏例先喂）/无参 savestate 机械防/
diff_subroutines 内部超时=FAIL。**逐条打**：
- 「修了 spin 挂死」→ 真（JMP-to-self 40ms 限时打回实证）——但只覆盖**测试内部
  逻辑挂死**；
- 「修了 runner 挂死」→ 半真（内部 subprocess 600s 有界，**runner 进程自身无界**）；
- 「修了脚本挂死」→ 真（lint 禁用面）；
- **缺口**：**外部资源等待类挂死**（cargo artifact 锁/git index 锁）**零覆盖**——
  第二次卡死=锁等待类，与 task-38 靶向（逻辑挂死）**不同类**，非修复无效，
  是**靶类缺口**。task-38 ④ 原则「超时=FAIL 显式」未贯彻到 gate 级三处调用点。

## ③ 阻塞传播链（工作流卡死全貌）

**复现命令**（确定性）：
```sh
(flock 9; sleep 10) 9>target/release/.cargo-lock &
sleep 0.3 && timeout 5 cargo build --release -p game-oracle
# => "Blocking waiting for file lock on artifact directory"（实测 2026-10-17）
```

传播链：agent A 长构建持 target/*.cargo-lock → agent B commit→pre-commit→
gates check→gate_rust_tests→`cargo test --workspace`（无界）→**无限阻塞于
artifact 锁**→B 提交卡死→B 全工作流「卡死」→C/D/E 同法排队→全仓 commit 停摆
=「工作流卡死×2」全貌。同理 git_staged_files 于 index.lock 竞争同型。

## 修复建议（最小面，单独 commit+基线影响声明）

三处调用点补 `timeout=`（超时=FAIL 显式=task-38 ④ 原则贯彻）：git 30s/rust-tests
600s/runner dispatch 900s（外层篱笆>内部 600s）。超时判 FAIL（非 SKIP/DEFERRED
——门禁未能验证=阻塞，fail-loud）。

Evidence: 复现输出「Blocking waiting for file lock on artifact directory」/调用点
三处行号/gates cli 源码 timeout 字段审计（本账内）
Confidence: 93
