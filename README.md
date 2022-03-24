# LiquidRust


## TODO(RJ)

```
$ liquid-rust --crate-type=rlib liquid-rust-tests/tests/pos/surface/scope.rs
```

```



process param: pat#0 with type Ty { kind: AnonEx { path: Path { ident: Adt(DefId(0:9 ~ debug[8825]::rvec::RVec)), args: Some([Ty { kind: Base(Path { ident: Uint(U8), args: None, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:24: 8:26 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:24: 8:26 (#0) }]), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:19: 8:27 (#0) }, pred: Expr { kind: BinaryOp(<=, Expr { kind: Var(pat#0), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:28: 8:31 (#0) }, Expr { kind: Var(n#0), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:33: 8:34 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:28: 8:34 (#0) } }, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:19: 8:35 (#0) }

resolve_bare_sig FnSig { requires: [(pat#0, Ty { kind: AnonEx { path: Path { ident: Adt(DefId(0:9 ~ debug[8825]::rvec::RVec)), args: Some([Ty { kind: Base(Path { ident: Uint(U8), args: None, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:24: 8:26 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:24: 8:26 (#0) }]), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:19: 8:27 (#0) }, pred: Expr { kind: BinaryOp(<=, Expr { kind: Var(pat#0), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:28: 8:31 (#0) }, Expr { kind: Var(n#0), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:33: 8:34 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:28: 8:34 (#0) } }, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:19: 8:35 (#0) }), (target#0, Ty { kind: Ref(Immut, Ty { kind: Named(n#0, Ty { kind: Base(Path { ident: Adt(DefId(0:9 ~ debug[8825]::rvec::RVec)), args: Some([Ty { kind: Base(Path { ident: Uint(U8), args: None, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:53: 8:55 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:53: 8:55 (#0) }]), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:48: 8:56 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:48: 8:56 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:46: 8:56 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:45: 8:56 (#0) })], returns: Ty { kind: Base(Path { ident: Uint(Usize), args: None, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:61: 8:66 (#0) }), span: liquid-rust-tests/tests/pos/surface/debug.rs:8:61: 8:66 (#0) }, ensures: [], wherep: None, span: liquid-rust-tests/tests/pos/surface/debug.rs:8:11: 8:66 (#0) }
```



## Requirements

* [rustup](https://rustup.rs/)
* [liquid-fixpoint](https://github.com/ucsd-progsys/liquid-fixpoint)
* [z3](https://github.com/Z3Prover/z3)

Be sure that the `liquid-fixpoint` and `z3` executables are in your $PATH.

## Build Instructions

The only way to test LiquidRust is to build it from source.

First you need to clone this repository

```bash
git clone https://github.com/liquid-rust/liquid-rust
cd liquid-rust
```

To build the source you need a nightly version of rustc.
We pin the version using a [toolchain file](/rust-toolchain) (more info [here](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)).
If you are using rustup, no special action is needed as it should install the correct rustc version and components based on the information on that file.

Finally, build the project using `cargo`

```bash
cargo build
```

## Usage

### liquid-rust binary

You can run the liquid-rust binary with `cargo run`.
The liquid-rust binary is a [rustc driver](https://rustc-dev-guide.rust-lang.org/rustc-driver.html?highlight=driver#the-rustc-driver-and-interface) (similar to how clippy works) meaning it uses rustc as a library to "drive" compilation performing aditional analysis along the way.
In practice this means you can use liquid-rust as you would use rustc.
For example, the following command checks the file `test.rs` (everythins after the `--` are the arguments to the liquid-rust binary)

```bash
cargo run -- path/to/test.rs
```

The liquid-rust binary accepts the same flags than rustc.
You could for example check a file as a library instead of a binary like so

```bash
cargo run -- --crate-type=lib path/to/test.rs
```

Additionally, at the moment liquid-rust passes some
default flags (like `-O` and `-Cpanic=abort`) because
otherwise the resulting mir will have features
not yet supported.

### A tiny example

The following example declares a funcion `inc` that returns a integer greater than the input.
We use the nightly feature `register_tool` to register the `lr` tool in order to add refinement annotations to functions.

```rust
#![feature(register_tool)]
#![register_tool(lr)]

#[lr::ty(fn<n: int>(i32@n) -> i32{v: v > n})]
pub fn inc(x: i32) -> i32 {
    x + 1
}
```

You can save the above snippet in say `test0.rs` and then run

```
cargo run -- --crate-type=lib path/to/test0.rs
```

and you should get some output like

```
Ok(FixpointResult { tag: Safe })
```

## Test

You can run the various tests in the `tests/pos` and `tests/neg` directory using

```
$ cargo test -p liquid-rust-driver
```


## Limitations

This is a prototype! Use at your own risk. Everything could break and it will break.
