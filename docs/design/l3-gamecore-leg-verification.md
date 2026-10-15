# design：L3 game-core 腿验证设计（S5 前置，里程碑 B 配套）

> 状态：**v3**（2026-10-15，task-13 返工轮·全量）。v1（d0e3e1e）经独立对抗复核
> （abc6daf6，全文归档 [rounds/r9-task13-independent-review](../rounds/r9-task13-independent-review.md)）
> 判 4 阻断+8 严重+3 次要——本版逐条返工（对账表见 §7）；严重 5-12 修法按其建议。
> 地基：trace-diff 五轮硬化史（rounds/r5）+ state-fixture 基线（rounds/r7-design-closure）
> + 两层架构（docs/spec/two-layer-architecture.md）+ **A9 证伪**（窗口=区域 bank）。

## 0. 问题与不变式

S5 = game-core 帧引擎接入 L3 双腿对拍（oracle 腿已验）。**判定权不变式**：裁判=
diff_traces 同一裁定逻辑（无第二套扫描）；对拍只裁「对不对」、不证「全不全」；
行为钉死=结构可换（两层 §3.3）。

## 1. trace 格式契约（任务①）

**帧号口径（阻断-1）**：**f=1 起**（双腿实况：oracle frameCount 与 game-core tick 计数
均自 1 起；v1「f=0」废）。**inputPolled 序号 ≡ tick 序号**（同一输入帧恰一次 poll）。
**分母语义**：场景帧级比对分母 = manifest.frames（= tick 总数 = inputPolled 总数）；
缺帧/多帧 ⇒ diff_traces 帧集不一致 FAIL，不得按 f 起点歧义缩放分母。
**遗留-7 同步义务**：state-fixture F_a「绝对帧 f≥800」判据按 f=1 起口径复核一遍
（防一帧错位——夹具侧与本设计共用帧轴）。

**字段面现状（严重-5 修订：两面钉死，三面前置定义项）**：

| 面 | 现状 | 机械对拍规则 |
|---|---|---|
| RAM zp（$0000-$00FF） | **已钉死**（dump=512 hex 大写） | 逐字节；写集⊆watch 待机制（严重-7） |
| RAM iram（$0100-$07FF） | **已钉死**（3584 hex） | 同上；含栈页 |
| WRAM（**前置定义项**） | 机械区间未钉死：FrameMemory 8KB（$6000-$7FFF）vs「$6000-$6Cxx」软上界不同框——**钉死前涉 WRAM 场景标软边界弱化**（遗留-1） | 待：区间钉死 + memmap encode（严重-6） |
| 寄存器（**前置定义项**） | watch schema 需 base 地址、寄存器无地址槽位；「终值比对」范围未定义（末帧/逐帧）；「L2Out 同义」是类比非契约 | 待：manifest 增寄存器键（`regs_watch`）+ 比对范围声明（**逐帧**终值入 watch 面为默认，末帧-only 需显式） |
| RNG（**前置定义项**） | 地址/键名待 N-0012 定谳钉名；比对规则待定 | 待：全状态快照键（§2）+ 序列对账规则 |

**域归类改正（严重-5）**：`$6E/$6F` 向量槽=**零页面**（bank07:F309 `85 6E` STA zp $6E，
memmap/flow_scratch.rs）——v1 误列 WRAM 行（错域会导致 watch 重复/错域），已归 zp 面。

**契约规则**：
1. 字段面生成唯一源=memmap 层——**现状对账（严重-6）**：全域 encode() 目前仅
   enemy_slots::encode_slots；现行导出链 l3-dump→FrameEngine::watch_snapshot→
   FrameMemory::watch_snapshot（frame/memory.rs 手写 `hex_upper(&self.ram[…])` 切片）
   **不走 memmap**=过渡态。**改造点名**：watch_snapshot/l3-dump 改造为消费 memmap
   整面 encode（zp/iram/WRAM 三域）+ **按 manifest watch base/range 出面**（现状无视
   声明、固定 zp/iram 两键——声明 {zp,wram} 时 wram 缺失 fail-loud ✓ 但 iram 多余键
   静默通过 ✗）；过渡期契约文字降级为「memmap=目标唯一源（现状两处手写面已点名待退役）」；
