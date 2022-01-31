use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use itertools::{izip, Itertools};
use liquid_rust_common::index::{Idx, IndexGen, IndexVec};
use liquid_rust_fixpoint as fixpoint;
use rustc_hash::FxHashMap;
use rustc_middle::ty::TyCtxt;

use crate::ty::{
    BaseTy, BinOp, Expr, ExprKind, ExprS, KVid, Loc, Name, Pred, Sort, Ty, TyKind, Var,
};

pub struct PureCtxt {
    root: NodePtr,
}

pub struct Cursor<'a> {
    cx: &'a mut PureCtxt,
    node: NodePtr,
}

pub struct Snapshot {
    node: WeakNodePtr,
}

pub struct KVarStore {
    kvars: IndexVec<KVid, Vec<Sort>>,
}

struct Node {
    kind: NodeKind,
    /// Number of binding nodes between the root and this node's parent
    nbindings: usize,
    parent: Option<WeakNodePtr>,
    children: Vec<NodePtr>,
}

type NodePtr = Rc<RefCell<Node>>;
type WeakNodePtr = Weak<RefCell<Node>>;

enum NodeKind {
    Conj,
    Binding(Name, Sort, Pred),
    Loc(Name),
    Pred(Expr),
    Head(Pred),
}

struct FixpointCtxt<'a> {
    kvars: &'a KVarStore,
    name_gen: IndexGen<fixpoint::Name>,
    name_map: FxHashMap<Name, fixpoint::Name>,
}

impl PureCtxt {
    pub fn new() -> PureCtxt {
        let root = Node {
            kind: NodeKind::Conj,
            nbindings: 0,
            parent: None,
            children: vec![],
        };
        let root = Rc::new(RefCell::new(root));
        PureCtxt { root }
    }

    pub fn cursor_at_root<'a>(&'a mut self) -> Cursor<'a> {
        Cursor {
            node: Rc::clone(&self.root),
            cx: self,
        }
    }

    pub fn into_fixpoint(&self, kvars: KVarStore) -> fixpoint::Fixpoint {
        let mut cx = FixpointCtxt::new(&kvars);
        let constraint = self
            .root
            .borrow()
            .into_fixpoint(&mut cx)
            .unwrap_or(fixpoint::Constraint::TRUE);
        let kvars = kvars
            .kvars
            .into_iter_enumerated()
            .map(|(kvid, sorts)| fixpoint::KVar(kvid, sorts))
            .collect();
        fixpoint::Fixpoint::new(kvars, constraint)
    }
}

impl KVarStore {
    pub fn new() -> Self {
        Self {
            kvars: IndexVec::new(),
        }
    }

    pub fn fresh<S>(&mut self, var: Var, sort: Sort, scope: S) -> Pred
    where
        S: IntoIterator<Item = (Var, Sort)>,
    {
        let scope = scope.into_iter();

        let mut sorts = Vec::with_capacity(scope.size_hint().0 + 1);
        let mut args = Vec::with_capacity(scope.size_hint().0);

        sorts.push(sort);
        args.push(Expr::from(var));
        for (var, sort) in scope {
            args.push(Expr::from(var));
            sorts.push(sort);
        }

        let kvid = self.kvars.push(sorts);
        Pred::kvar(kvid, args)
    }
}

impl std::ops::Index<KVid> for KVarStore {
    type Output = Vec<Sort>;

    fn index(&self, index: KVid) -> &Self::Output {
        &self.kvars[index]
    }
}

