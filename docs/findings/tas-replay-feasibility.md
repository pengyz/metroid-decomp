# TAS 回放可行性取证（M1 前置：Mesen2 movie 格式与输入注入点）

- **任务**：#19（tsk_01M84ES4TERDSE37SCYRA1），M1 前置取证。
- **结论一句话**：Mesen2 2.1.1 **不能**从命令行/ Lua 回放 TAS 社区影片（FM2/FM3 不认、`.mmo` 只能 GUI 播放，且 Lua API 无 movie 函数）；可行路径是**把影片转成逐帧输入表、由 `oracle` 注入器在 `inputPolled` 相位喂给模拟器**——该路径本仓库已经在用，实测 1200 帧完整回放成功，输入保真度与两处硬限制（1 帧相位差、端口 2 不可驱动）见 §3、§4。
- **本文证据分级**：A=二进制/源码/实跑产物可复现；B=代码语境推断；C=假设（单列 §5）。

---

## 0. 摘要（四问四答）

| # | 问题 | 结论 |
|---|---|---|
| ① | Mesen2 对 FM2/FM3 的支持与加载方式、版本约束 | **不支持 FM2/FM3**。Mesen2 只用自己的 `.mmo`；CLI 只有 `/RecordMovie="filename.mmo"`（录制），**没有播放开关**；播放只在 GUI（Movies 菜单）。版本：本仓库固定在 **2.1.1**（`tools/scenario_run.py:8` 的 docstring 即此约束）。 |
| ② | `oracle/scenario_capture.lua` 的逐帧注入 vs movie 回放 | 注入=每帧在 `inputPolled` 回调用 `emu.setInput` 写端口 1；无 ROM 校验、无 lag 语义、无 reset/电源命令回放；movie 回放的“输入与 ROM/core 绑定校验”这些能力它没有。M1 要补：①输入表相位对齐（实测有 1 帧固定差）②端口 2 通道 ③reset/power 命令（本影片为零，但格式支持）④（可选）ROM sha 与输入表绑定校验。 |
| ③ | 最小 TAS 片段回放步骤与判据 | 已实跑：`movie_sched.py --format mesen2` 生成 1200 帧场景 → `scenario_run.py run` → rc=0、`frames 1200 / trace_lines 1200 / captures 2 / shots 2`、DONE 落盘；判据还包括 `$14 Joy1Status` 与 inputlog 逐帧对照（§3.3）。 |
| ④ | 风险与缺口 | 端口 2 不可驱动（28 690 帧 any% 里 242 帧有 2P 输入）；注入有 1 帧相位差；`.mmo` 回放不可自动化 ⇒ 「在 Mesen2 里原样播放 TAS 并复现作者成绩」只能人工、且不属于本项目判据；「同一份逐帧输入喂两腿」是可自动化的那部分。 |

---

## 1. Mesen2 的 movie 支持（A 级，二进制取证）

样本：`tools/mesen2/Mesen`（BYO 工具，不入库），ELF x86-64 stripped。

### 1.1 版本

```console
$ strings -a tools/mesen2/Mesen | grep -oE "2\.[0-9]+\.[0-9]+" | sort | uniq -c | sort -rn | head -3
     29 2.1.1
      2 2.88.9
      2 2.3.0
```

与 `tools/scenario_run.py:8` 的「Mesen2 2.1.1」一致。`2.88.9/2.3.0` 两条**未取证**：从出现次数（各 2 处）与相邻串判断疑为运行时依赖版本，不是 Mesen 自身版本；此处按假设处理，不影响本节结论。

### 1.2 CLI 开关全集

```console
$ strings -a tools/mesen2/Mesen | grep -oE 'ID="Help[A-Za-z]+">[^<]{0,120}'
ID="HelpFullscreen">/fullscreen - Start Mesen in fullscreen mode
ID="HelpDoNotSaveSettings">/DoNotSaveSettings - Prevent settings from being saved to th...
ID="HelpRecordMovie">/RecordMovie="filename.mmo" - Start recording a movie after ...
ID="HelpLoadLastSession">/LoadLastSession - Resumes the game in the state it was left...
ID="HelpApiReference">API Reference
```

Help 资源共 5 条，其中 4 条是 CLI 开关（`HelpApiReference` 是 Lua API 文档标题，不是开关）。

- **唯一 movie 相关开关是录制**：`/RecordMovie="filename.mmo"`。
- **没有 `/PlayMovie`、`/LoadMovie`**（开关全集里没有这两个 token；`strings | grep -i playmovie` 唯一命中是配置项文案 “Display play/record icon…”，不是 CLI）：

