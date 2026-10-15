# findings：decode-sweep r3——C27C 分发动态收口 + 待查格定谳（task-7 ①③）

> 状态：**r3 已证实**（2026-10-15，静态 ROM 字节级 + Mesen2 动态 trace 双轨）。
> 基准维持 BASELINE: PASS；换时刻双跑 34/34 三元组全同。
> 前篇：[decode-sweep-report.md](decode-sweep-report.md)（首轮）/
> [decode-sweep-r2-jump-tables.md](decode-sweep-r2-jump-tables.md)（r2）。
> 复跑：`python3 tools/decode_sweep.py`（静态）+ `python3 tools/scenario_run.py run c27c-dispatch`
> + `python3 tools/decode_sweep_dyn.py --traces … --traces2 … --static …`（动态合并）。

## 0. r3 方法增链（环⑤探针七连证伪→真执行语义，全程 A 级实证）

| 探针 | 问题 | 结论 |
|---|---|---|
| v1 | 回调内状态可信吗 | **cpu.pc=回调地址回显（非真 PC）**——pc 过滤法死；emu.read/栈读真 |
| v2 | 站点签名可行吗 | 可行但首版 off-by-one（site=ret-3 应为 ret-2）自纠 |
| v3 | 有单步事件吗 | 未知事件名被宽松接受但只在帧界触发——无指令流事件 |
| v4 | opcode fetch 可见吗 | **字符串 "execute" 注册下不可见**（C0BF 确证执行 0 命中；pos2 操作数可见）|
| v5 | emu API 全集 | 发现 getCdlData/getAccessCounters/step/callbackType 族 |
| v6 | 枚举表内容 | callbackType.exec=2 / eventType.nmi=0 / stepType.step=0… |
| v7 | **字符串 vs 枚举对照** | **`callbackType.exec`=真 opcode fetch 回调**：C0BF 0→55、C0D9 0→59（r2 F1 盲点直接可见）、C27C 0→55 |

**更正（2026-10-15，指向 rounds/r2-coverage-mechanism-review F1）**：「指令级 oracle
覆盖在已验证 Mesen2 Lua API 内不可得」→ **部分更正**：`emu.addMemoryCallback(fn,
emu.callbackType.exec, …)` 枚举注册提供真指令入口观测（v7 对照 A 级）；不可得的是
字符串 `"execute"` 注册（劣化=操作数/数据读）。r2 F1 全部怪象（漏 $C0D9、混操作数
字节）由此解释。**教训入知识库**：
[gotcha-mesen2-callbacktype-string-enum-split](../knowledge/gotcha-mesen2-callbacktype-string-enum-split.md)。

## 1. ③ C27C 跳转表 A 值域动态收口（task-9/FE83 族移植前置）——**交付**

场景 `oracle/scenarios/c27c-dispatch.txt`（F_a 入局链：samus-move3 解锁序列→spawn→
走+打，1600 帧）+ `c27c-boot.txt`（标题侧）。站点签名自校验（ret-2..ret==`20 7C C2`）
**100% 通过**（raw=valid=10968）；换时刻双跑 34/34 三元组、8/8 执行地址全同。

### 1.1 A 值域=handler 选择面（sig=jsr 真入口；窗口指纹随事件采）

| 站点 | A 值域（动态） | target 对账（静态表词） | 命中 | 窗口 bank |
|---|---|---|---|---|
| bank00:$8027 | 0,1,2,3,4,**22,27** | #0-4=$8071/$80D0/$80F9/$812C/$8142 ✓；**22→$90D7、27→$90BA 超静态 16 词** | 533 | 0 |
| bank1:$8B8C | 0,1,2 | $C45C/$8B9D/$8BD5（#0=bank7 固定 handler） | 2440 | 1-5 |
| bank7:$C13E | 0,2,3,8 | $C801/$C8D1/$C92B/$C9D7 ✓ | 1048 | 1-5 |
| bank7:$CC1F | 0,1,2 | $CC36/$CCC2/$D002 ✓（静态曾漏站，r3 修复） | 609 | 1-5 |
| bank7:$CC7A | 0,1,2 | $C45C/$CC98/$CFC3 ✓ | 47 | 1-5 |
| bank7:$D4D0 | 0 | $C45C（空槽 no-op） | 1830 | 1-5 |
| bank7:$D7BA | 0 | $C45C（空槽） | 610 | 1-5 |
| bank7:$EA19 | 0,1,2,3,4 | $C45C/$E5E2/$C45C/$E5E2/$EA26 ✓ 与静态 5 词逐词合 | 15 | 1-5 |
| bank7:$EAE1 | 1,2 | $EB06/$EB8C ✓ | 8 | 1-5 |
| bank7:$F36B | 0,1,2 | $C45C（空槽 1846 次）/$F3BE/$F3E6 ✓ | 2837 | 1-5 |
| bank1:$95E8（sig=other） | 4,5 | $99FA/$9A4C——**$8024 蹦床**（`4c 7c c2`）`JSR $8024` 同表约定（r3 新站点类） | 991 | 1-5 |

