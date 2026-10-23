---
type: gotcha
---

# 编译错误循环陷阱（局部修复→反复折腾）

## 问题现象

修复编译错误时陷入循环：见一个局部修一个→修完一个又出现新错误→反复折腾。

## 根因分析

1. **错误信息误导**：错误提示 "remove this `mut`"，但真正问题是 "expected item after doc comment"
2. **局部修复**：只看错误提示→盲目修复→未分析根本原因
3. **重复操作**：同样的修复反复执行→未验证效果→未分析错误链

## 教训

1. **获取完整错误**：先看 `cargo check 2>&1 | grep -A10 "^error"` 获取完整错误
2. **综合分析**：不要见一个局部修一个→先分析所有错误→一次性修复
3. **验证效果**：每次修复后验证→确认错误减少→再继续
4. **入知识库**：修复模式记录→避免重复错误

## 修复模式

```bash
# 1. 获取完整错误（非局部）
cargo check --package game-core 2>&1 | grep -A10 "^error"

# 2. 综合分析（所有错误）
# 3. 一次性修复（非局部）
# 4. 验证效果（错误减少）
```

## 触发条件

- 批量删除代码（如删除重复定义）→遗留孤立 doc comment
- 错误提示误导（"remove this mut" vs "expected item after doc comment"）
- 局部修复循环（见一个修一个）

## 关联知识

- [[gotcha-merge-stash-recovery]]（stash 恢复流程）
- [[pattern-tdd-six-steps]]（TDD 六步流程）
