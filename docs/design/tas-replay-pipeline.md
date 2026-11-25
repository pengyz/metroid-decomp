# TAS 回放对拍管线（M1 第一段）

**目标**：给定一份 TAS `inputlog` 与帧数 N，产出一份**双侧逐帧报告**（内存判等面哈希、采集点画面 sha/像素差、
首分歧帧与字段、双侧覆盖与 rc/耗时归一化），供「TAS 对拍可玩」的判据层自行裁量。
**本工具只出事实，不做判定**：整片逐帧全等 / 只判断点帧 / 关卡出口，都由调用方按报告裁。

## 1. 一条命令

```sh
python3 tools/tas_replay_diff.py replay \
    --inputlog movies/eien86-6725M.inputlog --frames 1200 --allow-rust-missing \
    --name tas-m1-1200 \
    --capture 1:start --capture 300:c300 --capture 600:c600 \
    --capture 900:c900 --capture 1200:end \
    --out analysis/tas-m1/eien86-1200
```

产物：

| 文件 | 内容 |
|---|---|
| `<out>/report.json` | 机器可读（键排序、无时间戳/绝对路径 ⇒ 可 diff） |
| `<out>/report.md` | 人读（逐帧表 + 采集点画面表 + 已知限制） |
| `<out>/scenario-mesen.txt` | emu 腿场景（movie_sched `--format mesen2` + `watch_range`） |
| `<out>/scenario-rust.sched` | rust 腿调度表（movie_sched `--format sched`，逐帧 `joy` 展开） |
| `<out>/rust.state.jsonl` | rust 腿逐帧状态行（`--state-out`） |
| `<out>/rust-shots/` | rust 腿采集点 PNG |

## 2. 报告字段

```jsonc
{
  "input":   {"inputlog": "...", "frames": 1200, "range": "0400-04FF", "tag": "ens"},
  "coverage":{"emu_frames": 1200, "rust_frames": 1179, "compared": 1179,
              "rust_missing": [2,4,5,6,9,10,11,19, ...]},
  "compare": {"range": "$0400-$04FF", "compared": 1179, "diverge_frames": 0,
              "first_divergence": null,          // 或 {frame, field:"$0412", offset, emu, rust, diff_bytes}
              "frames": [{"frame":1,"equal":true,"sha256_emu":"…","sha256_rust":"…","joy_equal":true}, …]},
  "shots":   [{"frame":300,"equal":true,"pixel_diff":0,"sha256_emu":"…","sha256_rust":"…"}, …]
}
```

- 内存判等面按 `--range` 取（默认 `0400-04FF` = 敌人槽页），逐帧算 sha256；`--tag` 是 emu 侧 `watch_range` 标签。
  **注意**：`$0400-$04FF` 在关卡早期（本样例前 1200 帧的大部分）常年全零，逐帧 sha 会反复是同一值，
  「0 分歧」在那段的信息量有限；M1 的判等面由调用方用 `--range` 指定（例如加上 `$0300-$030F` 的 Samus 面）。
- 按键面固定 `$0014 Joy1Status`，但**两侧采样时刻差一帧**：
  emu 的 `Joy1Status[f]` 是**应用前**（等于 inputlog 第 f−1 行），rust 的 `zp[0x14][f]` 是**应用后**（第 f 行）。
  报告里的 `joy_equal` 已按 `emu[f] ↔ rust[f−1]` 对齐；`joy_raw_equal` 保留原样比，只作溯源、**不是分歧判据**。
  `f=1` 没有 rust 的 f=0 行 ⇒ `joy_equal=null`（表中显示「—」）。
- 画面面只覆盖 `--capture` 声明的帧；`equal` 以**像素**为准（两侧 PNG 编码器不同，文件 sha 只作溯源）。

## 3. 依赖与耗时

- 依赖：`tools/movie_ref/movie_sched.py`（inputlog → 场景）、`tools/scenario_run.py`（Mesen2 腿）、
  `cargo run -p game-core --bin rust_screenshot`（Rust 腿）、Pillow（采集点像素比对）。
- 实测（本机，`eien86-6725M` 前 1200 帧、5 个采集点）：**端到端 32.6 s**（emu 腿约 30 s，rust 腿约 2 s）。
- Mesen 单实例：并发跑两实例需靠 `scenario_run.py` 的 per-run 配置域隔离（#26）；否则后连进程会静默转交并 `exit_group(0)`。

## 4. 确定性

