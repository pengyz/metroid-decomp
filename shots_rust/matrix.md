# L4 对拍矩阵（用例×采集点）

- 生成：`python3 tools/l4_diff_run.py run-all`（tools/l4_diff_run.py）
- 三 sha 锚（每行）：`scenario_sha256` + `rom_sha256` + 产物 sha（`artifacts`，逐件 sha 见 matrix.json `rows[].artifacts`）；rom_sha256=`e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6`
- **分母 = 用例×采集点 = 7**（含 blocked 0）——**blocked 计入分母、永不计 pass**（防假绿硬约束）
- 汇总：match 4 · diff 3 · blocked 0 ｜ **pass 4/7**（pass=match 数）
- 总评：❌ verdict=not-pass（blocked/diff 均在分母内且不计 pass）

## 用例行（三 sha 锚定）

| 用例 | scenario_sha256 | rom_sha256 | sched_sha256 | 产物sha | snapshot | 腿来源 rust/emu | match/diff/blocked |
|---|---|---|---|---|---|---|---|
| case-001-game-start-samus-interaction | `23b52bdc841e702020ad97ddf202822cc56e9b0c4669daa27a1cc7c9dadf970f` | `e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6` | `e1e42d2ff8d43dfb6f87bffc3345abac1a064ce941e9e117f2ad904a4a1eae6e` | 14 件 | none(上电起跑) | fresh/fresh | 4/3/0 |

## 采集点格（三态：match | diff | blocked；来源=腿 provenance）

| 用例 | frame | tag | 状态 | 来源 rust/emu | 差异像素/总 | 说明 |
|---|---|---|---|---|---|---|
| case-001-game-start-samus-interaction | 420 | game-in | ✅match | fresh/fresh | 0/61440 |  |
| case-001-game-start-samus-interaction | 550 | after-right | ✅match | fresh/fresh | 0/61440 |  |
| case-001-game-start-samus-interaction | 650 | after-left | ✅match | fresh/fresh | 0/61440 |  |
| case-001-game-start-samus-interaction | 720 | after-jump | ✅match | fresh/fresh | 0/61440 |  |
| case-001-game-start-samus-interaction | 820 | after-shoot | ❌diff | fresh/fresh | 513/61440 |  |
| case-001-game-start-samus-interaction | 950 | combo-action | ❌diff | fresh/fresh | 19084/61440 |  |
| case-001-game-start-samus-interaction | 1200 | end | ❌diff | fresh/fresh | 19212/61440 |  |

