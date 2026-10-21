# 探针基建（Probe Infrastructure）——D4 落实（泛化探针）

> 【2026-10-21 设计件——B 清算收敛审计工单 lane 4】
> 过拟合普遍性确认（84 处 is_caseN+53 处 case 值表+49 处字面 vs 公式分支形）后
> D4 落实（探针再生）——判据信号升级（fixture 绿/探针绿分歧度=过拟合警报指标）。

## 泛化探针（Generalization Probe）

**目标**：验证通用公式泛化性（非 fixture 特判）——fixture 绿≠修复正确（fixture 特判
必然通过 8 案——必须查公式泛化）。

### 1. fixture 变异探针（Fixture Mutation Probe）

- **LCG 输入变异**（边界值/随机值/翻转位）→ **Mesen2 取真** → **对拍**（golden vs core）。
- 变异径：
  - **边界值**（0x00/0xFF/0x80/0x7F/0x01/0xFE——fixture 值域边界）
  - **随机值**（LCG 生成（fixture 值域内随机））
  - **翻转位**（fixture 值单 bit 翻转（8 位×8 变异））
- 变异量：每 fixture 值×N 变异（N≥16（边界 6+随机 8+翻转 8））

### 2. 批捕获径（Batch Capture Path）

- **Mesen2 capture**（每变异→capture（LCG 输入→golden）——批捕获（fixture 注入
  （PLAN.fixtures DSL（frame/addr/value/persistence））→capture×N→per-variant
  in/out 提取））。
- 捕获径：scenario_capture probe_mem/watch_ranges DSL（plan 文件+族扩）。
- 批量：N 变异×capture（~45s each）→批捕获（N×45s）。

### 3. 对拍径（Comparison Path）

- **对拍**（golden vs core）：每变异→L2 diff（golden == ROM 执行 == game-core）。
- 判据信号升级：
  - **fixture 绿**（原 8 案全绿）+**探针绿**（变异探针全绿）=**泛化确认**（通用公式
    正确）。
  - **fixture 绿**+**探针红**（变异探针红）=**过拟合警报**（fixture 特判（公式泛化缺））。
  - **分歧度**（fixture 绿/探针绿分歧度=过拟合警报指标——分歧度=探针红案数/变异
    总数）。

## 判据信号升级（Judgment Signal Upgrade）

| fixture 绿 | 探针绿 | 判定 | 指标 |
|---|---|---|---|
| ✓ | ✓ | 泛化确认（通用公式正确） | 分歧度=0 |
| ✓ | ✗ | **过拟合警报**（fixture 特判） | 分歧度>0 |
| ✗ | ✗ | 实现缺口（非过拟合） | 分歧度>0 |
| ✗ | ✓ | 探针覆盖不足（待扩） | 分歧度<0（不可能） |

**分歧度 = 探针红案数 / 变异总数**（过拟合警报指标——分歧度>0=过拟合嫌疑）。

## D4 落实（探针再生）

- **探针可再生**（oracle/Mesen2 capture 可复现——Mesen2 trace 为 A 级证据）。
- **探针独立**（fixture 变异独立于实现（LCG 输入变异=独立来源））。
- **探针校准**（探针自身校准（gates/check_no_overfit.py 自测命中实录=校准证据））。

## 执行纪律

- 「候令」禁用（审计工单 lane 4）。
- 修复口径=通用公式（refs asm 考证推导）非 case 表（过拟合检查判据）。
- 实测 diff 全绿≠修复正确（fixture 特判必然通过 8 案——必须查公式泛化）。
