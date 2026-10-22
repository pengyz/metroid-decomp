# worktree 隔离纪律（内容回迁移）

> 【2026-10-22 用户裁定】认可，注意 worktree 隔离和内容回迁移，写入工作纪律和 timer 纪律。

## 核心问题

共享树 TOCTOU 竞写（多 agent 同时修改同一文件）→测试用例意外失败（测试跑不起来）（git index.lock races）。

## 一、worktree 隔离（每 agent 独立 worktree）

**worktree 隔离**（每 agent 独立 worktree）：
```
git worktree add ../metroid-decomp-decomp decomp
git worktree add ../metroid-decomp-port2 port2
git worktree add ../metroid-decomp-porter porter
git worktree add ../metroid-decomp-verifier verifier
```

**隔离规则**：
1. 每 agent 独立 worktree（避免 git index.lock 竞争）
2. 每 agent 独立文件（避免同一文件并发写入）
3. 每 agent 独立 CARGO_TARGET_DIR+CARGO_HOME（避免编译竞争）

## 二、文件级分工（每 agent 独立文件）

**文件级分工**（每 agent 独立文件）：
```
decomp: engine/frame.rs（独立文件）
port2: engine/nmi.rs（独立文件）
porter: engine/boot.rs（独立文件）
verifier: docs/rounds/（独立目录）
```

**分工规则**：
1. 每 agent 独立文件（避免同一文件并发写入）
2. 文件边界清晰（避免重叠）
3. 依赖关系明确（先底层（后上层））

## 三、内容回迁移（完成后合并）

**内容回迁移**（完成后合并）：
```
git merge decomp port2 porter verifier
```

**回迁移规则**：
1. 完成后合并（每 agent 完成后（合并到主分支））
2. 冲突解决（按 AGENTS.md §4 定位层（修复根因））
3. 验证合并（cargo test（穷举等价证明）（实测输出原文（A 级）））

## 四、工作纪律（worktree 隔离+内容回迁移）

**工作纪律**：
1. **worktree 隔离**（每 agent 独立 worktree（避免 git index.lock 竞争））
2. **文件级分工**（每 agent 独立文件（避免同一文件并发写入））
3. **内容回迁移**（完成后合并（冲突解决（验证合并）））
4. **timer 纪律**（worktree 检查（隔离检查（回迁移检查）））

## 五、timer 纪律（worktree 检查）

**timer 纪律**（worktree 检查）：
```
timer 检查（worktree 隔离）：
①worktree 隔离检查（每 agent 独立 worktree（是否隔离））
②文件级分工检查（每 agent 独立文件（是否分工））
③内容回迁移检查（完成后合并（是否回迁移））
④冲突解决检查（冲突（是否解决））
⑤验证合并检查（cargo test（是否验证））
```

> 【2026-10-22 用户澄清】内容回迁要协调者来做，迁移完成验证通过才继续派单。
>
> **内容回迁规则**（协调者执行）：
> 1. **协调者执行回迁**（lead 负责内容回迁（非 agent 自行合并））
> 2. **迁移完成验证**（cargo test（穷举等价证明）（实测输出原文（A 级）））
> 3. **验证通过才继续派单**（迁移完成验证通过后（才继续派单））
>
> **回迁流程**（协调者执行）：
> 1. agent 完成工作（独立 worktree（独立文件））
> 2. 协调者执行回迁（git merge（冲突解决））
> 3. 迁移完成验证（cargo test（穷举等价证明）（实测输出原文（A 级）））
> 4. 验证通过才继续派单（迁移完成验证通过后（才继续派单））
