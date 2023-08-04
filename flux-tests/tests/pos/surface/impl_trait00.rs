#![feature(register_tool)]
#![register_tool(flux)]

#[flux::sig(fn () -> impl Iterator<Item = i32{v:1000<=v}>)]
pub fn foo() -> impl Iterator<Item = i32> {
    Some(10).into_iter()
}
/*

pub trait Easy {
    type Thing;
}

impl<T> Easy for Option<T> {
    type Thing = T;
}

// fn(x:i32) -> impl Future<Output=i32{v: 0 < v}>
async fn(x:i32) -> i32{v: 0 < v} {
    10
}

// fn(x:i32) -> impl Future<Output=i32{v: x < v}>
async fn(x:i32) -> i32{v: x < v} {
    x + 1
}
*/
