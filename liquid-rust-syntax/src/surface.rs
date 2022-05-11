use std::fmt;

pub use rustc_ast::token::LitKind;
use rustc_ast::Mutability;
use rustc_hir::def_id::DefId;
pub use rustc_middle::ty::{FloatTy, IntTy, ParamTy, TyCtxt, UintTy};
pub use rustc_span::symbol::Ident;
use rustc_span::{Span, Symbol};

pub type AliasMap = rustc_hash::FxHashMap<Ident, Alias>;

#[derive(Debug)]
pub struct Qualifier {
    pub name: Ident,
    pub args: Vec<Param>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct Alias<T = Ident> {
    pub name: Ident,
    pub args: Vec<Ident>,
    pub defn: Ty<T>,
    pub span: Span,
}

#[derive(Debug)]
pub struct StructDef<T = Ident> {
    pub refined_by: Option<Params>,
    pub fields: Vec<Option<Ty<T>>>,
    pub opaque: bool,
}

#[derive(Debug)]
pub struct EnumDef {
    pub refined_by: Option<Params>,
    pub opaque: bool,
}

#[derive(Debug)]
pub struct Params {
    pub params: Vec<Param>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Param {
    pub name: Ident,
    pub sort: Ident,
}

#[derive(Debug)]
pub struct FnSig<T = Ident> {
    /// example: `requires n > 0`
    pub requires: Option<Expr>,
    /// example: `i32<@n>`
    pub args: Vec<Arg<T>>,
    /// example `i32{v:v >= 0}`
    pub returns: Ty<T>,
    /// example: `*x: i32{v. v = n+1}`
    pub ensures: Vec<(Ident, Ty<T>)>,
    /// source span
    pub span: Span,
}

#[derive(Debug)]
pub enum Arg<T = Ident> {
    /// example `a: i32{a > 0}`
    Indexed(Ident, Path<T>, Option<Expr>),
    /// example `x: nat` or `x: lb[0]`
    Alias(Ident, Path<T>, Indices),
    /// example `v: &strg i32`
    StrgRef(Ident, Ty<T>),
    /// example `i32`
    Ty(Ty<T>),
}

#[derive(Debug)]
pub struct Ty<R = Ident> {
    pub kind: TyKind<R>,
    pub span: Span,
}

#[derive(Debug)]
pub enum TyKind<T = Ident> {
    /// ty
    Path(Path<T>),

    /// t[e]
    Indexed { path: Path<T>, indices: Indices },

    /// ty{b:e}
    Exists { bind: Ident, path: Path<T>, pred: Expr },

    /// Mutable or shared reference
    Ref(RefKind, Box<Ty<T>>),

