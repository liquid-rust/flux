//! Encoding of the refinement tree into a fixpoint constraint.

use std::{hash::Hash, iter, ops::ControlFlow};

use flux_common::{
    bug,
    cache::QueryCache,
    dbg,
    index::{IndexGen, IndexVec},
    span_bug,
};
use flux_config as config;
use flux_fixpoint::FixpointResult;
use flux_middle::{
    fhir::FuncKind,
    global_env::GlobalEnv,
    intern::List,
    queries::QueryResult,
    rty::{
        self,
        fold::{TypeSuperVisitable, TypeVisitable, TypeVisitor},
        AliasPred, Constant, ESpan,
    },
};
use itertools::{self, Itertools};
use rustc_data_structures::{
    fx::FxIndexMap,
    unord::{UnordMap, UnordSet},
};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_index::newtype_index;
use rustc_span::{Span, Symbol};
use rustc_type_ir::DebruijnIndex;

use crate::{refine_tree::Scope, CheckerConfig};

newtype_index! {
    #[debug_format = "TagIdx({})"]
    pub struct TagIdx {}
}

#[derive(Default)]
pub struct KVarStore {
    kvars: IndexVec<rty::KVid, KVarDecl>,
}

#[derive(Clone)]
struct KVarDecl {
    self_args: usize,
    sorts: Vec<rty::Sort>,
    encoding: KVarEncoding,
}

/// How an [`rty::KVar`] is encoded in the fixpoint constraint
#[derive(Clone, Copy)]
pub enum KVarEncoding {
    /// Generate a single kvar appending the self arguments and the scope, i.e.,
    /// a kvar `$k(a0, ...)[b0, ...]` becomes `$k(a0, ..., b0, ...)` in the fixpoint constraint.
    Single,
    /// Generate a conjunction of kvars, one per argument in [rty::KVar::args].
    /// Concretely, a kvar `$k(a0, a1, ..., an)[b0, ...]` becomes
    /// `$k0(a0, a1, ..., an, b0, ...) ∧ $k1(a1, ..., an, b0, ...) ∧ ... ∧ $kn(an, b0, ...)`
    Conj,
}

/// Keep track of all the sorts we need to define in the fixpoint constraint. Currently, we encode
/// every aggregate sort as a tuple in fixpoint.
#[derive(Default)]
struct SortStore {
    /// Set of all the arities
    tuples: UnordSet<usize>,
}

impl SortStore {
    fn declare_tuple(&mut self, arity: usize) {
        self.tuples.insert(arity);
    }

    fn into_data_decls(self) -> Vec<fixpoint::DataDecl> {
        self.tuples
            .into_items()
            .into_sorted_stable_ord()
            .into_iter()
            .map(|arity| {
                fixpoint::DataDecl {
                    name: tuple_sort_name(arity),
                    vars: arity,
                    ctors: vec![fixpoint::DataCtor {
                        name: fixpoint::Var::TupleCtor { arity },
                        fields: (0..(arity as u32))
                            .map(|field| {
                                fixpoint::DataField {
                                    name: fixpoint::Var::TupleProj { arity, field },
                                    sort: fixpoint::Sort::Var(field),
                                }
                            })
                            .collect(),
                    }],
                }
            })
            .collect()
    }
}

pub mod fixpoint {
    use std::fmt;

    use rustc_index::newtype_index;

    newtype_index! {
        pub struct KVid {}
    }

    newtype_index! {
        pub struct LocalVar {}
    }

    newtype_index! {
        pub struct GlobalVar {}
    }

    #[derive(Hash, Debug, Copy, Clone)]
    pub enum Var {
        Global(GlobalVar),
        Local(LocalVar),
        TupleCtor {
            arity: usize,
        },
        TupleProj {
            arity: usize,
            field: u32,
        },
        /// Interpreted theory function. This can be an arbitrary string, thus we are assuming the
        /// name is different than the display implementation for the other variants.
        Itf(Symbol),
    }

    impl From<GlobalVar> for Var {
        fn from(v: GlobalVar) -> Self {
            Self::Global(v)
        }
    }

    impl From<LocalVar> for Var {
        fn from(v: LocalVar) -> Self {
            Self::Local(v)
        }
    }

