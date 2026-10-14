# rounds/r2-fix —— 完备性机制复审整改轮归档（ID 定义）

> 归档第二/三轮复审结论（此前仅存于会话——违反 AGENTS §7「聊天结论不作数」，本文件补正）。
> 原始复审对象：`96c0926`（机制落地）→ `32033ee`（首轮整改）→ `633add5`（二轮整改）→
> 本文档所在轮（三轮整改）。各轮审核员均为独立 verifier（证伪立场、只读、自重导）。

## ID 定义（供 master-plan / tools / r2 文档引用）

### F 系列（首轮复审 findings，r2-coverage-mechanism-review.md 已详载）
- **F1** execute 回调语义失真（严重）：覆盖集 ≠ 执行地址集；缺自校验
- **F2** Xvfb 孤儿路径（清场/try-finally）
- **F3** 「分母独立于测试」过度声明（B/R 面）+ 四类缺口未声明
- **F3a** D 面字段切分无字段级分母（→ S3 待办）
- **F3b** 跨模块交互（调用序/共享 RAM 契约/NMI 同步）无承载面（→ S4 前置登记）
- **F3c** 时序边界（帧相位/NMI 时序）无分母（→ S4 前置登记）
- **F4** 纪律检查（无违规项编号保留）
- **F5** 工程卫生：挂起未复现不入 gotcha；knowledge 指针悬空

### N 系列（二轮复审新发现）
- **N1** F5 整改内容失实：来源声明 4/4 归属错 +「其余条目与具体游戏无关」假断言
- **N2** r2 文档「2875 帧」与产物 `probe_meta frames=7271` 不符（已更正）
- **N3** 「S2 待研/S3 待办/S4 登记」无仓库锚点（已落 master-plan）
- **N4** 自校验 PASS 分支语义张力：不变量必要非充分，PASS 亦须钉死「仅线索」
  （→ `ROOT_SEMANTICS_VERIFIED` 开关：语义根治前盲区差集恒拒、并集恒不吸收）

### R 系列（三轮复审整改面）
- **R1** `completeness_denoms.py` B 面格 note 残留过度声明并持久化进
  `completeness_matrix.json`（关键词归零 + `--force` 重建）
- **R2** knowledge 事实表 brace 写法逃逸扫描器（15/18→**16/18**）+ hash/tXX/`.archive`
  类口径外指针未枚举（扫描正则不含 `{` 的教训）
- **R3** `coverage_from_trace.py` docstring 与 N4 行为不一致（`execute-callback-verified`
  旧层级残留）
- **R4**（本文件）：N/R 系列 ID 无仓库定义 → 归档定义

## 三轮收敛轨迹

| 轮 | verdict | 整改面 |
|---|---|---|
| 二轮 | needs_revision | 4 项（声明重写/措辞收窄/锚点/数字） |
| 三轮 | needs_revision | 3 处一行级（R1/R2/R3）+ R4 归档 |
| 四轮 | （闭环复核中） | 仅验 R1–R4 |

## 复审确认的「诚实残余」（pass 后无需整改，长期持有）

1. oracle 指令级覆盖语义不可得（S2 根治前覆盖=仅线索）
2. B/R 面无机械分母、完备性不可证
3. 四类缺口（D 字段/跨模块/时序/R 未写规则）靠晚期红灯 + 四分类路由兜底
4. gunnac 证据链（路径/hash/tXX）本仓不可复现，一律回源核对
5. emu.stop 未止脚本 / DONE 挂起现象未根因化
6. PASS 分支 exit=0（未验证态由 mode/N4 行/blind_spot_diff 三处表达）