    /// Strong reference, &strg<self: i32>
    StrgRef(Ident, Box<Ty<T>>),
}

#[derive(Debug, Clone)]
pub struct Indices {
    pub indices: Vec<Index>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Index {
    /// @n
    Bind(Ident),
    Expr(Expr),
}

#[derive(Debug)]
pub struct Path<R = Ident> {
    /// e.g. vec
    pub ident: R,
    /// e.g. <nat>
    pub args: Vec<Ty<R>>,
    pub span: Span,
}

#[derive(Debug, Copy, Clone)]
pub enum Res {
    Bool,
    Int(IntTy),
    Uint(UintTy),
    Float(FloatTy),
    Adt(DefId),
    Param(ParamTy),
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum RefKind {
    Mut,
    Shr,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Var(Ident),
    Literal(Lit),
    BinaryOp(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub struct Lit {
    pub kind: LitKind,
    pub symbol: Symbol,
    pub span: Span,
}

#[derive(Copy, Clone)]
pub enum BinOp {
    Iff,
    Imp,
    Or,
    And,
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mod,
    Mul,
}

impl Path<Res> {
    pub fn is_bool(&self) -> bool {
        matches!(self.ident, Res::Bool)
    }

    pub fn is_float(&self) -> bool {
        matches!(self.ident, Res::Float(_))
    }
}

impl<R> Arg<R> {
    #[track_caller]
    fn assert_ty(self) -> Ty<R> {
        match self {
            Arg::Ty(ty) => ty,
            _ => panic!("not a type"),
        }
    }
}

impl Params {
    pub fn empty(span: Span) -> Params {
        Params { params: vec![], span }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Param> {
        self.params.iter()
    }
}

impl IntoIterator for Params {
    type Item = Param;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.params.into_iter()
    }
}

impl fmt::Debug for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Iff => write!(f, "<=>"),
            BinOp::Imp => write!(f, "=>"),
            BinOp::Or => write!(f, "||"),
            BinOp::And => write!(f, "&&"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Le => write!(f, "<="),
            BinOp::Gt => write!(f, ">"),
            BinOp::Ge => write!(f, ">="),
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mod => write!(f, "mod"),
            BinOp::Mul => write!(f, "*"),
        }
    }
}

// ---------------------------------------------------------------------------
// -------------------------- DEFAULT Signatures -----------------------------
// ---------------------------------------------------------------------------

fn default_refkind(m: &Mutability) -> RefKind {
    match m {
        Mutability::Mut => RefKind::Mut,
        Mutability::Not => RefKind::Shr,
    }
}

fn default_path(ty: rustc_middle::ty::Ty) -> Path<Res> {
    let (ident, args) = match ty.kind() {
        rustc_middle::ty::TyKind::Bool => (Res::Bool, vec![]),
        rustc_middle::ty::TyKind::Int(int_ty) => (Res::Int(*int_ty), vec![]),
        rustc_middle::ty::TyKind::Uint(uint_ty) => (Res::Uint(*uint_ty), vec![]),
        rustc_middle::ty::TyKind::Float(float_ty) => (Res::Float(*float_ty), vec![]),
        rustc_middle::ty::TyKind::Param(param_ty) => (Res::Param(*param_ty), vec![]),
        rustc_middle::ty::TyKind::Adt(adt, substs) => {
            let substs = substs.types().map(default_ty).collect();
            (Res::Adt(adt.did), substs)
        }
        _ => todo!("default_path: `{ty:?}`"),
    };
    Path { ident, args, span: rustc_span::DUMMY_SP }
}

fn default_ty(ty: rustc_middle::ty::Ty) -> Ty<Res> {
    let kind = match ty.kind() {
        rustc_middle::ty::TyKind::Ref(_, ty, m) => {
            let ref_kind = default_refkind(m);
            let tgt_ty = default_ty(*ty);
            TyKind::Ref(ref_kind, Box::new(tgt_ty))
        }
        _ => TyKind::Path(default_path(ty)),
    };
    Ty { kind, span: rustc_span::DUMMY_SP }
}

pub fn default_fn_sig(tcx: TyCtxt, def_id: DefId) -> FnSig<Res> {
    let rust_sig = tcx.erase_late_bound_regions(tcx.fn_sig(def_id));
    let args = rust_sig
        .inputs()
        .iter()
        .map(|rust_ty| Arg::Ty(default_ty(*rust_ty)))
        .collect();
    let returns = default_ty(rust_sig.output());
    FnSig { args, returns, ensures: vec![], requires: None, span: rustc_span::DUMMY_SP }
}

pub mod expand {
    use std::{collections::HashMap, iter};

    use rustc_span::symbol::Ident;

    use super::{AliasMap, Arg, BinOp, Expr, ExprKind, FnSig, Index, Indices, Path, Ty, TyKind};

    /// `expand_bare_sig(aliases, b_sig)` replaces all the alias-applications in `b_sig`
    /// with the corresponding type definitions from `aliases` (if any).
    pub fn expand_sig(aliases: &AliasMap, fn_sig: FnSig) -> FnSig {
        FnSig {
            args: expand_args(aliases, fn_sig.args),
            returns: expand_ty(aliases, &fn_sig.returns),
            ensures: expand_locs(aliases, fn_sig.ensures),
            requires: fn_sig.requires,
            span: fn_sig.span,
        }
    }

    fn expand_args(aliases: &AliasMap, args: Vec<Arg>) -> Vec<Arg> {
        args.into_iter()
            .map(|arg| expand_arg(aliases, arg))
            .collect()
    }

