# 「TAS 对拍可玩」验收口径、现状差距与里程碑

> 状态：草案（2026-11-23），作者：对影（QA/验收）。本文只定口径与拆分，不改代码/门禁/台账。
> 目标来源：Human 的总目标「完成银河战士的 Rust 转写 + 校验，代码架构符合 Rust 习惯，验收标准是 TAS 对拍可玩」。
> 证据纪律：每条结论带可复跑命令或仓库指针；**没跑过的写「未实测」**；不确定项进 §5 假设区并标 C 级。
> 两个 Human 决策点（TAS 用哪套 / 判到什么粒度）已在 §1.1、§1.2 给出推荐默认，标「待拍板」；拍板前 M1–M4 的数字按推荐默认生效。

## §0 工作组织与验收粒度（后续所有执行卡的派生规则）

这一节是 Human 2026-11-23 的补充要求：先定「怎么组织工作、怎么验收」，再谈具体数字。所有由本文派生的执行卡必须满足：

1. **卡的粒度 = 一个可独立复跑的判定点。** 一张卡要么「一条命令 + 一个期望数字」（如 `run case-x → 14/14 diff=0`），要么「一份带 A/B 级证据指针的文档结论」。判不了就拆卡。
   拆分按**根因**，不按现象：例如 L4 input-conflict 三格不是拆成三张卡，而是按「采集缝晚拍」「载房窗精灵页切写」两条根因各一张卡。
2. **一卡一写面，writeScopes 不重叠。** 同目录的卡用 `blockedBy` 串行，不并发写同一目录。emu（Mesen2）全队单实例：用前在 thread 报「谁在跑」，跑完报「已释放」；`rc=0` 但产物目录没有 `frames.jsonl`/`DONE` 视为被占，不得当作成功。
3. **三级验收，缺一级不算完成：**
   - 卡级：`qaChecklist` 逐条过，**验收人独立跑**，未跑项在提交里显式声明「未跑」；
   - 步骤级：按改动层选门禁——改参考投影跑 `seed`、改 game-core/game-data 跑 `trace`、改渲染跑 `visual`、任何提交前跑 `check`；
   - 里程碑级：L4 172 格 ratchet、room_sweep 三区 + 对照区、**TAS 整片回放（本文新增）**。
4. **已知差异只进台账/基线。** 已知差异写入 `docs/bug-ledger.md`（`BUG-xxx`）或基线 `TOLERATED` 项；阈值/白名单**只增不改**，动白名单必须 Human 同意。
5. **退回必须带 note。** 哪条判据没过、期望值 vs 实际值，写进卡 thread；不许口头/隐含退回。
6. **PMO 边界。** 主理人不实现、不验收，只负责拆解、派活、流转。

## §1 口径定义

### §1.1 TAS 素材：用哪几份、来源与帧数

仓库现有素材（`movies/`，仓内不含 ROM）：

| 素材 | 影片/作者 | 来源 | 帧数 | 结构 | 当前可用性（实测） |
|---|---|---|---|---|---|
| any% 3666 | 《NES Metroid any% (obsoleted)》/ The8bitbeast | https://tasvideos.org/3666M | 28,690 | bk2(NesHawk)，P1 起 f7，P2 起 f3265 | 输入表可用（见下），但文件名/来源记录错位 |
| 100% 2305 | 《Metroid 100% (Lord Tom, 33:25.70)》 | https://tasvideos.org/2305M | 120,540 | fm2(FCEUX 9816)，P1 起 f8，P2 起 f22449，reset 1 次 | **今天无法整片回放**：自检工具不支持 fm2 |
| eien86 28,408 帧 | eien86/alyosha/the8bitbeast 合片 | 仓内 provenance 记源为 `/tmp/m6725/…bk2` | 28,408 | 仅 .inputlog，无 .bk2 | 文件内容实为 any% 3666 的输入（见下） |

素材的机械身份（`python3 tools/movie_check.py movies/metroid-anyp-3666.bk2 movies/eien86-6725M.inputlog`）：

```
OK 28690 帧 P1 全部一致
```

即 `movies/eien86-6725M.inputlog` 的 28,690 帧与 any% 3666 的 bk2 **零容差**，它才是 any% 整片输入表；但它的文件名、`data/status/movie_ledger.json` 里的 provenance（28,408 帧、`/tmp` 来源、sha 4421f6e3…）都指向另一部电影。同时 `movies/metroid-anyp-3666.inputlog` 本体只有 3,260 行，是旧截断：

