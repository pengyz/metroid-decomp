# findings：事故②三靶类坏例注入验收（task-41，判决书焦点③执行面）

> 状态：**三类全打回+还原全绿**（2026-10-17）——mutation_verified 三拦纪律
> （注入→打回→还原）。防线自过证伪 ✓。复现命令逐条在表。

## 判决书矩阵（逐格勾销）

| 类 | 坏例注入 | 防线打回 | 还原 | 复现命令 |
|---|---|---|---|---|
| ① 逻辑挂死（P1 走带环） | L2 用例注入 entry=$C0D3+$1A=1（NMI 等待环永转） | **0.173s 超步数上限打回**：「执行超步数上限（疑似缺 stops 或不终止）entry=$C0D3」 | ✅ 441 案全绿 | `echo '{…task41-bad-tape…}' >> l2_cases.jsonl && time python3 tools/diff_subroutines.py` |
| ② 资源等待（flock 持锁） | fcntl WRLCK 占 `~/.cargo/.global-cache` @0x40000000（事故同形锁）25s | **timeout 6s→exit=124 限时打回**（非挂死） | ✅ 锁释放 | `python3 /tmp/hold_cargo_lock.py 25 & timeout 6 cargo build …` |
| ②b 锁免疫面 | 同上持锁 | **独立 CARGO_HOME=/tmp/cargo-home-41=免疫**（exit=0 4s） | ✅ | `CARGO_HOME=/tmp/cargo-home-41 cargo build …` |
| ③ 交互挂死（P4 裸 exec/MCP 无界） | sleep 300 无界 mock（MCP 无界调用同形） | **timeout 5s 包裹→exit=124 打回** | ✅ mock 杀 | `timeout 5 bash -c "python3 -c 'import time; time.sleep(300)'"` |

## ⚠️ 合流急情（比判决书更高的发现）

**CARGO_TARGET_DIR 单独不足**：`.global-cache` 锁持有时，仅独立 target 的构建
**照样阻塞**（exit=124 8s 实录）——**锁面在 CARGO_HOME**（`~/.cargo/.global-cache`
fcntl F_SETLK @0x40000000，strace 实录 26×F_SETLK）：
- lead 的 CARGO_TARGET_DIR 独立化=**治 target artifact 锁**（verifier flock 面）；
- **cache 锁另面**（~/.cargo/.global-cache 字节段锁——并发 cargo 共享 CARGO_HOME 即撞）；
- **完整免疫=CARGO_HOME 亦独立**（或全依赖超时护栏）——建议补令：
  `export CARGO_HOME=/tmp/cargo-home-decomp`（或 target+home 双独立）。

复现：`python3 /tmp/hold_cargo_lock.py 25 & CARGO_TARGET_DIR=/tmp/tgt-decomp timeout 8 cargo build …`
→ exit=124（本判决书实录）。

## 锁面勘定（strace 实录）

```sh
strace -f -e trace=openat,fcntl -o /tmp/cargo-lock2.trace cargo build …
# → 26x F_SETLK /home/peng/.cargo/.global-cache（fd 6，offset 0x40000000+）
```
flock(1)≠fcntl 锁型（独立锁空间——flock target 目录无效实录）；真锁=fcntl
字节段（F_SETLK 非阻塞失败→cargo 内部重试=自旋=外观挂死）。

## 三拦纪律记录

- 注入 3 次（bad-tape 用例/持锁 25s/无界 mock）；
- 打回 3 次（0.173s/6s/5s 全限时）；
- 还原全绿（L2 441 案 0 失败+行数 diff 一致+锁/模拟全清）。
