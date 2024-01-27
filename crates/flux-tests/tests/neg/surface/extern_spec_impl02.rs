// use flux_rs::extern_spec;

// Trait with assoc-pred
#[flux::predicate{ f : (int) -> bool }]
pub trait MyTrait {
    fn foo() -> i32;
}

// Function that uses assoc-pred generically
#[flux::trusted]
#[flux::sig(fn (_x:&T) -> i32{v: <T as MyTrait>::f(v)})]
pub fn bob<T: MyTrait>(_x: &T) -> i32 {
    <T as MyTrait>::foo()
}

impl MyTrait for usize {
    #[flux::trusted]
    fn foo() -> i32 {
        10
    }
}

// // extern impl
// // #[extern_spec]
// // #[flux::predicate{ f = |x:int| { 10 < x } }]
// // impl<T> MyTrait<Vec<T>> for usize {}

#[allow(dead_code)]
struct __FluxExternStruct1usize();

#[allow(dead_code)]
#[flux::extern_spec]
#[flux::predicate{ f = |x:int| { 10 < x } }]
impl __FluxExternStruct1usize {
    #[allow(unused_variables)]
    fn __flux_extern_impl_fake_method<A: MyTrait>(x: usize) {}
}

// // test

#[flux::sig(fn () -> i32{v: 100 < v})]
pub fn test_fail() -> i32 {
    let u: usize = 0;
    bob(&u) //~ ERROR refinement type
}

#[flux::sig(fn () -> i32{v: 10 < v})]
pub fn test_ok() -> i32 {
    let u: usize = 0;
    bob(&u)
}