```
python3 tools/movie_ref/movie_sched.py --inputlog movies/metroid-anyp-3666.inputlog --frames 300 --out /tmp/x.sched
→ movies/metroid-anyp-3666.inputlog: 行数 3260 ≠ 影片 28690（旧产物或截断）   （exit 2）
```

两份 any% 输入表前 **3,260 帧**（帧级、0-based、跳过 9 行 `#` 头）有 **1,993 帧**不同，首差 **f=227（0-based）**；旧表是脏的，**不得使用**。注意别直接按原始文本行比：inputlog 是 `<p1> <p2> <cmd>` 十六进制行，bk2 是 `|` 分隔的位字段，逐行文本比会得到另一组数，不是本口径。
100%/lagshift 两份 `.inputlog` 头写 `# source movies/lordtom-metroid-100p.fm2`，自检直接崩：

```
python3 tools/movie_ref/movie_sched.py --inputlog movies/lordtom-metroid-100p.inputlog --frames 120540 --out /tmp/y.sched
→ tools/movie_check.py:24 open_bk2 → zipfile.BadZipFile: File is not a zip file
```

原因是 `tools/movie_check.py` 只解析 bk2（zip），不支持 fm2；`movie_sched.py` 的自检是「读 `# source` → 源存在则逐帧零容差比对，源不存在则打印跳过」，没有 fm2 分支。

**选项与推荐（待 Human 拍板）：**

- **甲（推荐，主素材）：any% 3666 全片**，输入表用 `movies/eien86-6725M.inputlog`（28,690 帧，零容差已验）。理由：最短通关路径（§3），Rust 腿全片实测 0.39 s 可跑完，emu 腿约 9.7 min（按 20.2 ms/帧实测外推），是「最短反馈环」。
- **乙（推荐，覆盖素材）：100% 2305 全片**，需先补 fm2 自检（或加显式 `--skip-source-check` 并记录 sha），P2 段（f22,449 起）单独定 2P 口径。
- **丙（不推荐）：eien86 28,408 帧合片**——仓内只有 `.inputlog`，无影片可自检，身份与品牌都存疑。

许可与引用：TASVideos 影片页公开（3666M/2305M），仓内保留来源 URL 与 sha256（`data/status/movie_ledger.json`）；本文只作验收素材引用，不再分发影片本体。

### §1.2 对拍粒度：比哪些面、按什么规则判等

**逐帧面**（两侧同形，`crates/game-core/src/memmap/observe/frame_face.rs:32-35` 定义 watch 四段）：

| 面 | 地址 | 字节 | 备注 |
|---|---|---|---|
| zp | `$0000-$00FF` | 256 | |
| iram | `$0100-$07FF` | 1,792 | 与 zp 合计 = 全 RAM 2,048 B |
| wram | `$6000-$7FFF` | 8,192 | **判等只用已 typed 的 224 B（`$6AF4-$6BD3`）**；其余 7,968 B 两侧未建模/未写入，不参与判等 |
| pw | `$6988-$69B3` | 44 | |

**采集点面**（每 N 帧或指定帧）：`*.png`（像素）+ `*.ppu.hex`（PPU 寄存器/名表/调色板/OAM 快照）+ `*.ram.hex`（2,272 B，= 全 RAM 2,048 + typed WRAM 224）+ `*.apu.hex`（音频写桶，仅记录不判等，见 BUG-090）。

**判等规则（必须写进 diff 工具，不许口头裁量）：**

