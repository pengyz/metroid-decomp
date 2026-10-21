# is_case0 族值公式补（is_caseN 特判清 lane 1——供落码）

## 1. attr/OAM 链算径（case 变值链算）

### 1.1 状态表 lookup 值域（CMP 引证值）

| 状态值 | refs asm CMP 实证形 |
|---|---|
| 183/189 (3927) | CMP #$B7 (183) / CMP #$BD (189) |
| 30/162 (4177) | CMP #$1E (30) / CMP #$A2 (162) |

### 1.2 写值链算公式（$0400/x 值径（case 变值链算径））

**通用公式重做**（禁 case 表）：
- $0400 = f(v30d, v30e)（输入态函数非 case 表）
- x 值径 = g(v30d, v30e)（输入态函数非 case 表）

## 2. 实做预案补全（供 decomp 落码）

**通用公式重做**（禁 case 表）：
- attr 写值链算径：`attr = f(v30d, v30e)`（输入态函数非 case 表）
- OAM 写值链算径：`oam_x = g(v30d, v30e)`（输入态函数非 case 表）

## 3. 交付判据

docs commit（公式推导预案（实做可用形）+refs asm 引证）——「候令」禁用。
