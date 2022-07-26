#![feature(register_tool)]
#![register_tool(flux)]

// Test definition and checking of const

#[flux::constant(usize[42])]    //~ ERROR invalid flux attribute
const FORTY_TWO: usize = 21 + 21;

#[flux::sig(fn() -> bool[true])]
pub fn test0() -> bool {
    FORTY_TWO == 40 + 2
}