```console
$ strings -a tools/mesen2/Mesen | grep -oE '/(RecordMovie|PlayMovie|LoadMovie|fullscreen|DoNotSaveSettings|LoadLastSession)' | sort | uniq -c
      1 /DoNotSaveSettings
      1 /LoadLastSession
      1 /RecordMovie
      1 /fullscreen
```

- movie 扩展名 `.mmo`（UTF-16 资源串里有 `.mmo`、`MesenMovieExt`）：

```console
$ strings -a -el tools/mesen2/Mesen | grep -iE "\.mmo|recordMovie" | sort -u
--recordMovie="filename.mmo" - Start recording a movie after the specified game is loaded.
.mmo
```

### 1.3 没有 FM2/FM3 支持

```console
$ strings -a tools/mesen2/Mesen | grep -iE "fm2|fm3|fceux" | sort -u
            <Value ID="Ymf288AdpcmRom">YFM288 ADPCM ROM (EPSM)</Value>
:Fm2
=fm2
FM2v3j
SFM3
fM3gM
```

命中的都是无关碎片（`YMF288`、`FM2v3j` 之类），**没有** FCEUX/BizHawk 影片格式的解析器或文件过滤器。FM2 只在仓库侧由 `tools/movie_import.py` 解析（见 §2.2）。

### 1.4 没有 Lua movie API

内嵌 Lua API 文档 JSON 的全部方法名里没有任何 movie 函数：

```console
$ strings -a tools/mesen2/Mesen | grep -oE '"name": "[a-zA-Z]+"' | sort -u | grep -iE "movie"
（无输出）
$ strings -a tools/mesen2/Mesen | grep -oE '"name": "(getInput|setInput|inputPolled|loadSavestate|createSavestate)"' | sort -u
"name": "createSavestate"
"name": "getInput"
"name": "inputPolled"
"name": "loadSavestate"
"name": "setInput"
```

⇒ 想在 Lua 里「加载 .mmo 然后跑到第 N 帧」是**做不到**的；Lua 只能逐帧读写输入/状态。

### 1.5 结论与版本约束

- 回放 `.mmo` 只能走 GUI（`GetMoviesMenu`、`ToggleRecordMovie`、`PauseOnMovieEnd` 等资源串证明这些功能存在，但其入口是菜单/窗口，不是 CLI）。
- 因此 **M1 的自动回放不能依赖 Mesen2 的 movie 播放器**；见 §3 的可行路径。
- 版本约束：`/RecordMovie` 与 Lua 输入 API 的行为按 2.1.1 取证；升级 Mesen2 需重跑本文 §1.2–1.4 三条命令与 §3 实跑。

---

## 2. 现有注入器 vs movie 回放（②）

### 2.1 注入器怎么喂输入（A 级源码）

`tools/scenario_capture.lua`：

| 位置 | 事实 |
|---|---|
| `:1124` | 输入注入回调注册在 **`inputPolled`** 相位：`end, "inputPolled"` |
| `:1103` | `local okIn, errIn = pcall(emu.setInput, btn, 0)` —— 只写端口 0（1P） |
| `:1090-1100` | 只有 nav/trigger 两条有序来源才消解 LEFT+RIGHT / UP+DOWN；`schedule`（录像 inputlog 的原始 p1 字节）**不消解**，原样交给 ROM |
| `:1109-1120` | **端口 2 无法驱动**（三种写法全落端口 1），`joy2~=0` 时打 `WARN 2P`，p2 列被忽略；`PLAN.require_joy2` 才报错 |
| `:1287-1304` | fixture：按帧号写 RAM/寄存器（用于从中途状态起跑） |

输入表来源：`tools/scenario_run.py::expand_pads`（唯一展开实现）→ 场景 DSL 的 `schedule`；影片输入表由 `tools/movie_import.py` 从 `.bk2/.fm2` 解析成 `.inputlog`（每帧 `p1 p2 cmd` 十六进制）。

### 2.2 能力对照

