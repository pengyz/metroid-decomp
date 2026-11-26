# TAS 100% 覆盖率：主线 HEAD 重跑（round 4）

> 任务：#75。目的不是重做 TAS，而是把 `analysis/tas-100/coverage.json` 这份主证据从
> **lane 树（`e4d23d6f`，不在主线历史）** 重新绑到 **主线 HEAD**，让
> `tools/evidence_bind.py verify` 能过。
> 上一版报告见 `docs/findings/tas-100-round3.md`（结论保留：120540/120540、首分歧 f22895）。

## 0. 结论

| 项 | 读数 |
|---|---|
| 主线 HEAD | `847fac49f0f269ee04e5a7e7be0282c08bea4334`（`d7433e2d` = #71 A-1 落卡，`847fac49` = rust_test 基线 1175→1176） |
| 覆盖 | `ran_frames = total_frames = 120540`，`coverage_pct = 100.0`，13/13 段 |
| 段状态 | pass 2（seg0/seg1）、diverge 11、fail 0 |
| **首分歧** | `{frame: 22895, field: '$0405', offset: 5, emu: 90, rust: 26, diff_bytes: 8}` |
| 与 lane 版对比 | 首分歧仍是 f22895，但 `diff_bytes` **11 → 8**（#71 A-1 生效：`$0400/$0401/$040A` 归零） |
| `evidence_bind verify` | **OK**（`kind=coverage head=847fac49… at=2026-11-26T02:33:52Z`，rc=0） |
| 确定性 | `deterministic = true`；`env.git_dirty_crates = []` |

## 1. 环境指纹（来自 `coverage.json.env`）

```json
{"at": "2026-11-26T02:33:52Z",
 "git_head": "847fac49f0f269ee04e5a7e7be0282c08bea4334",
 "git_dirty_crates": [],
 "rust_screenshot_sha256": "ee2f15f9c92dba72",
 "rust_screenshot_mtime": 1795603436}
```

- 输入：`movies/lordtom-metroid-100p.inputlog`，`sha256` 前 24 位 `094de4afdc1eff604002bbb1`（与 #66 记录一致）。
- 观察面：`ens $0400-$04FF`（`tag=ens`，`range=0400-04FF`，段长 10000）。
- rust 腿由工具内 `cargo run -q -p game-core --bin rust_screenshot` 现场编译（`tools/tas_replay_diff.py:110-116`）；
  `CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target`，`GUNNAC_MESEN=/home/peng/workspace/metroid-decomp/tools/mesen2/Mesen`。
- 主线工作树当时只有他人未跟踪文档（`docs/design/rom-data-audit.md`、`docs/design/typed-struct-plan.md`、`docs/spec/items-equipment.md`），代码域干净 ⇒ `git_dirty_crates: []`。

## 2. 命令与耗时

```bash
cd /home/peng/workspace/metroid-decomp
git rev-parse HEAD                      # 847fac49f0f269ee04e5a7e7be0282c08bea4334
cp -a analysis/tas-100 analysis/tas-100-lane-bak     # 旧 lane 产物先备份
rm -rf analysis/tas-100 && mkdir -p analysis/tas-100
env CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target \
    GUNNAC_MESEN=$PWD/tools/mesen2/Mesen \
    python3 tools/tas_replay_diff.py replay \
      --inputlog movies/lordtom-metroid-100p.inputlog \
      --frames 120540 --segments 10000 --allow-rust-missing \
      --out analysis/tas-100 --timeout 1800
```

- 墙钟：约 40 分 22 秒（09:53:30 → 10:33:52，含首次 `cargo` 编译约 2 分钟；`rust/full.sched` 落盘 09:55:38）。
- 工具内计时：rust 腿 115.7s；13 段合计 2172.4s（≈36 分 12 秒）。
- 产物 2.4G，落 `analysis/tas-100/`（不入库，`.gitignore` 覆盖 `analysis/`）。

## 3. 逐段表（`analysis/tas-100/coverage.json`）