impl Cursor<'_> {
    pub fn name_gen(&self) -> IndexGen<Name> {
        let gen = IndexGen::new();
        gen.skip(self.node.borrow().nbindings);
        gen
    }

    pub fn breadcrumb<'a>(&'a mut self) -> Cursor<'a> {
        Cursor {
            cx: self.cx,
            node: Rc::clone(&self.node),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            node: Rc::downgrade(&self.node),
        }
    }

    pub fn scope(&self) -> Vec<(Var, Sort)> {
        self.scope_at(&self.snapshot()).unwrap()
    }

    pub fn scope_at(&self, snapshot: &Snapshot) -> Option<Vec<(Var, Sort)>> {
        let parents = ParentsIter::new(snapshot.node.upgrade()?);
        let scope = parents
            .filter_map(|node| {
                if let NodeKind::Binding(name, sort, _) = node.borrow().kind {
                    Some((Var::Free(name), sort))
                } else {
                    None
                }
            })
            .collect();
        Some(scope)
    }

    pub fn push_pred(&mut self, expr: impl Into<Expr>) {
        self.node = self.push_node(NodeKind::Pred(expr.into()));
    }

    pub fn push_binding<F, P>(&mut self, sort: Sort, f: F) -> Name
    where
        F: FnOnce(Name) -> P,
        P: Into<Pred>,
    {
        let fresh = Name::new(self.node.borrow().nbindings);
        let pred = f(fresh).into();
        self.node = self.push_node(NodeKind::Binding(fresh, sort, pred));
        fresh
    }

    pub fn subtyping(&mut self, tcx: TyCtxt, ty1: Ty, ty2: Ty) {
        let mut cursor = self.breadcrumb();

        match (ty1.kind(), ty2.kind()) {
            (TyKind::Refine(bty1, e1), TyKind::Refine(bty2, e2)) if e1 == e2 => {
                cursor.bty_subtyping(tcx, bty1, bty2);
                return;
            }
            (TyKind::Exists(bty1, p1), TyKind::Exists(bty2, p2)) if p1 == p2 => {
                cursor.bty_subtyping(tcx, bty1, bty2);
                return;
            }
            (TyKind::Exists(bty, p), _) => {
                let fresh =
                    cursor.push_binding(bty.sort(), |fresh| p.subst_bound_vars(Var::Free(fresh)));
                let ty1 = TyKind::Refine(bty.clone(), Var::Free(fresh).into()).intern();
                self.subtyping(tcx, ty1, ty2);
                return;
            }
            (TyKind::Ref(..), _) => {
                todo!()
            }
            _ => {}
        }

        match (ty1.kind(), ty2.kind()) {
            (TyKind::Refine(bty1, e1), TyKind::Refine(bty2, e2)) => {
                cursor.bty_subtyping(tcx, bty1, bty2);
                cursor.push_head(ExprKind::BinaryOp(BinOp::Eq, e1.clone(), e2.clone()).intern());
            }
            (TyKind::Refine(bty1, e), TyKind::Exists(bty2, p)) => {
                cursor.bty_subtyping(tcx, bty1, bty2);
                let p = p.subst_bound_vars(e.clone());
                cursor.push_head(p.subst_bound_vars(e.clone()))
            }
            (TyKind::StrgRef(loc1), TyKind::StrgRef(loc2)) => {
                assert_eq!(loc1, loc2);
            }
            (_, TyKind::Uninit) => {
                // FIXME: we should rethink in which situation this is sound.
            }
            (TyKind::Param(param1), TyKind::Param(param2)) => {
                debug_assert_eq!(param1, param2)
            }
            (TyKind::Exists(..), _) => {
                unreachable!("subtyping with unpacked existential")
            }
            _ => {
                unreachable!("unexpected types: `{:?}` `{:?}`", ty1, ty2)
            }
        }
    }

    fn bty_subtyping(&mut self, tcx: TyCtxt, bty1: &BaseTy, bty2: &BaseTy) {
        match (bty1, bty2) {
            (BaseTy::Int(int_ty1), BaseTy::Int(int_ty2)) => {
                debug_assert_eq!(int_ty1, int_ty2);
            }
            (BaseTy::Uint(uint_ty1), BaseTy::Uint(uint_ty2)) => {
                debug_assert_eq!(uint_ty1, uint_ty2);
            }
            (BaseTy::Bool, BaseTy::Bool) => {}
            (BaseTy::Adt(did1, substs1), BaseTy::Adt(did2, substs2)) => {
                debug_assert_eq!(did1, did2);
                debug_assert_eq!(substs1.len(), substs2.len());
                let variances = tcx.variances_of(*did1);
                for (variance, ty1, ty2) in izip!(variances, substs1.iter(), substs2.iter()) {
                    self.polymorphic_subtyping(tcx, *variance, ty1.clone(), ty2.clone());
                }
            }
            _ => unreachable!("unexpected base types: `{:?}` `{:?}`", bty1, bty2),
        }
    }

    fn polymorphic_subtyping(
        &mut self,
        tcx: TyCtxt,
        variance: rustc_middle::ty::Variance,
        ty1: Ty,
        ty2: Ty,
    ) {
        match variance {
            rustc_middle::ty::Variance::Covariant => {
                self.subtyping(tcx, ty1, ty2);
            }
            rustc_middle::ty::Variance::Invariant => {
                self.subtyping(tcx, ty1.clone(), ty2.clone());
                self.subtyping(tcx, ty2, ty1);
            }
            rustc_middle::ty::Variance::Contravariant => {
                self.subtyping(tcx, ty2, ty1);
            }
            rustc_middle::ty::Variance::Bivariant => {}
        }
    }

    pub fn push_loc(&mut self) -> Loc {
        let fresh = Name::new(self.node.borrow().nbindings);
        self.push_node(NodeKind::Loc(fresh));
        Loc::Abstract(fresh)
    }

    pub fn push_head(&mut self, pred: impl Into<Pred>) {
        self.push_node(NodeKind::Head(pred.into()));
    }

    fn push_node(&mut self, kind: NodeKind) -> NodePtr {
        debug_assert!(!matches!(self.node.borrow().kind, NodeKind::Head(_)));
        let node = Node {
            kind,
            nbindings: self.node.borrow().nbindings + self.node.borrow().is_binding() as usize,
            parent: Some(Rc::downgrade(&self.node)),
            children: vec![],
        };
        let node = Rc::new(RefCell::new(node));
        self.node.borrow_mut().children.push(Rc::clone(&node));
        node
    }
}