1. 逐帧面：`frame N` 对 `frame N` 逐字节相等。Rust 侧调度表约定「Rust 第 N 帧消费录像第 N−1 帧」（`tools/movie_ref/movie_sched.py` 头注），不许整体平移对齐。
2. 采集点面：像素差 **0** 才算过；已知渲染/采样时序类残差按 `BUG-089` 逐点登记（CHR/名表/属性/调色板四面同才算「非内容分歧」），登记点位的像素差只记账不判失败。
3. 已登记差异清单：`docs/bug-ledger.md` 中 `BUG-001`（morph×楔）、`BUG-002`（Ridley 弹种计时互扰）、`BUG-091`（竖滚房敌可见性轴错配）、`BUG-092`（2P 回密码屏）属**保真复现原版**，正确行为是「和原版一样」，不是容忍；只有 `BUG-089`（渲染/采样时序）与 `BUG-090`（音频输出缺失）是判等豁免。
4. 音频：Stage A 不判 APU 输出波形（BUG-090，lead 已裁定不修）；但**声音引擎对 RAM 的写**仍要判等——写进哪个字节、哪个时序都算游戏状态。
5. 逐帧全量落盘太贵（2,048+8,192+44 B/帧 × 120,540 帧 ≈ 1.2 GB），工具实现用「逐帧指纹（SHA-256）+ 首分歧帧两侧全量 dump + 每 60 帧全量 dump」。指纹输入固定为「全 RAM 2,048 + typed WRAM 224 + pw 44」= 2,316 B。
   口径要说清：2,316 B 是**判等面**（wram 只判 typed 224 B），不是 `--state-out` 的落盘量。实测 `--state-out` 每帧写 `zp` 512 hex（256 B）+ `iram` 3,584 hex（1,792 B）+ `wram` 16,384 hex（8,192 B）+ `pw` 16 hex（8 B）≈ 20,554 B/行，any% 全片 28,580 行 = 587,508,069 B（`/tmp/tas-anyp-full2.state.jsonl`）；行结构 `{"f":N,"rl":载房相位,"w":{zp,iram,wram,pw}}`。

**「可玩」的操作定义（每项可机械判定）：**

| # | 判据 | 判定方法（可复跑） | 当前状态 |
|---|---|---|---|
| W1 | 实时性 | release 二进制全片耗时 ÷ 帧数 ≤ 16.67 ms/帧 | 已满足：28,690 帧 5.22 s（含逐帧状态导出）= 0.18 ms/帧；纯跑 ≈0.3–0.4 s（本机单次 0.39 s，拆骨同机实测 0.31 s，机时波动，不写死单次读数） |
| W2 | 无卡死/崩溃 | 全片 exit 0，抓帧数 = 请求帧数，无 `unported!` panic，gap 采集点 = 0 | 已满足（any%）：57/57 采集点，`缺口阻塞 0 采集点` |
| W3 | 状态机完整 | 末帧 `AT_ENDING($6883)==1` 且进入结局序列；标题→开始→房间转移→通关全链 | **未满足**：any% 末帧 `$6883=0`（详见 §2.2） |
| W4 | 密码/继续路径 | `case-password-justin` 14/14 格零差，且输入密码后进入游戏状态一致 | **未满足**：当前 12/14（f700 23px、f1100 42px） |
| W5 | 人测 | 人类用键盘从开屏玩到结局，无崩溃/卡死；记录关键节点 | 未做（可选，Stage A 收口时做一次） |

### §1.3 失败判据

出现任一条即判「不通过」，并要给出首个分歧帧/格：

- **F1** 逐帧面在未登记位置出现字节差（首个分歧帧 + 字段名 + 两侧值必须报出）。
- **F2** 采集点像素差 > 0，且不在 `BUG-089` 登记点位。
- **F3** 全片运行出现 `unported!` panic、gap（缺口记账）新增，或采集点被阻塞。
- **F4** 帧数不齐 / 卡死：实际推进帧数 ≠ 请求帧数，或全片耗时超预算（W1）。
- **F5** 已知差异未登记：发现新差异，但既没进 `docs/bug-ledger.md` 也没进基线 TOLERATED，就直接改了阈值/白名单。
- **F6** 判等工具本身被放宽（改比对范围、改帧对齐、跳过指纹），视为判据失效，不是「通过」。

## §2 现状差距清单

### §2.1 素材与工具链缺口（每条带复现命令）