    fn expand_arg(aliases: &AliasMap, arg: Arg) -> Arg {
        match arg {
            Arg::Alias(x, path, indices) => {
                match expand_alias(aliases, &path, &indices) {
                    Some(TyKind::Exists { bind: e_bind, path: e_path, pred: e_pred }) => {
                        let subst = mk_sub1(e_bind, x);
                        let x_pred = subst_expr(&subst, &e_pred);
                        Arg::Indexed(x, e_path, Some(x_pred))
                    }
                    _ => panic!("bad alias app: {:?}[{:?}]", &path, &indices),
                }
            }
            Arg::Indexed(x, path, e) => Arg::Indexed(x, expand_path(aliases, &path), e),
            Arg::Ty(t) => Arg::Ty(expand_ty(aliases, &t)),
            Arg::StrgRef(x, t) => Arg::StrgRef(x, expand_ty(aliases, &t)),
        }
    }

    fn expand_alias(aliases: &AliasMap, path: &Path, indices: &Indices) -> Option<TyKind> {
        let id = path.ident;
        // let id_alias = aliases.get(&id);
        // println!("ALIAS: expand_alias: {:?} -> {:?}", id, id_alias);
        match aliases.get(&id) {
            Some(alias) /* if path.args.is_empty() */ => {
                let subst = mk_sub(&alias.args, &indices.indices);
                let ty = subst_ty(&subst, &alias.defn);
                Some(ty.kind)
            }
            _ => None,
        }
    }

    fn expand_path(aliases: &AliasMap, path: &Path) -> Path {
        Path {
            ident: path.ident,
            args: path.args.iter().map(|t| expand_ty(aliases, t)).collect(),
            span: path.span,
        }
    }

    fn expand_ty(aliases: &AliasMap, ty: &Ty) -> Ty {
        let kind = expand_kind(aliases, &ty.kind);
        Ty { kind, span: ty.span }
    }

    fn expand_kind(aliases: &AliasMap, k: &TyKind) -> TyKind {
        match k {
            TyKind::Path(p) => TyKind::Path(expand_path(aliases, &p)),
            TyKind::Exists { bind, path, pred } => {
                TyKind::Exists {
                    bind: bind.clone(),
                    path: expand_path(aliases, &path),
                    pred: pred.clone(),
                }

                //                match expand_path(aliases, &path) {
                //                    None => TyKind::Exists { bind, path, pred },
                //                    Some(TyKind::Path(ep)) => TyKind::Exists { bind, path: ep, pred },
                //                    Some(TyKind::Exists { bind: e_bind, path: e_path, pred: e_pred }) => {
                //                        let subst = mk_sub1(e_bind, bind);
                //                        TyKind::Exists {
                //                            bind,
                //                            path: e_path,
                //                            pred: and(pred, subst_expr(&subst, &e_pred)),
                //                        }
                //                    }
                //                    Some(_) => panic!("expand_path:unexpected:exists"),
                //                }
            }
            TyKind::Indexed { path, indices } => {
                match expand_alias(aliases, &path, &indices) {
                    Some(k) => k,
                    None => {
                        TyKind::Indexed {
                            path: expand_path(aliases, &path),
                            indices: indices.clone(),
                        }
                    }
                    // None => TyKind::Indexed { path, indices },
                    // Some(TyKind::Path(ep)) => TyKind::Indexed { path: ep, indices },
                    // Some(_) => panic!("expand_path:unexpected:index"),
                }
            }
            TyKind::Ref(rk, t) => TyKind::Ref(rk.clone(), Box::new(expand_ty(aliases, &*t))),
            TyKind::StrgRef(rk, t) => {
                TyKind::StrgRef(rk.clone(), Box::new(expand_ty(aliases, &*t)))
            }
        }
    }

    fn _and(e1: Expr, e2: Expr) -> Expr {
        let span = e1.span;
        let kind = ExprKind::BinaryOp(BinOp::And, Box::new(e1), Box::new(e2));
        Expr { kind, span }
    }

