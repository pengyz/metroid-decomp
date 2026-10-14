---
type: gotcha
title: CRLF 检出让字节锚定判"篡改"——文本锚点 LF 归一哈希 + 写侧 newline="\n"，二进制禁归一
date: 2026-10-14
status: confirmed
---

# gotcha：Windows CRLF 检出伪造"基线被手工改动"告警

## 现象

中文 Windows 机器（`core.autocrlf=true`）检出仓库后，`python gates/cli.py status` 三条 sha 判定全红，
措辞却是最严重的篡改级告警：

- `rom-integrity`：`baseline.json / baseline_history.jsonl: sha256 不符【基线文件被手工改动】`
- `seed-integrity`：`reference/seed/metroid.asm: sha256 与冻结基准不符`
- `baseline-integrity`：`baseline.json 被手工改动（与历史 sha 链不符）`

但 `git status` 工作树干净；把文件字节做 `\r\n`→`\n` 归一后重算 sha256，与 SHA256SUMS 锚点、
baseline.json 冻结值**逐一精确吻合**——没有任何文件被改过，纯平台假阳性。

## 为什么

- 锚点/sha 链历史值按 **LF 字节面**计算（写侧 payload 用 `"\n"` 拼接、冻结在 Linux 语境），
  而 Windows 上 `core.autocrlf=true` 检出 + Python 文本模式 `write_text`/`open("w")` 的换行翻译
  会把工作文件字节面变成 CRLF——`sha256(原始字节)` 自然对不上。
- 门禁把"换行符差异"误判成"内容篡改"，恰好违反本项目告警分级直觉：最响的告警配了最假的证据。
  反向陷阱同样存在：若图省事把**所有**哈希都归一，`rom/` 二进制的"任何字节改动=严重违规"红线
  就被击穿（ROM 里的 `\r\n` 字节序列会被静默吞掉）。

## 何时使用

- **文本锚定文件**（baseline.json/baseline_history.jsonl/seed 投影等）：哈希前 LF 归一
  （`gates/cli.py` 的 `sha256_text_file`），写侧显式 `newline="\n"`，仓库侧 `.gitattributes`
  钉 `* text=auto eol=lf`。三件套缺一，换一个平台又红。
- **二进制锚定文件**（`rom/`、资产 dump）：必须原始字节判定，禁任何归一（`sha256_file`）。
- 判"是否篡改"前先问一句：**两侧字节面同源吗**？跨平台/跨工具链传递的字节锚定，
  显式声明换行语义再比对。
- 反例（本条来源）：门禁在 Windows 上把 CRLF 检出报成"基线文件被手工改动"。

## 证据指针

- 修复前 `python gates/cli.py status` 输出（2026-10-14，三条 sha 级 FAIL）；
  LF 归一化对账一行命令（结果=锚点逐项吻合）：
  `python -c "import hashlib,pathlib; [print(p, hashlib.sha256(pathlib.Path(p).read_bytes().replace(b'\r\n',b'\n')).hexdigest()) for p in ['data/status/baselines/baseline.json','data/status/baselines/baseline_history.jsonl','reference/seed/metroid.asm']]"`
- 修复后 `python gates/cli.py status`：seed-integrity PASS、baseline-integrity OK
  （`sha 链与状态锚点一致`），rom-integrity 仅剩 BYO-ROM 缺失一项。
- `gates/cli.py selftest` 37/37（含"手工改动断 sha 链被检出"负向用例——归一化未削弱篡改检出）。
- 修复面：`tools/seed_build.py` 写侧 `newline="\n"`、`gates/cli.py` `sha256_text_file`、
  `.gitattributes`（2026-10-14 fix(gates)/fix(tools)/chore(repo) 提交）。
