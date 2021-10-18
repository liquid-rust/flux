use rustc_span::{Span, Symbol};

#[derive(Debug)]
pub struct FnSig {
    pub params: Vec<(Ident, RType)>,
    pub args: Vec<Ty>,
    pub ret: Ty,
}

#[derive(Debug)]
pub struct Ty {
    pub name: Ident,
    pub refine: Expr,
}

#[derive(Debug)]
pub struct RType {
    pub bty: Ident,
    pub pred: Expr,
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Var(Ident),
    Literal(Lit),
    BinaryOp(BinaryOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
pub struct Lit {
    pub kind: LitKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug)]
pub enum LitKind {
    Int(i128),
}

#[derive(Debug)]
pub enum BinaryOp {
    Eq,
}

#[derive(Debug)]
pub struct Ident {
    pub symbol: Symbol,
    pub span: Span,
}