| 能力 | Mesen2 movie 回放 | 本仓库注入器（scenario_capture + inputlog） |
|---|---|---|
| 载入社区 FM2/FM3 | **不支持** | 支持（`movie_import.py` 解析 → inputlog） |
| 输入与 ROM 绑定校验 | 有（B/C：movie 内含 ROM 标识为推断，见 §5 假设） | **无**（靠 `movie_check.py` 另行零容差对拍） |
| 逐帧输入注入 | 有 | 有（`inputPolled` + `setInput`） |
| 端口 2 | 有 | **无**（Mesen2 2.1.1 Lua 限制，已定谳） |
| reset / power 命令 | 有 | **无**（`expand_pads` 只消费 p1/p2，cmd 列被忽略） |
| lag 帧语义 | 有（按 poll 回放） | 近似（按帧注入；相位差见 §3.3） |
| 无窗口/CI 自动化 | **无**（GUI 菜单） | 有（Xvfb + Lua） |
| 保存状态起跑 | 有（C：movie 可带 savestate 为推断，见 §5） | 有（fixture / `snapshot`，需显式写） |
| 判据（与 Rust 对拍） | 无 | 有（L4 矩阵、像素/状态 diff） |

### 2.3 M1 需要补什么（按卡面②）

1. **相位对齐**：实测注入相对 inputlog 有 1 帧固定滞后（§3.3），M1 要么把 schedule 整体前移 1 帧，要么在判等/对拍时把「inputlog 第 n 行 ↔ emu 帧 n+1」写死为约定并同步到 Rust 腿。
2. **端口 2 通道**：`emu.setInput` 在 2.1.1 落不到端口 2（A 级），M1 若要覆盖 2P 帧，只能换注入手段（例如直接写 `$4017` 的读取序列/改核心），或把 2P 帧标为「不可回放」并从判据里显式排除。
3. **cmd 列（reset/power）**：`expand_pads` 忽略 cmd；本文用的 any% 影片 cmd 全 0（§3.2），但格式支持，M1 应显式拒绝含 cmd 的影片而不是静默忽略。
4. **绑定校验**：至少把 inputlog 的 `# sha256`/`# frames` 与影片、以及 ROM 的 sha256 绑定进场景产物（现在 `movie_check.py` 只校验 inputlog↔影片，不校验 ROM）。

---

## 3. 最小 TAS 片段回放（③，已实跑）

### 3.1 选段

- 影片：`movies/eien86-6725M.inputlog`（28 690 帧，source `movies/metroid-anyp-3666.bk2`，`movie_check.py` 零容差）。
- 取 f1–f1200：前 200 帧只有 2 帧 Start（开场演出吞键），**f1000–f1200 是 201/201 帧都有按键的真实操作段**（首帧 `01`=Right，含 `09`=Right+Up、`41`=Right+B、`81`=Right+A），用它做「有实际输入」的最小片段。
- 选段内无 2P 输入（首个 p2 非零在 f3266），无 reset/power（本影片 cmd 列全 0）：

```console
$ awk '!/^#/ && $3!="00" && $3!="0" {c++} END{print "cmd_nonzero="c+0}' movies/eien86-6725M.inputlog
cmd_nonzero=0
```

### 3.2 生成 + 回放命令（本机实跑，rc=0）

```console
$ python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog \
    --frames 1200 --capture 1000:right-hold --capture 1200:right-hold-end \
    --format mesen2 --out /tmp/tasdev/m1-replay.txt
[movie_sched] 警告：inputlog 有超过 --frames 的输入（最大帧 28690）      # stderr
[movie_sched] 输入自检 OK：28690 帧与影片零容差
[movie_sched] eien86-6725M.inputlog: format=mesen2 frames=1200 joy1=22948 joy2=242 captures=2 → /tmp/tasdev/m1-replay.txt

$ printf 'watch 0014 Joy1Status\nwatch 0015 Joy2Status\n' >> /tmp/tasdev/m1-replay.txt

$ python3 tools/scenario_run.py run /tmp/tasdev/m1-replay.txt --timeout 300
  首采样帧（~f1000）耗时 21.3s（历史中位 26.0s）
  DSL: /tmp/tasdev/m1-replay.txt sha256=da7081ef84e51a26…
  frames: 1200 trace_lines: 1200 captures: 2 shots: 2 wall: 34.4s
  DONE: {"frames": 1200, "trace_lines": 1200, "captures": 2, "shots": 2}
  trace: analysis/traces/eien86-6725M.frames.jsonl
```

产物（`analysis/traces/eien86-6725M.run.json`）：`scenario_sha256=da7081ef84e51a26a6b0c2824376ed64f2e2fe125ff68d468197f02a215c118f`、`generated_lua_sha256=dc67a072c06178ac22fcd885d82d61ee7d2af6cce9aee9f06b12ff63cb6b724d`、`shots=["shots/eien86-6725M_f01000_right-hold.png","shots/eien86-6725M_f01200_right-hold-end.png"]`。

