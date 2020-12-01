use crate::{bblock::BBlockId, operand::Operand};

use std::fmt;

pub enum Terminator {
    Return,
    Goto(BBlockId),
    Assert(Operand, bool, BBlockId),
}

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Return => "return".fmt(f),
            Self::Goto(bb_id) => write!(f, "goto {}", bb_id),
            Self::Assert(op, true, bb_id) => write!(f, "assert({}) -> {}", op, bb_id),
            Self::Assert(op, false, bb_id) => write!(f, "assert(!{}) -> {}", op, bb_id),
        }
    }
}
