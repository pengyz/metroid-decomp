# oracle/subroutines/ —— L2 子程序级差分用例

**期望 = ROM 执行结果，实际 = game-core 同名函数**；三方全等才算绿：
`case.out（golden，手推自 A 级反汇编语义） == 6502 核执行 ROM 原字节 == game-core::l2ports`。

运行：

```bash
python3 tools/diff_subroutines.py        # subroutine-diff 门禁入口（薄封装）
cargo test --workspace                   # 同一差分引擎单元测试（rust-tests 门禁内）
```

## case 格式（JSONL，一行一条）

```json
{"id":"…","subroutine":"…","entry":"bankNN:AAAA","window_bank":N,"stops":["bankNN:AAAA",…],
 "in":{"regs":{"a":N,"x":N,"y":N,"p":N},"ram":{"hexaddr":byte}},
 "out":{"regs":{"a":N,"x":N,"y":N},"writes":{"hexaddr":byte}}}
```

- `entry`/`stops` 为 `bankNN:AAAA`（CPU 地址 + bank 标签）。Metroid MMC1 布局：
  bank0-6 → `$8000` 可切窗口，bank7 → `$C000` 固定（rounds/r1 向量证据）。
- **`window_bank`（可选，task-14 窗口参数化契约面，∈0..8）**：$8000 窗口 bank——ROM 腿
  （6502 核）与 game-core 腿（`L2In.window_bank`）**同取此值**。缺省=entry bank 标签
  （bank_of(entry)；既有案行为钉死）。显式携带=多窗口用例族：A9 动态证伪后窗口读表
  （$95CE/$95CF/$968B+Y）语义=**按区域查表**（game 期窗口=banks1-5 区域 bank——
  [decode-sweep-r3-dynamic-close.md](../../docs/findings/decode-sweep-r3-dynamic-close.md) §1.2，
  A 级）；bank 标签（代码所在 bank）与窗口 bank 是**两个独立契约面**，不得混同。
- `stops` 非空 ⇒ 执行到命中其一；为空 ⇒ 预压哨兵返回地址跑到 RTS（调用约定）。
- `in.ram` 地址归一至 `$0000-$07FF`；`out.writes` 同（写集只记 RAM）。
- **机械栈流量（JSR/PHA/哨兵压栈）不入写集**；栈页里的游戏变量（显式 STA $01xx）照常记录。
- **写集 = 最终值映射**（同址多次写 last-wins，引擎侧 `final_writes` 折叠；写事件序列不入比对面）。
- 差分面 = regs{a,x,y} + 写集；P 为执行副产物不入比对面（输入 P 影响分支，仍进 `in`）。
- `out.regs` 缺省 0；未知键静默忽略（rounds/r3 F3 声明）。
- 有 PPU/APU/MMC1 寄存器副作用的例程不设 L2 用例（降级 L3，pipeline R3）。

## 用例登记（先写用例再实现——TDD RED→GREEN）

| id | subroutine | 语义来源（A 级） | 覆盖 |
|---|---|---|---|
| l2-c16d-set-ptr-lo-hi | sub_C16D | reference/seed bank07:C16D-C174：`LDA #1/STA $01/LDA #$FF/STA $00` | zp 写序、立即数 |
| l2-c376-absx-zero-store | sub_C376 | bank07:C376-C37A：`LDA #0/STA $07A1,X`（stops 截断 PLA 尾部） | abs,X 变址 |
| l2-c376-absx-x0 | sub_C376 | 同上，X=0 边界 | 变址边界 |
| b-f282-*-w0..w7 / b-f2ed-full-w* / b-f2e8-fall-full-w*（32 案） | sub_F282/F2ED/F2E8 | 手推自 bank07:F282-F310 五路出口语义 + 逐 bank ROM 字节（prg[w×0x4000+0x15CE/0x168B]，python 一行自证） | **多窗口用例族**（task-14）：window_bank 0..7 全窗口，窗口表/向量按区域取数行为钉死 |

新增用例规则：golden 必须手推自 seed 指令语义（A 级，注明行范围）；game-core 侧实现
（`crates/game-core/src/l2ports.rs`）与用例同 commit 落地——**未实现例程不得入用例文件**
（缺口走完备性矩阵 L0 + `unported!` 桩，不以红灯形式挂在门禁里）。
