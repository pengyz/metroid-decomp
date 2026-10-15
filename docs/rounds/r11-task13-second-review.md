# rounds/r11 — task-13 独立二审账本（17f13cec，fd760ed+0172725）

> 归档自二审终报（其自存 /tmp/task13-r10-independent-review.md=仓外，本文件=仓内载体，
> 链规⑪即时归档——r9 漏归档教训不重犯）。审读基准 HEAD 6172856；门禁预检 PASS=10 FAIL=0 SKIP=2。

## verdict: needs_revision

r9 的 15 项：已修 10 / 带残余 2 / 不彻底 2 / 修法不当 1。四点属契约/机械判据级：
①严重-8 谓词引用不存在字段（N-0012 无 status，全库 grep 零命中）②§8.1 定案未接线 v3 主体
（§5.3 tick 末 encode≠spin 出口采样；恒 1 断言未进 gate；现行采样下断言 598/600 帧必红=自家数据自伤）
③阻断-2 逐字不一致（矩阵格语义分叉：记 L0 vs 回 L0 前态）+检测面不可机械（GameRng 桩 payload 无
bank:addr；「0 落盘」≠文件不存在——stale dump fail-open）④§8③ asm 证据指针错位。

## A. r9 逐条核销表

| r9 项 | 判定 | 复现输出要点 |
|---|---|---|
| 阻断-1 f=1+分母 | 已修（残余 2 小项） | l3-dump.rs:73 `for f in 1..=frames`；scenario_capture.lua:48 frame=0+inputPolled:362 先自增；boot-title 300 行 f 1..300 dups 0 逐字复现。残余：(a) §1「oracle frameCount」误引存活（frameCount 仅 probeLog :308-315）(b)「同一输入帧恰一次 poll」与超帧跳过 ReadJoyPads（asm:251-252/:268）互斥缺慢帧实证 |
| 阻断-2 unported! 细分 | 已修不彻底 | pipeline:113 同 commit ✓；检测面措辞两文件不一（「bank:addr」vs「`unported @ bank:addr`」）；路由「记 L0」vs「回 L0 前态」=两种矩阵动作 |
| 阻断-3 注入点两定义 | 已修不彻底 | 拆两定义 ✓；三则判据不可机械闭环（引爆桩位表=game-core 侧/RNG 消费计数无机制/inputPolled=oracle 侧）；「runner 拒跑」未指名 |
| 阻断-4 红线+P7-F1 | 已修 ✓ | analysis/ 不入库；P7-F1 过读撤除；正向采信改引 reference-mesen2-trace-determinism |
| 严重-5 五面+$6E/$6F | 已修 ✓ | 两面钉死/三面缺口清单；$6E/$6F 归 zp（flow_scratch.rs:15） |
| 严重-6 memmap 对账 | 已修 ✓ | 唯一 encode=enemy_slots；memory.rs:68-73 手写切片；l3-dump.rs:31 dead_code+固定两键 |
| 严重-7 隐藏字段 | 已修 ✓ | 键集白名单+schema 校验+导出器声明+写集降级+编号勘误 |
| 严重-8 N-0012 机械门 | **修法不当 ✗** | schema={id,symbol,kind,old,new,level,evidence,confidence,note} 无 status；谓词不可检 |
| 严重-9 RNG 全状态 | 已修 ✓ | 义务四则+「四则全过才许称全状态」 |
| 严重-10 引证/漏键 | 已修（间隔偏软） | 引证改正+时点义务+scenario_sha256 三处 ✓；双跑间隔非机械定义 |
| 严重-11 DEFERRED | 已修 ✓ | l3-deferred 独立账+同场景复跑解锁+L3Deferred 覆盖面+绝不计 PASS |
| 严重-12 双门+mutation_verified | 已修 ✓（实质） | 门 1 族谱对表 r7 战绩；门 2 判别值域；「打回 0/2≠判别力」不计配额 |
| 次要-13 红灯全形态 | 已修 ✓ | 无首分歧分支/sha 停裁换锚/exit 2 先修 schema |
| 次要-14 r5 遗留 | 已修 ✓ | 遗留 4 入验收、5 入并发复检 |
| 次要-15 陈旧注 | 已修 ✓ | boot-title.txt 已改 |

## B. §8.1 契约定案核
- B1 spin=跨帧论证链自洽 ✓（归谬成立+实现已采纳）；**§8③ 指针错位**：ClearNMIStat(:1117)=函数体非每帧环；清 0/spin 在 :210-218、@loop 轮询 :1113-1114——语义对、指针错。
- B2 恒 1 断言部分可机检：game-core 单测+栈页排除（INTERRUPT_FRAME_EXCLUDE 显式计数）✓；diff_traces 无 $1A 逐帧断言、oracle 采样仍 endFrame、采样点迁移无实施槽；§5.3「tick 末 encode」（$1A=0）≠§8.1「spin 出口」（$1A=1）——**按自家数据 598/600 帧必红**。
- B3「采样点盲区」定案结论成立但机制归因与自家数据相抵（v=1:600+v=0:599=写全被捕获）——真机制=**归属失效**（scanline/pc 回显陈旧+静态只扫 85 1A 漏 84 1A）；6172856 §6:112「写者二选一」错账未撤（实为 $C0C9/$C436/$C10B 三家）。
- B4 §8.1②「三字节=中断帧影子」与 e3f2473 定谳不同框（$01FD 主写=$C0C2 JSR 链 1821 次）——共写区窄化，处置不受影响论证弱化。

## C. 新问题
N1 矩阵格语义分叉；N2 检测面不可机械（GameRng 桩 payload 无址/stale dump fail-open）；N3=B2 接线缺口（阻断级）；N4 inputPolled≡tick 与超帧跳过互斥待实证（重复 f 现则路由缺「游戏减速 quirk」分支）；N5/N6=B1/B4。其余交叉面无矛盾。

## D. 修法质量
实质 ✓（五面降级/写集诚实降级/双门对表）；修措辞没修实质 ✗（严重-8、§8.1 点 2/3）。

## E. r9 归档自足性自验
成立（15 指针逐一重导）；瑕疵：行号漂移三处（L300 实 :308-315；L55-61 现 :68-73；L342-344 现 :383-385）+动态工件无复现命令——建议内容锚（遗留-6）。

## 遗留风险清单
1 采样点迁移⇒oracle_sha256 换锚（并入遗留-3）+历史「帧末 $1A」口径作废明示；2 stale dump fail-open S5 前必补；3 慢帧/inputPolled 实测闭环（重复 f⇒减速 quirk 分支）；4 §6:112 错账待销+watch 处置复查；5 栈页归因收口；6 行号改内容锚；7 N-0012 可机读 status 或换现存字段谓词；8 r9 遗留 1-6 续持；9 pipeline↔design 双写同步需门禁/锚点。

> 【2026-10-15 更正】6266650 commit Evidence 行「grep -Fc '修法不当'…=1」与实测输出不符
> （实为 2：verdict 摘要行+A 核销表各一）——命令级断言与输出脱钩（链规③族又一案，
> Lead 自犯）。文件内容无损，计数以本注为准。

## 返工最小集（v4）
①§8.1 定案回写 §5.3+实施序（采样点迁移槽+断言落 diff_traces）②严重-8 谓词可检+指名 runner
③检测面改「exit 101 ∧ 本次无新 dump（sha 不匹配）」+l3-dump 旧件清理④两文件逐字对齐（含矩阵格语义）
⑤§8③ 行号更正（:210-218/:1113-1114）+§6:112 错账销+栈页归因收口+残余 2 小项。