    impl fmt::Display for KVid {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "k{}", self.as_u32())
        }
    }

    impl fmt::Display for Var {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Var::Global(v) => write!(f, "c{}", v.as_u32()),
                Var::Local(v) => write!(f, "a{}", v.as_u32()),
                Var::TupleCtor { arity } => write!(f, "Tuple{arity}"),
                Var::TupleProj { arity, field } => write!(f, "Tuple{arity}${field}"),
                Var::Itf(name) => write!(f, "{name}"),
            }
        }
    }

    flux_fixpoint::declare_types! {
        type Sort = String;
        type KVar = KVid;
        type Var = Var;
        type Tag = super::TagIdx;
    }
    pub use fixpoint_generated::*;
    use rustc_span::Symbol;
}

type KVidMap = UnordMap<rty::KVid, Vec<fixpoint::KVid>>;
type ConstMap = FxIndexMap<Key, ConstInfo>;

#[derive(Eq, Hash, PartialEq)]
enum Key {
    Uif(rustc_span::Symbol),
    Const(DefId),
}

pub struct FixpointCtxt<'genv, 'tcx, T: Eq + Hash> {
    comments: Vec<String>,
    genv: &'genv GlobalEnv<'genv, 'tcx>,
    kvars: KVarStore,
    sorts: SortStore,
    fixpoint_kvars: IndexVec<fixpoint::KVid, FixpointKVar>,
    env: Env,
    const_map: ConstMap,
    kvid_map: KVidMap,
    tags: IndexVec<TagIdx, T>,
    tags_inv: UnordMap<T, TagIdx>,
    alias_preds: UnordMap<AliasPred, rty::Expr>,
    const_name_gen: IndexGen<fixpoint::GlobalVar>,
    /// [`DefId`] of the item being checked. This could be a function/method or an adt when checking
    /// invariants.
    def_id: LocalDefId,
}

struct FixpointKVar {
    sorts: Vec<fixpoint::Sort>,
    orig: rty::KVid,
}

/// Environment used to map [`rty::Var`] into [`fixpoint::LocalVar`]. This only supports
/// mapping of [`rty::Var::LateBound`] and [`rty::Var::Free`].
struct Env {
    local_var_gen: IndexGen<fixpoint::LocalVar>,
    fvars: UnordMap<rty::Name, fixpoint::LocalVar>,
    /// Layers of late bound variables
    layers: Vec<Vec<fixpoint::LocalVar>>,
}

impl Env {
    fn new() -> Self {
        Self { local_var_gen: IndexGen::new(), fvars: Default::default(), layers: Vec::new() }
    }

    fn fresh_name(&self) -> fixpoint::LocalVar {
        self.local_var_gen.fresh()
    }

    fn insert_fvar_map(&mut self, name: rty::Name) -> fixpoint::LocalVar {
        let fresh = self.fresh_name();
        self.fvars.insert(name, fresh);
        fresh
    }

    fn remove_fvar_map(&mut self, name: rty::Name) {
        self.fvars.remove(&name);
    }

    fn get_fvar(&self, name: rty::Name) -> Option<fixpoint::LocalVar> {
        self.fvars.get(&name).copied()
    }

    fn get_late_bvar(&self, debruijn: DebruijnIndex, idx: u32) -> Option<fixpoint::LocalVar> {
        let depth = self.layers.len().checked_sub(debruijn.as_usize() + 1)?;
        self.layers[depth].get(idx as usize).copied()
    }

    fn push_layer_with_fresh_names(&mut self, count: usize) {
        let layer = (0..count).map(|_| self.fresh_name()).collect();
        self.layers.push(layer);
    }

    fn last_layer(&self) -> &[fixpoint::LocalVar] {
        self.layers.last().unwrap()
    }
}

struct ExprCtxt<'a, 'tcx> {
    genv: &'a GlobalEnv<'a, 'tcx>,
    env: &'a Env,
    const_map: &'a ConstMap,
    /// Used to report bugs
    dbg_span: Span,
}

struct ConstInfo {
    name: fixpoint::GlobalVar,
    sym: rustc_span::Symbol,
    sort: fixpoint::Sort,
    val: Option<Constant>,
}