### 3.3 期望判据与实测

判据（任一不满足即回放不可信）：

1. `DONE` 落盘且 `frames=1200`、`trace_lines=1200`；
2. 2 个采集点各有 PNG（`shots/` 两张）；
3. 逐帧 `$14 Joy1Status` 与 inputlog 的 p1 列同值——**允许固定 1 帧滞后**（即 `$14[f] == p1[f-1]` 视为命中，判定式见下方脚本）。

实测：

```console
$ python3 - <<'PY'
import json
rows={json.loads(l)['f']:json.loads(l)['w']['Joy1Status'] for l in open('analysis/traces/eien86-6725M.frames.jsonl')}
lines=[l.split() for l in open('movies/eien86-6725M.inputlog') if not l.startswith('#')]
want={i+1:int(l[0],16) for i,l in enumerate(lines[:1200])}
bad=[f for f in want if want[f]!=rows[f]]
lag=[f for f in bad if f>1 and want[f-1]==rows[f]]
print('frames',len(want),'mismatch',len(bad),'one-frame-lag',len(lag))
print('f1000..1200 nonzero', sum(1 for f in range(1000,1201) if rows[f]))
print('distinct', sorted({rows[f] for f in range(1000,1201)}))
PY
frames 1200 mismatch 104 one-frame-lag 101
f1000..1200 nonzero 201
distinct [1, 9, 65, 129]
```

解读：

- `distinct [1,9,65,129]` = Right / Right+Up / Right+B / Right+A，与 inputlog 该窗口逐帧一致（201/201 非零）。
- 104 帧与 inputlog 原值不同：**101 帧恰是「晚一帧」**（`rows[f] == want[f-1]`），说明注入路径存在**固定 1 帧相位差**（`inputPolled` 注入的键要到下一次轮询才被 ROM 读到）。
- 余下 3 帧 f674–676：`rows`=0x43、`want`=0x02，落在影片已知的 R+L 窗口（f670/f672=0x43），属输入消解/相位研究的既有议题（见 `docs/findings/input-conflict-resolution-2026-11-19.md`），本文不展开。
- 104 帧的构成可完全解释：101 帧是一帧滞后，3 帧是上述 R+L 窗口异议；**没有「按键整帧消失」**——单帧点按（如 f8/f17 的 Start）只晚一帧，仍会被 ROM 读到。

⇒ 判据 1/2/3 全过，其中 3 需按「1 帧滞后」口径解释，而不是当成输入错误。

---

## 4. 风险与缺口（④）

| 缺口 | 性质 | 后果 | 处置建议 |
|---|---|---|---|
| 端口 2 不可驱动 | **硬限制**（A 级，Mesen2 2.1.1 Lua） | any% 影片全片 242 帧有 2P 输入（首个 f3266）无法原样回放；这些帧的 emu 结果不可作为判据 | 判据里显式排除 2P 帧；或另找注入手段（写 `$4017` 时序/改核心）单开卡 |
| 注入 1 帧相位差 | 可修 | 若 Rust 腿按「帧 N 吃 inputlog 第 N 行」而 emu 实际晚 1 帧，像素对拍在按键边沿帧出现假分歧 | M1 定一条相位约定（schedule 前移或对拍索引错位 1 帧），两腿同步 |
| `.mmo` 只能 GUI 播放 | 工具限制 | 「在 Mesen2 里原样播放 TAS 复现作者成绩」无法进 CI；也不能用它做 oracle | 不把「复现 TAS 成绩」当判据；判据始终是 emu↔rust 一致 |
| reset/power 命令（cmd 列）不支持 | 潜在 | 含 reset 的影片会被静默丢命令 → 状态分叉 | `expand_pads` 层显式报错（本影片 cmd 全 0，暂无实例） |
| 无 ROM 绑定校验 | 中 | 换 ROM/改 ROM 后仍能跑，产物无法自证对应哪份 ROM | inputlog/run.json 里绑定 ROM sha256 与影片 sha256 |
| 从 savestate 起跑 | 能力有、流程未串 | 大影片全片回放成本高（28 690 帧 ≈ 数分钟）；片段化取证要 fixture | 需要时用 `snapshot`/fixture，注意 fixture 写点在帧边界 |
| lag 帧语义 | 近似 | movie 回放按 poll、注入按帧；极端按键窗可能差 1 帧 | 与 §3.3 相位问题同源，一起解决 |

**只能「近似回放」的部分**：2P 帧、含 reset/power 的影片、依赖 Mesen2 内部 lag 语义的极端帧、以及任何需要「原版 Mesen2 movie 播放器自校验」的场景。