要点：
- **$C45C=RTS 共享空 handler**（动态高频确认：空敌槽/空分发格 no-op 返回）；
- **$8024 蹦床发现**（banks0-6 共享头 `4c 7c c2`）：`JSR $8024` 站点同表约定——
  静态扫描已补该站点类（r3 `SITE_PATTERNS`）；
- **A=22/27（$8027 表）超静态 16 词上界**——表长静态近似不足，动态值域是表长真值来源；
- 静态→动态互证：凡动态出现的 (site,A,tgt) 均与静态表词逐字节吻合（对账列 ✓）。

### 1.2 **A9 判定：证伪**（l2-proof-assumptions A9 的 r3 动态收口）

- **窗口指纹自证**（$8000-$800F 16B×1600 帧 + 每分发事件快照；离线对各 PRG bank
  首 16B 比对）：f1-541=bank0（标题），**f542-1600 全部=banks1-5 类（区域 bank）**；
  bank7 指纹 `8A48A205…` 唯一可辨，**游戏期 0 帧、0 个分发事件落在 bank7**；
- 即：bank7 引擎代码（F36B/F2CA 族）运行时 $8000 窗口=**区域 bank**（敌人属性表
  $95CE/$95CF/$968B+Y 按区域取数——语义即「按区域查表」）；
- **结论（A 级动态）**：A9「运行时 $8000 窗口=bank7」**不成立**——game-core const 表
  按 window_bank=7 烘焙需**窗口参数化**（L2 case window_bank 字段即契约面）；
  影响 F282/F2E8/F2ED/F2CA 族（task-8/9）。原假设文件已加更正注（更正文化）。

## 2. ① 待查格定谳（bank7 七格；banks0-6 两大岛缓排照旧）

| 格 | 岛/未覆盖段 | r3 判定 | 证据 |
|---|---|---|---|
| D-REG-07-011 | $D3A8-$D4BE | **定谳 code** | 静态 r3（尾分发终止语义）coverage→1.0 |
| D-REG-07-027 | $E770-$E791，未覆盖 $E77B-$E782 | **定谳 code** | **A 级执行**：$E77B/$E77D/$E77F/$E780=指令入口（未命中 4B=操作数）；静态 0.765+3 入口 |
| D-REG-07-028 | $E8F1-$E90E，未覆盖 $E8FC-$E903 | **定谳 code** | **A 级执行**：$E8FC/$E8FE/$E900/$E901 同上 |
| D-REG-07-012 | $D51F-$D5E3，未覆盖 $D58B-$D5C0 | **定谳混合** | 段=内嵌表（消费者 $D53C:LDA/$D541:LDA）；岛体 code 0.726+7 入口 → 代码段入 S、表段留 D |
| D-REG-07-007 | $D150-$D1ED，尾 $D1D7-$D1ED(23B) | **基本定谳 code** | 岛体 0.854+尾段 0 执行/0 消费者（路径受限）——尾段待查收窄至 23B |
| D-REG-07-018 | $DAD2-$DAD9 | **基本定谳 code** | 0.875+尾 1B 待查 |
| D-REG-07-048 | $FB88-$FBDC，未覆盖 $FBCA-$FBDC(19B) | 残余待查 | 0 执行（路径受限）+0 消费者 |
| D-REG-07-050 | $FCC1-$FD8E，未覆盖 $FD08-$FD5E(87B)+$FD84-$FD8E(11B) | 残余待查 | 同上（87B 大段；疑非 C27C 表族数据） |

r2 已定项复核维持：051（$FE79-$FE82 表消费者 $FE6A:LDA + $FE83-$FECA 代码）、
数据面确认 5 格（009/019/021/025/047，消费者证据）。
**残余待查收窄：22 疑似格 → 2 格（048/050）+ 2 个尾段（007 尾 23B、018 尾 1B）+ banks0-6 缓排 2 格。**