2. **帧记录键集白名单（严重-7）**：帧记录键集=`{f, w}`+显式声明扩展键（`ram` 等）——
   **schema 校验白名单进 diff_traces**（未知键=FAIL，现状静默忽略）；双腿导出器各自
   **显式声明键集**（oracle PLAN.ramframe 的 `{"f","w","ram"}` 额外键=现成隐藏字段
   实例，必须入声明或关断）；「写集⊆watch」升强制**须挂写集跟踪机制**（现状推导只在
   verify_shim L2 层——帧级写集跟踪=改造点名，未落前该条降级为「L2 层已保证、
   帧级待机制」；出处勘误：本条=r4 遗留③/r7-design-closure 遗留 2 续持，v1 误标
   「r7 遗留③」）；
3. **工件可复现性（阻断-4）**：双腿 dump 工件落 **`analysis/`（可再生工作区，
   不入库——AGENTS §2 红线）**；可复现性=场景+输入脚本+工具+ROM sha 可重跑**再生**
   工件（dump_sha256 于可再生 run.json 登记比对），**不要求**工件入 git。
   > 【引证更正（严重-10 配套）】正向确定性采信引
   > **reference-mesen2-trace-determinism**（同场景双跑逐字节一致 A 级实证）；
   > gotcha-negative-assertion-time-race 的「换时刻」是**否定性断言**的时效竞态纪律
   > （配套记录时点义务），不作正向采信依据——v1 此处范畴滑移已改。
   > P7-F1 仅引更正后教训：**工件路径口径要显式**（run1/run2 对称命名）。

## 2. 确定性口径（任务②）

**可复现性声明（三方缺一不可）**：

1. **虚拟时钟**：1 tick ≜ 1 帧（f=1 起）；单调、无真实时间依赖；
2. **GameRng 全状态桥接**：确定性声明 = 「同输入脚本 + **同 RNG 全状态** ⇒ 同 tick
   状态序列」。**证明义务（严重-9）**：被快照的 RNG 面=PRNG **全部**状态（无面外隐藏态：
   寄存器暂存/额外 zp 字节）——义务四则：①序列等价②快照可往返（serialize→注入→
   serialize 全等）③消费次数每 tick 对账④**状态面写集闭环**（取值例程经执行序引爆
   定位后，其读写面与钉名状态面互证：面外读写=0）。①-④ 全过才许称「全状态」。
   **机械门（严重-8：declarative→gate）**：L3 双腿启用前置断言——
   `N-0012.status == applied ∧ N-0012.level ∈ {A, B} ∧ RNG 序列 L2 案 ≥1`；
   任一不满足 ⇒ **l3 runner 拒跑**（物理开关=diff_traces 腿启用文件存在与否之上
   再加此断言；「定谳」=可检验条件而非文档形容词）。现状安全（GameRng=unported! 桩
   引爆不落盘）是巧合不是制度——门落位后制度接管；
3. **Mesen2 侧**：ROM sha256 锚定 + 输入脚本 sha（**manifest 补 `scenario_sha256` 键
   （严重-10 漏键补上）**）+ 冷启动 + RNG 全状态 watch。**双跑采信**：两时点各跑、
   dump_sha256 全同方采信——**记录时点义务**（何时/何命令/何状态三则随采信记录）；
   双跑间隔/窗口：至少一个并行作业窗口（同 r3 先例量级），记录内写明。

**发散归因序**（首分歧帧 n）：①帧号对齐/分母语义层②RNG 全状态不等（注入失败→
夹具层）③确定性破坏（同状态复跑不复现→引擎竞态/平台依赖，fail-loud 停止差分裁决）
④其余→§3 路由。

