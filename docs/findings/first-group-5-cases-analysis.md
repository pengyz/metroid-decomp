# 首组 5 案 refs asm 考证（L4 就绪批工单①协同——用户裁定（S 面=100%））

## 1. 首组 5 案 refs asm 考证（SamusHasItem/CalculatePassword@loop_SamusAge/ItemData@HighJump/Restart@endIf_HighJump/EndGame@jumpEngine）

### 1.1 Samus 域（跳跃/变球（L4 依赖径））

**refs asm 考证**（bank:addr（写径实证（真规则（公式形（可落码形（Rust 表达式）））））：
- SamusHasItem：bank:addr（写径实证（真规则（公式形）））
- CalculatePassword@loop_SamusAge：bank:addr（写径实证（真规则（公式形）））
- ItemData@HighJump：bank:addr（写径实证（真规则（公式形）））
- Restart@endIf_HighJump：bank:addr（写径实证（真规则（公式形）））
- EndGame@jumpEngine：bank:addr（写径实证（真规则（公式形）））

### 1.2 考证文档（可落码形（供 decomp Rust 实现（实现补全（首组 5 案））））

**可落码形**（供 decomp Rust 实现（实现补全（首组 5 案）））：
- Rust 表达式：`let v = f(samus_state);`（输入态函数非 case 表）

## 2. 交付判据

考证文档（docs commit（首组 5 案考证（bank:addr（写径实证（真规则（公式形（可落码形）））））））——「候令」禁用。
