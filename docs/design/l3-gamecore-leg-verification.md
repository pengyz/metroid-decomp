# design：L3 game-core 腿验证设计（S5 前置，里程碑 B 配套）

> 状态：**v1**（2026-10-15，task-13）。地基：trace-diff 裁判五轮硬化史（rounds/r5，
> 终局 pass）+ state-fixture 实施基线（rounds/r7-design-closure，S5 起用）+
> 两层架构（docs/spec/two-layer-architecture.md）+ **A9 证伪**（decode-sweep r3：
> 运行时 $8000 窗口=区域 bank 非 bank7，const 表须窗口参数化）。
> 本文只做验证设计；game-core 帧引擎本体归 task-10（frame/{clock,engine,memory,rng}）。

## 0. 问题与不变式

S5 = game-core 帧引擎接入 L3 双腿对拍（oracle 腿已验：diff_traces 五轮硬化 pass）。
**判定权不变式**：裁判=diff_traces 同一裁定逻辑（无第二套扫描）；对拍只裁「对不对」、
不证「全不全」；行为钉死=结构可换（两层 §3.3——域实现替换切片转写时同批 L3 案全绿）。

## 1. trace 格式契约（任务①）

**可对拍字段面**（watch 声明域 ⊇ 下表；hex 大小写归一比对沿 diff_traces 现约）：

| 面 | oracle 腿（Mesen2 dump） | game-core 腿（帧引擎导出） | 对拍规则 |
|---|---|---|---|
| 帧号 f | frameCount（自 RESET 起） | 虚拟帧计数（60Hz tick 序号） | **逐帧对齐声明**：1 tick ≜ 1 帧；f=0=RESET 后首帧 |
| RAM zp/iram | `$0000-$07FF` 字节镜像 | memmap encode（typed state→字节域） | watch 声明区间逐字节；**写集⊆watch**（r7 遗留③升强制） |
| WRAM | `$6000-$6Cxx` 字节镜像（含 $6AFx 敌槽/$6E/$6F 向量） | memmap wram 视图 encode | 同上；**窗口参数化前置**（A9 证伪——见 §5 风险） |
| 寄存器 | A/X/Y/P/SP（PC 面=执行证据另账） | 域方法出口值（verify_shim L2Out 同义） | 终值比对；P 含 C/Z/N 位（carry 敏感件已证） |
| RNG 面 | zp RNG 面快照（task-10 定谳后钉名） | GameRng 状态 | **确定性桥接面**（§2）——未定谳前 L3 双腿不启用 |

**契约规则**：
1. 字段面生成唯一源=memmap 层（两层 §1「地址只活 memmap」）——game-core 侧导出器
   只消费 `encode()` 输出，不手写字节偏移；
2. l3_manifest 扩键（state-fixture §6 既有待办一并落）：`fixture_id`+`fixture_sha256`+
   `k_max`（夹具件）+ **`leg`（oracle|game-core）+ `dump_sha256`（腿侧产物锚定）**；
   夹具 writes 条目 `persistence` 键（per-scenario）不变式同 r7；
3. **trace 可复现性（r3 P7-F1 教训升格）**：双腿 dump 工件全部留仓或声明临时口径——
   双跑/复审不得依赖未归档工件（「跑2=0」类静默降级照 P7-F2 改 fail-loud）。

## 2. 确定性口径（任务②）

**可复现性声明（三方缺一不可）**：

1. **虚拟时钟**：game-core 60Hz tick 语义=「一 tick ≜ 一帧」；虚拟时钟单调、无真实
   时间依赖（game-core 无 I/O/平台依赖红线不变）；
2. **GameRng 种子对齐**：确定性声明 = 「**同输入脚本 + 同 RNG 面初值 ⇒ 同 tick 状态
   序列**」。种子=状态快照的一部分：oracle 腿首帧前读取 RNG 面初值 → 走 fixture 注入
   （state-fixture §3 anchor 写入）→ game-core 侧同点注入。**前置缺口（显式）**：
   RNG 算法未证（台账 N-0012 C 级、unported! fail-loud 桩）——算法定谳前 L3 双腿
   对拍**不得启用**（否则帧级发散不可判定是语义差还是种子差）；
3. **Mesen2 侧可复现性**：ROM sha256 锚定（e6e6b701…）+ 输入脚本 sha + 无 savestate
   冷启动 + RNG 面 watch；**换时刻双跑**（gotcha-negative-assertion-time-race）：
   关键场景两时点各跑一次，dump sha 全同方采信（r3 双跑 34/34 先例；工件双腿留仓）。

**发散归因序**（首分歧帧 n）：①帧号对齐漂移（f 锚定失败→时钟口径层）②RNG 面初值不等
（注入失败→夹具层）③确定性破坏（同种子复跑不复现→引擎竞态/平台依赖，fail-loud 停止
差分裁决）④其余→§3 路由。

## 3. unported! 引爆的帧级差分路由（任务③）

