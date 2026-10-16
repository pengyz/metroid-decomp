# findings：L3 首跑 A 引爆报告（harness 烟测——task-24 迁移面验收实证）

> 2026-10-16；lead 拆 A/B 两跑裁定的 A 跑交付物。A 跑成功判据=「引爆报告」（非分歧报告
> ——title 簇未接 ⇒ tick3 必爆 bank00:8000 ⇒ 无 dump 即无分歧报告，检测面设计使然）。

## 检测面四证据（运行时输出原文）

| 面 | 证据 | 判定 |
|---|---|---|
| fail-loud 退出 | 「bin 退出码=101」 | ✅ 引爆即非零退出 |
| stale fail-open 防御 | 「[stale-clean] 旧 dump 已清: analysis/dump/boot-title.frames.jsonl」（预置 888 帧伪件被清） | ✅ 旧腿不复用 |
| 引爆点精确 | 「unported @ bank00:8000」（panicked at frame/mainloop.rs:69=go_main_routine 会师点） | ✅ bank:addr 精确爆 |
| dump 零落盘 | 「dump 目录条目数 0」 | ✅ 禁半成品绿 |

## 三 tick 语义运行时复核

引爆位置=**tick3 主循环相**（main_loop_iter→go_main_routine）——tick1（PreSpin 链）/
tick2（NMI 全通+boot_post_spin 链）静默通过，tick3 进主循环相后爆会师点——
**契约 §8.1 tick 内序**（NMI 唤醒→帧工作→清 0→spin）运行时形态自证。

## 与 task-24 采样迁移面的关系

- 恒 1 断言（$1A spin 出口）：NMI 尾写面单测固化（nmi_tail_writes_constant_one）——
  A 跑无 dump 故无 diff 侧门控实测，**门控断言留 B 跑**（300 帧全跑时 diff_traces
  sample_point=spin 出口语义验收）。
- stale 防御/原子落盘（临时名+rename）=v4 配套两实施项的运行时复验 ✓（本报告即实证）。

## 预期首分歧面（B 跑备忘）

a) 主循环相 title 簇未接（会师点桩——port2 task-23 接齐后 300 帧可达标题稳态）
b) 帧界采样对齐（spin 出口 vs oracle inputPolled——task-24 对表项）
c) PPU 渲染面盲区（CHR 写不可观测=watch 面外——lead 注记照留）

## B 跑判据（原计划④）

300 帧全跑+首分歧分类报告（每类一张任务卡）；时机=port2 title 簇批 4b/5 接入后评估。

## 复现命令

```
echo '{"f":888,"w":{"zp":"AA","iram":"BB"}}' > analysis/dump/boot-title.frames.jsonl
cargo run -q -p game-oracle --bin l3-dump -- --scenario boot-title
# 预期：exit 101 / [stale-clean]… / unported @ bank00:8000 / dump 目录 0 项
```
