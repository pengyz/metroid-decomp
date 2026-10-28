# 语义层定谳与 L2 投影层退役（2026-10-28 架构裁定）

## 裁定链（证伪文化实录）
1. 初判（lead）：投影层降格为「L2 oracle 适配层」保留 —— 三层方案。
2. 用户第一性质询（两次，逐层深入）：
   - 验收唯一落在 L4；投影层非 L4 必要条件。
   - engine≈纯投影（543 fn）；game-oracle 对 engine 零消费（grep 实证）；L4 链
     （frame 语义主环→RenderState→渲染）不经 engine。
   - 双层=两次独立移植，互证不成立；L4 分歧时经我方投影诊断=用第二份可能出错的
     移植查第一份（自指）。
   - 真值链必须外部：语义代码↔m1disasm 汇编（人读对拍）+ 语义状态↔Mesen2 RAM/trace
     （机器对拍）。
3. 终裁（lead 覆写自身初判）：**投影层整体退役**（非降格、非 fixtures）。

## 删除面（本批 commit 谱系）
- game-core：engine/（17,215 行/543 投影签名 fn）、verify_shim/（6,931）、l2ports.rs
  （3,281）、lib.rs L2In/L2Out 契约类型、三处 engine 等价测试 mod
- game-oracle：l2.rs、cpu6502.rs（仅 L2 casegen 消费）、bins l2-casegen/l2-diff、
  tests/l2_engine.rs
- 资产：oracle/subroutines/（4.7M，l2_cases.jsonl+a*-pregen 族）、
  data/status/l2_gap_map.json
- 工具：tools/diff_subroutines.py、tools/anchor_provenance.py、
  gates/check_no_overfit.py（TARGET=已删 verify_shim）
- 门禁：gates/cli.py gate_subroutine_diff+registry 项+PROOF_SURFACE l2ports 行；
  diff_traces.py n12 RNG-L2 案计数腿；gates/README 行
- 基线：rust_test_count 1464→440（gates baseline retire 路径，kind=retire 双留痕）

## 语义迁移面（删除前置，行为零改动）
- room.rs：$EE4A/$EB85 域内实现
- title/step_glue.rs：verify_shim title 适配段机械变换（域件直调）
- boot.rs：C16D 直写内联
- samus/physics.rs+handler.rs+enemy/spawn_control.rs：engine 等价测试 mod 切除
  （语义 fn 本体保留，行为仲裁=L4+域测）

## 防衰退闸（遗留待办）
- 产品 lib 内（memmap/ram_map 除外）魔数地址字面量禁新增——gates 新闸（待建）；
  存量=域内 write-event fns 的 0x[67]xxx 运算址（下一步语义化对象）

## 验证
- cargo test --workspace 436 passed 0 failed（删后全绿）
- L4 run-all 23/39 逐格与清理前一致（match 23 diff 16 blocked 0）——零回归
- gates CHECK: PASS=13 FAIL=0 SKIP=1（retire 路径落账后）