| # | 缺口 | 复现 | 影响 |
|---|---|---|---|
| G1 | any% 唯一可用输入表文件名/provenance 错位 | `python3 tools/movie_check.py movies/metroid-anyp-3666.bk2 movies/eien86-6725M.inputlog` → `OK 28690 帧 P1 全部一致`；`data/status/movie_ledger.json` 里 eien86 记 28,408 帧 | 选材人拿错素材；须改名/重落 provenance（新卡） |
| G2 | `metroid-anyp-3666.inputlog` 是旧截断（3,260 行） | `movie_sched.py --inputlog movies/metroid-anyp-3666.inputlog --frames 300` → `行数 3260 ≠ 影片 28690`，exit 2 | 整片回放不可用，必须弃用 |
| G3 | 100% 整片无法生成调度表：fm2 不被自检支持 | 同上命令对 lordtom 100% → `BadZipFile` at `tools/movie_check.py:24` | M4 里程碑被阻塞，须先补 fm2 解析/显式跳过 |
| G4 | TAS 用例不在 `cases/`，L4 矩阵看不见它 | `ls cases/*.txt` 21 个，无 `inputlog` 行；movie 用例在 gitignored `movies/cases/`（例 `case-movie-100p-b26.txt`） | 「TAS 整片对拍」不进任何门禁，只能手跑 |

### §2.2 代码缺口：整片 any% 实测出的「通关必经」面

这里给出**机械口径**，替代手工从 unported 603 里挑「必经」：对整片素材跑 Rust 腿，`DIVERGENCES`（占位臂缺口记账，非致命）的唯一地址集就是这条路线实际踩到的未端口面。

```
cargo build -p game-core --release --bin rust_screenshot     # 构建 provenance 必记，见下
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog --frames 28690 --capture-every 500 --capture-from 400 --out /tmp/tas-anyp-full.sched
target/release/rust_screenshot --schedule /tmp/tas-anyp-full.sched --out /tmp/tas-anyp-full
→ [rust] 完成：产出 57 采集点，缺口阻塞 0 采集点
→ 缺口记账 63235/6 → /tmp/tas-anyp-full/eien86-6725M.divergences.json
```

构建 provenance（2026-11-23 21:29 复核）：HEAD `915763b9` + 工作树（仅 `crates/game-core/src/bin/rust_screenshot.rs` 的注释差异），`target/release/rust_screenshot` sha256 `579a7b93ec218b070548d58fcfc11f24e83ca6a0d2c707dd43a62d812ca80515`，`cargo build -p game-core --release --bin rust_screenshot` 报 `Finished release profile`（0.64 s，无重编）。重建后复跑同一调度表：`real 0.347s`、`count=63235 unique=6`，六地址计数与下表逐项相同——本文的整片读数来自当前代码，不是旧二进制。

`divergences.json`：`count=63235, unique=6`，events 首 tick=466、末 tick=28690。首记帧取 `events` 里该 `addr` 首次出现的 `tick`（`python3 -c "import json;d=json.load(open(…));f={};[f.setdefault(e['addr'],e['tick']) for e in d['events']];print(f)"`，地址键为十进制：$CBEF=52207、$CBF9=52217、$CC36=52278、$CC98=52376、$CE25=52773、$FB7B=64379）。六个地址全部落在 bank7 固定区，逐个落点：

| 地址 | 次数 | 首记帧 | Rust 落点 | 性质 |
|---|---|---|---|---|
| `$CC98` ×10,993 | 最多 | **466** | `crates/game-core/src/samus/physics.rs:1721-1722`（stand→run 转换径） | **逻辑**：站立/跑步状态分叉根（同帧 L4 状态直比：金 status=1+accelX=208，我 status=0+accelX=0） |
| `$FB7B` ×49,029 | 第二 | **466** | `crates/game-core/src/enemy/slot_cycle.rs:156`（d05 bit5 静息位 + 坠落 delay 相位） | **逻辑**：敌槽 bit2/bit5 位源未定，坠落 delay 差 1 帧 |
| `$CC36` ×3,150 | 第三 | **466** | `crates/game-core/src/samus/handler.rs:1225`（Samus 动作表转换） | **逻辑**：与 $CC98 同族的转换/写序面 |
| `$CBEF` ×42 | 42 | **953** | `crates/game-core/src/samus/handler.rs:1492`（SFX_SamusBall） | 音频面（声包未端口） |
| `$CBF9` ×1 | 1 | **685** | `crates/game-core/src/world/update.rs:213`（PowerUpMusic） | 音频面（声包未端口） |
| `$CE25` ×20 | 20 | **2,163** | `crates/game-core/src/samus/physics.rs:1133`（IsSamusDead/CheckHealthBeep 死亡链） | **逻辑**：死亡/低血蜂鸣深件 |

`unported!` 硬桩（`crates/game-core/src/gap.rs:27`，记录后 panic）本次**未触发**：全片 exit 0、`缺口阻塞 0 采集点`。也就是说，对 any% 这条路线，硬桩不是瓶颈，六个软缺口才是。

