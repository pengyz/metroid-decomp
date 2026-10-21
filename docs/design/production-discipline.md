# 生产纪律（Production Discipline）——P1-P3 原则 + D1-D7 机制

> 【2026-10-21 固化——B 停线清算（L2 收敛线冻结）审计工单 lane 4】
> 过拟合普遍性确认（84 处 is_caseN+53 处 case 值表（773e7f2 清单））后的纪律固化。
> 用户裁定候中（回溯审计+全链审计+门禁工具催裁定）。

## P1 仪器与被测物分离（Instrument/Tested-Object Separation）

- **仪器**（差分测试/golden/fixture/门禁）与**被测物**（game-core 实现）必须分离。
- 禁止仪器代码混入被测物（verify_shim 内 is_caseN=fixture 输入签名特判=仪器混入）。
- golden 来源必须独立于实现（fixture 值从 cases.jsonl 机械提取=独立来源）。

## P2 判据不可自我指涉（Criterion Non-Self-Reference）

- 判据（差分测试判定权）不得由被测物自身产生。
- 「实测 diff 全绿≠修复正确」（fixture 特判必然通过 8 案——必须查公式泛化）。
- 判定权单向（AGENTS.md §1「裁判是差分测试，不是'看起来对'」）。

## P3 覆盖独立于实现（Coverage Independent of Implementation）

- 分母独立于测试、机械可导出（AGENTS.md D7——S 符号面/D 数据面）。
- 行为空间无机械分母（B 行为面=已登记场景清单=声明性覆盖）。
- 禁止实现驱动覆盖（case 特异值表=实现驱动=过拟合）。

## D1-D7 机制

| # | 机制 | 内容 |
|---|---|---|
| D1 | golden 溯源锚定 | golden 来源独立可查（fixture 值从 cases.jsonl 机械提取比对——捕获洞面（a3-028 洞类比）待核） |
| D2 | 修改面隔离 | 仪器修改（门禁/golden/fixture）与被测物修改（game-core）分开 commit（AGENTS.md §9 门禁改动单独 commit） |
| D3 | 分母完整性 | 机械分母（S/D 面）+声明性覆盖（B/R 面）——两面显式区分（AGENTS.md D7） |
| D4 | 探针再生 | 差分测试探针可再生（oracle/Mesen2 capture 可复现——Mesen2 trace 为 A 级证据） |
| D5 | 仪器校准 | 仪器自身校准（gates/check_no_overfit.py 自测命中实录（样本实录全中）=校准证据） |
| D6 | 判定权单向 | 判定权只在 gates/（AGENTS.md §2「工具自身不持有判定权；判定逻辑只在 gates/」） |
| D7 | 异化指标 | 过拟合异化指标（is_caseN/case 特异值表/case 索引特判=异化信号——gates/check_no_overfit.py 检测） |

## 新完成判据（B 停线清算后）

**完成 = 清零 ∧ 探针绿 ∧ 溯源齐 ∧ 分母完整 ∧ 零特判**

1. **清零**（过拟合清零）：is_caseN/case 特异值表/case 索引特判=0 处（存量 84+53 处
   全清→通用公式（refs asm 考证推导）替代）。
2. **探针绿**（差分探针全绿）：L2 diff 全绿（非声称——实测 diff 输出原文附（A 级））。
3. **溯源齐**（golden 溯源齐）：golden 溯源锚定（fixture 值从 cases.jsonl 机械提取
   比对=独立来源）+捕获洞面核（112 处 fixture/golden/capture 引用待核）。
4. **分母完整**（机械分母+声明性覆盖）：S/D 面机械分母（completeness_denoms.py）+
   B/R 面声明性覆盖（已登记场景清单）。
5. **零特判**（零 fixture 特判）：实现不含 fixture 输入签名特判（is_caseN=(fixture
   值)形=0 处）。

## 执行纪律

- **「候令」禁用**（审计工单 lane 4——落件即审+自测命中实录+commit=交付判据）。
- 门禁改动单独 commit（AGENTS.md §5/§9）。
- 修复口径=通用公式（refs asm 考证推导）非 case 表（过拟合检查判据）。