/// An alias for additional bindings introduced when ANF-ing index expressions
/// in the course of encoding into fixpoint.
pub type Bindings = Vec<(fixpoint::LocalVar, fixpoint::Sort, fixpoint::Expr)>;

pub fn stitch(bindings: Bindings, c: fixpoint::Constraint) -> fixpoint::Constraint {
    bindings.into_iter().rev().fold(c, |c, (name, sort, e)| {
        fixpoint::Constraint::ForAll(
            fixpoint::Var::Local(name),
            sort,
            fixpoint::Pred::Expr(e),
            Box::new(c),
        )
    })
}

/// An alias for a list of predicate (conjuncts) and their spans, used to give
/// localized errors when refine checking fails.
type PredSpans = Vec<(fixpoint::Pred, Option<ESpan>)>;

impl<'genv, 'tcx, Tag> FixpointCtxt<'genv, 'tcx, Tag>
where
    Tag: std::hash::Hash + Eq + Copy,
{
    pub fn new(genv: &'genv GlobalEnv<'genv, 'tcx>, def_id: LocalDefId, kvars: KVarStore) -> Self {
        let (const_map, const_name_gen) = fixpoint_const_map(genv);
        Self {
            comments: vec![],
            kvars,
            sorts: SortStore::default(),
            genv,
            env: Env::new(),
            const_map,
            fixpoint_kvars: IndexVec::new(),
            kvid_map: KVidMap::default(),
            tags: IndexVec::new(),
            tags_inv: Default::default(),
            alias_preds: Default::default(),
            const_name_gen,
            def_id,
        }
    }

    /// Collect all the sorts that need to be defined in fixpoint to encode `t`
    pub(crate) fn collect_sorts<T: TypeVisitable>(&mut self, t: &T) {
        struct Visitor<'a> {
            sorts: &'a mut SortStore,
        }
        impl TypeVisitor for Visitor<'_> {
            fn visit_expr(&mut self, expr: &rty::Expr) -> ControlFlow<!> {
                if let rty::ExprKind::Aggregate(_, flds) = expr.kind() {
                    self.sorts.declare_tuple(flds.len());
                }
                expr.super_visit_with(self)
            }

            fn visit_sort(&mut self, sort: &rty::Sort) -> ControlFlow<!> {
                match sort {
                    rty::Sort::Tuple(flds) => self.sorts.declare_tuple(flds.len()),
                    rty::Sort::Adt(sort_def, _) => self.sorts.declare_tuple(sort_def.fields()),
                    _ => {}
                }
                sort.super_visit_with(self)
            }
        }
        t.visit_with(&mut Visitor { sorts: &mut self.sorts });
    }

    pub(crate) fn with_name_map<R>(
        &mut self,
        name: rty::Name,
        f: impl FnOnce(&mut Self, fixpoint::LocalVar) -> R,
    ) -> R {
        let fresh = self.env.insert_fvar_map(name);
        let r = f(self, fresh);
        self.env.remove_fvar_map(name);
        r
    }

    fn assume_const_val(
        cstr: fixpoint::Constraint,
        var: fixpoint::GlobalVar,
        const_val: Constant,
    ) -> fixpoint::Constraint {
        let e1 = fixpoint::Expr::Var(fixpoint::Var::Global(var));
        let e2 = fixpoint::Expr::Constant(const_val);
        let pred = fixpoint::Pred::Expr(e1.eq(e2));
        fixpoint::Constraint::Guard(pred, Box::new(cstr))
    }

    pub fn check(
        self,
        cache: &mut QueryCache,
        constraint: fixpoint::Constraint,
        config: &CheckerConfig,
    ) -> QueryResult<Vec<Tag>> {
        if !constraint.is_concrete() {
            // skip checking trivial constraints
            return Ok(vec![]);
        }
        let span = self.def_span();

        let kvars = self
            .fixpoint_kvars
            .into_iter_enumerated()
            .map(|(kvid, kvar)| {
                fixpoint::KVar::new(kvid, kvar.sorts, format!("orig: {:?}", kvar.orig))
            })
            .collect_vec();

        let mut constraint = constraint;
        for const_info in self.const_map.values() {
            if let Some(val) = const_info.val {
                constraint = Self::assume_const_val(constraint, const_info.name, val);
            }
        }

        let qualifiers = self
            .genv
            .qualifiers(self.def_id)?
            .map(|qual| qualifier_to_fixpoint(self.genv, span, &self.const_map, qual))
            .collect();

        let constants = self
            .const_map
            .into_values()
            .map(|const_info| {
                fixpoint::ConstInfo {
                    name: fixpoint::Var::Global(const_info.name),
                    orig: const_info.sym.to_string(),
                    sort: const_info.sort,
                }
            })
            .collect();

        let task = fixpoint::Task {
            comments: self.comments,
            constants,
            kvars,
            constraint,
            qualifiers,
            scrape_quals: config.scrape_quals,
            data_decls: self.sorts.into_data_decls(),
        };
        if config::dump_constraint() {
            dbg::dump_item_info(self.genv.tcx, self.def_id, "smt2", &task).unwrap();
        }

        let task_key = self.genv.tcx.def_path_str(self.def_id);

        match task.check_with_cache(task_key, cache) {
            Ok(FixpointResult::Safe(_)) => Ok(vec![]),
            Ok(FixpointResult::Unsafe(_, errors)) => {
                Ok(errors
                    .into_iter()
                    .map(|err| self.tags[err.tag])
                    .unique()
                    .collect_vec())
            }
            Ok(FixpointResult::Crash(err)) => span_bug!(span, "fixpoint crash: {err:?}"),
            Err(err) => span_bug!(span, "failed to run fixpoint: {err:?}"),
        }
    }

    pub fn tag_idx(&mut self, tag: Tag) -> TagIdx
    where
        Tag: std::fmt::Debug,
    {
        *self.tags_inv.entry(tag).or_insert_with(|| {
            let idx = self.tags.push(tag);
            self.comments.push(format!("Tag {idx}: {tag:?}"));
            idx
        })
    }

    pub fn pred_to_fixpoint(&mut self, pred: &rty::Pred) -> (Bindings, PredSpans) {
        match pred {
            rty::Pred::Expr(expr) => {
                let mut bindings = vec![];
                let mut preds = vec![];
                self.pred_to_fixpoint_internal(expr, &mut bindings, &mut preds);
                (bindings, preds)
            }
            rty::Pred::Alias(alias_pred, args) => {
                // 1. collect the alias_pred_app (to later generate name+sort)
                // tcx.def_path_str(def_id) to get a full string from def_id
                let func = self.alias_pred_func(alias_pred, args.len());
                let alias_pred_app = rty::Expr::app(func, args.clone(), None);
                // 2. convert to expr and recurse Expr
                self.pred_to_fixpoint(&rty::Pred::Expr(alias_pred_app))
            }
        }
    }

    fn pred_to_fixpoint_internal(
        &mut self,
        expr: &rty::Expr,
        bindings: &mut Bindings,
        preds: &mut PredSpans,
    ) {
        match expr.kind() {
            rty::ExprKind::BinaryOp(rty::BinOp::And, e1, e2) => {
                self.pred_to_fixpoint_internal(e1, bindings, preds);
                self.pred_to_fixpoint_internal(e2, bindings, preds);
            }
            rty::ExprKind::KVar(kvar) => {
                preds.push((self.kvar_to_fixpoint(kvar, bindings), None));
            }
            _ => {
                let span = expr.span();
                preds.push((fixpoint::Pred::Expr(self.as_expr_cx().expr_to_fixpoint(expr)), span));
            }
        }
    }

    fn kvar_to_fixpoint(&mut self, kvar: &rty::KVar, bindings: &mut Bindings) -> fixpoint::Pred {
        self.populate_kvid_map(kvar.kvid);

        let decl = self.kvars.get(kvar.kvid);

        let all_args = iter::zip(&kvar.args, &decl.sorts)
            .map(|(arg, sort)| fixpoint::Var::Local(self.imm(arg, sort, bindings)))
            .collect_vec();

        let kvids = &self.kvid_map[&kvar.kvid];

        if all_args.is_empty() {
            let fresh = self.env.fresh_name();
            let var = fixpoint::Var::Local(fresh);
            bindings.push((
                fresh,
                fixpoint::Sort::Unit,
                fixpoint::Expr::eq(fixpoint::Expr::Var(var), fixpoint::Expr::Unit),
            ));
            return fixpoint::Pred::KVar(kvids[0], vec![var]);
        }

        let kvars = kvids
            .iter()
            .enumerate()
            .map(|(i, kvid)| {
                let args = all_args.iter().skip(kvids.len() - i - 1).copied().collect();
                fixpoint::Pred::KVar(*kvid, args)
            })
            .collect_vec();

        fixpoint::Pred::And(kvars)
    }

    fn populate_kvid_map(&mut self, kvid: rty::KVid) {
        self.kvid_map.entry(kvid).or_insert_with(|| {
            let decl = self.kvars.get(kvid);

            let all_args = decl.sorts.iter().map(sort_to_fixpoint).collect_vec();

            if all_args.is_empty() {
                let sorts = vec![fixpoint::Sort::Unit];
                let kvid = self.fixpoint_kvars.push(FixpointKVar::new(sorts, kvid));
                return vec![kvid];
            }

            match decl.encoding {
                KVarEncoding::Single => {
                    let kvid = self.fixpoint_kvars.push(FixpointKVar::new(all_args, kvid));
                    vec![kvid]
                }
                KVarEncoding::Conj => {
                    let n = usize::max(decl.self_args, 1);
                    (0..n)
                        .map(|i| {
                            let sorts = all_args.iter().skip(n - i - 1).cloned().collect();
                            self.fixpoint_kvars.push(FixpointKVar::new(sorts, kvid))
                        })
                        .collect_vec()
                }
            }
        });
    }

    fn imm(
        &self,
        arg: &rty::Expr,
        sort: &rty::Sort,
        bindings: &mut Vec<(fixpoint::LocalVar, fixpoint::Sort, fixpoint::Expr)>,
    ) -> fixpoint::LocalVar {
        match arg.kind() {
            rty::ExprKind::Var(rty::Var::Free(name)) => {
                self.env.get_fvar(*name).unwrap_or_else(|| {
                    span_bug!(self.def_span(), "no entry found for key: `{name:?}`")
                })
            }
            rty::ExprKind::Var(_) => {
                span_bug!(self.def_span(), "unexpected variable")
            }
            _ => {
                let fresh = self.env.fresh_name();
                let pred = fixpoint::Expr::eq(
                    fixpoint::Expr::Var(fresh.into()),
                    self.as_expr_cx().expr_to_fixpoint(arg),
                );
                bindings.push((fresh, sort_to_fixpoint(sort), pred));
                fresh
            }
        }
    }

    fn as_expr_cx(&self) -> ExprCtxt<'_, 'tcx> {
        ExprCtxt::new(self.genv, &self.env, &self.const_map, self.def_span())
    }

    fn def_span(&self) -> Span {
        self.genv.tcx.def_span(self.def_id)
    }

    fn alias_pred_sort(arity: usize) -> rty::PolyFuncSort {
        let mut sorts = vec![];
        for i in 0..arity {
            sorts.push(rty::Sort::Var(rty::SortVar::from(i)));
        }
        sorts.push(rty::Sort::Bool);
        rty::PolyFuncSort::new(arity, rty::FuncSort { inputs_and_output: List::from_vec(sorts) })
    }

    fn fresh_alias_pred(&mut self, arity: usize) -> Symbol {
        let n = self.alias_preds.len();
        let sym = Symbol::intern(&format!("alias_pred_{n:?}"));
        let name = self.const_name_gen.fresh();
        let fsort = Self::alias_pred_sort(arity);
        let sort = func_sort_to_fixpoint(&fsort);
        let sort = fixpoint::Sort::Func(sort);
        let cinfo = ConstInfo { name, sym, sort, val: None };
        self.const_map.insert(Key::Uif(sym), cinfo);
        sym
    }

    // returns the 'constant' UIF used to represent the alias_pred, creating and adding it to the
    // const_map if necessary
    fn alias_pred_func(&mut self, alias_pred: &AliasPred, arity: usize) -> rty::Expr {
        match self.alias_preds.get(alias_pred) {
            Some(func) => func.clone(),
            None => {
                let sym = self.fresh_alias_pred(arity);
                let func = rty::Expr::global_func(sym, FuncKind::Asp);
                self.alias_preds.insert(alias_pred.clone(), func.clone());
                func
            }
        }
    }
}

