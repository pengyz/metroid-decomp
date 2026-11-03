# 淡入闪现窗 FC 奇偶翻格定谳：emu 截屏采样竞争（非 state 差、非配对差）（2026-11-03）

> **状态：已证实**（A 级——226 run 归档考古 + 62 点×6 run 密集探针 + 同回调双读实录）。
> boot 帧计时初始化对齐根修任务的第一步「非确定性源三分判别」产物。
> **结论一句话**：淡入窗（SamusIntro :C9D7-:CA13，Timer3<31 窗 FC&1 奇帧画 Samus）翻格
> =**emu 侧 `emu.takeScreenshot()` 在帧界回调内读到未翻转的渲染面**（sprite 层缺失/邻帧图像），
> **rust 侧确定、emu RAM/state 面确定、配对层同 run 无罪**——翻格全部由 emu png 面单侧抖动产生。

## 0. 三分判别结果（任务 Step 1）

| # | 判别面 | 实验 | 结果 | 裁定 |
|---|---|---|---|---|
| 1 | rust 侧确定性 | rust_screenshot 3 连跑（case-scrollseam-probe 同一 .sched） | f560/f700/f760 png sha **3 次恒同**（且≡run 19:34 产物）；ram.hex $2D=21/AD/E9 恒同 | **纯代码确定（应然）** |
| 2 | emu 侧确定性 | ① 226 个归档 run 考古（analysis/traces/.archive/）② scrollseam 3 连跑隔离目录 ③ 62 点×6 run 密集探针（3 干净+3 满载） | 归档中 killenemy/scrollseam/jumpledge 翻格 run 实存（run X3=19:21 档 scrollseam f560/f700/f760/f800 翻）；密集探针 5/62 格翻（run4/5/6 满载组）+**同回调双读 4 次实录分歧** | **emu png 面非确定（大发现）** |
| 3 | 配对/比较层 | matrix 每行 fresh/fresh 三 sha 锚 + 归档 run 均为同 run 产物对 | 翻格 run 的 RAM 面与正常 run **逐字节全同**（0 diff）；翻格只出现在 png 面 | **配对层无罪**（同 run 对、锚定完备） |

## 1. 关键实证链

### 1a. RAM/state 面跨 run 恒同（翻格 run 与正常 run 零差）

    scrollseam f560/f590：flip-run(19:21 档) vs normal-run(19:28 档) ram.hex 逐字节 diff = 0
    （$1A=00、$2D FC=21/3F 全同；SpriteRAM $0200-02FF 同在 dump 面内亦全同）

⇒ **游戏侧 CPU/帧计时/OAM 影子跨 run 完全一致**——「boot 帧计时初始化偏移」假说
（emu NMI 计时 vs rust 虚拟时钟）**在 run 间维度不成立**（state 层 FC 0 差基线与此互证）。
翻格是**渲染输出/截屏读取面**的单侧现象。

### 1b. 翻格=png 双变体（blink 真值面）

淡入窗内 png 变体由 FC 奇偶 × 调色板级（SamusFadeInTimeTbl $1E/$14/$0B/$04）完全决定：

| 变体 | sha(8) | 含义 |
|---|---|---|
| noSamus | `8cbf4a35` | 偶 FC（或 T3≥31/窗口外）——无 Samus |
| drawn-s1..s4 | `69dedfbf/d70820c5/596bb5b0/2c33c066` | 奇 FC + 调色板级 1..4 |

jumpledge 逐点锚（FC=ram.hex $2D，*=奇）：f420 T3=26 窗外→noSamus；f550 FC=17*→drawn-s1；
f705 FC=B2→noSamus；f730 FC=CB*→drawn-s3；f755 FC=E4→noSamus；f780 FC=FD*→drawn-s4。
**blink 判定=parity(FC at capture)**（6/6 吻合）——翻格=该真值被采成对侧变体。

### 1c. 同回调双读实录（机制铁证——采样竞争）

探针（saveShot 内同一次 endFrame 回调连续两次 `emu.takeScreenshot`）：

    run4 f645: read1=drawn-s2(翻)  read2=noSamus(真值)
    run5 f750: read1=noSamus(翻)   read2=drawn-s3(真值)
    run6 f680: read1=noSamus(翻)   read2=drawn-s2(真值)
    run6 f690: read1=noSamus(翻)   read2=drawn-s3(真值)

**同一次回调内（µs 级间隔）两次读到不同帧图像**——且后读=FC 真值一致像。
⇒ `emu.takeScreenshot` 读的是**帧界翻转竞态中的渲染面**（前帧 posted 像 vs 本帧 completed 像）。
run6 f675 另有双读同翻（两读皆旧）——竞态窗可跨双读（满载下视频线程滞后 ms 级），
单靠「双读一致」不充分，**必须把采样点移出帧界竞态窗**。

