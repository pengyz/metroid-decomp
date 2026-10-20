#!/usr/bin/env bash
# gates/miri.sh —— Miri UB 面门禁（代码质量预防性门禁）
#
# 判定逻辑只在 gates/（AGENTS.md §2）；门禁不得绕过/弱化/删除。
# 基线影响=新门禁全绿起步（全 workspace 0 unsafe=预防性门禁——防未来 unsafe 引入 UB）。
#
# 用法：bash gates/miri.sh [-p <crate>]（默认 game-core（纯逻辑 crate=最佳面））
# 判定：Miri 全测 0 失败=PASS；任何 UB/失败=fail-loud FAIL。
# 限制（显式注）：
#   - Miri=解释器（慢）：game-core 537 测 ~4m（预提交不跑——CI/周期门禁面）
#   - Miri 不支持 FFI/asm（game-oracle Mesen 面不适用——该 crate 不进 Miri 门禁）
#   - Miri 通过≠代码无 bug（只证 UB 面）——绿≠完整（D7 纪律）
#   - nightly 方案（stable 1.94.0 无 miri 组件——rustup component add miri 受限）：
#     rustup toolchain install nightly --component miri；目标发现须 --lib 显式
#     （nightly cargo 默认目标发现=0 测回归面——--lib 修正=537 测）
set -euo pipefail
CRATE="${2:-game-core}"
if ! command -v cargo >/dev/null 2>&1; then echo "MIRI FAIL: cargo 不在 PATH"; exit 1; fi
if ! rustup toolchain list 2>/dev/null | grep -q nightly; then
  echo "MIRI FAIL: nightly 工具链未装（rustup toolchain install nightly --component miri）"; exit 1
fi
echo "== gates/miri.sh —— Miri UB 面门禁（crate=${CRATE}，慢（解释器）——CI/周期面） =="
export CARGO_HOME="${CARGO_HOME:-/tmp/cargo-home-miri}" CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/tgt-miri}"
START=$(date +%s)
if cargo +nightly miri test -p "${CRATE}" --lib 2>&1 | tee /tmp/miri-run.log | tail -6; then
  if grep -qE 'test result: ok\. [0-9]+ passed; 0 failed' /tmp/miri-run.log; then
    END=$(date +%s)
    echo "MIRI PASS: ${CRATE} UB 面全净（537 测 0 失败（解释器耗时 $((END-START))s）——Miri 通过≠代码无 bug（只证 UB 面，绿≠完整（D7 纪律））"
    exit 0
  fi
fi
echo "MIRI FAIL: ${CRATE} UB 面有失败（见 /tmp/miri-run.log）——fail-loud，禁止弱化"
exit 1
