# 分界线修（rom_tables→game-data（模块重组）——L4 就绪批（TOCTOU 同块（verify_shim/mod.rs 独占（decomp））））

> 【2026-10-21 分界线修】game-core=逻辑（SamusState/enemy_state（计算））vs game-data=数据表/资产解析（rom_tables（bank:addr（表值（表用途）））——refs 引用必写（bank:addr 引证（表读径实证（A 级原文）））。

## 分界线（game-core vs game-data）

- **game-core**（逻辑（SamusState/enemy_state（计算）——Samus 域（跳跃/变球）））。
- **game-data**（数据表/资产解析（rom_tables（bank:addr（表值（表用途）））——ROM 数据提取））。

## verify_shim/mod.rs 引用更新（rom_tables→game-data（168/246/255 行））

- `memmap::rom_tables::rom_vec` → `game_data::rom_tables::rom_vec`（168/255 行）。
- `memmap::rom_tables::table_968b` → `game_data::rom_tables::table_968b`（246 行）。
- **TOCTOU 同块**（verify_shim/mod.rs 独占（decomp（串行化协议照条）））。

## 编译核+留档可追溯

- **编译核**（cargo check（**0 错**））。
- **refs 引用**（bank:addr 引证（表读径实证（A 级原文））——rom_tables（bank:addr（表值（表用途）））。
- **留档可追溯**（refs 引用（bank:addr）+分界线修（game-core/game-data（模块重组）+verify_shim/mod.rs 引用更新）——docs commit（追溯径））。
