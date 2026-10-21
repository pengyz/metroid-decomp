# is_case0 族值公式实做形对照表（B 清算收口关键路径④——预案→落码形）

## 1. is_case0 族值公式实做形对照表（32 处+4175 系 49 处）

### 1.1 对照表（每处→公式形（可落码形（Rust 表达式）））

| 处 | 状态值 | 公式形（Rust 表达式） | 备注 |
|---|---|---|---|
| is_case0 族 32 处 | 183/189 (3927) | `let v = (v30d as u16 * 3 + v30e as u16) as u8;` | 通用公式（禁 case 表） |
| is_case0 族 32 处 | 30/162 (4177) | `let v = (v30d as u16 + v30e as u16 * 3) as u8;` | 通用公式（禁 case 表） |
| 4175 系 49 处 | 6 vs v30e-8 | `let v = if v30e >= 8 { v30e - 8 } else { v30e };` | 通用公式（禁 case 表） |

### 1.2 state_table_lookup（enemy_state8/9_is_special 形）后值公式

| 处 | 状态值 | 公式形（Rust 表达式） | 备注 |
|---|---|---|---|
| state_table_lookup | 239/6/5 | `let v = v30d;` | 通用公式（禁 case 表） |
| state_table_lookup | 215/240 | `let v = v30d;` | 通用公式（禁 case 表） |

## 2. 实做预案补全（供 decomp 落码（纯公式形（零字面特值）））

**通用公式重做**（禁 case 表）：
- attr 写值链算径：`attr = f(v30d, v30e)`（输入态函数非 case 表）
- OAM 写值链算径：`oam_x = g(v30d, v30e)`（输入态函数非 case 表）

## 3. 交付判据

docs commit（对照表（可落码形（decomp 直接可用）））——「候令」禁用。
