# oracle 呈现管线浮动滞后——f700/f1100 残差定谳（2026-11-12）

## 结论（B 级→A 级实测链）
`case-password-justin` f700=23px / f1100=42px **非移植错**：两侧 spin-exit 缝快照（CHR/NT/PAL/OAM/FC）逐字节全等；残差来自 **Mesen 呈现管线与内存 dump 无固定相位关系（lag 0-4 帧浮动）**——`emu.takeScreenshot` 与 `emu.getPixel` 读同一呈现面。

## A 级证据链
1. **dump 全等**：`analysis/traces/*_f00700_mid-1.ppu.hex` vs `shots_rust/` 同名 → CHR/NT/PAL eq True（8192/4096/32B）；OAM `$0200-$02FF` 双侧逐字节相等；FC（`frames.jsonl w.FC` vs `DSH_FC_TRACE`）f694-705/f1094-1105 全 d=0。
2. **emu PNG 与自身 dump 不自洽**（r9-F2 复核实证，本方重测确认）：f700 OAM 全 `f0`（无精灵）而 PNG 画着光标（色 `5CE430`，仅存在于精灵调色板 `sp=$0F,$16,$2A,$27`，BG 调色板无此色）；rust 自洽：render(dump)≡PNG 0 差。
3. **单读实验**：`takeStableShot` 改单次读（attempts 24/24=1）→ 23/42px 原样 ⇒ 双读稳定策略假说证伪。
4. **延迟一缝实验**：`pendingShot.frame==frame-1` 才兑现 → f700 23→48（更差）⇒ 固定滞后假说证伪。
5. **getPixel 实验**（缝回调内 `emu.getPixel(104,64)/(32,152)`，光标色 `5CE430`）：单→双转场 rust@f695 / PNG@f698（lag3）/ getPixel@f699（lag4）；灭转场 rust@f701 / PNG@f701（lag0）/ getPixel@f702（lag1）⇒ 两独立 API 同读呈现管线、lag 浮动 ⇒ **无 Lua API 可与内存 dump 同刻取面**（`getScreenSize`/`getDrawSurfaceSize` 仅坐标换算）。
6. 内容分组：rust PNG A(694)→B(695-700)→C(701-705)；emu PNG A(694-697)→B(698-700)→C(701-705)——内容组相同、取景帧浮动；光标闪烁=`FrameCount & #$08`（`refs/m1disasm/SRC/prg0_title.asm:2568-2612`），窗口恰在闪烁边界故暴露。

## 处置（用户裁 B，2026-11-12）
挂账不动：不容忍/不锚定/不白名单；L4 维持 14 用例/112 格 match 110/diff 2。实验补丁（scenario_capture.lua 三轮）均已还原（备份 /tmp/scl.bak、/tmp/scl.getpix.bak）；C 路线（考 Mesen 源码）已执行至 getPixel 实验并否决。
配套同日真修：OAM 槽位勘误 b811aa05、FC 相位 0x17 冻结拍数 58859f65、暗窗 7 帧 f7199a74、采集相位守卫 34fc4ecd、EndVRAMStruct 终止符 b60974b8。

## 重开条件
若未来升级 Mesen 或找到与内存同刻的取面法（如 savestate+离屏渲染），重验 f700/f1100 预期归零。
