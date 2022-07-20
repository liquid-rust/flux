#![feature(register_tool)]
#![register_tool(flux)]

#[flux::sig(fn(x: &i32[@a], y: &i32[b] where b <= a) -> i32[a-b])]
fn sub(x: &i32, y: &i32) -> i32 {
    *y - *x
}

#[flux::sig(fn() -> i32[5])]
pub fn test() -> i32 {
    let a = 20;
    let b = 15;
    sub(&a, &b)
}
