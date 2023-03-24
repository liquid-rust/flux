#![feature(register_tool)]
#![register_tool(flux)]
#![feature(custom_inner_attributes)]
#![flux::defs(
      fn pow2(x:int) -> bool { pow2bv(int2bv(x)) }
      fn pow2bv(x:bitvec<32>) -> bool { bv_and(x, bv_sub(x, int_to_bv32(1))) == int_to_bv32(0) }
  )]

// Define a new type to wrap `usize` but indexed by actual bitvec
#[flux::refined_by(value: bitvec<32>)]
struct UsizeBv {
    inner: usize,
}

// Define "cast" functions
#[flux::trusted]
#[flux::sig(fn (n:usize) -> UsizeBv[int_to_bv32(n)])]
fn to_bv(inner: usize) -> UsizeBv {
    UsizeBv { inner }
}

#[flux::trusted]
#[flux::sig(fn (bv:UsizeBv) -> usize[bv32_to_int(bv)])]
fn from_bv(bv: UsizeBv) -> usize {
    bv.inner
}

fn bv_and(x: UsizeBv, y: UsizeBv) -> UsizeBv {
    UsizeBv { inner: x.inner & y.inner }
}

#[flux::sig(fn (index: usize, size:usize{1 <= size && pow2(size)}) -> usize{v: v < size})]
pub fn wrap_index(index: usize, size: usize) -> usize {
    // size is always a power of 2
    // assert(is_power_of_two(size));
    from_bv(bv_and(to_bv(index), to_bv(size - 1)))
    // define `&` with precise semantics for BV sort.
}