**通关未证（失败判据 F4 的边界）**：末帧状态 `GAME_MODE($001D)=0`、`X($030E)=04`（`crates/game-core/src/ram_map.rs:163` `SAMUS_X=0x030E`；ROM 侧 `refs/m1disasm/SRC/constants_ram.asm:238` `x db ;$030E`）、`health=$0106=64`、`AT_ENDING($6883)=0`，即**没有到达结局状态**；f28,400 的采集图显示角色仍在中后期房间（不是结局画面）。因此「能跑完 28,690 帧输入」≠「能通关」，W3 明确判未满足。

```
target/release/rust_screenshot --schedule /tmp/tas-anyp-full.sched --out /tmp/tas-anyp-full2 --state-out /tmp/tas-anyp-full2.state.jsonl
python3 - <<'PY'
import json
last=None
for line in open("/tmp/tas-anyp-full2.state.jsonl"): last=json.loads(line)
zp=bytes.fromhex(last["w"]["zp"]); ir=bytes.fromhex(last["w"]["iram"]); wr=bytes.fromhex(last["w"]["wram"])
print(last["f"], "mode=",zp[0x1D], "X=",ir[0x30E-0x100], "health=",ir[0x106-0x100], "ending=",wr[0x6883-0x6000])
PY
→ 28690 mode= 0 X= 4 health= 64 ending= 0
```

### §2.3 unported 603 与「通关必经」的关系

当前命令口径（`python3 tools/routine_inventory.py --stats`，2026-11-23 21:29 复核）：`code 例程 1135 条：ported 380 / candidate 152（13.4%）/ unported 603；data label 2641 条（不计缺口）`。三者互斥：1135 = 380 + 152 + 603；`candidate` 只命中 Rust 符号名、尚未按 asm 复核行为等价，**不得当 ported 用**。
落盘快照 `data/status/routine_inventory.json` 已陈旧（counts={code 1135, ported 380, candidate 147, unported 608}，因门禁重建时 overrides 379 条重算），**引用以 `--stats` 现算为准**，门禁口径 `unported 603（无增长）`。分布前几名：prg7_engine 352、areas_common 65、prg0_title 59、sound_engine 56、prg3_tourian 22、kraid 13、metroid 6。

**不能**从这 603 里手挑「必经」——没有覆盖证据时，任何「这些不用做」都是 C 级判断。可机械化的只有两件事：

1. 对选定素材跑整片，取 `DIVERGENCES` 唯一集（any% = 上表 6 个）与 `unported!` panic 集（any% = 空）；
2. 用 `tools/coverage_from_trace.py` 做 bank 级覆盖（bank7 `$C000-$FFFF` 可精确映射 seed 行；`$8000-$BFFF` 运行时 bank 不明，只按窗口地址计数；`data/status/coverage_union.json` 当前为空，2026-10-14 重置）。

### §2.4 L4 剩余 6 格与 TAS 全片对拍的关系

`data/status/l4_ratchet_baseline.json`：21 案 / 172 格 / 166 match / 6 diff：

| 用例 | 格（帧） | 残差 px | 归属 |
|---|---|---|---|
| case-mechanism-input-conflict | 673 / 674 / 676 | 84 / 84 / 303 | #11 采集缝 + #12 载房窗精灵页切写 |
| case-password-justin | 700 / 1100 | 23 / 42 | #10 密码屏深路 |
| case-scrollseam-probe | 1100 | 266 | #6 Samus 姿态 |

它们是**微场景、单点位**对拍，与「整片逐帧对拍」不是同一粒度、不可互相抵扣：172 格全绿也不证明 28,690 帧逐帧相等。反过来，整片对拍能覆盖它们（微场景输入是整片的子序列），但整片失败时仍要回落到微场景定位。因此 M3（L4 清零）与 M1/M2（TAS 整片）**并列，互不替代**。

### §2.5 覆盖率的算法（把 3.3%/3.6% 说清）

- any%：`frames_synced 1,039 / frames 28,690 = 3.62%`（`data/status/movie_ledger.json` → tasvideos-3666-anyp）；
- 100%：`frames_synced 3,889 / 120,540 = 3.23%`（→ tasvideos-2305-lordtom-100p；`usable_until=3889`，首分歧 f3890，8 px）。

