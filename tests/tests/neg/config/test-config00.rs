#![flux::opts(check_overflow = "do it!")] //~ ERROR invalid attribute: incorrect type in value for setting `check_overflow`, expected bool

#[flux::sig(fn(x: i32, y: i32) -> i32)]
pub fn test(x: i32, y: i32) -> i32 {
    x / y
}
