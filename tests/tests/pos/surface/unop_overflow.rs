#![flux::opts(check_overflow = true)]

#[flux::sig(fn(a: i32{a != i32::MIN}) -> i32[-a])]
pub fn neg_overflow_i32(a: i32) -> i32 {
    -a
}
