# Norfair（area 1）46 房对拍台账：FC 相位纠偏前后

判据与工具见 `tools/movie_ref/room_sweep.py`：每房注入 `$005A=<room>` 让引擎自己载入
（不走路、与区无关），采集 7 个点（`arrived`/`pre`/`prev60`/`f60`/`prev120`/`prevend`/`f120`），
`ok` = 两腿跑完 + 每个采集点 diff==0 + 注入被接受（`$005A` 走完 `F0→…→FF` 且 `$33/$34`
落在该房流区间）+ `f900..f906` `$001E MainRoutine` 全 3。

## 1. 总量（`data/status/room_sweep_1.json`）

| 口径 | ok | mismatch | blocked | 备注 |
|---|---|---|---|---|
| 镜像修复后（`mirrorfix-area1`） | 13 | 27 | 6 | 名表镜像动态化前 |
| Crawler 按区 + Squeept 尾件后（`crawlerfix-area1`） | 14 | 29 | 3 | |
| **电梯换区帧会计后（`fcalign-area1`）** | **21** | **22** | 3 | 本次（+7 房，零回退） |

本次转绿（7 房）：**15 / 23 / 29 / 38 / 41 / 42 / 43**；`lost = []`（无回退）。
耗时 1928.6 s（41 s/房）。

机制：这些房里有「按 `FrameCount` 取模/移位做门」的敌人（Crawler 38 槽 / Swooper 族）。
换区迭代多烧 2 帧 ⇒ `FrameCount` 常差 2 ⇒ 位移节拍相位错 1 拍。
修复与证据见 `docs/findings/bank-switch-frame-accounting.md`。

## 2. 剩余 22 房 mismatch 的分层画像

- **`clean`→`sprite` 型（17 房）**：`arrived`/`pre`/`f60` 全 clean，`prevend`/`f120`
  才出现 57–697 px 精灵层差（8/9/10/11/12/14/16/18/19/21/22/24/26/27/30/33/35/37/40）。
  ⇒ 载房后 ~120 帧内「后出现的敌/弹/爆炸」的差异，不是房间静态数据差。
- **`scroll_drift` 型（3 房 + 2 房转 ok）**：`dx=±2` 成对口径族，取 `shift1` 后归零的房
  （13/29）判 ok；44 取小后 350 px、45 取小后仍 11280 px ⇒ **不是**单纯相位配对，另有内容差。
- **blocked（28/31/32）**：`f120` 采集 blocked（模拟器侧采集缺失），需重采而非修代码。

## 3. 已知的仪器边界（读数时必须记住）

- `prev120` 在所有房都是 `error`（lag 帧取不到参考图），按 `4db31751` **不参与判定**；
  台账里保留 `notes` 显式标注。
- 台账 `diff_pixels` 是**取小后**（`same` 与 `shift1` 两种配对取小）；判断"是否相位族"
  要看 `layers` 的 `scroll_drift` 标记，不要只看数值。
- 每轮 sweep 的产物文件名带独立 token（`_tmp_sweep_<pid>-<epoch>_a<area>_r<room>_f<frame>_<tag>`），
  **必须按台账 `captures[].file` 取 token**，用上一轮的 token 会配到已一致的一对（误判 0px）。

## 4. 下一步优先级（按机制归并，不逐房修）

1. 从剩余 22 房里取 `f60` 最早分叉者，按「敌槽级逐字节比对 → 机制锚 → 最小重现」解剖；
   Swooper0（24 槽/9 房）与 Multiviola（10 槽/6 房）是这一层最可能的公共机制。
2. room 45 的 11280 px 单独处理（量大且 layers 全段 scroll_drift ⇒ 可能是载入/滚动错位，
   而非敌逻辑）。
3. blocked 3 房重采（先确认模拟器侧采集稳定性，不修代码）。
