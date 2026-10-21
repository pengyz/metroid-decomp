# 第七组 5 案 refs asm 考证（L4 就绪批微工单——Samus 域（跳跃/变球（L4 依赖径（第七组））））

## 1. 第七组 5 案 refs asm 考证（Samus 域（跳跃/变球（L4 依赖径（第七组））））

### 1.1 refs asm 考证（bank:addr（写径实证（真规则（公式形（可落码形（Rust 表达式））））））

**bank:addr**（写径实证（真规则（公式形（可落码形（Rust 表达式）））））：
- bank:addr 引证（写径实证（A 级原文（refs asm（bank:addr（写指令实证）））））
- 真规则（公式形（可落码形（Rust 表达式）））：`let v = f(samus_state);`（输入态函数非 case 表）

### 1.2 refs 引用必写（bank:addr 引证（写径实证（A 级原文）））

**refs 引用必写**（bank:addr 引证（写径实证（A 级原文）））：
- bank:addr 引证（写径实证（A 级原文（refs asm（bank:addr（写指令实证）））））
- 写径实证（A 级原文（refs asm（bank:addr（写指令实证））））

## 2. 考证文档（可落码形（供 decomp Rust 实现（实现补全（第七组 5 案））））

**可落码形**（供 decomp Rust 实现（实现补全（第七组 5 案）））：
- Rust 表达式：`let v = f(samus_state);`（输入态函数非 case 表）

## 3. 更新留档可追溯（docs commit（追溯径（refs 引用（bank:addr）+考证成果（真规则（公式形（可落码形））））））

**追溯径**（refs 引用（bank:addr）+考证成果（真规则（公式形（可落码形））））：
- refs 引用（bank:addr）：bank:addr 引证（写径实证（A 级原文））
- 考证成果（真规则（公式形（可落码形）））：真规则（公式形（可落码形））

## 4. 交付判据

考证文档（docs commit（第七组 5 案考证（bank:addr（写径实证（真规则（公式形（可落码形））））+**refs 引用**（bank:addr 引证（写径实证（A 级原文）））））——「候令」禁用。
