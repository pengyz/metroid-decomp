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
