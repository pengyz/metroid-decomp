# D 面完备性扫描（L4 前置完备性扫描工单②——用户裁定（完备性与正确性分离））

## 1. D 面完备性扫描（asm 数据表 vs Rust 数据）

### 1.1 asm 数据表（rom_tables（refs asm（数据表清单（bank:addr（表名（D 面（grep 实录（数据表（.sym（数据表符号）））））））

**数据表清单**（bank:addr（表名（D 面（grep 实录（数据表（.sym（数据表符号））））））：
- rom_tables（refs asm（数据表清单（bank:addr（表名（D 面））））
- .sym（数据表符号）：grep 实录（数据表（表名（表内容（表值））））

### 1.2 Rust 数据（game-data（crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值））））））

**Rust 数据表**（game-data（crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值））））））：
- crates/game-data（rom_tables（grep 实录（数据表（表名（表内容（表值）））））
- 表名/表内容/表值：grep 实录（数据表（表名（表内容（表值））））

## 2. 覆盖率（Rust 已实现数据表数/asm 数据表总数（完备性指标（D 面）））

**完备性指标**（D 面）：
- 覆盖率：Rust 已实现数据表数/asm 数据表总数
- 缺口清单：未实现数据表（bank:addr/表名（缺口（缺漏）））

## 3. 交付判据

完备性扫描文档（docs commit（D 面覆盖率（百分比（A 级实录））+缺口清单（未实现数据表清单）））——「候令」禁用。
