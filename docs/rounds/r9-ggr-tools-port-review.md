# r9 复核档：GGR 工具资产移植卡（diff_forensics / vram_snap / 写钩三件套+CUT）

- 复核对象：三 commit `9ed53988`（件1 diff_forensics.py 移植）→ `acfa4baf`（件2 vram_snap 中帧 VRAM 快照）→ `c5650951`（件3 write_hooks nt|chr|oam + CUT 纪律），HEAD=c5650951
- 实施方：porter（3267f925 会话）；复核方：verifier（01197506 会话，证伪立场）
- 源参考仓 /home/peng/workspace/GuardicGaidenRemake（只读）
- **verdict: pass（Confidence 88）**

## 逐点裁定（verifier 独立重导实录）

1. **工具红线 ✓**：三 commit diff 面 7 文件（tools/×3 + oracle/scenarios/×4 smoke txt），gates/、crates/、reference/、data/ 零触及；diff_forensics.py:27 明示「本工具不持有判定权」。
2. **写钩对齐 GGR v2 ✓**：tools/scenario_capture.lua:841-950 NT=$2006 锁存+$2007/CHR v2=$2000 ctrl bit2 增量(+1/+32) 含 PC/OAM 含 PC；HOOK_CAP=3M 触顶显式 `{kind:"CUT"}`；probe_mem/ctread CUT 补缺——与 GGR tools/scenario_capture.lua:126-300 语义一致。
3. **扫描线域 0-261 ✓**：NTSC 总扫描线全域（Mesen ppu.scanline），tools/scenario_run.py:313 解析界含端点。
4. **fail-loud 实测 ✓**：构造 100 字节假 .ppu.hex → exit=1，stderr 显式拒收（长度 ≠ 12323）。（次瑕：非 hex 输入裸 traceback——已随本档收口为 SystemExit 整洁报错。）
5. **独立复跑（A 级）**：`--selftest` PASS（合成 32/分域 chr=1 nt=2 pal=1 phase=1）；件1 真实对拍独立重导——blink610 vs blink795 全文件仅 3 差异字节 0x3011-0x3013=$3F11-$3F13 sprite pal0 色1-3（05→16/09→19/17→27），与 porter 归因逐字节吻合；件2 产物核（vrsnap-smoke-nmi.vrsnap.jsonl：235 快照、sl250=228 压倒性、零重复、hex 文件字节长恒定 0 非法）；件3 产物核（nt 7118/chr 12430 含 PC C305/oam 11456 含 PC C1CA，malformed=0）。
6. **证伪尝试**：①「NMI 后窗」主张支持但限幅——A 级代码锚 reference/seed/metroid.asm:100229-100234（bank07:C0CB-C0D2 主循环自旋 NMI 帧标志 $1A）+ smoke 实测集中；但仅单一 240f 场景，**跨关卡泛化未测，引用时须标注「本场景实证、未泛化定谳」**。②「枚举注册免疫盲区」推理成立（代码级可核）：GGR v1 字符串兜底劣化面 vs 本仓 emu.callbackType 枚举先行 + 产物 cb:true 佐证。
7. **纪律审计 ✓**：gates 零触；.scenario.lock 强制在 runner；crates/ 零改动；analysis/ 不入库（git ls-files = 0）；4 个 smoke txt 符合仓库 scenario 惯例。

## 非阻塞整改（已收口）

1. NT 钩内 $2006-$2007 exec 探针钩补注释（用途=写回调零命中时的可见性诊断信号，非数据源）——已补 tools/scenario_capture.lua。
2. .ppu.hex 非 hex 输入裸 traceback → SystemExit 整洁报错——已补 tools/diff_forensics.py load_ppu。
3. 「NMI 后窗」结论引用标注「本场景实证、未泛化定谳」——登记于本档，后续 KB/台账引用遵循。

## 结论

三件资产移植忠实、验证实录可复现、证伪尝试未推翻核心主张；两处泛化主张已限幅标注。复核档随整改 commit 入库。

Evidence: tools/diff_forensics.py --selftest PASS + blink610/795 逐字节重算 0x3011-0x3013 + vrsnap/hooks 产物独立核数（7118/12430/11456 malformed=0）+ reference/seed/metroid.asm:100229-100234 NMI 自旋锚
Confidence: 88
