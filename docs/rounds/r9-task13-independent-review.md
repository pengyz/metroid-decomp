# rounds/r9 — task-13 独立对抗复审账本（abc6daf6 第三方，d0e3e1e）

> 归档说明：本报告由第三方独立审核员（abc6daf6）产出，此前仅存 chat（lead 路由疏漏，
> 链规⑪「产出归宿是仓库」对自己同样生效——案记）。本文件=task-13 返工清单的权威载体。
> 审读基准：commit d0e3e1e；只读复审；门禁预检 PASS=10 FAIL=0 SKIP=2。

## verdict: needs_revision

②确定性口径与④突变 schema 骨架方向成立；①「五面完整无歧义无隐藏字段」与③「第 7 类无缝衔接」不成立；②④各有机械化缺口。

## 【阻断-1｜A 契约】帧号口径与双腿实况矛盾（f=0 vs f=1）
设计称「f=0=RESET 后首帧」。实况：oracle 腿 scenario_capture.lua:48 `local frame=0`，inputPolled 先自增后写盘→首帧 f=1（Lua 自计数器，非设计所引 frameCount，frameCount 仅在 probeLog L300）；game-core 腿 l3-dump.rs `for f in 1..=frames`→首帧 f=1；真实工件 boot-title.frames.jsonl 300 条 f range **1 300**。按字面 0-based 实现即全帧错位；diff_traces 只报「帧集合不一致」（compare_frames）无 tick n，落不进 7 类模板；manifest.frames=300 分母语义随口径漂移。**修法**：契约改「f=1 起、inputPolled 序号≡tick 序号」+分母语义写明（或双侧同步改造并写明）。

## 【阻断-2｜C 路由】unported!-hit「第 7 类」与 pipeline §4.2 正面冲突
pipeline §4.2 表第 1 行即「panic @ unported!(bank:addr)｜实现缺口｜spec-writer 补规则→porter 排期（矩阵格记 L0）」——unported! 已是 6 类成员，流程图注明「表内类名为准」。设计另立第 7 类=分类法分叉：同一引爆落两个互斥类且路由不同（pipeline 先 spec-writer，设计直跳 porter 还禁夹具维护）；「混入移植错虚增错误率」系稻草人（pipeline 从未如此归类）。state-fixture §6「崩溃/截断⇒实现缺口/移植错」二分与第 7 类边界也未写。**真有价值部分**：l3-dump「引爆即不落盘」使 unported! 可与静默移植错机械区分（检测面=非零退出+0 文件）。**修法**：unported!-hit=「实现缺口」的显式细分（检测面=panic payload 含 bank:addr+dump 不落盘），路由沿 §4.2 spec-writer→porter，**同步修订 pipeline §4.2 表**。

## 【阻断-3｜B 确定性】GameRng 种子注入点自相矛盾，与 T4「帧 0 写不可行」冲突
设计「oracle 腿首帧前读 RNG 面初值→走 fixture 注入（state-fixture §3 anchor 写入）→game-core 同点注入」焊了两个不相容注入点：state-fixture §3 注入模型=同链重放到 F_a 帧写入；T4 明训「帧 0 写不可行：机器刚过 RESET 走 boot/标题/入局链，新游戏初始化必然重写」——t0 注入必被 zp 初始化序冲掉（game-core boot_init 双清零环/zp 初始化序实证同向）。boot-title 类从 RESET 起跑无 F_a 概念，注入点未定义。**修法**：拆两定义——夹具场景=F_a anchor 语义；boot 起跑场景=「初始化序完成后、RNG 面首消费前」+机械判据。

## 【阻断-4｜E 红线】「双腿 dump 工件全部留仓…留仓优先」与「ROM 衍生 trace 不入库」红线冲突；P7-F1 依据过读
红线三处：AGENTS §2「analysis/ 可再生工作区不入库」、l3_manifest.json note「ROM 衍生 trace 不入库，红线」、diff_traces.py 头注同。「留仓/100KB 成本」措辞指向 git 入库口径⇒直踩红线；若本意 analysis/ 工作区留存则「成本」不成立。P7-F1 原文已【2026-10-15 更正】「工件双腿俱在」，残余仅命名对称建议——引作「工件留仓升契约」依据=对已更正结论的过读（该升契约的是 P7-F2 工具 fail-loud，设计引对了）。**修法**：契约写明工件归档位置（analysis/ 不入库）+双跑复审工件保留口径+命名对称（run1/run2）。