impl FixpointKVar {
    fn new(sorts: Vec<fixpoint::Sort>, orig: rty::KVid) -> Self {
        Self { sorts, orig }
    }
}

fn fixpoint_const_map(genv: &GlobalEnv) -> (ConstMap, IndexGen<fixpoint::GlobalVar>) {
    let const_name_gen = IndexGen::new();
    let consts = genv
        .map()
        .consts()
        .sorted_by(|a, b| Ord::cmp(&a.sym, &b.sym))
        .map(|const_info| {
            let name = const_name_gen.fresh();
            let cinfo = ConstInfo {
                name,
                sym: const_info.sym,
                sort: fixpoint::Sort::Int,
                val: Some(const_info.val),
            };
            (Key::Const(const_info.def_id), cinfo)
        });
    let uifs = genv
        .func_decls()
        .sorted_by(|a, b| Ord::cmp(&a.name, &b.name))
        .filter_map(|decl| {
            match decl.kind {
                FuncKind::Uif => {
                    let name = const_name_gen.fresh();
                    let sort = func_sort_to_fixpoint(&decl.sort);
                    let cinfo = ConstInfo {
                        name,
                        sym: decl.name,
                        sort: fixpoint::Sort::Func(sort),
                        val: None,
                    };
                    Some((Key::Uif(cinfo.sym), cinfo))
                }
                _ => None,
            }
        });
    (itertools::chain(consts, uifs).collect(), const_name_gen)
}

