use itertools::Itertools;
use rustc_hash::FxHashSet;

use super::{BaseTy, Expr, ExprKind, Index, KVar, Loc, Name, Path, Pred, Ty, TyKind};

pub trait TypeVisitor: Sized {
    fn visit_fvar(&mut self, name: Name) {
        name.super_visit_with(self);
    }
}

pub trait TypeFolder: Sized {
    fn fold_fvar(&mut self, name: Name) -> Name {
        name.super_fold_with(self)
    }

    fn fold_ty(&mut self, ty: &Ty) -> Ty {
        ty.super_fold_with(self)
    }
}

pub trait TypeFoldable: Sized {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self;
    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V);

    fn fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        self.super_fold_with(folder)
    }

    fn visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        self.super_visit_with(visitor)
    }

    fn fvars(&self) -> FxHashSet<Name> {
        struct CollectFreeVars(FxHashSet<Name>);

        impl TypeVisitor for CollectFreeVars {
            fn visit_fvar(&mut self, name: Name) {
                self.0.insert(name);
            }
        }

        let mut collector = CollectFreeVars(FxHashSet::default());
        self.visit_with(&mut collector);
        collector.0
    }

    fn replace_holes(&self, factory: &mut impl FnMut(BaseTy) -> Pred) -> Self {
        self.fold_with(&mut ReplacePreds { factory, filter: |pred| matches!(pred, Pred::Hole) })
    }

    fn replace_preds_with_holes(&self) -> Self {
        self.fold_with(&mut ReplacePreds { factory: &mut |_| Pred::Hole, filter: |_| true })
    }
}

impl TypeFoldable for Ty {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Ty {
        match self.kind() {
            TyKind::Indexed(bty, indices) => {
                Ty::indexed(
                    bty.fold_with(folder),
                    indices
                        .iter()
                        .map(|idx| idx.fold_with(folder))
                        .collect_vec(),
                )
            }
            TyKind::Exists(bty, pred) => Ty::exists(bty.fold_with(folder), pred.fold_with(folder)),
            TyKind::Tuple(tys) => {
                Ty::tuple(tys.iter().map(|ty| ty.fold_with(folder)).collect_vec())
            }
            TyKind::Ptr(path) => Ty::ptr(path.fold_with(folder)),
            TyKind::Ref(rk, ty) => Ty::mk_ref(*rk, ty.fold_with(folder)),
            TyKind::Float(_)
            | TyKind::Uninit
            | TyKind::Param(_)
            | TyKind::Never
            | TyKind::Discr => self.clone(),
        }
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        match self.kind() {
            TyKind::Indexed(bty, indices) => {
                bty.visit_with(visitor);
                indices.iter().for_each(|idx| idx.visit_with(visitor));
            }
            TyKind::Exists(bty, pred) => {
                bty.visit_with(visitor);
                pred.visit_with(visitor);
            }
            TyKind::Tuple(tys) => tys.iter().for_each(|ty| ty.visit_with(visitor)),
            TyKind::Ref(_, ty) => ty.visit_with(visitor),
            TyKind::Ptr(path) => path.visit_with(visitor),
            TyKind::Param(_)
            | TyKind::Never
            | TyKind::Discr
            | TyKind::Float(_)
            | TyKind::Uninit => {}
        }
    }

    fn fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        folder.fold_ty(self)
    }
}

impl TypeFoldable for Index {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        Index { expr: self.expr.fold_with(folder), is_binder: self.is_binder }
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        self.expr.visit_with(visitor);
    }
}

impl TypeFoldable for BaseTy {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        match self {
            BaseTy::Adt(adt_def, substs) => {
                BaseTy::adt(adt_def.clone(), substs.iter().map(|ty| ty.fold_with(folder)))
            }
            BaseTy::Int(_) | BaseTy::Uint(_) | BaseTy::Bool => self.clone(),
        }
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        match self {
            BaseTy::Adt(_, substs) => substs.iter().for_each(|ty| ty.visit_with(visitor)),
            BaseTy::Int(_) | BaseTy::Uint(_) | BaseTy::Bool => {}
        }
    }
}

