---
type: pattern
title: Miri UB 面门禁（nightly 方案+--lib 修正）
date: 2026-10-20
evidence: Miri 实测 cargo +nightly miri test -p game-core --lib=537 passed; 0 failed（417.30s）+--lib 修正验证（0→537 测发现）
confidence: 95
---


> 类别：pattern（可复用模式）｜写入：2026-10-20｜写路径例外注记：lead 显式指派
> （本任务四步含 docs/ 落档——standing write-path duty 的显式例外）

## 用法

Miri（Rust 官方 UB 检测器=解释器）防未来 unsafe 引入 UB（全 workspace 0 unsafe=
预防性门禁）。门禁面：`bash gates/miri.sh`（判定逻辑只在 gates/（AGENTS.md §2））。

```bash
rustup toolchain install nightly --component miri   # stable 1.94.0 无 miri 组件
cargo +nightly miri test -p game-core --lib          # --lib 显式目标（关键）
```

## 关键 gotcha：--lib 显式目标（nightly 默认发现回归面）

- `cargo +nightly test -p game-core -- --list` = **0 tests**（nightly cargo 默认目标
  发现回归面）→ `cargo +nightly miri test -p game-core --lib -- --list` = **537 tests**
- 修正=**`--lib` 显式 lib 目标**（测试在 `#[cfg(test)] mod tests`（engine/mod.rs:42）
  内=lib 测试目标）——gates/miri.sh 已固化 `--lib`。

## 限制（显式注）

1. **Miri=解释器（慢）**：game-core 537 测 **~4m**（417s 实测）——**预提交不跑**
   （pre-commit 太慢）——**CI/周期门禁面**（gates/miri.sh 独立调用）。
2. **Miri 不支持 FFI/asm**：game-oracle（Mesen 面）不适用——该 crate 不进 Miri 门禁
   （game-core 纯逻辑 crate=最佳面（无 I/O））。
3. **Miri 通过≠代码无 bug**（只证 UB 面）——**绿≠完整**（D7 纪律）；门禁不弱化。
4. **nightly 方案**（stable 受限的 fallback）：stable 1.94.0 `rustup component add miri`
   =unavailable（组件仅 nightly 可装）——nightly 1.101.0 装载（miri 0.1.0）。

## 验证记录（2026-01-20 实测）

- `cargo +nightly miri test -p game-core --lib` = **537 passed; 0 failed**（417.30s）
  =UB 面全净（全 workspace 0 unsafe 预防性门禁全绿起步）。
- 基线影响=新门禁全绿起步（0 unsafe=预防性——防未来 unsafe 引入 UB）。