## 【严重-5｜A 契约】五面中仅 RAM 两面可机械对拍
假想帧双侧推演（真实工件+两侧代码）：双腿记录均 `{"f":7,"w":{"zp":"<512 hex>","iram":"<3584 hex>"}}`——现状 2 面可对拍 ✓。缺口：**寄存器面**无序列化槽位（watch schema 要 base 地址，寄存器无地址）、「终值比对」范围未定义（末帧？逐帧？）、manifest 扩键清单无寄存器/RNG 键、「verify_shim L2Out 同义」是类比非契约（L2Out=子程序出参/写集≠帧末寄存器面）；**RNG 面**地址「定谳后钉名」、比对规则「确定性桥接面」不是规则；**WRAM** 区间「$6000-$6Cxx」软上界与 FrameMemory 8KB 口径不同框，机械区间须钉死；**地址域归类错**：「$6E/$6F 向量」列 WRAM 行——实为零页（memmap/flow_scratch.rs：bank07:F309 `85 6E` STA zp $6E），已在 zp 面内，列入 WRAM 致 watch 重复/错域，与「memmap=唯一源」自相矛盾。**修法**：降级主张为「RAM 两面已钉死，WRAM/寄存器/RNG 三面前置定义项（附缺口清单）」并补定义。

## 【严重-6｜A 契约】memmap「字段面唯一源」与实况不一致
memmap 全域 encode() 只有 enemy_slots::encode_slots，其余 decode_*；无 zp/iram/WRAM 整面 encode。现行导出链 l3-dump→FrameEngine::watch_snapshot→FrameMemory::watch_snapshot（frame/memory.rs L55-61）=手写切片 `hex_upper(&self.ram[0x000..0x100])`/`[0x100..0x800]` 不走 memmap，违契约规则 1 字面。l3-dump 无视 manifest watch base/range（只校验非空），固定输出 zp/iram 两键——manifest 声明 {zp,wram} 时 diff 报 wram 缺失（fail-loud）但 iram 多余键静默通过。**修法**：契约与骨架（c73ab66）显式对账（谁退役谁接替），watch_snapshot/l3-dump 改造点名。

## 【严重-7｜A 契约】「无隐藏字段」无机械背书，且现成隐藏字段实例
diff_traces.load_frames 只校验 'f'/'w' 在——帧记录未知键**静默忽略**（watch 条目未知键倒是严格拒 WATCH_KEYS）；oracle 腿 PLAN.ramframe 开启时实产 `{"f","w","ram"}` 额外键（scenario_capture.lua L342-344）=隐藏字段现成实例。「写集⊆watch 升强制」无写集跟踪机制可挂（写集推导只在 verify_shim L2 层）；编号不准：实为 r4 遗留③、r7-design-closure 遗留清单第 2 条续持，非「r7 遗留③」。**修法**：帧记录键集白名单进 schema 校验+双腿导出器键集显式声明。

## 【严重-8｜B 确定性】N-0012 硬前置是声明不是门
启用开关物理事实=「analysis/dump/<scenario>.frames.jsonl 文件存在」（diff_traces main 判腿启用），无 gates 机制在 N-0012 仍 C 级时拒绝双跑。现况安全靠「GameRng 是 unported! 桩、引爆不落盘」的巧合而非制度。**修法**：定机械门（如启用前置断言：N-0012 status=applied ∧ level∈{A,B}），「定谳」写成可检验条件。

## 【严重-9｜B 确定性】「种子=状态快照」缺状态完备性前提
启用后「同 RNG 面初值⇒同 tick 状态序列」隐含未写入的子命题：被快照的 RNG 面=PRNG **全部**状态（无面外隐藏态：寄存器暂存/额外 zp 字节）。若不完全落在钉名面内，桥接断而双跑仍绿（同输入同残缺快照）。**修法**：实施序「RNG 语义定谳」追加「RNG 面=全状态」证明义务（执行序引爆定位取值例程+状态面写集闭环）。

## 【严重-10｜B 确定性】双跑「换时刻」引证范畴滑移+输入脚本 sha 漏键
gotcha-negative-assertion-time-race 的「换时刻」是**否定性断言**的时效竞态复核纪律（配套「记录时点」义务），不是确定性采样依据——正向采信应引 reference-mesen2-trace-determinism（同场景双跑逐字节一致 A 级实证）。双跑「两时点」间隔/窗口未定义（r3 先例=背靠背跑1/跑2），采信记录未继承「何时/何命令/何状态」义务。§2.3 采信三件含「输入脚本 sha」但 manifest 扩键清单无此键（scenario_sha256 只在可再生 run.json L656），§3 差异模板却要求「场景 id+脚本 sha」——契约漏键。**修法**：引证改正+记录时点义务继承+manifest 补 scenario_sha256 键。

