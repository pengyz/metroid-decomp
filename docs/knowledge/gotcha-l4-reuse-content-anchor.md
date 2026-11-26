---
type: gotcha
---

# gotcha：L4 复用只验输入锚不验产物内容 = 不同源产物混入（2026-11-26，#87）

## 症状

同一棵树、同一用例、同一输入（scenario/ROM/sched 三 sha 全同），`case-password-justin` 的
失配格在不同 run 之间漂移：`f1430 spawn-t4s = 169px`、`f950 mid-3 = 71px`、
`f1150 pw-done = 94px`，另一些 run 是 14/14 全一致；清掉该 case 的
`shots/`、`shots_rust/`、`analysis/traces/case-password-justin*` 后连跑两次稳定 14/14
（#83 期间的实测记录）。

## 机制

`tools/l4_diff_run.py` 的 emu 腿缓存（`emu_cache_hit` → `check_reuse`）在 v1 里只验两件事：

1. 三输入锚（`scenario_sha256` / `rom_sha256` / `sched_sha256`）与 `<name>.l4prov.json` 一致；
2. 每个采集点的截图 + trace **文件存在**（`emu_artifacts_complete`）。

它**不验产物内容**。于是下面这些「文件在、锚也对，但字节不是这次输入产出」的情况全部放行：

- 从另一棵树/另一轮拷来的 `shots/*.png`（#60 期间复核方就把主线 emu 锚整体拷进 lane 用过）；
- 被别的 run 覆盖过、或被杀进程写了一半的截图 / trace；
- 换过 Mesen 二进制（BYO 工具换版本）后沿用旧产物。

放行后 `do_diff` 拿「这一轮的 rust 截图」对「来历不明的 emu 截图」，失配格就随
哪份文件被覆盖而随机出现——表现为「输入没变、矩阵读数会飘」。

## 修法（v2，2026-11-26）

`l4_prov.json` 升到 `schema: 2`，在原有三输入锚之外追加：

- `artifacts`：该腿每件产物（采集点 PNG、`<name>.frames.jsonl`、`<name>.captures.jsonl`、
  rust 侧 `<name>.rust.captures.json` 与 `.sched`）的 `sha256`；
- `mesen_sha256`（emu 腿）：产出这批截图的模拟器二进制 sha；
- `git_head`：审计面（**不作 emu 复用判据**——emu 产物只由 ROM/用例/调度/Mesen 决定，
  代码改动不改变它，见 `tools/l4_diff_run.py` 的 `emu_cache_hit` 注释与矩阵 18 分钟→只跑 rust 的设计）。

`check_reuse` 复用前逐件复算 sha：文件缺 → 拒；内容不符 → 拒；`schema` 不是 2 → 拒
（旧存证没有内容锚，无法证明磁盘上的文件就是它记录的产物）。emu 缓存分支会把拒因打到
`[emu-cache] 拒复用：…`，不再静默回退。

拒复用的代价是重采一次该用例（emu 腿，分钟级）；静默复用的代价是矩阵读数不可信，
后者更贵。

## 负例自测（可复现）

```bash
cd <worktree>
# ① 篡改一张 emu 截图（模拟跨树拷贝/写一半）
python3 tools/l4_diff_run.py run case-password-justin --fresh-emu     # 先产 v2 存证
printf 'tamper' >> shots/case-password-justin_f01430_spawn-t4s.png
python3 tools/l4_diff_run.py diff case-password-justin; echo rc=$?    # rc=2，报「产物内容不符」
# ② 篡改 trace
printf '\n{"f":99999,"w":{}}\n' >> analysis/traces/case-password-justin.frames.jsonl
python3 tools/l4_diff_run.py diff case-password-justin; echo rc=$?    # rc=2，报 trace 内容不符
```

## 判据边界

- v2 证明「产物字节 = 存证记录的字节」，不能证明「存证记录的字节一定来自 ROM 正确执行」；
  后者由 `l4report` 的 rom/scenario/sched 锚 + `scenario_run` 的隔离域负责。
- 换 `GUNNAC_MESEN` 指向不同二进制 ⇒ 必须重采（`mesen_sha256` 不符会拒），这是有意的：
  换模拟器等于换解释器。