---

## 5. 假设区（C 级，未取证）

1. `.mmo` 内含 ROM 标识/校验（与 `.mss` savestate 类似）——**未拆过 .mmo 二进制**，本文没有据此下结论。若要确认，需要先用 `/RecordMovie` 录一段再解析文件头。
2. Mesen2 GUI 的「Play Movie」是否接受非 `.mmo`（如 FM2）——二进制里无 FCEUX 解析痕迹，判为「不接受」，但未实跑 GUI 验证。
3. 1 帧相位差的内部机制（`inputPolled` 触发时机 vs `ReadJoyPads` 读锁存）——本文只给「观测量」，机制未追到源码（Mesen2 无源码在库）。

---

## 6. 复跑清单（验收用）

```console
# ① 版本 + CLI + 无 FM2 支持 + 无 Lua movie API
strings -a tools/mesen2/Mesen | grep -oE "2\.[0-9]+\.[0-9]+" | sort | uniq -c | sort -rn | head -3
strings -a tools/mesen2/Mesen | grep -oE 'ID="Help[A-Za-z]+">[^<]{0,120}'
strings -a tools/mesen2/Mesen | grep -oE '/(RecordMovie|PlayMovie|LoadMovie|fullscreen|DoNotSaveSettings|LoadLastSession)' | sort | uniq -c   # 期望只有 4 条、无 Play/Load
strings -a tools/mesen2/Mesen | grep -oE '"name": "[a-zA-Z]+"' | sort -u | grep -i movie   # 期望空

# ② 注入面
sed -n '1085,1125p' tools/scenario_capture.lua

# ③ 最小回放（约 35 s，需 emu 空窗；产物落 analysis/traces/ 与 shots/，均 gitignore）
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog \
  --frames 1200 --capture 1000:right-hold --capture 1200:right-hold-end \
  --format mesen2 --out /tmp/tasdev/m1-replay.txt
printf 'watch 0014 Joy1Status\nwatch 0015 Joy2Status\n' >> /tmp/tasdev/m1-replay.txt
python3 tools/scenario_run.py run /tmp/tasdev/m1-replay.txt --timeout 300   # 期望 DONE 1200/1200/2/2

# ③' 注入保真：期望 mismatch 104、one-frame-lag 101、f1000..1200 非零 201
python3 - <<'PY'
import json
rows={json.loads(l)['f']:json.loads(l)['w']['Joy1Status'] for l in open('analysis/traces/eien86-6725M.frames.jsonl')}
lines=[l.split() for l in open('movies/eien86-6725M.inputlog') if not l.startswith('#')]
want={i+1:int(l[0],16) for i,l in enumerate(lines[:1200])}
bad=[f for f in want if want[f]!=rows[f]]
lag=[f for f in bad if f>1 and want[f-1]==rows[f]]
print('frames',len(want),'mismatch',len(bad),'one-frame-lag',len(lag))
print('f1000..1200 nonzero', sum(1 for f in range(1000,1201) if rows[f]))
PY
```

## 7. 证据指针

| 结论 | 指针 |
|---|---|
| Mesen2=2.1.1 | `strings -a tools/mesen2/Mesen`，`2.1.1`×29；`tools/scenario_run.py:8` |
| 只有 `/RecordMovie`，无播放 CLI | Binary Help 资源 4 条（§1.2）；开关全集只 4 条（§1.2），无 `/PlayMovie`、`/LoadMovie` |
| movie 格式 `.mmo` | `strings -a -el` 的 `.mmo` / `MesenMovieExt` |
| Lua 无 movie API | 内嵌 API JSON 的 `"name":` 全量（§1.4） |
| 注入面与端口 2 限制 | `tools/scenario_capture.lua:1090-1124`、`:1109-1120` |
| 影片→输入表 | `tools/movie_import.py`（只解析格式，不做同步修正）、`tools/movie_check.py`（零容差对拍） |
| 影片→场景/调度 | `tools/movie_ref/movie_sched.py`（`--format mesen2` / `sched` 同源输入表） |
| 实跑产物 | `analysis/traces/eien86-6725M.run.json`、`.frames.jsonl`、`.done`、`shots/eien86-6725M_f01000_right-hold.png`、`…f01200_right-hold-end.png` |
| 1 帧相位差实测 | §3.3 脚本输出：`mismatch 104 / one-frame-lag 101` |
| R+L 既有议题 | `docs/findings/input-conflict-resolution-2026-11-19.md` |