impl Node {
    fn into_fixpoint(&self, cx: &mut FixpointCtxt) -> Option<fixpoint::Constraint> {
        match &self.kind {
            NodeKind::Conj | NodeKind::Loc(_) => children_to_fixpoint(cx, &self.children),
            NodeKind::Binding(name, sort, pred) => {
                let fresh = cx.fresh_name();
                cx.name_map.insert(*name, fresh);
                let (bindings, pred) = pred_to_fixpoint(cx, pred);
                Some(stitch(
                    bindings,
                    fixpoint::Constraint::ForAll(
                        fresh,
                        *sort,
                        pred,
                        Box::new(children_to_fixpoint(cx, &self.children)?),
                    ),
                ))
            }
            NodeKind::Pred(expr) => Some(fixpoint::Constraint::Guard(
                expr_to_fixpoint(cx, expr),
                Box::new(children_to_fixpoint(cx, &self.children)?),
            )),
            NodeKind::Head(pred) => {
                let (bindings, pred) = pred_to_fixpoint(cx, pred);
                Some(stitch(bindings, fixpoint::Constraint::Pred(pred)))
            }
        }
    }

    /// Returns `true` if the node kind is [`Binding`] or [`Loc`].
    ///
    /// [`Binding`]: NodeKind::Binding
    /// [`Loc`]: NodeKind::Loc
    fn is_binding(&self) -> bool {
        matches!(self.kind, NodeKind::Binding(..) | NodeKind::Loc(_))
    }
}

impl<'a> FixpointCtxt<'a> {
    fn new(kvars: &'a KVarStore) -> Self {
        Self {
            kvars,
            name_gen: IndexGen::new(),
            name_map: FxHashMap::default(),
        }
    }

    fn fresh_name(&self) -> fixpoint::Name {
        self.name_gen.fresh()
    }
}

fn children_to_fixpoint(
    cx: &mut FixpointCtxt,
    children: &[NodePtr],
) -> Option<fixpoint::Constraint> {
    let mut children = children
        .into_iter()
        .filter_map(|node| node.borrow().into_fixpoint(cx))
        .collect_vec();
    match children.len() {
        0 => None,
        1 => children.pop(),
        _ => Some(fixpoint::Constraint::Conj(children)),
    }
}

fn pred_to_fixpoint(
    cx: &mut FixpointCtxt,
    refine: &Pred,
) -> (Vec<(fixpoint::Name, Sort, fixpoint::Expr)>, fixpoint::Pred) {
    let mut bindings = vec![];
    let pred = match refine {
        Pred::Expr(expr) => fixpoint::Pred::Expr(expr_to_fixpoint(cx, expr)),
        Pred::KVar(kvid, args) => {
            let args = args.iter().zip(&cx.kvars[*kvid]).map(|(arg, sort)| {
                if let ExprKind::Var(Var::Free(name)) = arg.kind() {
                    cx.name_map[name]
                } else {
                    let fresh = cx.fresh_name();
                    let pred = fixpoint::Expr::BinaryOp(
                        BinOp::Eq,
                        Box::new(fixpoint::Expr::Var(fresh)),
                        Box::new(expr_to_fixpoint(cx, arg)),
                    );
                    bindings.push((fresh, *sort, pred));
                    fresh
                }
            });
            fixpoint::Pred::KVar(*kvid, args.collect())
        }
    };
    (bindings, pred)
}

