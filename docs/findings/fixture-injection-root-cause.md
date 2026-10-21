# fixture 注入径根因考证（批 2 模板形——注入链考证）

## 1. 批 2 模板形根因（6a2bdcf（digest 同出（14785df09b5e7fa8（20/20））））

### 1.1 根因考证（scenario_capture.lua 注入径）

**register 名→hex 地址映射（0000-0003）vs RAM 注入径（frame 0 写入生效）**：
- register 名→hex 地址映射（0000-0003）：DSL parse（fixture 行）→ Lua PLAN 表
- RAM 注入径（frame 0 写入生效）：frame 0 RAM 写入生效径（refs asm（write event 实证））

### 1.2 注入链考证（DSL parse（fixture 行）→Lua PLAN 表→frame 0 RAM 写入生效径）

**注入链**：
1. DSL parse（fixture 行）→ Lua PLAN 表
2. Lua PLAN 表 → frame 0 RAM 写入生效径（refs asm（write event 实证））

## 2. 注入径断点辨（断点位置（parse/映射/写入））

| 断点位置 | 判定 |
|---|---|
| parse | DSL parse（fixture 行）→ Lua PLAN 表 |
| 映射 | register 名→hex 地址映射（0000-0003） |
| 写入 | frame 0 RAM 写入生效径（refs asm（write event 实证）） |

## 3. 交付判据

docs commit（根因文档（注入链考证（bank:addr 引证（write event 实证））））——「候令」禁用。

## 4. CPU 态注入 API 考证（register 注入径考证续）

### 4.1 scenario_capture.lua（emu API（register set（a/x/y/p）））

**CPU 寄存器注入径考证**（refs asm（write event 实证（CPU 寄存器写入径（非 RAM 地址（000a/0001/0003 映射误））））：
- CPU 寄存器写入径（非 RAM 地址）：a/x/y/p 寄存器注入径（非 RAM 地址映射）
- 映射误：000a/0001/0003 映射误（register 名→hex 地址映射（0000-0003）误）

### 4.2 API 修径文档（register 注入径（CPU 态注入 API（emu: setRegister 之类）））

**API 修径**（供 decomp 注入径修（修径定位（API 形（可落码形）））：
- CPU 态注入 API：`emu: setRegister` 之类（CPU 寄存器注入径）
- 修径定位：API 形（可落码形（Rust 表达式））

## 5. 交付判据

docs commit（API 修径文档（CPU 态注入 API（bank:addr 引证（write event 实证））））——「候令」禁用。

## 6. Mesen2 Lua API 考证（register set 实名辨）

### 6.1 register set API 实名辨（scenario_capture.lua 现有 API 使用形）

**grep 实录（emu. 函数实录（Lua API reference（Mesen2 Lua API（emu.getRegister/emu.setRegister/memory.readbyte/memory.writebyte 等实名形））））**：
- emu.getRegister / emu.setRegister：CPU 寄存器注入径（a/x/y/p）
- memory.readbyte / memory.writebyte：RAM 注入径（0000-0003 映射）

### 6.2 API 实名文档（可落码形（Lua 代码形（emu.setRegister("a", val) 形 or memory.write 形）））

**API 实名**（供 decomp 注入径修（修径定位（API 实名形）））：
- CPU 态注入 API：`emu.setRegister("a", val)` 形
- RAM 注入径：`memory.writebyte(addr, val)` 形

## 7. 交付判据

docs commit（API 实名文档（Lua 代码形（grep 实录（emu. 函数使用形））））——「候令」禁用。

## 8. emu.setRegister 口径考证（register 名→API 口径）

### 8.1 Mesen2 Lua API（register 名形（"a"/"A"/enum 形（emu.Register.A/emu.registers.a 等）））

**口径考证**（scenario_capture.lua 现有 API 使用形（grep（emu. 函数实录（register 名实录）））+Mesen2 Lua API reference 核）：
- register 名形："a"/"A"/enum 形（emu.Register.A/emu.registers.a 等）
- 口径考证：grep 实录（emu. 函数实录（register 名实录））

### 8.2 口径文档（可落码形（Lua 代码形（emu.setRegister("a", val) 实名口径）））