任务描述里的「3.3%/3.6%」即 100%≈3.2%、any%≈3.6%，四舍五入方向不同；本文一律用上式现算。注意这两段「同步」都是**旧口径的窗口内同步**（any% 只用了 3,260 帧窗口，100% 只推到 f3,889），不是整片。

### §2.6 盲区（不修，但必须在收口时声明）

- WRAM `$6000-$7FFF` 8,192 B 中只有 224 B（`$6AF4-$6BD3`）进比对面；其余 **7,968 B 盲**（门禁 `nes-map`：比对面 2,272 B / Unmodeled 611 B / Undeclared 1,287 B）。
- `$8000-$BFFF` 的 bank 归属在运行时不可判（coverage 工具只按窗口地址计），跨 bank 调用路径的覆盖不可精确归因。
- 2P 段（100% f22,449 起、any% f3,265 起）只有单点抽查，未整段对拍。

## §3 里程碑拆分（按「最短可通关路径」排序）

每个里程碑的出口都是「一条命令 + 一个数字」。卡数按现存证据估算，实际拆卡由 PMO 按 §0 第 1 条再切。

| # | 里程碑 | 出口（命令 → 期望） | 预计卡数 | 依赖 / 写面 |
|---|---|---|---|---|
| M0 | 素材定谳（改名/重落 provenance、补 fm2 自检、TAS 用例入 `cases/`） | `movie_sched.py` 对 any% 与 100% 都输出「零容差」且 exit 0；`cases/` 新增 1 个 inputlog 用例可跑 | 1–2 | 无；写面 `movies/`+`tools/`（或 `cases/`+`tools/`） |
| M1 | 整片逐帧对拍工具（emu 全 RAM watch + Rust `--state-out` 指纹 + 首分歧定位） | 命令输出 `frames=28690 first_div=<帧或 none> unexplained=<n>`；首分歧帧两侧 dump 落盘 | 2 | M0；写面 `tools/` |
| M2 | any% 首分歧清零（按根因拆：`$CC98/$CC36` 站立↔跑步、`$FB7B` 敌槽位/坠落 delay、`$CE25` 死亡链；音频两项按 BUG-090 豁免） | 整片 `first_div=none over 28690 frames`，采集点像素 0 | 4–8 | M1；写面 `crates/game-core/src/samus/`、`enemy/`（按根因分卡，不并发同目录） |
| M3 | L4 6 格清零（已有 #11/#12/#10/#6） | `python3 tools/l4_diff_run.py run-all` → diff 0 / 172 格 | 0 新增（已有卡） | 无；与 M1/M2 并列 |
| M4 | 100% 整片（fm2 自检 + f22,449 起 2P 段口径） | `frames=120540 first_div=none`（P1+P2） | 3–5 | M1、M2；写面 `tools/`+`crates/game-core/src/input/` |
| M5 | 可玩证据（W1–W5：状态机到结局、密码 14/14、人测） | 末帧 `AT_ENDING=1`；`run case-password-justin` 14/14；人测记录 1 份 | 2–3 | M2/M3；写面 `docs/findings/` |
| M6 | 架构收口（typed-first 规约、门禁复核） | `python3 gates/cli.py check` → PASS 23/FAIL 0；`no-env-hooks`、`arch-contract`、`data-boundary` 均 PASS | 1–2 | 全文；写面 `docs/design/` |

排序理由：M0/M1 是所有数字的前提；M2 是「整片能对齐」的唯一主线（六个缺口里四个是游戏逻辑）；M3 已有卡在跑，不额外占人；M4 在 M2 之后才值得开（否则 120,540 帧只会放大同一批分歧）；M5 是 Human 能直接感知的「可玩」证据；M6 收口架构口径。

**估算依据**：M2 的 4–8 张按「1–2 张/根因」×4 个逻辑根因 + 缓冲；M4 的 3–5 张 = fm2 自检 1 + 首分歧定位 1 + 2P 口径 1 + 缓冲。M5 的 W3 依赖 M2 通关路径，W4 依赖 #10。

## §4 架构口径：「Rust 习惯」的可检查项

衔接已验收文档 `docs/design/fc-memory-projection-architecture.md`（410 行，结论：**分阶段重构**；立即做 A/B/C，P4–P7 带触发条件推迟）。本文把「Rust 习惯」落成下面这些能机械检查的条目：