## 3. 静态 r3 演进（三修后 BASELINE 仍 PASS）

```
[decode-sweep] verdict：code=42  data=224  未决=19（岛 285；被采纳流 404，被拒 235，入口候选 709；盲扫升级 5；C27C 跳转表入口 253）
  正对照 bank07:$F0A2-$F148 所在岛 → code ✓  9/9 调用点复现 ✓
  负对照 4/4 所在岛 → data ✓
BASELINE: PASS
```

r2→r3 静态三修（动态破案直接驱动）：
1. **尾分发终止语义**：`JSR $C27C`/`JSR $8024` 永不返回（JMP ($000C) 分发+handler RTS
   回原调用者）⇒ 线性流视作终止——解 CC36/CC1F「穿表假非法」；
2. **窗口目标域**：banks0-6 站点词可落 bank7 固定区（$C45C 案）——分类/试读补
   $C000-$FFFF 归属 bank7；
3. **蹦床站点类**：`20 24 80`（JSR $8024）入 SITE_PATTERNS，同表约定。
code 35→42（D0B5-D146/D3A8-D4BE 等迁定）、跳转表词 118→253。

## 4. 否定结论三查执行记录（换工具+换时刻+记时点）

- **换工具**：静态（decode_sweep）× 动态（callbackType.exec 执行集）× 消费者核查；
- **换时刻**：c27c-dispatch 双跑（14:51/15:0x 两时点）：**34/34 三元组、8/8 执行地址
  全同**（与 reference-mesen2-trace-determinism 一致）；中途一跑 Mesen 静默夭折
  （634/1600 帧 done=False）被 run 清单如实拦截重跑——产物完整性检查不可省；
- **记时点**：场景 sha256（`14be0f79…`）+ generated_lua sha256 入 run.json；
  0 执行结论一律标「路径受限」（本场景=单房间走打，未覆盖门/Boss/道具路径）。

## 5. 证据分级

| 结论 | 级 | 依据 |
|---|---|---|
| callbackType.exec=真指令入口（vs 字符串劣化） | **A** | v7 双注册对照（C0BF/C0D9/C27C 计数翻转）+ 签名 100% 自校验 |
| A 值域表（§1.1） | **A** | 10968 事件、双跑全同、target 与 ROM 表词逐字节对账 |
| **A9 证伪**（窗口=区域 bank） | **A** | 窗口指纹×1600 帧×每事件快照，bank7 指纹 0 命中 |
| 027/028 定谳 code | **A** | 执行地址+静态流+入口佐证 |
| 007/018 基本定谳、051/012 段级定谳 | **A/B** | 静态流+消费者（B）+部分执行（A） |
| 048/050 待查 | — | 显式缺口类（D7），非结论 |
| 8027 表 A=22/27 超静态上界 | **B** | 动态值域为表长真值来源；静态双规则是近似（遗留） |

## 6. 遗留

1. **048/050 残余**（19B+98B）：需门/Boss/道具场景路径（新场景 DSL）或非 C27C 表族解析；
2. **A=22/27 表长**：动态值域并入静态表长推断（decode_sweep 读 --traces 融合）待做；
3. banks0-6 大岛（缓排）；判据④（循环同构）仍未实现；
4. getAccessCounters/getCdlData/stepType.step 语义未摸完（v5-v7 边际）——CDL 全量
   覆盖面是 ① 的终极收口，留后续；
5. **A9 参数化改造归 task-8/9**（本报告为证据面；L2 case window_bank 契约字段）。

## 附录：产物指针

- 场景：`oracle/scenarios/c27c-dispatch.txt` / `c27c-boot.txt`（DSL 扩展
  `coverage_pc`/`probe_dispatch`，tools/scenario_run.py + scenario_capture.lua）；
- 动态产物：`analysis/traces/c27c-dispatch.*`（dispatch_summary/exec_cov/window/run.json）
  + `.run1/`（换时刻跑 1 存档）；
- 合并器：`tools/decode_sweep_dyn.py`；静态：`tools/decode_sweep.py --json`；
- 诊断探针（环⑤七连）：`tools/probe_dispatch_diag.lua`（v7 终版）+
  `analysis/probe_dispatch_diag*.log`。
