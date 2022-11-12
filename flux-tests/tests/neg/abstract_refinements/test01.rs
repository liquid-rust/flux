#![feature(register_tool)]
#![register_tool(flux)]

#[flux::refined_by(a: int, b: int, p: (int, int) -> bool)]
struct Pair {
    #[flux::field(i32[@a])]
    fst: i32,
    #[flux::field({i32[@b] : p(a, b)})]
    snd: i32,
}

#[flux::sig(fn(Pair[@a, @b, |a, b| a <= b]) -> i32{v: v > 0})]
fn test00(pair: Pair) -> i32 {
    pair.snd - pair.fst //~ ERROR postcondition
}

fn test01() {
    let pair = Pair { fst: 1, snd: 0 };
    let x = test00(pair); //~ ERROR precondition
}