**注入点（阻断-3：两定义拆开）**：
- **夹具场景**：注入点=**F_a anchor 帧**（state-fixture §3 既有；三条件断言照 r7）；
- **boot 起跑场景**：注入点=「**初始化序完成后、RNG 首消费前**」——**机械判据**：
  boot_init 全单元消化（引爆桩位表为空）∧ RNG 消费计数=0 ∧ 当前 tick 边界
  （inputPolled n 之前）。三则全真才许注入，否则 runner 拒跑。

## 3. unported! 引爆与帧级差分路由（任务③）

**分类（阻断-2）**：unported! 不新立第 7 类——是 pipeline §4.2「实现缺口」的
**显式细分**（表内类名为准，pipeline 表已同步）：

| 细分 | 检测面 | 路由 |
|---|---|---|
| 实现缺口·unported!-hit | panic payload（`unported @ bank:addr`）+ **exit 101** + **0 落盘** | spec-writer（缺口登记）→ porter（排期）；矩阵格回 L0 前态 |
| 实现缺口·崩溃/截断（非 unported!） | panic payload 无 unported! 形态 / dump 中途截断 | 按 §4.2 常规二分（实现缺口/移植错） |

**红灯全形态落点（次要-13 补）**：
- **无首分歧帧**（跑满且逐帧全等）⇒ **PASS**+显式覆盖边界声明（watch 域内全等≠语义
  全等；绿≠完整）；
- 帧集不一致（缺帧/多帧）⇒ diff_traces 既有裁决（截断/崩溃二分）；
- **oracle sha 不符 manifest** ⇒ oracle 腿自体确定性破裂——停裁、重采集、按 §2
  双跑换锚流程；
- diff exit 2（schema 级）⇒ 先于一切比对，修 schema 后重跑。

环境瞬态不进路由（gotcha-rate-limit 同族）：超时类走 **L3 专属 DEFERRED 账
（严重-11）**——与 proof_deferred.jsonl **分账**（`kind: l3-deferred` 独立文件
`gates/l3_deferred.jsonl`）：**L3 专属解锁判据**=同场景复跑 PASS（非「任一次全量
proof-checks PASS」——语义错位）；commit 覆盖面=触及 crates/game-core|game-oracle
|oracle/traces 的 commit 在欠账期须带 `L3Deferred: <id>`（不只 S_intx）；「DEFERRED
绝不计 PASS」随行（SUMMARY 单列、收尾行不打裸 PASS）。遗留-4：DEFERRED 上线时复检
「绝不计 PASS/欠账防删账」随行到 trace 门禁。

**帧级差异分类模板**：输入序列（场景 id+**scenario_sha256**）/现象（tick n、字段、
CPU 地址=memmap 映射反查）/分类（六类+unported!-hit/崩溃截断细分）/处理/门禁输出原文。

## 4. 帧级突变敏感性复审 schema（任务④）

**双门制（严重-12 补齐）**：每 L3 场景 ≥2 假想缺陷，其中 **≥1 取自 L2 高危族清单**
（按寻址模式/运算族指派）；每条突变过双门：

- **门 1·高危族覆盖**：族 ∈ L2 实证族谱（EOR zp 误读立即数（45 FF）/ LDA·CMP 立即数
  误读 zp（A9/C9）/ zp,X 8 位变址回绕 / abs,Y WRAM 域误读 / SBC 进位链断裂 / 16 位
  取负降级 / 共享尾误配 / 窗口越界执行 / 末值 A 链误取 / AND #1 掩码丢失）或帧级新族
  （时序/RNG 消费/确定性——先登记族谱再计入）；
- **门 2·判别值域在场**：场景输入含该族判别值域（偶 ram[FF] / X=0xFF 回绕 / wrap
  边界 / $00≠0 取负 / A_in=0 对照…）——值域不在场的突变打回=0 时**不计配额**
  （「打回 0/2」≠「判别力 2/2」）；
- **mutation_verified 字段（P1-F3 补）**：突变实现须证真（实际实现=宣称缺陷——防
  「突变定义错」假判别，r7 批 1 m1 定义错先例）。