    fn expand_locs(aliases: &AliasMap, locs: Vec<(Ident, Ty)>) -> Vec<(Ident, Ty)> {
        locs.into_iter()
            .map(|(x, ty)| (x, expand_ty(aliases, &ty)))
            .collect()
    }

    type Subst = HashMap<Ident, Expr>;

    fn mk_sub1(src: Ident, dst: Ident) -> Subst {
        HashMap::from([(src, Expr { kind: ExprKind::Var(dst), span: dst.span })])
    }

    fn mk_sub(src: &Vec<Ident>, dst: &Vec<Index>) -> Subst {
        if src.len() != dst.len() {
            panic!("mk_sub: invalid args")
        }
        let mut res = HashMap::new();
        for (src_id, dst_ix) in iter::zip(src, dst) {
            match dst_ix {
                Index::Expr(e) => {
                    res.insert(*src_id, e.clone());
                }
                Index::Bind(_) => panic!("cannot use binder in type alias"),
                // TyKind::Path(p) if p.args.is_empty() => {
                //     res.insert(*src_id, p.ident);
                // }
                // _ => panic!("mk_sub: invalid arg"),
            }
        }
        res
    }

    fn subst_expr(subst: &Subst, e: &Expr) -> Expr {
        match &e.kind {
            ExprKind::Var(x) => {
                match subst.get(x) {
                    Some(y) => y.clone(),
                    None => e.clone(),
                }
            }
            ExprKind::Literal(l) => Expr { kind: ExprKind::Literal(*l), span: e.span },
            ExprKind::BinaryOp(o, e1, e2) => {
                Expr {
                    kind: ExprKind::BinaryOp(
                        *o,
                        Box::new(subst_expr(subst, e1)),
                        Box::new(subst_expr(subst, e2)),
                    ),
                    span: e.span,
                }
            }
        }
    }

    fn subst_path(subst: &Subst, p: &Path) -> Path {
        let mut args = vec![];
        for t in p.args.iter() {
            args.push(subst_ty(subst, &t));
        }
        Path { ident: p.ident, args, span: p.span }
    }

    fn subst_ty(subst: &Subst, ty: &Ty) -> Ty {
        Ty { kind: subst_tykind(subst, &ty.kind), span: ty.span }
    }

    fn subst_indices(subst: &Subst, i_indices: &Indices) -> Indices {
        let mut indices = vec![];
        for i in i_indices.indices.iter() {
            indices.push(subst_index(subst, i));
        }
        Indices { indices, span: i_indices.span }
    }

    fn subst_index(subst: &Subst, i: &Index) -> Index {
        match i {
            super::Index::Expr(e) => Index::Expr(subst_expr(subst, e)),
            super::Index::Bind(_) => i.clone(),
        }
    }

    fn subst_tykind(subst: &Subst, k: &TyKind) -> TyKind {
        match k {
            TyKind::Path(p) => TyKind::Path(subst_path(subst, p)),
            TyKind::Indexed { path, indices } => {
                TyKind::Indexed {
                    path: subst_path(subst, path),
                    indices: subst_indices(subst, indices),
                }
            }
            TyKind::Exists { bind, path, pred } => {
                TyKind::Exists {
                    bind: *bind,
                    path: subst_path(subst, path),
                    pred: subst_expr(subst, pred),
                }
            }
            TyKind::Ref(rk, t) => TyKind::Ref(*rk, Box::new(subst_ty(subst, &*t))),
            TyKind::StrgRef(rk, t) => TyKind::StrgRef(*rk, Box::new(subst_ty(subst, &*t))),
        }
    }
}
pub mod zip {

    use std::{collections::HashMap, iter};

    use itertools::Itertools;
    use rustc_span::Symbol;

    use super::{Arg, FnSig, Ident, Path, RefKind, Res, Ty, TyKind};

    type Locs = HashMap<Symbol, Ty<Res>>;

