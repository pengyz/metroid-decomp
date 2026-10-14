# rounds/r3 — G2/G3 L2 harness 步骤对抗复核（2026-10-14）

- 被审对象: `32cb33e` feat(core,oracle): G2 L2 子程序级 harness + G3 unported! 桩
- 审核方: 独立审核员（verifier，证伪立场，只读，含 27 项外部语义探针）
- **verdict: needs_revision** → 本文件记录整改与 RED 留痕（F5 补正），复审见后续行

## 通过项（独立复验）

- **6502 核语义**：151 条 documented 指令机器比对 == 官方集；ADC/SBC 含 V、比较 C、分支符号扩展、
  JMP 间接页 bug、JSR/RTS 栈序、izx/izy/zpx 回绕——外部探针 27 项 0 失败，**未发现语义错误**。
- 三方裁定真实：golden（手推）== ROM 原字节执行 == game-core，独立复跑 3/3 PASS。
- 写集契约（机械栈流量不入、STA $01xx 入）探针实证成立；门禁/基线留痕合规。

## findings 与整改（编号=r3 F 系列）

| # | 分类 | 内容 | 整改 |
|---|---|---|---|
| F1（中高） | 实现缺陷 | 哨兵约定下超步数检查恒假 → 非终止例程静默截断为 Ok（误分类为 golden≠ROM） | ✅ l2.rs 删 `!stops.is_empty()` 前置，任一约定跑满步数即 Err；回归测试 `sentinel_convention_nonterminating_fails_loud`（RED→GREEN） |
| F2（中） | 实现缺陷 | game-core sub_c376 写地址未归一 `&0x7FF`，X≥0x5F 三方破裂（潜伏 bug） | ✅ 归一修复 + 边界用例 X=0x5E/0x5F/0xFF 三条入 l2_cases.jsonl（6/6 PASS） |
| F3（中低） | 契约违规 | 写集语义双重性（事件序列 vs 最终值映射）未声明 | ✅ 引擎 `final_writes` last-wins 折叠 + README 声明（含 out.regs 缺省 0/未知键忽略） |
| F4（中） | 声明失实 | 矩阵格 C376「L2 已验证」夸大（截断尾部 PLA 拉 A 入比对面、双层返回不可表达） | ✅ 格注收窄为「部分覆盖：$C376-$C37A 头部 + X∈{0,2,0x5E,0x5F,0xFF}；PLA/PLA/RTS 尾部与 X 全域未验」 |
| F5（中低） | 声明失实 | TDD RED 无落点；8 fail + 1 声明先行 ≠ 10 算术不闭合 | ✅ 本文件补 RED 留痕（见下）；口径闭合=8 fail + **2** infra 先行（gap 登记簿 + l2ports::lookup 注册表），第二项先行此前漏声明 |
| F6（中） | 工程卫生 | target/ 235 文件入库（AGENTS §2 红线） | ✅ 独立 chore 提交出库 + gitignore（已核实 0 跟踪） |
| F7（低） | 门禁韧性 | 空用例集静默 PASS | ✅ l2-diff 空集 exit=2 fail-loud；`#[test]` 文本计数弱点列为遗留 |

## F5 RED 留痕（补正）

**RED 阶段实录**（`cargo test --workspace`，todo!() 桩期）：
- game-oracle: `cpu_lda_imm_sets_a_and_flags / cpu_sta_zp_records_write / cpu_jsr_rts_roundtrip /
  l2_sub_c16d_golden_from_rom_bytes / l2_sub_c376_absx_indexed_write / l2_unsupported_opcode_fails_loud`
  = **6 FAILED**（panicked at cpu6502.rs:24 / l2.rs:34 = todo!()）
- game-core: `sub_c16d_writes_pointer_and_returns_ff / sub_c376_indexed_zero_store` = **2 FAILED**
  （panicked at l2ports.rs = todo!()）
- game-core: `lookup_returns_registered_ports / unported_stubs_fail_loud_and_record` = **2 passed**
  （**基础设施先行**：l2ports 注册表 + gap 登记簿——均非行为面实现，声明补正）
- 口径闭合：**8 FAILED + 2 infra passed = 10**；r3 回归新增 3 测试（F1×1/F3×1/F2×1）亦 RED→GREEN，
  现 13 项全绿。

## r3 遗留风险（审核员清单，长期持有）

1. MMC1 mapper 写（$8000-$FFFF）静默丢弃——bank 切换例程不可设 L2 用例（社交约束无机械强制；
   建议后续对 $2000+ 写 fail-loud，同 BRK）；
2. PPU/APU 读 stub 恒 0（vblank 轮询类例程受限）；
3. window_bank 窗口态假设（文档已更正；bank07 用例令 window_bank=7）；
4. 初始 SP=0xF8 固定未声明（真实调用点 SP 未知）；
5. golden/引擎共享同一 stops 截断窗口（同源考题）——关键格建议 L3/trace 背书语义边界；
6. seed 数据区含 undoc 操作码（DCP）——L2 触及即 fail-loud（设计内）；
7. `#[test]` 文本计数可被注释污染；8. iNES 无 trainer 假设。


## 终局结论（r3 第二轮，verdict: pass）

F1-F7 全部经独立复现证实（含 /tmp 独立构造：哨兵/双约定非终止 Err、6 条自选 X 镜像用例真 ROM
全过、双写折叠判别力实证、todo!() RED 态回放 8 failed+2 passed 名单逐一吻合、3 项回退红灯复现）。
E2E：cargo test 13/13、diff_subroutines 6/6、gates PASS=8 FAIL=0 SKIP=3。**G2/G3 正式闭环**。

审核员新增遗留（并入持有清单）：红灯实录颗粒度偏松（建议贴输出/CI artifact）；矩阵 covered:true
与「部分覆盖」note 并存建议引入 coverage 级别字段；MAX_STEPS=1M 唯一终止护栏；C16D「全区」措辞
仅指地址域；Mesen2 D3 真相闭环未完成（继承项）。
