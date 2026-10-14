//! game-core —— 纯逻辑核（AGENTS.md D1/D7；无 I/O、确定性）。
//!
//! L2 子程序级移植面：`l2ports` 每个函数对应 `oracle/subroutines/l2_cases.jsonl` 一条用例，
//! 三方全等裁定：case.out(golden) == 6502 核执行 ROM 原字节 == 本 crate 同名函数。

pub mod gap;
pub mod l2ports;

/// L2 入参：寄存器 + 2KB RAM 快照（$0000-$07FF，镜像归一后）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2In {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<u8>,
}

impl Default for L2In {
    fn default() -> Self {
        Self { a: 0, x: 0, y: 0, p: 0x24, ram: vec![0; 0x800] }
    }
}

/// L2 出参：寄存器（a/x/y；p 为执行副产物不入比对面）+ 写集（addr 归一至 $0000-$07FF）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2Out {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub writes: Vec<(u16, u8)>,
}