## 【严重-11｜C 路由】DEFERRED 并账污染风险（三未定义）
proof_deferred.jsonl 是 proof-checks 专属账：解锁判据=「任一次全量 proof-checks PASS」（对 L3 场景复跑语义错位）、ProofDeferred commit-msg 强制仅限 S_intx 触及（game-core/L3 commit 不强制⇒欠账可静默带病提交）、另立账则未命名未定义销账。**修法**：写明账本归属（独立 kind 或独立文件）+L3 专属解锁判据+commit 覆盖面+「绝不计 PASS」随行。

## 【严重-12｜D 突变 schema】四类未继承 L2 突变矩阵实证战绩（高危族×判别值域双缺）
判别力=突变族×场景判别值的乘积（L2 战绩：漏变址 X=0 幸存、EOR 误读 5/10、取负位宽 2/14 靠 F11-F1 补 $00≠0 边界案、SBC 进位链 8/14）。设计四类按观察面切，不要求突变取自高危族（变址漏/位运算误读/取负位宽/进位链/WRAM-RAM 域混淆/立即数误读），不要求场景含判别值（X≠0、$00≠0、奇偶 $FF）——「≥2/场景」可被低危突变凑满而历史盲族零探测。缺 P1-F3「突变定义证真」字段（突变实现≠宣称缺陷）。survived 归因 ✓（P3-F1 诚实文化）。**修法**：①每场景 ≥2 突变中 ≥1 取自「L2 高危族清单」并按寻址模式/运算族指派②场景库附判别值要求③schema 增 `mutation_verified` 字段。

## 【次要-13｜C】无首分歧帧的失败形态无落点
红灯还有：oracle sha 不符 manifest（oracle 腿自体确定性破裂）、diff exit 2（schema 级）、帧集不一致/非 unported! 崩溃截断——7 类模板缺「无首分歧帧」分支与 unported!/崩溃截断边界判据（检测面=panic payload 含 unported!/bank:addr+exit 101+0 落盘）。

## 【次要-14｜E】r5 遗留交叉引用不全
§6 只引 R4-3/R4-5/R4-6；r5 遗留另有 4「S5 后需双侧重跑 T4/T5 路径（game-core 腿启用时）」（恰是本设计启用期义务，应进实施序/验收）与 5「多实例并发双写互踩」未引。

## 【次要-15｜E】陈旧注释交叉矛盾
oracle/scenarios/boot-title.txt 注「Metroid MMC1 无 PRG-RAM」与 findings/ram-region-6af4-contradiction.md 定谳（$6000-$6Cxx=WRAM 暂存区）矛盾——该注正是现 watch 只含 zp/iram 的错误理由。WRAM 面实施前须更正（已转 decomp task-15 顺手）。

## 经核对成立（对账用）
N-0012 内容/编号/evidence 与台账逐字一致 ✓；A9 窗口参数化风险与 l2-proof-assumptions 一致 ✓；state-fixture §6 键衔接逐字一致 ✓；判定序「先 (c) 后行为」（v6 B2）✓；基模证真前置门←P1-F3 ✓；「task-8 1/1 正中先例」实存（r7-porter-review L377/L393）✓；1tick≜1帧与 clock.rs 一致 ✓；ROM sha 锚点 ✓；hex 归一「沿 diff_traces 现约」✓；发散归因序框架 ✓；「对拍只裁对不对」与 D7 一致 ✓。

## 遗留风险清单（长期持有）
1. WRAM 上界未定——涉 WRAM 场景分母带软边界，与 A9 同标「弱化面」；
2. tick 内瞬态盲区（帧末快照不见写后回滚）——登记固有盲区，L2 手术级兜底；
3. RNG 定谳后 watch 面/dump 结构变更⇒oracle_sha256 全量换锚（重采集+双跑换锚流程含 run1/run2 对称）；
4. DEFERRED 上线时复检「绝不计 PASS/欠账防删账」随行到 trace 门禁；
5. 共享工作树并发态——裁定以 d0e3e1e 已入库内容+analysis/ 既有工件为准（记时点）；
6. 突变配额与判别值双门并行，防凑配额降判别值；
7. 帧号口径修订（f=1）须同步核对 state-fixture F_a「绝对帧 f≥800」解释口径防一帧错位。
