//! game-core::memmap::views —— **语义视图层**：领域代码与内存字节之间的唯一接口。
//!
//! 形态（每个域一个视图）：
//! ```ignore
//! mem.samus().set_speed_y(Fixed::from_int(-2));   // 领域语言
//! let st = mem.samus().status();                  // SamusStatus，不是 u8
//! mem.enemy(x).set_status(EnemyStatus::Explode);
//! ```
//! 视图持有对私有存储的手柄，**内部**做地址换算；调用方看不到地址。将来把内部存储从
//! `[u8; 0x800]` 换成真结构体时，只改 view 的实现，调用方签名不变；那时 `observe/` 的
//! 字节导出成为唯一一处「状态 → NES 布局」映射，且只影响观测、不影响判定。
//!
//! 三条细则（防重演历史事故）：
//! 1. **同一字节多义不美化**：`$0406` 在 asm 里三名共存（chunkyExplosionTimer /
//!    movementInstrIndex / speedSubPixelY）——按消费者各给访问器，同址异名并注明，禁
//!    合成一个「好听名字」。
//! 2. **多字节字段封装**：health 跨 `+0/+1`、16 位加速量、8.8 定点——视图出 `u16`/`Fixed`，
//!    把「进位写进下一字节」的规则关在视图里。
//! 3. **借用互斥**：视图持 `&mut`，一次只能持一个域——逼着按 asm 的执行顺序逐域操作。

pub mod title;
