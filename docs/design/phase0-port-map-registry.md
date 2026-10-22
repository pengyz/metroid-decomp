# Phase 0 架构设计：port-map registry v0 + port_coverage.py

> 2026-10-22 · 用户裁定（认可（按计划推进））· 纪律强化（refs 引用必写+更新留档可追溯）·
> 四项核（对抗复核核（证伪立场（独立重导证据）））· Evidence A 级（bank:addr 引证）Confidence 95

## 0. S 面分母识别（label 类型辨·纯子程序面）

refs asm 实证（refs/m1disasm/SRC/ 全量机械计数）：

| label 类型 | 判别 | 实测计数 | refs 引证样本 |
|---|---|---|---|
| 子程序入口 | `Name: ; 0N:XXXX`（bank 前缀）+后随指令 | **1321** | prg7_engine.asm:33（RandomNumbers ; 07:C000）/59（Startup 07:C01A）/199（MainLoop 07:C0BC）/233（NMI 07:C0D9） |
| 数据标签 | `Name: ; $XXXX`（无 bank 前缀）+后随 .byte/.word | **431** | common_sprite_data.asm:831（ObjFrame06 ; $8802 后随 .byte） |
| 局部标签 | `LxNNN:`（函数内段标） | **50** | prg7_engine.asm:3480（Lx019 07:D015）等 |
| 跳转表条目 | PtrTableEntry/.word 表目 | 计入数据面 | common_sprite_data.asm:283-354（ObjFramePtrTable 106 条目） |

**分母基辨正（证伪立场·原地记录）**：工单称 8659 条（可调用子程序面）——出处=
docs/design/l4-readiness-batch.md（L4 就绪批精化覆盖率基准分母，:15/:94）。与本实测三基数
（1321 code labels / 431 data labels / completeness_denoms.py S=492）**计数基不同**：
8659=L4 批量考证任务面（含数据表条目+帧数据逐条），1321=bank:addr 子程序入口形标签，
S=492=分母导出 L0 登记面。port_coverage.py 以 **1321 bank:addr 子程序入口形** 为机械核
基准（可调用子程序面），8659 台账面由 L4 批量工单另行核对（不混基）。

## 1. port-map registry v0：bank:addr→crate::module::fn 双射

**双射规则：每 asm 子程序恰一归宿**——语义 fn（移植实现）或 `unported!` fail-loud 桩
（爆点=bank:addr，D7 缺口左移机制②）。禁止孤儿（不参与编译的实现=空洞断言，l4-readiness-batch.md:94 教训）。

registry 行形（tools/port_map_registry.jsonl）：
`{bank_addr, symbol, rust_module, rust_fn, status, l2_case_ids, l3_scenarios, l4_scenarios}`
- bank_addr 形：`07:D002`（bank:addr 规范化，与 m1disasm 注释形一致）
- status 勾格四态：`未实现→已实现→有L2→已验证`（D7 左移）
- 归宿优先级：①game-core 纯逻辑 fn（无 I/O/确定性）②game-data 数据表直读
  ③`unported!` 桩（fail-loud 爆 bank:addr）——三者恰居其一

双射核算法（port_coverage.py）：
1. 解析 m1disasm SRC 全部 `Name: ; 0N:XXXX` → S 面集合（1321 基准）
2. 解析 registry bank_addr 集合
3. `S − registry` = 遗漏（FAIL）；`registry − S` = 多余（FAIL）；相等=双射 PASS
4. status≠已验证 行输出勾格清单（左移待办）

## 2. shared 子程序辨（jsr 多引用→独立方法）

实证：prg7_engine.asm 内 jsr 去重目标 **346** 个（`jsr X` 多处引用=共享子程序，
独立方法登记；仅单例程内 Lx 标签→组合进宿主方法（Phase 0 改写策略 §0 同法））。

## 3. refs 引证汇总（bank:addr）

prg7_engine.asm:33/59/199/233（子程序入口形）/3480（Lx 局部标签）；
common_sprite_data.asm:831（数据标签 $8802）/283-354（ObjFramePtrTable 106 条目）；
tools/completeness_denoms.py 输出（S=492 D=193 B=231 R=0，covered=57）；
docs/design/l4-readiness-batch.md:15/:94（8659 基准出处+孤儿教训）。

Evidence: 上述 bank:addr/行号引证（m1disasm SRC 机械计数+denoms 输出实录）。
Confidence: 95
