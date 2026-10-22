# Phase 0 架构设计·UpdateWorld·NMI 顺序契约 spec（用户裁定（认可（按计划推进）））

## 1. UpdateWorld·NMI 顺序契约 spec（19 子系统（权威帧管线顺序））

### 1.1 UpdateWorld 顺序（bank7:$CB29（19 子系统（权威帧管线顺序）））

**权威帧管线顺序**（bank7:$CB29 UpdateWorld（19 子系统））：
- bank7:$CB29 UpdateWorld（19 子系统（权威帧管线顺序））
- refs asm 考证（bank:addr（UpdateWorld 顺序（19 子系统（权威帧管线顺序））））

### 1.2 NMI 顺序（bank7:$C0D9（vblank 相（OAMDMA($0200)→[仅当 NMIStatus==0：NMIScreenWrite→CheckPaletteWrite→CheckVRAMStructBufferWrite→WritePPUCtrl→WriteScroll→ReadJoyPads]→SoundEngine→UpdateAge→NMIStatus=1）））

**vblank 相顺序**（bank7:$C0D9 NMI）：
- OAMDMA($0200)
- 仅当 NMIStatus==0：NMIScreenWrite→CheckPaletteWrite→CheckVRAMStructBufferWrite→WritePPUCtrl→WriteScroll→ReadJoyPads
- SoundEngine→UpdateAge→NMIStatus=1

## 2. NMIStatus 掉帧语义（NMIStatus=帧完成旗（主循环超时→掉帧跳过 vblank 后半））

**NMIStatus 掉帧语义**：
- NMIStatus=帧完成旗（主循环超时→掉帧跳过 vblank 后半）
- 帧预算记账方案（周期常数自 Mesen trace 标定）

## 3. refs 引用必写（bank:addr 引证（A 级原文））

**refs 引用必写**（bank:addr 引证（A 级原文））：
- bank7:$CB29 UpdateWorld（19 子系统（权威帧管线顺序））
- bank7:$C0D9 NMI（vblank 相）

## 4. 交付判据

设计文档实录（docs/design/phase0-update-world-nmi-contract.md（UpdateWorld·NMI 顺序契约 spec（19 子系统+vblank 相+NMIStatus 掉帧语义+帧预算记账方案）））——「候令」禁用。
