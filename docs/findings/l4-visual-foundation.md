# findings：L4 视觉对拍奠基（task-31，验证体系收官层）

> 状态：**L4 落成**（2026-10-16）——visual-diff 门禁 SKIP→**ENABLED**（7 金面帧全绿）。
> 复跑：`python3 tools/diff_visual.py`（门禁面）/ `--selftest`（正负对照）。

## 1. 交付四件

| 件 | 状态 |
|---|---|
| ① tools/diff_visual.py | **像素级对拍**（纯 stdlib PNG 解码：IHDR+IDAT+unfilter，8 位 RGB/RGBA 非隔行，其余显式拒收）；白名单机制（glob+rect）；**正负对照随跑自证**（selftest 破即拒跑——判别力基线原则） |
| ② 截图采集面 | oracle 侧在位（emu.takeScreenshot→shots/，任务链既有）；**game-core 渲染腿=S6**（显式 note 同 L3 game-core 腿口径，启用后接第二腿同机制） |
| ③ gates/visual_whitelist.json | **建立（空表零容差）**——唯一允许的例外清单（AGENTS §5）；条目格式 {file glob, rect, reason, approved_by}；变更需用户确认 |
| ④ 首例视觉场景 | boot-title 7 帧金面（f30 boot/f60 probe/f120 init/f180 probe/f240 title-in/f299 title/f300 probe）——**CHR/PPU 面=L3 RAM 盲面补全** |

## 2. 正负对照（判据①，输出原文）

```
[正对照] t_same.png: 全同 → ✓
[负对照·1 像素] t_diff.png: 白名单外 1 像素分歧（首 @2,1 金=102030ff 采=102130ff…）→ ✓
[白名单放行] t_diff.png: 全同（白名单内 1 处排除） → ✓
[尺寸不符] t_size.png: 尺寸不一致 金面 8×8 vs 复采 4×4 → ✓
selftest: PASS
```

**同帧全同 ✓ / 1 像素差异打回 ✓（零容差）/ 白名单矩形放行 ✓ / 尺寸不符打回 ✓**。

## 3. 门禁启用（判据②，输出原文）

```
[PASS     ] visual-diff (ENABLED, README: S6 启用) L4 视觉 全绿（tools/diff_visual.py）；
runner: L4 视觉裁定: 7 金面帧, 0 失败, 白名单条目 0 | （game-core 渲染腿未启用（S6）…）
```

L1(subroutine)→L2(golden)→L3(trace)→**L4(visual) 四层全在**——验证体系收官层落成。

## 4. 白名单纪律（AGENTS §5）

- 空表起步=**像素级零容差**；任何放行条目=用户确认+reason+approved_by 四元组；
- 例外面候选（字体/调色板映射）待 S6 渲染腿接通后按需立条（每条一议）。

## 5. 遗留

1. game-core 渲染腿（S6）：渲染输出面接入后 diff_visual 二腿同机制；
2. 场景扩面：samus-move3/转场帧金面随任务链推进立金；
3. 金面更新流：CHR/PPU 变更时金面重采（双跑确定性先行——PNG 编码确定性已随
   boot-title 双跑验证）。