同一输入连跑两次，`report.json` **逐字节一致**。已验证两组：120 帧/3 采集点（`0832dbf4…`），
以及 1200 帧/5 采集点分别输出到 `analysis/tas-m1/eien86-1200` 与 `/tmp/tasv2`（两份 sha 均 `e86b457f…`，
说明报告里没有绝对路径）。
报告不含时间戳、绝对路径、随机 id；`shots` 只放 sha 与像素差。

## 5. fail-loud（不静默）

以下情况直接非零退出并打印原因，不会输出「全绿」报告：

- inputlog 不存在 / movie_sched 任一侧失败 / emu 腿 rc≠0（超时也在内）；
- emu 逐帧 trace **缺帧**（`analysis/traces/<name>.frames.jsonl` 少帧）；
- emu 观察面缺失（场景没挂 `watch_range`）或按键面缺失；
- 采集点单侧缺图 / 两侧画面尺寸不等 / rust 状态行缺 `f`/`iram`/`zp`；
- rust 腿 rc≠0 或空跑无状态行。

**Rust 腿未 tick 的帧**（span / 载房窗：引擎一跳多帧，本帧不跑 `tick`）默认**也是 fail-loud**：
不加 `--allow-rust-missing` 时直接 rc=2 并列出前 8 个缺帧——卡面要求「覆盖双侧各 N 帧」，
不允许把「少 21 帧」悄悄算成通过。显式加 `--allow-rust-missing` 才降级为「记入
`coverage.rust_missing`（`rust_missing_policy="listed-not-compared"`）并继续」，
`report.md` 头部会写「本次 N 帧不可比……以 --allow-rust-missing 放行」，`compared` 只算两侧都有记录的帧，
**缺口永远不会被算成一致**。本样例（放行后）：emu 1200 帧、rust 1179 帧、可比 1179 帧、`diverge_frames=0`。

另外 `--frames` 超过 inputlog 行数会在**建场景前**直接 `ReplayError`（rc=2），不会挂到 emu 超时。

## 6. 差异检测自测

```sh
python3 tools/tas_replay_diff.py selftest
# [selftest] OK 阴性 0 分歧；阳性首分歧 f33 $0405 emu=27 rust=26
```

自测合成 60 帧、每帧 16 B 判等面：先验证「无差异不报」，再在第 33 帧第 5 字节翻一位，
断言报告给出 `frame=33 / field=$0405`，否则非零退出。这是对「检测器本身失效」的负向保障。

## 7. 已知限制

- **端口 2 不可驱动**（`tools/scenario_capture.lua:1109-1120`）：含 2P 输入的片段（如 any% 的 `PauseMode` A+UP）
  只能比 1P 面；`joy2` 不进本报告的判等。
- **首帧画面相位**：`f1` 采集点 rust 腿没有「上一帧缝快照」可复用，与 emu 的 f1 全屏差（示例 61440 px）；
  逐帧内存面与 f≥2 的画面不受影响。判首分歧时请从 f2 起看画面面。
- **画面面是采样点，不是逐帧**：逐帧截图成本高（1200 张）；内存面才是逐帧全覆盖。
  需要更多画面点就多加 `--capture`。
- **不涉及判定阈值**：工具不给「通过/失败」结论，避免把 Human 尚未定的判等粒度写死。

## 8. 整片（M2）：28690 帧、分段与覆盖率

一次跑完整片（`--frames` 省略即取 inputlog 全长；`--chunk` 只切**报告**，见 8.2）：

```
time python3 tools/tas_replay_diff.py replay \
  --inputlog movies/eien86-6725M.inputlog --allow-rust-missing \
  --chunk 5000 --name t33-full --out analysis/tas-m1/t33-full
```

### 8.1 实测（2026-11-24，单机，Mesen GUI 腿 + Rust release 腿）

| 跑法 | 帧数 | wall | 备注 |
|---|---|---|---|
| 1200 帧 / 5 采集点 | 1200 | **32.6 s** | M1 基准 |
| 3000 帧（chunk 3000） | 3000 | **1 m 04 s** | 分段等价样本 |
| 整片（chunk 5000） | 28690 | **8 m 27 s** | 一次跑通，rc=0 |

整片覆盖率（`report.json → coverage` / `compare`）：

