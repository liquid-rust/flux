#![feature(register_tool)]
#![register_tool(flux)]

#[flux::sig(fn(bool[true]))]
pub fn assert(_: bool) {}

// Test that we support async function returning unit
#[flux::sig(async fn())]
pub async fn test() {
    let x = make_nat().await;
    assert(x >= 0);
}

#[flux::sig(async fn(n:i32) -> i32{v: n <= v})]
pub async fn make_nat(n:i32) -> i32 {
    n + 1
}
