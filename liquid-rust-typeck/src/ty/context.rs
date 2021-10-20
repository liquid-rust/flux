use std::cell::RefCell;

use hashconsing::{HConsign, HashConsign};
use rustc_middle::ty::{IntTy, UintTy};

use super::{BinOp, Constant, EVar, Expr, ExprKind, ExprS, Refine, Ty, TyKind, TyS, Var};

pub struct LrCtxt {
    interner: RefCell<Interner>,
}

struct Interner {
    types: HConsign<TyS>,
    exprs: HConsign<ExprS>,
}

impl LrCtxt {
    pub fn new() -> Self {
        LrCtxt {
            interner: RefCell::new(Interner::new()),
        }
    }

    // Types

    pub fn mk_ty(&self, kind: TyKind) -> Ty {
        self.interner.borrow_mut().intern_ty(kind)
    }

    pub fn mk_int(&self, n: impl Into<Refine>, int_ty: IntTy) -> Ty {
        self.mk_ty(TyKind::Int(n.into(), int_ty))
    }

    pub fn mk_uint(&self, n: impl Into<Refine>, int_ty: UintTy) -> Ty {
        self.mk_ty(TyKind::Uint(n.into(), int_ty))
    }

    // Exprs

    pub fn mk_expr(&self, kind: ExprKind) -> Expr {
        self.interner.borrow_mut().intern_expr(kind)
    }

    pub fn mk_var(&self, x: Var) -> Expr {
        self.mk_expr(ExprKind::Var(x))
    }

    pub fn mk_evar(&self, x: EVar) -> Expr {
        self.mk_expr(ExprKind::EVar(x))
    }

    pub fn mk_constant(&self, c: Constant) -> Expr {
        self.mk_expr(ExprKind::Constant(c))
    }

    pub fn mk_bin_op(&self, bin_op: BinOp, e1: Expr, e2: Expr) -> Expr {
        self.mk_expr(ExprKind::BinaryOp(bin_op, e1, e2))
    }
}

impl Interner {
    fn new() -> Self {
        Interner {
            types: HConsign::empty(),
            exprs: HConsign::empty(),
        }
    }

    fn intern_ty(&mut self, kind: TyKind) -> Ty {
        self.types.mk(TyS { kind })
    }

    fn intern_expr(&mut self, kind: ExprKind) -> Expr {
        self.exprs.mk(ExprS { kind })
    }
}