- `emu_frames=28690`、`rust_frames=28580`、`rust_missing=110`（span/载房窗一跳多帧，逐帧列出）、`compared=28580`。
- `diverge_frames=27297`、`equal=1283`、**首分歧 `f1305 $0456`：emu=`0x00`、rust=`0x27`**（slot5 `$0450`+6 = `DATA06`，三名同址 `chunkyExplosionTimer / movementInstrIndex / speedSubPixelY`，此处按 `speedSubPixelY` 读），`diff_bytes` 全程 1–25、其中 **495 帧只差 1 字节**，末帧仍差 18 字节。
- 读法：**一次早期字节差会让该槽后续坐标/动画全都不同**，所以「27297 帧分歧」不是 27297 个独立问题，正确读法是「首分歧 f1305，此后再无整页相同的帧」。报告已把 `first_divergence` 单列，逐帧表保留 `diff_bytes` 便于区分「仅 1 字节」与「多槽级联」。
- 产物体积：`report.json` 10.8 MB、`report.md` 1.6 MB、`rust.state.jsonl` ≈600 MB、`emu-traces/*.frames.jsonl` 数十 MB。整片跑完请按需清理 `<out>`（`analysis/` 在 gitignore 内）。

### 8.2 「分段」的两种含义（别用假分段）

- **报告分段（推荐，`--chunk N`）**：同一趟回放跑完，把逐帧表切成 `parts/part-<lo>-<hi>.json`（含 `diverge_frames` 与 `first_divergence`）+ `parts/index.json` 汇总。内存与总耗时都和整段一致，只是审阅/比对粒度变细。实测：`--frames 3000 --chunk 3000` 与整片跑的前 3000 帧逐行同哈希（见 8.3）。
- **窗口裁剪（`--start S`）**：只裁比较窗口（`coverage.frame_window`），**回放仍从第 1 帧开始**——Mesen CLI 没有 load-savestate/PlayMovie，无法从任意帧起播。因此 `--start` 省不了时间（耗时≈`--frames`，不是窗口长度）。想要「只跑后半段」必须先做 savestate 注入，那是另一张卡。
- 不建议把整片拆成 6 个进程各跑 `--frames 5000/10000/…`：每段都要从第 1 帧重放，总耗时≈Σ段末帧（对 28690 帧是 6 倍多）。

### 8.3 分段等价（实跑）

`--frames 3000 --chunk 3000` 与整片跑的 `parts/part-00001-05000.json` 在 1–3000 帧上逐行比对：

- 两侧行数都是 **2975**（2975 个双侧共有的 tick 帧）；
- `(frame, sha256_emu, sha256_rust, equal)` 四元组集合 **完全相同**（`seg vs full identical: True`、`seg vs part identical: True`）；
- 分歧帧集合一致：两边都是 **1692** 帧（首分歧同为 `f1305 $0456`）。

### 8.4 失败路径清理（实跑，不是口头保证）

`finally` 已包住整个 replay 主体（emu 腿 → rust 腿 → fail-loud → 写报告 → 搬产物），任一处抛错都会删 `analysis/.tas-m1-runs/<name>`。实测：

```
python3 tools/tas_replay_diff.py replay --inputlog movies/eien86-6725M.inputlog \
  --frames 300 --name t33-fail --out analysis/tas-m1/t33-fail
# → [tas] FAIL: rust 腿 14/300 帧未 tick … ; rc=2
ls -A analysis/.tas-m1-runs   # → 空（无残留）
```

越界同理：`--frames 28700` 在建场景前 rc=2，不建 staging。

## 9. 长片分段与续跑（task-45）

100% 影片（`movies/lordtom-metroid-100p.inputlog`，120540 帧）单跑约 53 分钟（实测 emu ≈38 帧/秒）。
分段模式让它可中断、可续跑，并把「跑到哪、哪段开始不一致」写进机读报告。

### 9.1 一条命令

```
# 从头按 10000 帧/段推进（每段自己落下一段入口检查点）
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 120540 --segments 10000 --out analysis/tas-100
# 中断后继续（跳过 coverage.json 里 status=pass 的段）
python3 tools/tas_replay_diff.py replay ... --resume
```

### 9.2 分段怎么工作

- **emu 腿按段跑**：段 k 覆盖 `[a,b)`，场景 = `snapshot checkpoints/cp-<a>` + `resume_at <a>`，
  段内 `sample_point spin-exit` + `bundle_capture <b> checkpoints/cp-<b>` ⇒ 段末检查点就是下一段入口，
  不需要额外的「先扫一遍所有检查点」pass。上电第一段没有 `snapshot`。
- **emu 腿多跑 1 帧**（`frames = b+1`，capture 目标取 `b`）：`bundle_capture` 在 `frames == target`
  的 run 里**不触发**（那一帧的 spin 出口落在收尾之后，实测 seg 链因此断在第三段）。
- **rust 腿不分段**：全片连续跑一次（1200 帧 1.5s；120540 帧对这个工具量级可忽略），行号即绝对帧。
  原因见 9.4 第 1 条。