| seg | 帧区间 | status | sha256_agg_emu（前 20） | 比较帧 | diverge | 耗时 |
|---|---|---|---|---|---|---|
| 0 | [0,10000) | pass | `a285c127de02c5fb3132` | 9962 | 0 | 179.6s |
| 1 | [10000,20000) | pass | `5f459d1ed2dc38b9e91f` | 10001 | 0 | 178.2s |
| 2 | [20000,30000) | diverge | `0e20a8de679d45c489e4` | 9970 | 7086 | 178.7s |
| 3 | [30000,40000) | diverge | `ce5a88df80e03a3a6bda` | 10001 | 10001 | 179.0s |
| 4 | [40000,50000) | diverge | `c03c8cba82199f7e1b1c` | 9968 | 9968 | 180.0s |
| 5 | [50000,60000) | diverge | `b9c7615043a79fe0e98e` | 10001 | 10001 | 179.0s |
| 6 | [60000,70000) | diverge | `01250c4d347a7e2e8f81` | 9951 | 9951 | 178.4s |
| 7 | [70000,80000) | diverge | `29f79fb819005595a55a` | 9986 | 9986 | 178.9s |
| 8 | [80000,90000) | diverge | `6ae6eee90a8dc868d1a5` | 9979 | 9979 | 179.7s |
| 9 | [90000,100000) | diverge | `b9990734331c116eef7a` | 9998 | 9998 | 179.2s |
| 10 | [100000,110000) | diverge | `9c1c394486b289fa456f` | 9943 | 9943 | 178.5s |
| 11 | [110000,120000) | diverge | `77118f99d023f66ca6f6` | 9966 | 9966 | 181.4s |
| 12 | [120000,120540) | diverge | `8444edf10a108c385e4c` | 541 | 541 | 21.8s |

全段 sha 原文见 `analysis/tas-100/coverage.json`（此处只取前 20 位便于对读）。
`first_divergence` 取「首个真实分歧」＝段内首个 diverge 段（seg2）的首帧；日志末行
`[tas] 段 13/13 已跑；到帧 120540/120540；失败段 0；分歧段 11；首个真实分歧 f22895 $0405`。

## 4. 与 lane 版（`analysis/tas-100-lane-bak/`，HEAD `e4d23d6f`）的差异

1. **首分歧从 11 字节收到 8 字节**：`$0400/$0401/$040A` 归零（#71 的 A-1 生效），剩 8 字节全是
   六槽 `data05` bit6 相位差（A-2，归 #72）。这与 #71 的分段读数
   `f22895=8 / f22896=3 / f22897=1` 逐项一致。
2. **4 段的 `sha256_agg_emu` 与 lane 版不同**（seg4 `613ea24d`→`c03c8cba`、seg6 `2dfae3a8`→`01250c4d`、
   seg10 `ec127583`→`9c1c3944`、seg11 `c0bf9b4d`→`77118f99`），其余 9 段相同。
   **这不是 emu 跑得不一致**：emu 逐段 trace 文件（`analysis/tas-100/emu/seg-*.jsonl`）与 lane 版
   **13/13 逐字节相同**；差异来自该聚合值的定义——`tools/tas_replay_diff.py:587` 只对
   **emu 与 rust 共有的帧**（`common`）求 sha，而 rust 腿落盘行数在两轮间从 120349 变成 120347
   （rust 侧少数帧无状态行），`common` 集合随之变化。所以凡包含这些帧的段，聚合值会变；
   段本身的 emu 数据没变。

## 5. 门禁

- `python3 tools/evidence_bind.py verify -f analysis/tas-100/coverage.json`
  → `[verify] OK … kind=coverage head=847fac49f0f269ee04e5a7e7be0282c08bea4334 at=2026-11-26T02:33:52Z`（rc=0）。
  这一版**没有** `git_head_generated` 字段（不是 rebind 得来的），所以不存在「生成树不是祖先」的红灯。
- `python3 gates/cli.py check` → 见提交说明里的实测摘要（trace-diff 是 oracle 腿环境项，与本次改动无关）。

## 6. 未覆盖 / 边界（前置声明）

- **首分歧未归零**：f22895 仍差 8 字节，全部是 A-2 相位差，按卡面边界留给 #72；本卡只负责把主证据绑回主线。
- **观察面仍只有 `$0400-$04FF`**：`$6AF4-$6B53`（EnsExtra）不在本工具的比对面内，本次未扩面。
- **段级 `sha256_agg_emu` 不是 emu trace 的完整指纹**：它是「common 帧」的聚合（见 §4-2），
  要校验 emu 本身请直接用 `analysis/tas-100/emu/seg-*.jsonl` 的 sha256。
- 未跑 `run-all`（本卡验收不含 L4）；未改 `crates/`、`tools/`、判据。

## 7. 复核命令（只读）

```bash
cd /home/peng/workspace/metroid-decomp
python3 - <<'PY'
import json
d = json.load(open('analysis/tas-100/coverage.json'))
print(d['env']['git_head'], d['ran_frames'], d['total_frames'], d['deterministic'])
print(d['first_divergence'])
print([(s['index'], s['status'], s['diverge_frames']) for s in d['segments']])
PY
python3 tools/evidence_bind.py verify -f analysis/tas-100/coverage.json
# emu 逐段指纹（应为 13 段，全部存在）
ls analysis/tas-100/emu/seg-*.jsonl | wc -l
```