unported! fail-loud 桩在 tick 内引爆 ⇒ 该帧 dump 缺失/截断。对 6 类分类法
（pipeline §4.2）的适用性裁定：

- **「未移植引爆」不入 6 类**——它不是差分裁决（不判对错，判存在性），混入「移植错」
  会虚增移植错误率；立 **第 7 显式类 `unported!-hit`**：路由=回 porter 排期（矩阵格
  回「已实现(有 unported! 清零记录)」前态），禁止进夹具维护/域审查；
- **环境瞬态不进路由**（gotcha-rate-limit 同族）：CPU 饥饿/超时类走 DEFERRED 语义
  （proof-checks v2 先例）——超时≠差分失败，重试 1 次后 DEFERRED+欠账+复检判据；
- 首分歧帧 n 的常规路由（6 类）沿 §4.2 现行：夹具类⓪（state-fixture (c) 后置门先行）
  → 无夹具基线分流 → 4.2 六分类；**判定序不变**（先 (c) 后行为，r7 B2）。

**帧级差异分类模板**（r3 范式扩展）：输入序列（场景 id+脚本 sha）/现象（tick n、字段、
CPU 地址=memmap 映射反查）/分类（7 类）/处理/门禁输出原文。

## 4. 帧级突变敏感性复审 schema（任务④）

L2 突变矩阵精神扩展到帧级——**每 L3 场景 ≥2 假想缺陷**，专设判别面（task-8 的 1/1
正中先例）：

| 突变类 | 假想缺陷例 | 预估首分歧（帧+字段） |
|---|---|---|
| 时序 | tick 内 NMI 相位偏移一帧 / 双 tick 顺序互换 | n=首次相位敏感帧，字段=动画相位/坐标 |
| 状态 | 写集漏字段 / RNG 消费次数差 1 / 寄存器透传错 | n=消费后首帧，字段=RNG 面/出口寄存器 |
| 场景 | 输入脚本 1 帧偏移（含夹具写入帧漂移） | n=F_a±1，字段=写入地址（(b) 采样一致性） |
| 确定性 | 同种子复跑引入时钟抖动 | 复跑 diff ≠ 空（确定性破坏探针） |

**复审 schema（突变记录逐条）**：

```yaml
mutant_id: <id>          kind: <时序|状态|场景|确定性>
expected_first_divergence: {frame: n, field: <watch label+offset>}
observed_first_divergence: {frame: m, field: ...}   # 空=未打回（判别力缺口，如实记）
verdict: caught(<n>) | survived(固有盲因 | 缺口)     # 幸存须归因，禁虚报全判别
```

基模证真前置门（r7 方法面）：**同批 L3 案双跑全绿 + dump sha 锚定后**才许开突变矩阵。

## 5. 实施序与风险

**实施序**（S5 就绪判定）：
1. **RNG 语义定谳**（task-10 消 N-0012 C 级；zp RNG 面+取值例程执行序引爆定位）——
   L3 双腿启用的硬前置；
2. memmap encode 全域覆盖 watch 声明面（WRAM 含敌槽/向量/$6E/$6F）；
3. game-core 腿 dump 导出器（帧引擎 tick 末 encode → 与 oracle dump 同构 jsonl）；
4. l3_manifest 双腿扩展 + diff_traces fixture 扩展（state-fixture §3(a)(b)(c) 三条件
   断言落点——(c) 为窗口末后置门，判定序先 (c)）；
5. 首批双场景（boot-title / samus-move3）+ §4 突变 schema 首轮（≥2 突变/场景）。

**风险登记**：
- **A9 窗口参数化未落**（task-8/9 处置）：L3 集成期 ROM 表/向量读值须按当前窗口 bank
  查表——const 表烘焙件（rom_tables/rom 向量）在集成层参数化前，L3 涉表场景结果
  标注「窗口契约未参数化」显式弱化；
- RNG 算法缺口（上述）；
- 帧号漂移（Mesen frameCount vs 虚拟帧）——漂移检测=帧锚定+速率声明（60Hz NTSC）；
- 全 RAM watch 比对成本——分段 watch（zp/iram/WRAM 域分组）+ 首分歧二分（diff_traces
  既有能力）；
- 双腿工件留仓成本（~100KB/腿）vs 可复现性（P7-F1 教训）——**留仓优先**。

## 6. 与既有账本的衔接

- rounds/r5：diff_traces 硬化史=裁判工具可信基（R4-3/R4-5/R4-6 遗留项在双腿扩展时复检）；
- rounds/r7-design-closure：state-fixture 三条件断言/判定序/持久性分类=夹具侧实施基线；
- docs/spec/two-layer-architecture.md：memmap=字段面唯一源；行为钉死不变式；
- l2-proof-assumptions：A1-A9 消费（A9 已证伪→窗口参数化）；
- rounds/r7-decode-sweep-review P7-F1/F2：trace 工件留仓+工具 fail-loud 升本设计契约。