impl KVarStore {
    pub fn new() -> Self {
        Self { kvars: IndexVec::new() }
    }

    fn get(&self, kvid: rty::KVid) -> &KVarDecl {
        &self.kvars[kvid]
    }

    /// Generate a fresh [kvar] under several layers of [binders]. The variables bound in the last
    /// layer (last element of the `binders` slice) are used as the self arguments. The rest of the
    /// binders are appended to the `scope`.
    ///
    /// Note that the returned expression will have escaping variables and it is up to the caller to
    /// put it under an appropriate number of binders.
    ///
    /// [binders]: rty::Binder
    /// [kvar]: rty::KVar
    pub fn fresh(
        &mut self,
        binders: &[List<rty::Sort>],
        scope: &Scope,
        encoding: KVarEncoding,
    ) -> rty::Expr {
        if binders.is_empty() {
            return self.fresh_inner(0, [], encoding);
        }
        let args = itertools::chain(
            binders.iter().rev().enumerate().flat_map(|(level, sorts)| {
                let debruijn = DebruijnIndex::from_usize(level);
                sorts
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(move |(idx, sort)| (rty::Var::LateBound(debruijn, idx as u32), sort))
            }),
            scope
                .iter()
                .map(|(name, sort)| (rty::Var::Free(name), sort)),
        );
        self.fresh_inner(binders.last().unwrap().len(), args, encoding)
    }

