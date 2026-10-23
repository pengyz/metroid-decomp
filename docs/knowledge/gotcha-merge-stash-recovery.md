---
type: gotcha
---

# gotcha: merge 丢失实现（stash 恢复流程固化）

> 【2026-10-22 发现】merge（"Merge branch 'porter'"）可能丢失 staged working tree 变更（stash（WIP commit）保留）——恢复径（git restore --source=stash（干净 fast-forward））

## 现象

merge（"Merge branch 'porter'"）可能丢失 staged working tree 变更（stash（WIP commit）保留）——恢复径（git restore --source=stash（干净 fast-forward））

## 根因

merge（"Merge branch 'porter'"）可能丢失 staged working tree 变更（stash（WIP commit）保留）——恢复径（git restore --source=stash（干净 fast-forward））

## 恢复径

1. **检查 stash**：`git stash list`（查看 stash 列表）
2. **恢复实现**：`git restore --source=<stash-commit> -- <file>`（干净 fast-forward）
3. **验证恢复**：`grep -n "pub fn" <file>`（确认实现全在）
4. **验证测试**：`cargo test --workspace`（穷举等价证明）

## 教训

- merge（"Merge branch 'porter'"）可能丢失 staged working tree 变更（stash（WIP commit）保留）
- 恢复径（git restore --source=stash（干净 fast-forward））
- **验证恢复**（grep 确认实现全在）+**验证测试**（cargo test（穷举等价证明））

## 证据

- **stash@{0}（14090744（"WIP on master: 3fc0c7f0"（08:02:13（merge 前被 stash（另一窗 merge 工作流）））））**（frame.rs 131+ lines（块 131/142 实现+测试）+mod.rs 73+ lines）
- **恢复**：`git restore --source=14090744 -- crates/game-core/src/engine/frame.rs`（干净 fast-forward（当前 frame.rs=块 132（stash=块 132+块 131/142）））
- **六实现全在**（grep 实录（apply_samus_slot_search/check_samus_table_compare/dispatch_samus_anim_base（块 131）+get_table_ptr/get_enemy_type_facing/apply_enemy_inc_dec（块 142）（pub fn ×1（各））））
