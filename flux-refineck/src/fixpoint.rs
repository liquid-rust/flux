use std::{fs, io::Write, iter};

use fixpoint::FixpointResult;
use flux_common::{
    cache::QueryCache,
    config::CONFIG,
    index::{IndexGen, IndexVec},
};
use flux_fixpoint as fixpoint;
use flux_middle::{
    global_env::GlobalEnv,
    rty::{self, Binders, BoundVar},
};
use itertools::Itertools;
use rustc_hash::FxHashMap;
use rustc_hir::def_id::DefId;
use rustc_index::newtype_index;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;

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
    args: Vec<rty::Sort>,
    scope: Vec<rty::Sort>,
    encoding: KVarEncoding,
}

/// How an [rty::KVar] is encoded in the fixpoint constraint
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

pub trait KVarGen {
    fn fresh(&mut self, sorts: &[rty::Sort], kind: KVarEncoding) -> Binders<rty::Expr>;
}

type NameMap = FxHashMap<rty::Name, fixpoint::Name>;
type KVidMap = FxHashMap<rty::KVid, Vec<fixpoint::KVid>>;
type ConstMap = FxHashMap<DefId, ConstInfo>;

pub struct FixpointCtxt<'genv, 'tcx, T> {
    genv: &'genv GlobalEnv<'genv, 'tcx>,
    kvars: KVarStore,
    fixpoint_kvars: IndexVec<fixpoint::KVid, Vec<fixpoint::Sort>>,
    kvid_map: KVidMap,
    name_gen: IndexGen<fixpoint::Name>,
    name_map: NameMap,
    const_map: ConstMap,
    tags: IndexVec<TagIdx, T>,
    tags_inv: FxHashMap<T, TagIdx>,
}

#[derive(Debug)]
struct ConstInfo {
    name: fixpoint::Name,
    sym: Symbol,
    val: i128,
}

