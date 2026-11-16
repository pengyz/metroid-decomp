# 录像输入面：把社区 TAS 的逐帧输入当作对拍输入（P0 已跑通）

> 2026-11-16。目标：把 L4 的输入面从「我们自己手写的 15 个场景」扩到**社区录像的逐帧输入流**，
> 用像素零容差去裁「emu↔rust 是否一致」。当前状态：**管线跑通，第一部录像实测 `frames_synced = 1039`**。

## 0. 定位纪律（先写死，否则会得出假结论）

录像**不是**「官方升级版脚本」。它的同步绑定**具体模拟器核心与版本**——同一条 `.fm2` 换 FCEUX
小版本都会十分钟后失步（TASVideos 论坛有大量此类案例）。所以：

- **我们不需要**「录像在 Mesen2 里跑出作者想要的结果」；
- **我们只需要**同一份逐帧输入喂给 emu 与 rust 两腿，判据始终是**两腿互相一致**；
- 因此「录像在 Mesen2 里 desync」这件事对我们**无害**，但「录像依赖的机制我们没实现」会真实暴露——
  这类分歧要**分类**（未实现 / 实现错 / 输入不适用），不能一股脑当 bug 修。

## 1. 格式与选型

| 容器 | 结构 | 结论 |
|---|---|---|
| `.fm2`（FCEUX） | 纯文本：头部 key-value + 每帧 `commands\|RLDUTSBA\|port1\|port2`；`commands` bit0=软复位 bit1=硬复位 | 交换中枢，解析最简 |
| `.bk2`（BizHawk） | ZIP：`Header.txt` + `Input Log.txt`（每行 `\|` 开头，分组 `Power+Reset \| P1(8) \| P2(8)`） | 社区发布主力，本工具首选支持 |
| `.mmo`（Mesen2） | 输入 + 可选快照 | 不采用：不需要 Mesen2 去「播电影」 |

**关键设计**：不让 Mesen2 播放电影（会接管输入、和 Lua 的 `emu.setInput` 打架，且绕开采集点）。
做法是 **解析录像 → 归一化成逐帧输入表 → 走现有 `sched`/`PLAN.schedule` 双腿管线**，
输入单源、采集点/`fixture`/像素判定全部复用。

按钮位映射三处同构（任一处漂移即「两腿跑不同输入」）：
`RLDUTSBA` 字符序 == bit0..bit7 == `tools/scenario_run.py::BUTTON_BITS` == `PadState`（A=bit7 … Right=bit0）。

## 2. 落地件

- `tools/movie_import.py`（新）：`.bk2`/`.fm2` 解析 → `.inputlog`（每帧 `p1 p2 cmd` 十六进制）
  + `.provenance.json`（来源/sha256/帧数/首个 2P 帧/复位次数）。支持 bizhawk 的「外层 zip 套内层 .bk2」形态。
- `tools/scenario_run.py`：DSL 新增 `inputlog <路径>`——该表对 1..N 帧**绝对优先**覆盖
  `press/hold` 的合成结果（录像即权威输入）；`expand_schedule` 仍是**唯一逐帧展开点**。
- `data/status/movie_ledger.json`：录像登记（URL/sha256/帧数/`frames_synced`），**不含录像本体**。
- `.gitignore` 加 `movies/`：录像与其派生的 `.inputlog` 不入库（TASVideos 许可：不得转发未改内容、须署名）。

## 3. P0 实测（第一部录像）

来源 [TASVideos 3666M](https://tasvideos.org/3666M)「NES Metroid any%」by The8bitbeast（07:57.38，
已被取代），`.bk2` / NesHawk 核心 / 28690 帧 / 从 Power-on 起（无 savestate）。

- 只取前 **3260** 帧（首个 2P 输入在 f3265，本 pilot 不碰 2P）。
- 结果：**f1–f1039 两侧逐像素全同**（5 个粗采样点 0px）；
  **f1040 首分歧 = 154px**，其后 f1120=318、f1140=15614、f1200≈21504（滚屏/位置整体岔开）。
- 首分歧形态：HUD（`EN 14`）、房间图形、敌人全同，**只有 Samus 自身精灵面不同**（见
  `shots_rust/case-movie-anyp-p0_f01040_t1040.diff.png` 的 DIFF 栏）。

⇒ `frames_synced = 1039`，登记入 ledger。这就是 movie ratchet 的第一个刻度。

## 4. 判据：movie ratchet（下一步接门禁）

每部录像登记 `frames_synced`（首分歧帧 − 1），**只增不减**；每修好一个真实分歧它就该涨，
涨不动的那个帧就是新 findings 的标题。采样策略：每 60–200 帧比状态，一旦不齐即二分到帧。

这把它从「一次性 demo」变成**可累积的分母**：录像覆盖全游戏（any% 路线穿过 Brinstar/Norfair/Kraid），
正好补 B 方向缺的区域分母。

## 5. 边界与未做

- **2P 输入未接**：`emu.setInput(btn, 0)` 只 port0，`FrameMemory` 只有 `joy1`。而 Metroid any%
  的核心技巧是「2P 上+A 重启」，所以录像的 f3265 之后必须补 port1 才能继续跑。
- **复位命令未接**：`.fm2` 的 `commands` bit0/bit1（软/硬复位）本片未使用，已在 `.inputlog` 里保留字段。
- **ROM 身份**：bk2 的 `Header.txt` 写 `SHA1 166A5B13…`，与我们 ROM 文件 sha1（`ecf39ec5…`）不一致——
  该字段是 BizHawk 侧的 ROM 视图哈希，**未作为门**使用；真正的身份判据是行为（前 1039 帧逐像素全同）。
  这条要在接更多录像前解决（否则「录像对应另一版 ROM」无法机械排除）。
- 本 pilot 用例在 `movies/cases/`（gitignored），不进 `cases/` 矩阵——**缺录像就不可复现**的用例
  不允许混进权威矩阵。