impl TypeFoldable for Pred {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        match self {
            Pred::Kvars(kvars) => {
                Pred::kvars(
                    kvars
                        .iter()
                        .map(|kvar| kvar.fold_with(folder))
                        .collect_vec(),
                )
            }
            Pred::Expr(e) => Pred::Expr(e.fold_with(folder)),
            Pred::Hole => Pred::Hole,
        }
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        match self {
            Pred::Expr(e) => e.visit_with(visitor),
            Pred::Kvars(kvars) => kvars.iter().for_each(|kvar| kvar.visit_with(visitor)),
            Pred::Hole => {}
        }
    }
}

impl TypeFoldable for KVar {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        let KVar(kvid, args) = self;
        KVar::new(*kvid, args.iter().map(|e| e.fold_with(folder)).collect_vec())
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        self.1.iter().for_each(|e| e.visit_with(visitor));
    }
}

impl TypeFoldable for Expr {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        match self.kind() {
            ExprKind::FreeVar(name) => Expr::fvar(name.fold_with(folder)),
            ExprKind::BoundVar(idx) => Expr::bvar(*idx),
            ExprKind::Constant(c) => Expr::constant(*c),
            ExprKind::BinaryOp(op, e1, e2) => {
                Expr::binary_op(*op, e1.fold_with(folder), e2.fold_with(folder))
            }
            ExprKind::UnaryOp(op, e) => Expr::unary_op(*op, e.fold_with(folder)),
            ExprKind::Proj(e, proj) => Expr::proj(e.fold_with(folder), *proj),
            ExprKind::Tuple(exprs) => {
                Expr::tuple(exprs.iter().map(|e| e.fold_with(folder)).collect_vec())
            }
            ExprKind::Path(path) => Expr::path(path.fold_with(folder)),
        }
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        match self.kind() {
            ExprKind::FreeVar(name) => name.visit_with(visitor),
            ExprKind::BinaryOp(_, e1, e2) => {
                e1.visit_with(visitor);
                e2.visit_with(visitor);
            }
            ExprKind::UnaryOp(_, e) | ExprKind::Proj(e, _) => e.visit_with(visitor),
            ExprKind::Tuple(exprs) => {
                for e in exprs {
                    e.visit_with(visitor);
                }
            }
            ExprKind::Path(path) => path.visit_with(visitor),
            ExprKind::Constant(_) | ExprKind::BoundVar(_) => {}
        }
    }
}

impl TypeFoldable for Path {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        Path::new(self.loc.fold_with(folder), self.projection.clone())
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        self.loc.visit_with(visitor);
    }
}

impl TypeFoldable for Loc {
    fn super_fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        match self {
            Loc::Local(local) => Loc::Local(*local),
            Loc::Free(name) => Loc::Free(name.fold_with(folder)),
        }
    }

    fn super_visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        match self {
            Loc::Local(_) => {}
            Loc::Free(name) => name.visit_with(visitor),
        }
    }
}

impl TypeFoldable for Name {
    fn super_fold_with<F: TypeFolder>(&self, _folder: &mut F) -> Self {
        *self
    }

    fn super_visit_with<V: TypeVisitor>(&self, _visitor: &mut V) {}

    fn fold_with<F: TypeFolder>(&self, folder: &mut F) -> Self {
        folder.fold_fvar(*self)
    }

    fn visit_with<V: TypeVisitor>(&self, visitor: &mut V) {
        visitor.visit_fvar(*self)
    }
}

struct ReplacePreds<'a, F1, F2> {
    factory: &'a mut F1,
    filter: F2,
}

impl<'a, F1, F2> TypeFolder for ReplacePreds<'a, F1, F2>
where
    F1: FnMut(BaseTy) -> Pred,
    F2: FnMut(Pred) -> bool,
{
    fn fold_ty(&mut self, ty: &Ty) -> Ty {
        if let TyKind::Exists(bty, pred) = ty.kind() && (self.filter)(pred.clone()) {
            Ty::exists(bty.super_fold_with(self), (self.factory)(bty.clone()))
        } else {
            ty.super_fold_with(self)
        }
    }
}