fn expr_to_fixpoint(cx: &FixpointCtxt, expr: &ExprS) -> fixpoint::Expr {
    match expr.kind() {
        ExprKind::Var(Var::Free(name)) => fixpoint::Expr::Var(cx.name_map[name]),
        ExprKind::Constant(c) => fixpoint::Expr::Constant(*c),
        ExprKind::BinaryOp(op, e1, e2) => fixpoint::Expr::BinaryOp(
            *op,
            Box::new(expr_to_fixpoint(cx, e1)),
            Box::new(expr_to_fixpoint(cx, e2)),
        ),
        ExprKind::UnaryOp(op, e) => fixpoint::Expr::UnaryOp(*op, Box::new(expr_to_fixpoint(cx, e))),
        ExprKind::Var(Var::Bound) => {
            unreachable!("unexpected bound variable")
        }
    }
}

fn stitch(
    bindings: Vec<(fixpoint::Name, Sort, fixpoint::Expr)>,
    c: fixpoint::Constraint,
) -> fixpoint::Constraint {
    bindings.into_iter().rev().fold(c, |c, (name, sort, e)| {
        fixpoint::Constraint::ForAll(name, sort, fixpoint::Pred::Expr(e), Box::new(c))
    })
}

struct ParentsIter {
    node: Option<NodePtr>,
}

impl ParentsIter {
    fn new(node: NodePtr) -> Self {
        Self { node: Some(node) }
    }
}

impl Iterator for ParentsIter {
    type Item = NodePtr;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node.take() {
            self.node = node.borrow().parent.as_ref().and_then(|n| n.upgrade());
            Some(node)
        } else {
            None
        }
    }
}

mod pretty {
    use std::fmt::{self, Write};

    use itertools::Itertools;
    use liquid_rust_common::format::PadAdapter;
    use rustc_middle::ty::TyCtxt;

    use super::*;
    use crate::pretty::*;

    impl Pretty for PureCtxt {
        fn fmt(&self, cx: &PPrintCx, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            define_scoped!(cx, f);
            w!("{:?}", &self.root)
        }

        fn default_cx(tcx: TyCtxt) -> PPrintCx {
            PPrintCx::default(tcx).kvar_args(Visibility::Truncate(1))
            // PPrintCx::default(tcx).kvar_args(Visibility::Show)
        }
    }

    impl Pretty for NodePtr {
        fn fmt(&self, cx: &PPrintCx, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            define_scoped!(cx, f);
            let node = self.borrow();
            match &node.kind {
                NodeKind::Conj | NodeKind::Loc(_) => {
                    w!("{:?}", &node.children)
                }
                NodeKind::Binding(name, sort, pred) => {
                    let bindings = vec![(*name, *sort, pred)];

                    let vars = bindings.iter().format_with(", ", |(var, sort, _), f| {
                        f(&format_args_cx!("{:?}: {:?}", ^var, ^sort))
                    });

                    let preds = bindings
                        .iter()
                        .map(|(_, _, pred)| pred)
                        .filter(|p| !p.is_true())
                        .collect_vec();

                    let preds_fmt = preds.iter().format_with(" ∧ ", |pred, f| {
                        if pred.is_atom() {
                            f(&format_args_cx!("{:?}", pred))
                        } else {
                            f(&format_args_cx!("({:?})", pred))
                        }
                    });

                    w!("∀ {}.", ^vars)?;
                    if preds.is_empty() {
                        w!("{:?}", &node.children)
                    } else {
                        w!(PadAdapter::wrap_fmt(f), "\n{} ⇒{:?}", ^preds_fmt, &node.children)
                    }
                }
                NodeKind::Pred(expr) => {
                    let expr = if cx.simplify_exprs {
                        expr.simplify()
                    } else {
                        expr.clone()
                    };
                    if expr.is_atom() {
                        w!("{:?} ⇒{:?}", expr, &node.children)
                    } else {
                        w!("({:?}) ⇒{:?}", expr, &node.children)
                    }
                }
                NodeKind::Head(pred) => {
                    if pred.is_atom() {
                        w!("{:?}", pred)
                    } else {
                        w!("({:?})", pred)
                    }
                }
            }
        }
    }

    impl Pretty for Vec<NodePtr> {
        fn fmt(&self, cx: &PPrintCx, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            define_scoped!(cx, PadAdapter::wrap_fmt(f));
            match &self[..] {
                [] => w!(" true"),
                [n] => w!(" {:?}", Rc::clone(n)),
                _ => w!("\n{:?}", join!("\n", self.iter().map(Rc::clone))),
            }
        }
    }

    impl fmt::Debug for Cursor<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{{{}}}",
                self.scope()
                    .into_iter()
                    .rev()
                    .format_with(", ", |(var, sort), f| f(&format_args!(
                        "{:?}: {:?}",
                        var, sort
                    )))
            )
        }
    }

    impl_debug_with_default_cx!(PureCtxt);
}