- 产物：`emu/seg-<a>-<b>.jsonl`（绝对帧）、`rust/full.state.jsonl`、`rust/full.sched`、
  `checkpoints/cp-<f>.{mss,json}`，汇总 `coverage.json` + `coverage.md`。

### 9.3 实测（本卡跑出的数字）

- **1200 帧单段 vs 单段模式**：段 `[0,1200)` status=pass、compared 1179、diverge 0，
  双方聚合 sha 都是 `1bb9ddea68a45140…`（单段模式同窗口用 `compare_window(..., rust_base=0)` 复算得同一 sha）。
- **2100 帧 / 3 段 × 700**：seg0、seg1 pass；seg2 在 **f1787** 出现 1 字节分歧
  （`$0429`：emu=0x02 / rust=0x01）。
- **该分歧不是分段引入的**（三角验证）：从 cp-1400 续跑的 emu 与「emu 连续跑到 2100」**696/696 帧一致**；
  而「emu 连续 vs rust 连续」首分歧同样是 **f1787**（此后 313 帧不一致）。
  ⇒ 这是真实的 emu↔rust 行为分歧，属后续差分卡的输入，不是长片工具的假阳性。

**整片跑通（120540 帧 / 10000 帧一段 / 13 段，`analysis/tas-100`）**：

- 覆盖率 `ran_frames=120540`（100%），13/13 段执行完，无段失败（`失败段 0`）。
- 每段 ~178s（末段 20.7s），13 段合计 **2159.8s ≈ 36 分钟**，与「≈56 帧/秒」的口径一致。
- 逐段 `status=diverge`：seg0 是真实分歧（f1787），seg1–12 全部标「承接已分歧」——
  分叉一旦发生不会自愈，后续段的分歧帧数只表示漂移规模（如 seg11 单帧最多差 26 字节），
  不是 12 个新缺陷。报告用 `coverage.json` 的 `first_divergence` 字段统一出口。
- 中断可续：DSH 重启打断过一次（已完成 seg0–8），`--resume` 现在把 `pass` 与 `diverge`
  都当已完成、只重跑 `fail`/缺条目，从 seg9 续上，没重跑前 9 段。
- 退出码口径：只要没有段失败就 `rc=0`；`diverge` 不算失败（分叉不会自愈，后续段必然也
  diverge，把它算失败会让「跑完整片」反而返回非零）。有段失败才 `rc=2`。
- `--out` 允许绝对路径（如 `/tmp/...`）：检查点等产物按可相对化则存相对路径，否则存绝对路径。

### 9.4 两个必须写清的限制

1. **rust 腿不能从存档续跑**。`restore_mss` 只恢复 ram/wram/regs，rust 侧另有不在 NES 内存里的运行态：
   实测「rust 从第 700 帧存档续跑」与「rust 连续跑」在 **f922 起分叉、476/697 帧不一致**；
   同一对 emu 腿（连续 vs 续跑）**0 不一致**。因此分段只作用于 emu 腿。
   要修的是 `crates/game-core` 的恢复面（不在本卡写面），修好后可把 rust 腿也分段。
2. **`bundle_capture` 的边界语义**：目标帧必须真的被「采样点」覆盖，`frames == target` 不触发。

### 9.5 失败语义

- 缺段入口检查点 ⇒ 直接 `ReplayError`（不静默回退上电起跑）。
- 段内 emu 提前退出/超时 ⇒ 该段 status=fail，`failure` 记「已落 trace 的最大帧 + lua.log 末 6 行」，
  后续段停止（它们依赖这段的检查点）。
- 段内出现分歧（status=diverge）**不阻断**后续段：检查点照常产出，coverage 逐段累计分歧帧与首分歧。
- **串行锁排队**：仓库级 `analysis/.scenario.lock` 被别的场景占用时，分段模式不会整链失败，
  而是按 20s × 最多 60 轮排队重试（只对「串行锁被占用/另一场景运行中」这一类错误重试，
  其它错误立即抛，不被重试掩盖）。长片要跑几十分钟，别人一次几十秒的场景不该打断它。

### 9.6 资源与耗时（实测，120540 帧整片）

- emu 腿 ≈56 帧/秒（10000 帧/段 ≈ 179s），整片 13 段 ≈ **36–40 分钟**。
- rust 腿全片连续跑一次 ≈ 2 分钟，但 `--state-out` 每帧写全量 `zp+iram+wram`，
  120540 帧的 `full.state.jsonl` 约 **2.5GB**（`analysis/` 下、不入库）。工具按行流式读取，
  常驻内存 ~100MB；如果把 `state-out` 面收窄（只写判等窗口）可把这一项降到 ~30MB。