| # | 可检查项 | 检查命令 / 指针 | 当前状态 |
|---|---|---|---|
| A1 | FC 内存投影只到 memmap 边界：业务代码不直接按裸地址读写，typed 域走 `memmap`/`ram_map` | `python3 gates/cli.py status` 的 `no-raw-addr`（typed 域 518/524 处 /23 文件，具名常量 151/151） | PASS（裸地址仍在棘轮内） |
| A2 | 无逻辑层 env 诊断钩子；诊断走启动参数或独立测试 | `no-env-hooks` 门禁（白名单 1 处=资产包定位） | PASS |
| A3 | 平台依赖隔离：game-core 无平台依赖，平台实现独立 crate | `arch-contract`（G11 依赖方向判据已按 m21400 移除，重构期放开）+ `crates/` 目录结构 | PASS（口径弱化，见下） |
| A4 | 数据边界：ROM 字节入口只在 `extract.rs`/`pack-tool`，其他 crate 不接收 ROM 字节 | `tools-boundary`（ROM 字节入口仅 2 处）+ `data-boundary`（7 条登记：data 0/fixture 5/symbol 2） | PASS（2 处 fixture 类待挪，WARN） |
| A5 | 门禁 ratchet 只升不降；已知差异一律进台账 | `gates/cli.py check` → PASS 23/FAIL 0/SKIP 3/TOLERATED 1（2026-11-23 21:22 实测 22.47 s） | PASS |
| A6 | TAS 对拍工具不持有判定权，判定逻辑在 `gates/` | 本文 §1.2 的判等规则将来落到 `gates/` 的 TAS 层；`tools/` 只产出 dump | 待做（M1） |

注意 A3 现在只是「目录结构上没有反向依赖」，`arch-contract` 的依赖方向判据已在 r19→m21400 裁剪中移除（重构期放开），所以「平台依赖隔离」目前**没有机械门禁**——M6 若要把它写进验收，需要先补判据（单独卡，且动门禁须单独 commit）。

## §5 假设区（C 级，未验证，禁止据此改代码）

1. **C-1** any% 素材的「通关必经」= 六个 `DIVERGENCES` 地址（§2.2）——这是「本次运行踩到的」集合，**不等于**「修完这六个就能通关」；通关还受未暴露的路径影响（例如只有特定受伤/拾取才走的分支）。
2. **C-2** 100% 整片的分歧面与 any% 同构（`movie_ledger` 只推到 f3,889，2P 段单点）——未验证。
3. **C-3** WRAM 未 typed 的 7,968 B 对玩法无影响——没有证据，只是当前 projection 的边界；Stage B 若要建模，须先出字段级分母。
4. **C-4** emu 腿 20.2 ms/帧的线性外推（any% 9.7 min / 100% 40.6 min）——只基于 `case-movie-100p-b26.run.json` 的 4,030 帧；长跑是否有内存/句柄增长未测。
5. **C-5** 「可玩」的 W5 人测判据（人类能否通关）本身不可机械化，只能作为收口时的一次性证据。

## §6 复跑清单（一条条照跑即可复核本文章节）

```
# §1.1 素材身份
python3 tools/movie_check.py movies/metroid-anyp-3666.bk2 movies/eien86-6725M.inputlog
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog --frames 28690 --capture-every 500 --capture-from 400 --out /tmp/tas-anyp-full.sched
python3 tools/movie_ref/movie_sched.py --inputlog movies/metroid-anyp-3666.inputlog --frames 300 --out /tmp/x.sched      # 期望 exit 2「行数 3260 ≠ 28690」
python3 tools/movie_ref/movie_sched.py --inputlog movies/lordtom-metroid-100p.inputlog --frames 120540 --out /tmp/y.sched # 期望 BadZipFile

# §2.2 整片缺口与末帧状态
target/release/rust_screenshot --schedule /tmp/tas-anyp-full.sched --out /tmp/tas-anyp-full2 --state-out /tmp/tas-anyp-full2.state.jsonl
python3 -c "import json;d=json.load(open('/tmp/tas-anyp-full2/eien86-6725M.divergences.json'));print(d['count'],d['unique'],d['gap_map'])"
# 末帧状态（mode/health/ending）见 §2.2 内联脚本

# §2.4 L4 现状
python3 -c "import json;b=json.load(open('data/status/l4_ratchet_baseline.json'));print(len(b['cases']))"

# §4 架构门禁
python3 gates/cli.py status
python3 gates/cli.py check
```

