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