    fn fresh_inner<A>(&mut self, self_args: usize, args: A, encoding: KVarEncoding) -> rty::Expr
    where
        A: IntoIterator<Item = (rty::Var, rty::Sort)>,
    {
        let mut sorts = vec![];
        let mut exprs = vec![];

        let mut flattened_self_args = 0;
        for (i, (var, sort)) in args.into_iter().enumerate() {
            let is_self_arg = i < self_args;
            let var = var.to_expr();
            sort.walk(|sort, proj| {
                if !matches!(sort, rty::Sort::Loc | rty::Sort::Func(..)) {
                    flattened_self_args += is_self_arg as usize;
                    sorts.push(sort.clone());
                    exprs.push(rty::Expr::field_projs(&var, proj));
                }
            });
        }

        let kvid = self
            .kvars
            .push(KVarDecl { self_args: flattened_self_args, sorts, encoding });

        let kvar = rty::KVar::new(kvid, flattened_self_args, exprs);
        rty::Expr::kvar(kvar)
    }
}

impl std::fmt::Display for TagIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u32())
    }
}

impl std::str::FromStr for TagIdx {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_u32(s.parse()?))
    }
}

pub fn sort_to_fixpoint(sort: &rty::Sort) -> fixpoint::Sort {
    match sort {
        rty::Sort::Int => fixpoint::Sort::Int,
        rty::Sort::Real => fixpoint::Sort::Real,
        rty::Sort::Bool => fixpoint::Sort::Bool,
        rty::Sort::BitVec(w) => fixpoint::Sort::BitVec(*w),
        // There's no way to declare user defined sorts in the fixpoint horn syntax so we encode
        // user declared opaque sorts and type variable sorts as integers. Well-formedness should
        // ensure values of these sorts are properly used.
        rty::Sort::App(rty::SortCtor::User { .. }, _) | rty::Sort::Param(_) => fixpoint::Sort::Int,
        rty::Sort::App(ctor, sorts) => {
            let ctor = match ctor {
                rty::SortCtor::Set => fixpoint::SortCtor::Set,
                rty::SortCtor::Map => fixpoint::SortCtor::Map,
                rty::SortCtor::User { .. } => unreachable!(),
            };
            let sorts = sorts.iter().map(sort_to_fixpoint).collect_vec();
            fixpoint::Sort::App(ctor, sorts)
        }
        rty::Sort::Adt(_, _) => todo!(),
        rty::Sort::Tuple(sorts) => {
            let ctor = fixpoint::SortCtor::Data(tuple_sort_name(sorts.len()));
            let args = sorts.iter().map(sort_to_fixpoint).collect();
            fixpoint::Sort::App(ctor, args)
        }
        rty::Sort::Func(sort) => fixpoint::Sort::Func(func_sort_to_fixpoint(sort)),
        rty::Sort::Var(k) => fixpoint::Sort::Var(k.index.try_into().unwrap()),
        rty::Sort::Err | rty::Sort::Infer(_) | rty::Sort::Loc => {
            bug!("unexpected sort {sort:?}")
        }
    }
}