| 突变类 | 假想缺陷例 | 预估首分歧（帧+字段） |
|---|---|---|
| 时序 | tick 内 NMI 相位偏移一帧 / 双 tick 顺序互换 | n=首次相位敏感帧，字段=动画相位/坐标 |
| 状态 | 写集漏字段 / RNG 消费次数差 1 / 寄存器透传错 | n=消费后首帧，字段=RNG 面/出口寄存器 |
| 场景 | 输入脚本 1 帧偏移（含夹具写入帧漂移） | n=F_a±1，字段=写入地址（(b) 采样一致性） |
| 确定性 | 同状态复跑引入时钟抖动 | 复跑 diff ≠ 空（确定性破坏探针） |

**复审 schema**：

```yaml
mutant_id: <id>          kind: <时序|状态|场景|确定性>
high_risk_family: <族名>            # 门 1（每场景 ≥1 条须来自 L2 族谱）
discriminating_domain: <值域声明>    # 门 2（场景输入在场证据）
mutation_verified: <bool|证据指针>   # P1-F3：突变实现=宣称缺陷的证真
expected_first_divergence: {frame: n, field: <watch label+offset>}
observed_first_divergence: {frame: m, field: ...}   # 空=未打回
verdict: caught(<n>) | survived(固有盲因 | 缺口)     # 幸存须归因，禁虚报全判别
```

基模证真前置门：同批 L3 案双跑全绿 + dump_sha256 锚定后才许开突变矩阵。

## 5. 实施序与风险

**实施序**（S5 就绪判定）：
1. **RNG 全状态定谳**（task-10：取值例程执行序引爆定位 → 状态面写集闭环 →
   N-0012 升级 rename 流程 → 机械门解锁）——L3 双腿启用硬前置；
2. memmap 整面 encode + watch_snapshot/l3-dump 改造点名件（严重-6）+ WRAM 机械
   区间钉死（严重-5）+ 帧记录键集白名单进 diff_traces（严重-7）；
3. game-core 腿 dump 导出器（tick 末 encode → 同构 jsonl 落 analysis/）；
4. l3_manifest 双腿扩展（fixture_id/fixture_sha256/k_max/leg/dump_sha256/
   **scenario_sha256**/regs_watch/RNG 全状态键）+ diff_traces fixture 扩展
   （(a)(b)(c) 三条件，(c) 后置门）；
5. 首批双场景（boot-title / samus-move3）+ §4 双门突变 schema 首轮；
6. **验收义务（次要-14 补）**：r5 遗留 4「S5 后双侧重跑 T4/T5 路径」在双腿落地时
   执行并入验收记录；r5 遗留 5「多实例并发双写互踩」在多场景并发采集前复检。

**风险登记（含 r9 遗留 1-7）**：
- WRAM 软上界（遗留-1）：涉 WRAM 场景分母带软边界，与 A9 同标弱化面；
- **tick 内瞬态盲区（遗留-2）**：帧末快照不见写后回滚——登记固有盲区，L2 手术级兜底；
- RNG 定谳后 watch 面/dump 结构变更 ⇒ oracle_sha256 全量换锚（遗留-3：重采集+
  双跑换锚流程，run1/run2 对称命名）；
- DEFERRED 上线复检（遗留-4）；
- **共享工作树并发态（遗留-5）**：复审裁定以已入库内容+analysis/ 既有工件为准，
  **记录时点**（本设计 v3 复核时点=2026-10-15T16:xx+08:00）；
- 突变配额×判别值双门并行（遗留-6）；
- f=1 口径与 state-fixture F_a「f≥800」同步复核（遗留-7，§1 已列义务）；
- A9 窗口参数化未落：涉表场景标「窗口契约未参数化」。

## 6. 与既有账本的衔接

- rounds/r5：diff_traces 硬化史=裁判可信基；遗留 4/5 进实施序（§5.6）；R4-3/R4-5/
  R4-6 在双腿落地时复检；
