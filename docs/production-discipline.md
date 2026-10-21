# 生产纪律（production-discipline）

> B 清算收口——D2 制度修件。本文件为卡制分离规约（实现/用例 commit 分离）的单一真相源。

## 1. 卡制分离规约（D2 落实）

### 1.1 实现/用例 commit 分离

**流程债**（feat(core,oracle) 混提形系统性违例（audit-ledger ②（dc8a253）））——**D2 落实**：

- **实现 commit**（feat(core)）：仅触及 `crates/` 下实现文件（game-core/game-data/game-platform/game-oracle 的 src 代码）。
- **用例 commit**（feat(oracle)）：仅触及 `oracle/` 下用例文件（cases.jsonl/fixtures/trace 等）。
- **禁止混提**：一个 commit 不得同时触及 `crates/` 与 `oracle/`（违例红）。

### 1.2 D2 修改面隔离判据

| 修改面 | 允许触及 | 禁止触及（违例红） |
|---|---|---|
| 实现 commit | `crates/**` | `oracle/**` |
| 用例 commit | `oracle/**` | `crates/**` |
| 混合面 | — | `crates/` + `oracle/` 同 commit = **违例红** |

### 1.3 后续卡制须分离

**后续卡制须分离**（实现 commit/用例 commit 分离）：
- 每张卡的实现改动与用例改动分属两个独立 commit。
- commit message 明确标注归属（实现/用例）。
- 门禁 `gates/cli.py check` 对混提形报红（subroutine-diff/rust-tests 联动）。

## 2. D2 制度修背景

- **audit-ledger ②（dc8a253）**：feat(core,oracle) 混提形系统性违例——实现与用例同 commit 提交，导致回滚/审计面混淆。
- **D2 制度修**：卡制分离规约（实现/用例 commit 分离）——本文件为规约形落。

## 3. 交付判据

production-discipline.md 补充 commit（规约形落）——「候令」禁用。