    /// `zip_bare_def(b_sig, d_sig)` combines the refinements of the `b_sig` and the resolved elements
    /// of the (trivial/default) `dsig:DefFnSig` to compute a (refined) `DefFnSig`
    pub fn zip_bare_def(b_sig: FnSig, d_sig: FnSig<Res>) -> FnSig<Res> {
        let mut locs: Locs = HashMap::new();
        let default_args = d_sig.args.into_iter().map(|arg| arg.assert_ty()).collect();
        FnSig {
            args: zip_args(b_sig.args, default_args, &mut locs),
            returns: zip_ty(b_sig.returns, &d_sig.returns),
            ensures: zip_ty_locs(b_sig.ensures, &locs),
            requires: b_sig.requires,
            span: b_sig.span,
        }
    }

    /// `zip_ty_locs` traverses the bare-outputs and zips with the location-types saved in `locs`
    fn zip_ty_locs(bindings: Vec<(Ident, Ty)>, locs: &Locs) -> Vec<(Ident, Ty<Res>)> {
        let mut res = vec![];
        for (ident, ty) in bindings {
            if let Some(default) = locs.get(&ident.name) {
                let dt = zip_ty(ty, default);
                res.push((ident, dt))
            } else {
                panic!("missing location type for `{}`", ident)
            }
        }
        res
    }

    /// `zip_ty_binds` traverses the inputs `bs` and `ds` and
    /// saves the types of the references in `locs`
    fn zip_args(binds: Vec<Arg>, defaults: Vec<Ty<Res>>, locs: &mut Locs) -> Vec<Arg<Res>> {
        if binds.len() != defaults.len() {
            panic!(
                "bind count mismatch, expected: {:?},  found: {:?}",
                binds.len(),
                defaults.len()
            );
        }
        let binds = iter::zip(binds, &defaults)
            .map(|(arg, default)| zip_arg(arg, default))
            .collect_vec();
        for (arg, default) in iter::zip(&binds, defaults) {
            if let (Arg::StrgRef(bind, _), TyKind::Ref(RefKind::Mut, default)) = (arg, default.kind)
            {
                locs.insert(bind.name, *default);
            }
        }
        binds
    }

    fn zip_arg(arg: Arg, default: &Ty<Res>) -> Arg<Res> {
        match (arg, &default.kind) {
            (Arg::Ty(ty), _) => Arg::Ty(zip_ty(ty, default)),
            (Arg::Indexed(bind, path, pred), TyKind::Path(default)) => {
                Arg::Indexed(bind, zip_path(path, default), pred)
            }
            (Arg::StrgRef(bind, ty), TyKind::Ref(RefKind::Mut, default)) => {
                Arg::StrgRef(bind, zip_ty(ty, default))
            }
            _ => panic!("incompatible types `{default:?}`"),
        }
    }

    fn zip_ty(ty: Ty, default: &Ty<Res>) -> Ty<Res> {
        let kind = match (ty.kind, &default.kind) {
            (TyKind::Path(path), TyKind::Path(default)) => TyKind::Path(zip_path(path, default)),
            (TyKind::Indexed { path, indices }, TyKind::Path(default)) => {
                TyKind::Indexed { path: zip_path(path, default), indices }
            }
            (TyKind::Exists { bind, path, pred }, TyKind::Path(default)) => {
                TyKind::Exists { bind, path: zip_path(path, default), pred }
            }
            (TyKind::StrgRef(loc, ty), TyKind::Ref(RefKind::Mut, default)) => {
                TyKind::StrgRef(loc, Box::new(zip_ty(*ty, default)))
            }
            (TyKind::Ref(rk, ty), TyKind::Ref(default_rk, default)) if rk == *default_rk => {
                TyKind::Ref(rk, Box::new(zip_ty(*ty, default)))
            }
            _ => panic!("incompatible types `{default:?}`"),
        };
        Ty { kind, span: ty.span }
    }

    fn zip_path(path: Path, default: &Path<Res>) -> Path<Res> {
        if path.args.len() != default.args.len() {
            panic!(
                "argument count mismatch, expected: {:?},  found: {:?}",
                default.args.len(),
                path.args.len()
            );
        }
        let args = iter::zip(path.args, &default.args)
            .map(|(ty, default)| zip_ty(ty, default))
            .collect();

        Path { ident: default.ident, args, span: path.span }
    }
}