fn tuple_sort_name(arity: usize) -> String {
    format!("Tuple{arity}")
}

fn func_sort_to_fixpoint(fsort: &rty::PolyFuncSort) -> fixpoint::PolyFuncSort {
    let params = fsort.params();
    let fsort = fsort.skip_binders();
    fixpoint::PolyFuncSort::new(
        params,
        fsort.inputs().iter().map(sort_to_fixpoint).collect(),
        sort_to_fixpoint(fsort.output()),
    )
}

impl<'a, 'tcx> ExprCtxt<'a, 'tcx> {
    fn new(
        genv: &'a GlobalEnv<'a, 'tcx>,
        env: &'a Env,
        const_map: &'a ConstMap,
        dbg_span: Span,
    ) -> Self {
        Self { genv, env, const_map, dbg_span }
    }

    fn expr_to_fixpoint(&self, expr: &rty::Expr) -> fixpoint::Expr {
        match expr.kind() {
            rty::ExprKind::Var(var) => fixpoint::Expr::Var(self.var_to_fixpoint(var).into()),
            rty::ExprKind::Constant(c) => fixpoint::Expr::Constant(*c),
            rty::ExprKind::BinaryOp(op, e1, e2) => {
                fixpoint::Expr::BinaryOp(
                    *op,
                    Box::new([self.expr_to_fixpoint(e1), self.expr_to_fixpoint(e2)]),
                )
            }
            rty::ExprKind::UnaryOp(op, e) => {
                fixpoint::Expr::UnaryOp(*op, Box::new(self.expr_to_fixpoint(e)))
            }
            rty::ExprKind::FieldProj(e, proj) => {
                let (arity, field) = match *proj {
                    rty::FieldProj::Tuple { arity, field } => (arity, field),
                    rty::FieldProj::Adt { def_id, field } => {
                        let arity = self.genv.adt_sort_def_of(def_id).fields();
                        (arity, field)
                    }
                };
                let proj = fixpoint::Var::TupleProj { arity, field };
                fixpoint::Expr::App(proj, vec![self.expr_to_fixpoint(e)])
            }
            rty::ExprKind::Aggregate(_, flds) => {
                let ctor = fixpoint::Var::TupleCtor { arity: flds.len() };
                let args = flds.iter().map(|e| self.expr_to_fixpoint(e)).collect();
                fixpoint::Expr::App(ctor, args)
            }
            rty::ExprKind::ConstDefId(did) => {
                let const_info = self.const_map.get(&Key::Const(*did)).unwrap_or_else(|| {
                    span_bug!(self.dbg_span, "no entry found in const_map for def_id: `{did:?}`")
                });
                fixpoint::Expr::Var(const_info.name.into())
            }
            rty::ExprKind::App(func, args) => {
                let func = self.func_to_fixpoint(func);
                let args = self.exprs_to_fixpoint(args);
                fixpoint::Expr::App(func, args)
            }
            rty::ExprKind::IfThenElse(p, e1, e2) => {
                fixpoint::Expr::IfThenElse(Box::new([
                    self.expr_to_fixpoint(p),
                    self.expr_to_fixpoint(e1),
                    self.expr_to_fixpoint(e2),
                ]))
            }
            rty::ExprKind::Hole(..)
            | rty::ExprKind::KVar(_)
            | rty::ExprKind::Local(_)
            | rty::ExprKind::Abs(_)
            | rty::ExprKind::GlobalFunc(..)
            | rty::ExprKind::PathProj(..) => {
                span_bug!(self.dbg_span, "unexpected expr: `{expr:?}`")
            }
        }
    }