- rounds/r7-design-closure：state-fixture 三条件断言/判定序=夹具侧基线；
- docs/spec/two-layer-architecture.md：memmap=目标唯一源（现状对账见 §1 规则 1）；
- l2-proof-assumptions：A1-A9（A9 已证伪→窗口参数化）；
- rounds/r7-decode-sweep-review：P7-F2（工具 fail-loud）；P7-F1 仅引更正后教训；
- **reference-mesen2-trace-determinism**：正向确定性采信的 A 级依据（严重-10）；
- pipeline §4.2 表：unported!-hit 细分行已同步（阻断-2 配套，返工令授权）。

## 7. v3 返工对账表（r9 逐条）

| r9 项 | 修法落点 |
|---|---|
| 阻断-1 帧号 | §1：f=1 起+inputPolled≡tick+分母语义+遗留-7 同步义务 |
| 阻断-2 第 7 类 | §3：实现缺口细分+崩溃/截断边界；pipeline §4.2 表同步 |
| 阻断-3 注入点 | §2：夹具=F_a / boot 起跑=三则机械判据 |
| 阻断-4 红线 | §1 规则 3：analysis/ 不入库+引证更正（P7-F1 过读撤除） |
| 严重-5 五面 | §1 表：两面钉死/三面前置定义项+缺口清单；$6E/$6F 错域改正 |
| 严重-6 memmap | §1 规则 1：现状对账+watch_snapshot/l3-dump 改造点名 |
| 严重-7 隐藏字段 | §1 规则 2：键集白名单+导出器声明+写集机制降级+编号勘误 |
| 严重-8 机械门 | §2.2：N-0012 可检验条件+runner 拒跑断言 |
| 严重-9 全状态 | §2.2：证明义务四则（含状态面写集闭环） |
| 严重-10 引证/漏键 | §1 规则 3+§2.3：引证改正+记录时点义务+scenario_sha256 键 |
| 严重-11 DEFERRED | §3：L3 专属分账/解锁判据/commit 覆盖面/绝不计 PASS |
| 严重-12 双门 | §4：≥1 高危族/场景+mutation_verified 字段 |
| 次要-13 | §3 红灯全形态落点（无首分歧/sha 不符/exit 2/崩溃截断） |
| 次要-14 | §5.6 实施序验收义务（r5 遗留 4/5） |
| 次要-15 | 已转 decomp task-15（boot-title.txt 陈旧注） |

## 8. C0D9 对表结论（2026-10-15，verifier×porter 会师四点+一刀口）

> 证据三方：porter asm 引证（prg7_engine.asm）+ 本设计 v3 契约面 + decomp
> nmi-frame-evidence.nmi.jsonl（599 帧相位实测，analysis/traces/ 可再生）。

**① 双 vblank 对齐** ✓ 无异议：启动双等=FFB0 序 `ad 02 20 10 fb`×2（我方字节核先证）；
稳态单相 **scanline=241 ×599/599**（实测零漂移）；seq 单调 ✓。契约：NMI 建模点=
tick 内 vblank 相（241）。

**② 栈页三字节排除** ✓ 定案：NMI 压栈 sp=0xFC（594/599 稳态）⇒ 硬件压 PCH/PCL/P 于
**$01FC-$01FE**——即批 21 实测「值漂移三字节」的真身（中断帧影子，非游戏状态）。
**watch 面口径：$01FC-$01FE 排除严格比对**（声明为 interrupt-frame 面；余 19 字节
主循环调用影子照比）。

**③ $1A 双写相位** ✓ 语义钉死（asm）：`ldy #$01/sty NMIStatus`（:279-280，NMI 尾=
「NMI finished」）+ `ClearNMIStat`（:1117，主端清 0）——每帧环**双写**（主清 0/NMI 置 1）；
**成环标志=spin 出口同源**（:1118-1121 @loop `lda NMIStatus/beq @loop`，退出=1）。实测
帧末 $1A：0×598 + 1×2——2 例=frameCount 边界恰落「NMI 写 1 后、主清 0 前」相位窗。
**处置**：$1A 列**相位敏感面**（不入严格逐帧 watch；或双腿采样点对齐至 spin 出口后恒 1
再比——二选一待 porter 带实现证据定案）。

