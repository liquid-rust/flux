#![feature(register_tool)]
#![register_tool(flux)]

#[flux::sig(fn(bool[true]))]
pub fn assert(_b: bool) {}

// #[flux::sig(fn(it: I) where I: Iterator<Item = i32{v:0<=v}>)]
pub fn foo<I>(it: I)
where
    I: Iterator<Item = i32>,
{
    for x in it {
        assert(0 <= x) //~ ERROR refinement type
    }
}

pub fn test() {
    let mut v = Vec::new();
    v.push(1);
    foo(v.into_iter());
}