## §7 与已验收架构文档的衔接

- 「FC 内存投影作为投影、逻辑脱离 FC 约束」的分界：见 `docs/design/fc-memory-projection-architecture.md` §2（数据流/所有权/依赖方向）与 §4.1（A：Undeclared 1,287→0；B：动态腿 3→21 案；C：typed-first 入规）。
- 本文的 TAS 对拍面（§1.2）依赖该文档的「比对面 = RAM 2,048 + typed WRAM 224」边界；WRAM 盲区（§2.6）是该文档 §4 的已知遗留，Stage A 不扩大比对面。
- 里程碑 M6 的出口与 §0 第 3 条「步骤级验收」对齐：改 memmap 边界必跑 `no-raw-addr`/`nes-map`/`gates check`。

## §8 返工自审记录（2026-11-23）

本稿在首次提交（revision 3）后被拆骨按验收动作打回为 in_progress（revision 4），而 `team thread afterSeq=2` 与 `team inbox` 均为空——被打回**没有留下理由文本**（与 `requestChanges` 的 note 不进 thread 的已知缺陷一致）。为不空转，我按「哪条结论最容易被独立复核推翻」自查，改了四处，均保留原结论、只把证据补硬：

| # | 原稿 | 现稿 | 取证命令 |
|---|---|---|---|
| 1 | §2.2 整片读数用的 `target/release/rust_screenshot` 未记构建来源 | 补构建 provenance（HEAD `915763b9` + 仅注释差异、sha256 `579a7b93…`、`Finished release profile`、重建后复跑同数） | `cargo build -p game-core --release --bin rust_screenshot`；`sha256sum target/release/rust_screenshot`；重跑 `real 0.347s`、`63235/6` |
| 2 | §2.2 首记帧写「466 前后」「—」 | 六个地址的首记 tick 精确值：$FB7B/$CC36/$CC98 = 466、$CBF9 = 685、$CBEF = 953、$CE25 = 2,163 | 从 `divergences.json` 的 `events` 取每 addr 首个 tick（地址十进制键见 §2.2） |
| 3 | §2.3 引落盘 json 的 counts（candidate 147 / unported 608），与当前命令口径不一致且未说明 | 改用 `--stats` 现算 380/152/603，并注明落盘 json 是陈旧快照、引用以现算为准 | `python3 tools/routine_inventory.py --stats` |
| 4 | §1.2 未说明 2,316 B 是判等面还是落盘量 | 补 `--state-out` 实测：每帧 ≈20,554 B、any% 28,580 行 = 587,508,069 B；2,316 B 是判等面（wram 只判 typed 224 B） | `--state-out /tmp/tas-anyp-full2.state.jsonl` + `ls -la` + 单行解析 |

另：若拆骨的打回理由是上述之外的一条，请把判据（哪一节、期望 vs 实际、复现命令）贴进本任务 thread，我按同口径再改；在没有理由文本的情况下我不改结论性判断（口径、判等规则、里程碑排序）。
| 5 | §2.2 末帧写 `X=04` 但内联脚本不输出它（拆骨 seq4 打回）：数字本身没错（$030E SAMUS_X=4），错在没有来源可复现 | 改 `X($030E)=04` 并给常量/ROM 指针；内联脚本加 `ir[0x30E-0x100]` 输出，现一行复现 `→ 28690 mode= 0 X= 4 health= 64 ending= 0` | 内联脚本（§2.2）；`crates/game-core/src/ram_map.rs:163`；`refs/m1disasm/SRC/constants_ram.asm:238` |
| 6 | §1.1「1,993 处不同（首差 f=227）」没写口径（拆骨 seq4 建议 1） | 写明帧级/0-based/跳过 9 行头，并警告按原始文本行比会得另一组数 | `tools/movie_check.py` 的 `p1_truth` + inputlog 首列逐帧比 → 1,993 帧、首差 0-based 227 |
| 7 | §1.2 W1 把单次 0.39 s 当定值（拆骨 seq4 建议 2） | 改「≈0.3–0.4 s（机时波动）」，并记拆骨同机 0.31 s | 同一调度表两次读数 |