    fn var_to_fixpoint(&self, var: &rty::Var) -> fixpoint::LocalVar {
        match var {
            rty::Var::Free(name) => {
                self.env.get_fvar(*name).unwrap_or_else(|| {
                    span_bug!(self.dbg_span, "no entry found for name: `{name:?}`")
                })
            }
            rty::Var::LateBound(debruijn, idx) => {
                self.env.get_late_bvar(*debruijn, *idx).unwrap_or_else(|| {
                    span_bug!(self.dbg_span, "no entry found for late bound var: `{var:?}`")
                })
            }
            rty::Var::EarlyBound(_) | rty::Var::EVar(_) => {
                span_bug!(self.dbg_span, "unexpected var: `{var:?}`")
            }
        }
    }

    fn exprs_to_fixpoint<'b>(
        &self,
        exprs: impl IntoIterator<Item = &'b rty::Expr>,
    ) -> Vec<fixpoint::Expr> {
        exprs
            .into_iter()
            .map(|e| self.expr_to_fixpoint(e))
            .collect()
    }

    fn func_to_fixpoint(&self, func: &rty::Expr) -> fixpoint::Var {
        match func.kind() {
            rty::ExprKind::Var(var) => self.var_to_fixpoint(var).into(),
            rty::ExprKind::GlobalFunc(_, FuncKind::Thy(sym)) => fixpoint::Var::Itf(*sym),
            rty::ExprKind::GlobalFunc(sym, FuncKind::Uif) => {
                let cinfo = self.const_map.get(&Key::Uif(*sym)).unwrap_or_else(|| {
                    span_bug!(
                        self.dbg_span,
                        "no constant found for uninterpreted function `{sym}` in `const_map`"
                    )
                });
                cinfo.name.into()
            }
            rty::ExprKind::GlobalFunc(sym, FuncKind::Def) => {
                span_bug!(self.dbg_span, "unexpected global function `{sym}`. Function must be normalized away at this point")
            }
            _ => {
                span_bug!(self.dbg_span, "unexpected expr `{func:?}` in function position")
            }
        }
    }
}

fn qualifier_to_fixpoint(
    genv: &GlobalEnv,
    dbg_span: Span,
    const_map: &ConstMap,
    qualifier: &rty::Qualifier,
) -> fixpoint::Qualifier {
    let mut env = Env::new();
    env.push_layer_with_fresh_names(qualifier.body.vars().len());

    let args: Vec<(fixpoint::Var, fixpoint::Sort)> =
        iter::zip(env.last_layer(), qualifier.body.vars())
            .map(|(name, var)| ((*name).into(), sort_to_fixpoint(var.expect_sort())))
            .collect();

    let cx = ExprCtxt::new(genv, &env, const_map, dbg_span);
    let body = cx.expr_to_fixpoint(qualifier.body.as_ref().skip_binder());

    let name = qualifier.name.to_string();
    let global = qualifier.global;

    fixpoint::Qualifier { name, args, body, global }
}
