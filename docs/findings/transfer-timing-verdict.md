# 转移时序定谳（r26 轮 36——probe 写事件帧序 A 级实证）

> 状态：**定谳（零延迟实证）**。对抗复核（verifier）修正增量①产物。probe.jsonl=A 级证据。

## 1. 写事件帧序（probe_mem——Mesen 真帧钟！）

| 帧 | 事件 | caller（ret-2 反推）| 语义 |
|---|---|---|---|
| f362 | Joy1Change=$10 | C219 | ReadJoyPads——press 边沿！ |
| f362 | SamusStat00=00 + BankSwitchPending($24)=02 | C0C2 | **InitializeStats（press 同帧零延迟！）** |
| f363 | $24=00 | C4E2 | CheckBankSwitch_OK（次迭代顶 ✓ asm prg7:199-201） |
| f363-367 | SwitchUpperBits($28) 写族 | C56F/C7C1/C606… | InitBank1 初始化链（ScreenNmiOff/CopyMap/…） |
| f367 | SpriteRAM.0.y=F0 | C815 | ClearAll（转移清屏） |

## 2. 裁定

1. **press→InitializeStats→$24 写 = 同帧（f362）零延迟**——与 asm 三处直接形（ChooseStartContinue $90D7 直接 jmp / InitializeStats $932B `a9 02 85 24 60` / rts）**完全一致**。game-core 端口忠实 ✓✓✓。
2. 「金侧转移晚 8 帧」= **双伪影**：
   - **双钟错位**：scenario `press 350`（inputPolled 自增计数！）↔ Mesen 真帧 **f362**（+12 偏移）；probe 行 f=st.frameCount，trace 行 f=inputPolled——两钟不同源，直减产伪偏移。
   - **采样洞**：金侧 f352-358 七行缺失（spin-exit 采样洞——转场主循环未过 $C0D3）。
3. **勘误**（对抗复核 blocker①）：BankSwitchPending=**$24**（`sta $24`！）非 $28（$28=SwitchUpperBits）；stepdiag.py 错标已勘误；GoBankInit=$C510（$C4DE=CheckBankSwitch/$C4E8=CheckBankSwitch_OK）。
4. **A-fix 重定**：转移时序非根。余根=UpdateWorld dispatch 缺件**静默分歧**（对抗复核 C blocker：19 件仅接 5，缺件无 unported! 桩=零爆点静默差）——**dispatch 壳补齐=收零前置**。

## 3. 工具增益（probe_mem 复用面——勿新写！）

- `probe_mem <lo> <hi>` DSL（tools/scenario_run.py:275）→ scenario_capture.lua:328-384 → `<name>.probe.jsonl`
- 行形：`{f, pc, a, v, caller, kind, sp, scanline}`（f=真帧钟！caller=栈 ret-2！）——比指令级轻一个量级
- 注意：probe 产物**无 sha 锚**（只进无锚场景副本！）；热区单点区间防 3M 截断；字段名 `a`/`v`（非 addr/value！）