### 1d. 帧锚（emu.getState）实测

    endFrame(N) 回调内：ppu.frameCount=N+15 / scanline=241 / cycle≈25-28（vblank 起点±ε）
    inputPolled/startFrame(N+1)：ppu.frameCount=N+16 / scanline=241 / cycle≈30-33

截图竞态窗=**vblank 起点（scanline 241 c0-30 附近）**的渲染面翻转；endFrame 回调正处窗内。
spin-exit 缝（$C0D3，prg7:218——NMI 后/帧工作前）在 NMI 例程跑完后命中（数千周期后），
**天然在竞态窗外**，且语义与双相位模型（§2c：png(N)≡on_spin_exit 缝）对齐。

### 1e. 归档考古（run 隔离戳实况）

- emu 腿有 run 隔离（scenario_run.archive_existing → analysis/traces/.archive/<name>-<stamp>）：
  killenemy 70+ 档中 1 档（19:18:45）f560/f590 翻；scrollseam 19:12:39 档 f530/f560 翻、
  19:21:07 档 f560/f700/f760/f800 翻；jumpledge 17:11:03 档 f730 翻。
- rust 腿**无归档**（shots_rust 原地覆盖，仅 .l4prov.json 锚）——姊妹面（F 卡）后续可补
  run 戳；本次判别不受阻（rust 3 连跑恒同自证）。

## 2. 与旧假说的关系（证伪更正文化）

> 【2026-11-03 更正】「boot 帧计时初始化偏移⇒FC 奇偶翻格」的共因假说**不成立**：
> 翻格 run 与正常 run 的 FC/SpriteRAM/state 逐字节全同（§1a）——不存在 run 间的
> boot 计时偏移。真因=emu 截屏读取相位竞态（§1c）。state 层 FC 0 差基线
> （walkfcprobe 1383/crawlprobe 943/fcprobe 445）不受影响、无需修 FC 计数径。
> animI 滞后/f1200 级联是否同根：**否**（那是内容真差，非采样翻格）——不随本件收敛，
> 保留原案。

## 3. 修径（lead 裁定三选一中的 C 径+ B 径留痕）

### 3a. C 径：Mesen harness 帧对齐采样（主修）

`tools/scenario_capture.lua`：采集点截图**移出帧界竞态窗**——
- endFrame(N) 回调只**登记**截取请求（requestShot；ram/ppu dump 留原位（活体读确定））；
- **spin-exit 缝（$C0D3 exec 回调——prg7:218，NMI 后/帧工作前）兑现截图**（fulfillShot），
  缝位=NMI 例程完成后（竞态窗外数千周期）+双相位模型的 png 语义位；
- **稳定双读**（两次连续一致才落盘；上限 8 次；耗尽=SHOT-UNSTABLE 显式日志+末读落盘留痕）；
- 末帧（capture@frames）：DONE+emu.stop **顺延至缝兑现后**（endFrame(frames+1) 兜底）；
- 漏缝（pending 跨到下一请求）=SHOT-SEAM-MISS 显式日志+即时兜底读（不静默）。

### 3b. B 径：flaky 态落 l4 锚 schema（残差留痕）

多 run 观测账（run id×格 verdict×双侧 png sha）→ flaky 判定（多 run N 次观测记录）
→ flaky 格**不计分母不锚值**、显式留痕（判据条文）。本次修后 3 连跑若全稳，
flaky 账为空转正（机制在位待命）。

### 3c. 判据（不修订）

淡入窗格（killenemy f560/f590/scrollseam f560/f700/f760/jumpledge f730）3 连跑 L4 逐格稳定；
943/1383/445 帧 state 0 差基线保持；475 测全绿+gates check；L4 无回退（flaky 格按 B 留痕）。

## 4. 复现命令

    cd /home/peng/workspace/metroid-decomp
    # rust 3 连跑确定性（隔离 out）
    for i in 1 2 3; do cargo run -q -p game-core --bin rust_screenshot -- \
      --schedule shots_rust/case-scrollseam-probe.sched --out analysis/boot-det/rust-$i; done
    # emu 隔离 3 连跑
    for i in 1 2 3; do GUNNAC_TRACES=analysis/boot-det/emu-$i/traces \
      GUNNAC_SHOTS=analysis/boot-det/emu-$i/shots python3 tools/scenario_run.py \
      run cases/case-scrollseam-probe.txt --timeout 300; done
    # 密集探针（62 点淡入窗）×3 干净+3 满载（16 busy workers）——翻格率+同回调双读
    # 分析脚本见 analysis/boot-det/（png sha×run 矩阵 + ram FC 真值对表）
    # 归档考古：analysis/traces/.archive/case-*-20261103-*/shots/*.png sha 矩阵