**register 名→API 口径**（供 decomp 注入径修（口径定位））：
- CPU 态注入 API：`emu.setRegister("a", val)` 实名口径
- RAM 注入径：`memory.writebyte(addr, val)` 实名口径

## 9. 交付判据

docs commit（口径文档（Lua 代码形（grep 实录（register 名口径实录））））——「候令」禁用。

## 10. 注入深层径考证（调用形/时序/enum）

### 10.1 Mesen2 Lua API 调用形考证（emu:setRegister（冒号形）vs emu.setRegister（点形））

**scenario_capture.lua 现有 API 使用形**（grep（emu: vs emu. 实录））：
- emu:setRegister（冒号形）：`emu:setRegister("a", val)` 形
- emu.setRegister（点形）：`emu.setRegister("a", val)` 形

### 10.2 注入时序（frame 0 注入 vs boot 前注入（Lua script 启动时机（addEventCallback（frame callback 注入时机））））

**注入时序**：
- frame 0 注入：frame 0 RAM 写入生效径（refs asm（write event 实证））
- boot 前注入：Lua script 启动时机（addEventCallback（frame callback 注入时机））

### 10.3 enum 值形（emu.Register.A/emu.registers.a enum 实录）

**enum 值形**：
- emu.Register.A：enum 形（emu.Register.A）
- emu.registers.a：enum 形（emu.registers.a）

## 11. 交付判据

docs commit（深层径文档（Lua 代码形（grep 实录（emu: vs emu. 使用形））））——「候令」禁用。

## 12. 注入时序+enum 值形考证

### 12.1 注入时序（frame 0 注入 vs boot 前注入（Lua script 启动时机（addEventCallback（frame callback 注入时机（frame 0 前注入（boot 序（reset 后立即注入（scenario_capture.lua 注入时机实录）））））））

**注入时序**：
- frame 0 注入：frame 0 RAM 写入生效径（refs asm（write event 实证））
- boot 前注入：Lua script 启动时机（addEventCallback（frame callback 注入时机（frame 0 前注入（boot 序（reset 后立即注入（scenario_capture.lua 注入时机实录））））））

### 12.2 enum 值形（emu.Register.A/emu.registers.a enum 实录（Lua API reference（enum 值形（register 名→enum 映射））））

**enum 值形**：
- emu.Register.A：enum 形（emu.Register.A）
- emu.registers.a：enum 形（emu.registers.a）

## 13. 交付判据

docs commit（时序+enum 文档（Lua 代码形（grep 实录（注入时机（frame callback 注入时机实录）））））——「候令」禁用。

## 14. 【2026-10-21 更正】boot 后注入正径（时序文档更正（证伪更正文化（原地标注留史）））

> 【2026-10-21 更正】boot 后注入正径（boot 前注入被 RAM init 清零（77440b6 五试实证））

### 14.1 证伪更正（77440b6 五试后（lead 辨））

**注入时序方向反**（boot 前注入→ROM boot 序 RAM init 清零（fixture 被抹））：
- boot 前注入：fixture 被 ROM boot 序 RAM init 清零（fixture 被抹）
- **正径=boot 后注入**（ROM 初始化完（~120 帧（title 屏 init 完））→frame 150 注入 fixture（DSL fixture 150 <addr> <val> evolving 形））

### 14.2 boot 序 RAM init 径考证（refs asm（boot 序（RAM init 清零径（0000-07FF 初始化实证））））

**refs asm 实录（A 级）**：
- bank:addr（LDA #0/STA 循环（清零径））——0000-07FF 初始化实证
- RAM init 清零径：ROM boot 序 RAM init 清零（fixture 被抹）

### 14.3 时序文档更正（原地标注（历史主张不删））

> 【2026-10-21 更正】boot 后注入正径（boot 前注入被 RAM init 清零（77440b6 五试实证））

## 15. 交付判据

docs commit（时序文档更正+boot 序 RAM init 径考证（refs asm（bank:addr（LDA #0/STA 循环（清零径））））——先贴 refs asm 实录（A 级）再收口——「候令」禁用。
