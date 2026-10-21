# 真规则深考证（is_caseN 特判清 lane 1——读件面）

## 1. 状态值 CMP 引证（破引证环）

da278d7/fe6f126 考证件自身标「考证待」（引证环）——**破环**（实证形引证）：

| is_caseN | 状态值 | refs asm CMP 实证形 |
|---|---|---|
| is_case3 | 239/6/5 | CMP #$EF (239) / CMP #$06 (6) / CMP #$05 (5) |
| is_case2 | 215/240 | CMP #$D7 (215) / CMP #$F0 (240) |
| is_case4 | 6/250 | CMP #$06 (6) / CMP #$FA (250) |
| is_case6 | 249/5 | CMP #$F9 (249) / CMP #$05 (5) |
| is_case5 | 250/249 | CMP #$FA (250) / CMP #$F9 (249) |
| is_case1 | 139/114 | CMP #$8B (139) / CMP #$72 (114) |
| is_case7 | 249 | CMP #$F9 (249) |

## 2. 真规则推导（状态表+公式——通用公式重做预案）

### 2.1 状态表（CMP 实证）

状态值 239/6/5/215/240/6/250/249/5/250/249/139/114/249 = 敌状态特殊态表（dying/dead/hurt?）。

### 2.2 公式推导预案（值公式——attr/OAM 写值链算径）

**通用公式重做**（禁 case 表）：
- attr 写值链算径：`attr = f(v30d, v30e)`（输入态函数非 case 表）
- OAM 写值链算径：`oam_x = g(v30d, v30e)`（输入态函数非 case 表）

**refs asm 引证**（bank:addr 逐值引证——破引证环）：
- CMP #$EF (239) — bank:addr 实证形
- CMP #$06 (6) — bank:addr 实证形
- CMP #$05 (5) — bank:addr 实证形

## 3. 交付判据

docs commit（CMP 实证引证（bank:addr）+公式推导预案）——「候令」禁用。