impl<'genv, 'tcx, Tag> FixpointCtxt<'genv, 'tcx, Tag>
where
    Tag: std::hash::Hash + Eq + Copy,
{
    pub fn new(genv: &'genv GlobalEnv<'genv, 'tcx>, kvars: KVarStore) -> Self {
        let name_gen = IndexGen::new();
        let const_map = fixpoint_const_map(genv, &name_gen);
        Self {
            kvars,
            genv,
            name_gen,
            fixpoint_kvars: IndexVec::new(),
            kvid_map: KVidMap::default(),
            name_map: NameMap::default(),
            const_map,
            tags: IndexVec::new(),
            tags_inv: FxHashMap::default(),
        }
    }

    pub fn with_name_map<R>(
        &mut self,
        name: rty::Name,
        to: fixpoint::Name,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.name_map.insert(name, to);
        let r = f(self);
        self.name_map.remove(&name);
        r
    }

    pub fn fresh_name(&self) -> fixpoint::Name {
        self.name_gen.fresh()
    }

    fn assume_const_val(
        cstr: fixpoint::Constraint<TagIdx>,
        const_info: &ConstInfo,
    ) -> fixpoint::Constraint<TagIdx> {
        let name = const_info.name;
        let e1 = fixpoint::Expr::from(name);
        let e2 = fixpoint::Expr::from(const_info.val);
        let pred = fixpoint::Pred::Expr(e1.eq(e2));
        fixpoint::Constraint::Guard(pred, Box::new(cstr))
    }

    pub fn check(
        self,
        cache: &mut QueryCache,
        did: DefId,
        constraint: fixpoint::Constraint<TagIdx>,
    ) -> Result<(), Vec<Tag>> {
        if !constraint.is_concrete() {
            // skip checking trivial constraints
            return Ok(());
        }

        let kvars = self
            .fixpoint_kvars
            .into_iter_enumerated()
            .map(|(kvid, sorts)| fixpoint::KVar(kvid, sorts))
            .collect_vec();

        let ordered_consts = self
            .const_map
            .values()
            .sorted_by(|a, b| Ord::cmp(&a.sym, &b.sym))
            .collect_vec();

        let mut closed_constraint = constraint;
        for const_info in ordered_consts.iter() {
            closed_constraint = Self::assume_const_val(closed_constraint, const_info);
        }

        let qualifiers = self
            .genv
            .qualifiers(did)
            .map(|qual| qualifier_to_fixpoint(&self.const_map, qual))
            .collect();

        let constants = ordered_consts
            .iter()
            .map(|const_info| (const_info.name, fixpoint::Sort::Int))
            .collect();

        let uifs = self.genv.uifs().map(uif_def_to_fixpoint).collect_vec();

        let sorts = self
            .genv
            .map()
            .sort_decls()
            .map(|sort_decl| sort_decl.name.to_string())
            .collect_vec();

        let task =
            fixpoint::Task::new(constants, kvars, closed_constraint, qualifiers, uifs, sorts);
        if CONFIG.dump_constraint {
            dump_constraint(self.genv.tcx, did, &task, ".smt2").unwrap();
        }

        let task_key = self.genv.tcx.def_path_str(did);

        match task.check_with_cache(task_key, cache) {
            Ok(FixpointResult::Safe(_)) => Ok(()),
            Ok(FixpointResult::Unsafe(_, errors)) => {
                Err(errors
                    .into_iter()
                    .map(|err| self.tags[err.tag])
                    .unique()
                    .collect_vec())
            }
            Ok(FixpointResult::Crash(err)) => panic!("fixpoint crash: {err:?}"),
            Err(err) => panic!("failed to run fixpoint: {err:?}"),
        }
    }

    pub fn tag_idx(&mut self, tag: Tag) -> TagIdx {
        *self
            .tags_inv
            .entry(tag)
            .or_insert_with(|| self.tags.push(tag))
    }

    pub fn pred_to_fixpoint(
        &mut self,
        pred: &rty::Expr,
    ) -> (Vec<(fixpoint::Name, fixpoint::Sort, fixpoint::Expr)>, fixpoint::Pred) {
        let mut bindings = vec![];
        let mut preds = vec![];
        self.pred_to_fixpoint_internal(pred, &mut bindings, &mut preds);
        (bindings, fixpoint::Pred::And(preds))
    }

    fn pred_to_fixpoint_internal(
        &mut self,
        expr: &rty::Expr,
        bindings: &mut Vec<(fixpoint::Name, fixpoint::Sort, fixpoint::Expr)>,
        preds: &mut Vec<fixpoint::Pred>,
    ) {
        match expr.kind() {
            rty::ExprKind::BinaryOp(rty::BinOp::And, e1, e2) => {
                self.pred_to_fixpoint_internal(e1, bindings, preds);
                self.pred_to_fixpoint_internal(e2, bindings, preds);
            }
            rty::ExprKind::KVar(kvar) => {
                preds.push(self.kvar_to_fixpoint(kvar, bindings));
            }
            _ => {
                preds.push(fixpoint::Pred::Expr(expr_to_fixpoint(
                    expr,
                    &self.name_map,
                    &self.const_map,
                )));
            }
        }
    }

    fn kvar_to_fixpoint(
        &mut self,
        kvar: &rty::KVar,
        bindings: &mut Vec<(fixpoint::Name, fixpoint::Sort, fixpoint::Expr)>,
    ) -> fixpoint::Pred {
        self.populate_kvid_map(kvar.kvid);

        let decl = self.kvars.get(kvar.kvid);

        let all_args = iter::zip(kvar.all_args(), decl.all_args())
            .map(|(arg, sort)| self.imm(arg, sort, bindings))
            .collect_vec();

        let kvids = &self.kvid_map[&kvar.kvid];

        if all_args.is_empty() {
            let fresh = self.fresh_name();
            bindings.push((
                fresh,
                fixpoint::Sort::Unit,
                fixpoint::Expr::eq(fixpoint::Expr::Var(fresh), fixpoint::Expr::Unit),
            ));
            return fixpoint::Pred::KVar(kvids[0], vec![fresh]);
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

            let all_args = decl.all_args().map(sort_to_fixpoint).collect_vec();

            if all_args.is_empty() {
                let sorts = vec![fixpoint::Sort::Unit];
                let kvid = self.fixpoint_kvars.push(sorts);
                return vec![kvid];
            }

            match decl.encoding {
                KVarEncoding::Single => {
                    let kvid = self.fixpoint_kvars.push(all_args);
                    vec![kvid]
                }
                KVarEncoding::Conj => {
                    let n = usize::max(decl.args.len(), 1);
                    (0..n)
                        .map(|i| {
                            let sorts = all_args.iter().skip(n - i - 1).cloned().collect();
                            self.fixpoint_kvars.push(sorts)
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
        bindings: &mut Vec<(fixpoint::Name, fixpoint::Sort, fixpoint::Expr)>,
    ) -> fixpoint::Name {
        match arg.kind() {
            rty::ExprKind::FreeVar(name) => {
                *self
                    .name_map
                    .get(name)
                    .unwrap_or_else(|| panic!("no entry found for key: `{name:?}`"))
            }
            rty::ExprKind::BoundVar(_) => panic!("unexpected free bound variable"),
            _ => {
                let fresh = self.fresh_name();
                let pred = fixpoint::Expr::eq(
                    fixpoint::Expr::Var(fresh),
                    expr_to_fixpoint(arg, &self.name_map, &self.const_map),
                );
                bindings.push((fresh, sort_to_fixpoint(sort), pred));
                fresh
            }
        }
    }
}

impl<F> KVarGen for F
where
    F: FnMut(&[rty::Sort], KVarEncoding) -> Binders<rty::Expr>,
{
    fn fresh(&mut self, sorts: &[rty::Sort], kind: KVarEncoding) -> Binders<rty::Expr> {
        (self)(sorts, kind)
    }
}

impl<'a> KVarGen for &mut (dyn KVarGen + 'a) {
    fn fresh(&mut self, sorts: &[rty::Sort], kind: KVarEncoding) -> Binders<rty::Expr> {
        (**self).fresh(sorts, kind)
    }
}

impl<'a> KVarGen for Box<dyn KVarGen + 'a> {
    fn fresh(&mut self, sorts: &[rty::Sort], kind: KVarEncoding) -> Binders<rty::Expr> {
        (**self).fresh(sorts, kind)
    }
}

fn fixpoint_const_map(
    genv: &GlobalEnv,
    name_gen: &IndexGen<fixpoint::Name>,
) -> FxHashMap<DefId, ConstInfo> {
    genv.map()
        .consts()
        .sorted_by(|a, b| Ord::cmp(&a.sym, &b.sym))
        .map(|const_info| {
            let name = name_gen.fresh();
            let cinfo = ConstInfo { name, sym: const_info.sym, val: const_info.val };
            (const_info.def_id, cinfo)
        })
        .collect()
}

impl KVarDecl {
    fn all_args(&self) -> impl Iterator<Item = &rty::Sort> {
        self.args.iter().chain(&self.scope)
    }
}

impl KVarStore {
    pub fn new() -> Self {
        Self { kvars: IndexVec::new() }
    }

    fn get(&self, kvid: rty::KVid) -> &KVarDecl {
        &self.kvars[kvid]
    }

    pub fn fresh<S>(
        &mut self,
        sorts: &[rty::Sort],
        scope: S,
        encoding: KVarEncoding,
    ) -> Binders<rty::Expr>
    where
        S: IntoIterator<Item = (rty::Name, rty::Sort)>,
    {
        let mut scope_sorts = vec![];
        let mut scope_exprs = vec![];
        for (name, sort) in scope {
            if !sort.is_loc() {
                scope_sorts.push(sort);
                scope_exprs.push(rty::Expr::fvar(name));
            }
        }

        let args = (0..sorts.len())
            .map(|idx| rty::Expr::bvar(BoundVar::innermost(idx)))
            .collect_vec();

        let kvid = self.kvars.push(KVarDecl {
            args: sorts.to_vec(),
            scope: scope_sorts.clone(),
            encoding,
        });

        let kvar = rty::KVar::new(kvid, args, scope_exprs.clone());
        Binders::new(rty::Expr::kvar(kvar), sorts)
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
        rty::Sort::Tuple(sorts) => {
            match &sorts[..] {
                [] => fixpoint::Sort::Unit,
                [_] => unreachable!("1-tuple"),
                [sorts @ .., s1, s2] => {
                    let s1 = Box::new(sort_to_fixpoint(s1));
                    let s2 = Box::new(sort_to_fixpoint(s2));
                    sorts
                        .iter()
                        .map(sort_to_fixpoint)
                        .map(Box::new)
                        .fold(fixpoint::Sort::Pair(s1, s2), |s1, s2| {
                            fixpoint::Sort::Pair(Box::new(s1), s2)
                        })
                }
            }
        }
        // There's no way to declare opaque sorts in the horn syntax in fixpoint so we encode
        // them as integers. Well-formedness should ensure values of this sort are only used to
        // test for equality.
        rty::Sort::User(_) => fixpoint::Sort::Int,
        rty::Sort::Func(sort) => fixpoint::Sort::Func(func_sort_to_fixpoint(sort)),
        rty::Sort::Infer | rty::Sort::Adt(_) | rty::Sort::Loc => {
            unreachable!("unexpected sort {sort:?}")
        }
    }
}

fn func_sort_to_fixpoint(sort: &rty::FuncSort) -> fixpoint::FuncSort {
    fixpoint::FuncSort {
        inputs_and_output: sort
            .inputs_and_output
            .iter()
            .map(sort_to_fixpoint)
            .collect(),
    }
}

/// TODO(nilehmann) we should abstract over the dumping files logic
fn dump_constraint<C: std::fmt::Debug>(
    tcx: TyCtxt,
    def_id: DefId,
    c: &C,
    suffix: &str,
) -> Result<(), std::io::Error> {
    let dir = CONFIG.log_dir.join("horn");
    fs::create_dir_all(&dir)?;
    let mut file = fs::File::create(dir.join(format!("{}{suffix}", tcx.def_path_str(def_id))))?;
    write!(file, "{c:?}")
}

fn uif_def_to_fixpoint(uif_def: &rty::UifDef) -> fixpoint::UifDef {
    let sort = func_sort_to_fixpoint(&uif_def.sort);
    fixpoint::UifDef::new(uif_def.name.to_string(), sort)
}

fn qualifier_to_fixpoint(const_map: &ConstMap, qualifier: &rty::Qualifier) -> fixpoint::Qualifier {
    let (args, body) = qualifier.with_fresh_fvars();
    let name_gen = IndexGen::skipping(const_map.len());
    let mut name_map = NameMap::default();
    let args = args
        .into_iter()
        .map(|(name, sort)| {
            let fresh = name_gen.fresh();
            name_map.insert(name, fresh);
            (fresh, sort_to_fixpoint(&sort))
        })
        .collect_vec();
    let name = qualifier.name.clone();
    let body = expr_to_fixpoint(&body, &name_map, const_map);
    let global = qualifier.global;
    fixpoint::Qualifier { body, args, name, global }
}

fn expr_to_fixpoint(expr: &rty::Expr, name_map: &NameMap, const_map: &ConstMap) -> fixpoint::Expr {
    match expr.kind() {
        rty::ExprKind::FreeVar(name) => {
            let name = name_map
                .get(name)
                .unwrap_or_else(|| panic!("no entry found for key: `{name:?}`"));
            fixpoint::Expr::Var(*name)
        }
        rty::ExprKind::Constant(c) => fixpoint::Expr::Constant(*c),
        rty::ExprKind::BinaryOp(op, e1, e2) => {
            fixpoint::Expr::BinaryOp(
                *op,
                Box::new([
                    expr_to_fixpoint(e1, name_map, const_map),
                    expr_to_fixpoint(e2, name_map, const_map),
                ]),
            )
        }
        rty::ExprKind::UnaryOp(op, e) => {
            fixpoint::Expr::UnaryOp(*op, Box::new(expr_to_fixpoint(e, name_map, const_map)))
        }
        rty::ExprKind::TupleProj(e, field) => {
            itertools::repeat_n(fixpoint::Proj::Snd, *field as usize)
                .chain([fixpoint::Proj::Fst])
                .fold(expr_to_fixpoint(e, name_map, const_map), |e, proj| {
                    fixpoint::Expr::Proj(Box::new(e), proj)
                })
        }
        rty::ExprKind::Tuple(exprs) => tuple_to_fixpoint(exprs, name_map, const_map),
        rty::ExprKind::ConstDefId(did) => fixpoint::Expr::Var(const_map[did].name),
        rty::ExprKind::App(func, args) => {
            let func = func_to_fixpoint(func, name_map);
            let args = exprs_to_fixpoint(args, name_map, const_map);
            fixpoint::Expr::App(func, args)
        }
        rty::ExprKind::IfThenElse(p, e1, e2) => {
            fixpoint::Expr::IfThenElse(Box::new([
                expr_to_fixpoint(p, name_map, const_map),
                expr_to_fixpoint(e1, name_map, const_map),
                expr_to_fixpoint(e2, name_map, const_map),
            ]))
        }
        rty::ExprKind::EVar(_)
        | rty::ExprKind::Hole
        | rty::ExprKind::KVar(_)
        | rty::ExprKind::Local(_)
        | rty::ExprKind::BoundVar(_)
        | rty::ExprKind::PathProj(..) => {
            panic!("unexpected expr: `{expr:?}`")
        }
    }
}

fn func_to_fixpoint(func: &rty::Func, name_map: &NameMap) -> fixpoint::Func {
    match func {
        rty::Func::Var(rty::Var::Free(name)) => {
            let name = name_map
                .get(name)
                .unwrap_or_else(|| panic!("no entry found for key: `{name:?}`"));
            fixpoint::Func::Var(*name)
        }
        rty::Func::Uif(func) => fixpoint::Func::Uif(func.to_string()),
        rty::Func::Var(var) => {
            panic!("unexpected var `{var:?}` in function application")
        }
    }
}

fn exprs_to_fixpoint<'a>(
    exprs: impl IntoIterator<Item = &'a rty::Expr>,
    name_map: &NameMap,
    const_map: &ConstMap,
) -> Vec<fixpoint::Expr> {
    exprs
        .into_iter()
        .map(|e| expr_to_fixpoint(e, name_map, const_map))
        .collect()
}

fn tuple_to_fixpoint(
    exprs: &[rty::Expr],
    name_map: &NameMap,
    const_map: &ConstMap,
) -> fixpoint::Expr {
    match exprs {
        [] => fixpoint::Expr::Unit,
        [e, exprs @ ..] => {
            fixpoint::Expr::Pair(Box::new([
                expr_to_fixpoint(e, name_map, const_map),
                tuple_to_fixpoint(exprs, name_map, const_map),
            ]))
        }
    }
}