**④ 帧界采样点** ✓：NMI 首帧 f=1 锚（实测行自 f=1 起）与 v3「f=1 起、inputPolled≡tick」
一致；契约=采样点=**spin 出口同源**（③ 的成环标志同一逻辑点——两腿同一采样语义）。

**⑤ 刀口·NMI-end 自旋建模语义 —— 裁定提案：跨帧自旋（spin=tick 边界构造物）**

asm 语义链：主端 ClearNMIStat（清 0）→ @loop spin → **下一次** NMI 完成尾写 1 → 退出
——spin 等待的是**下一圈** NMI（跨帧）。故 1tick≜1帧 引擎的忠实建模：

- spin=**tick 边界本身**（yield-until-next-NMI）：spin 退出点= tick N+1 的 NMI 尾
  （$1A 写 1 之后）；
- tick 内顺序=「NMI 处理（vblank 相 241）→ $1A 写 1 → spin 退出 → 主循环帧工作 →
  ClearNMIStat 清 0 → spin（跨入下一 tick）」；
- 「同帧退出」建模（NMI 于本 tick 早段、spin 本 tick 内退出）**不采纳**——与
  ClearNMIStat 先清后等的 asm 语义矛盾（清 0 后本 tick 内无 NMI 写 1 者，spin 永挂）；
- 帧级对拍面由此自洽：帧界采样=spin 出口（④），$1A 相位敏感面处置（③），栈页三字节
  排除（②）。

**待 porter 会签**：引擎侧 spin 建模按上述跨帧语义实现（或携带反证——若其 tick 结构
有同帧合法模型，须以 asm 语义链+599 帧实测对表）；$1A watch 面处置二选一请带实现
证据回来定案。

### 8.1 对表定案（2026-10-15 终稿，冲突解案后）

**证据冲突记录（不删）**：decomp 动态曾报「$1A 双写全在主循环相、NMI 零写、写点
scanline<241」与本节提案正面冲突。**定案**（静态决定性证据 + 我方②号复核）：
- `:279 ldy #$01` 为**常量写**（非 Y 依赖）——NMI 尾恒写 1；
- 我方复核 NMI 全流：入口段（php/pha/txa/pha/tya/pha/OAMADDR/OAMDMA）**无 rti 短路**、
  尾段 `sty NMIStatus` 位于 @endIf_A 超帧跳过目标之后=**无条件执行**——每条到达
  handler 尾的 NMI 必写 $1A=1；
- ⇒ decomp「NMI 零写」=**inputPolled 采样点相位盲区人工产物**（探针只覆盖主循环相，
  NMI 相写在其眼皮外）——task-15④ 注记按证伪更正文化更正（lead 令）。

**契约定案三点**：
1. **spin=跨帧建模**（spin=tick 边界构造物，yield-to-next-tick-NMI；boot 链跨 tick
   可恢复序列/boot_step 状态机=实现形态）；
2. **$1A watch=采样点对齐保留**（NMI 时序锚）——采样点=spin 出口同源（④）后
   相位敏感性消解；
3. **恒 1 入契约断言**：采样点 $1A==1 为断言项——**≠1=NMI 面退化诊断信号**
   （诊断力主张采纳：保留锚+断言 > 排除）。

附：NMI 地图（C0D9 实施素材，porter 实读）——超帧跳过（$1A≠0 跳过 NMIScreenWrite/
CheckPaletteWrite/CheckVRAMStructBufferWrite/WritePPUCtrl/WriteScroll/ReadJoyPads）
+OAMDMA（$200-$2FF→OAM）+SoundEngine/UpdateAge——里程碑 B「宽放 18 决策项」overframe
容忍面在此，C0D9 单元实施按定案展开。
