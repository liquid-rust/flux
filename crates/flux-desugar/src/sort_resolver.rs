use flux_common::iter::IterExt;
use flux_errors::FluxSession;
use flux_middle::fhir::{self};
use flux_syntax::surface;
use rustc_errors::ErrorGuaranteed;
use rustc_hash::FxHashMap;
use rustc_middle::ty::Generics;
use rustc_span::{
    def_id::DefId,
    sym::{self},
    symbol::kw::{self},
    Symbol,
};

use crate::errors;

type Result<T = ()> = std::result::Result<T, ErrorGuaranteed>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SelfRes {
    /// A `Self` parameter in a trait definition.
    Param { trait_id: DefId },
    /// An alias to another sort, e.g., when used inside an impl block
    Alias { alias_to: DefId },
    /// It is not valid to use `Self`, e.g., when used in a free function
    None,
}

pub(crate) struct SortResolver<'a, 'fhir> {
    sess: &'a FluxSession,
    map: &'a fhir::Map<'fhir>,
    generic_params: FxHashMap<Symbol, DefId>,
    sort_params: FxHashMap<Symbol, usize>,
    self_res: SelfRes,
}

impl<'a, 'fhir> SortResolver<'a, 'fhir> {
    pub(crate) fn with_sort_params(
        sess: &'a FluxSession,
        map: &'a fhir::Map<'fhir>,
        sort_params: &[Symbol],
    ) -> Self {
        let sort_params = sort_params
            .iter()
            .enumerate()
            .map(|(i, v)| (*v, i))
            .collect();
        Self { sess, map, generic_params: Default::default(), sort_params, self_res: SelfRes::None }
    }

    pub(crate) fn with_generics(
        sess: &'a FluxSession,
        map: &'a fhir::Map<'fhir>,
        generics: &Generics,
        self_res: SelfRes,
    ) -> Self {
        let generic_params = generics.params.iter().map(|p| (p.name, p.def_id)).collect();
        Self { sess, map, sort_params: Default::default(), generic_params, self_res }
    }

    pub(crate) fn resolve_sort(&self, sort: &surface::Sort) -> Result<fhir::Sort<'fhir>> {
        match sort {
            surface::Sort::Base(sort) => self.resolve_base_sort(sort),
            surface::Sort::Func { inputs, output } => {
                Ok(self.resolve_func_sort(inputs, output)?.into())
            }
            surface::Sort::Infer => Ok(fhir::Sort::Infer),
        }
    }

    fn resolve_func_sort(
        &self,
        inputs: &[surface::BaseSort],
        output: &surface::BaseSort,
    ) -> Result<fhir::PolyFuncSort<'fhir>> {
        let mut inputs_and_output: Vec<fhir::Sort> = inputs
            .iter()
            .map(|sort| self.resolve_base_sort(sort))
            .try_collect_exhaust()?;
        inputs_and_output.push(self.resolve_base_sort(output)?);
        Ok(fhir::PolyFuncSort::new(0, self.map.alloc_slice(&inputs_and_output)))
    }

    fn resolve_base_sort(&self, base: &surface::BaseSort) -> Result<fhir::Sort<'fhir>> {
        match base {
            surface::BaseSort::Ident(ident) => self.resolve_base_sort_ident(ident),
            surface::BaseSort::BitVec(w) => Ok(fhir::Sort::BitVec(*w)),
            surface::BaseSort::App(ident, args) => self.resolve_app_sort(*ident, args),
        }
    }

    fn resolve_sort_ctor(&self, ident: surface::Ident) -> Result<fhir::SortCtor> {
        if ident.name == SORTS.set {
            Ok(fhir::SortCtor::Set)
        } else if ident.name == SORTS.map {
            Ok(fhir::SortCtor::Map)
        } else {
            Err(self.sess.emit_err(errors::UnresolvedSort::new(ident)))
        }
    }

    fn resolve_app_sort(
        &self,
        ident: surface::Ident,
        args: &Vec<surface::BaseSort>,
    ) -> Result<fhir::Sort<'fhir>> {
        let ctor = self.resolve_sort_ctor(ident)?;
        let arity = ctor.arity();
        if args.len() == arity {
            let args: Vec<_> = args
                .iter()
                .map(|arg| self.resolve_base_sort(arg))
                .try_collect_exhaust()?;
            Ok(fhir::Sort::App(ctor, self.map.alloc_slice(&args)))
        } else {
            Err(self
                .sess
                .emit_err(errors::SortArityMismatch::new(ident.span, arity, args.len())))
        }
    }

    fn resolve_base_sort_ident(&self, ident: &surface::Ident) -> Result<fhir::Sort<'fhir>> {
        if ident.name == SORTS.int {
            Ok(fhir::Sort::Int)
        } else if ident.name == sym::bool {
            Ok(fhir::Sort::Bool)
        } else if ident.name == SORTS.real {
            Ok(fhir::Sort::Real)
        } else if ident.name == kw::SelfUpper {
            match self.self_res {
                SelfRes::Param { trait_id } => Ok(fhir::Sort::SelfParam { trait_id }),
                SelfRes::Alias { alias_to } => Ok(fhir::Sort::SelfAlias { alias_to }),
                SelfRes::None => Err(self.sess.emit_err(errors::UnresolvedSort::new(*ident))),
            }
        } else if let Some(def_id) = self.generic_params.get(&ident.name) {
            Ok(fhir::Sort::Param(*def_id))
        } else if let Some(idx) = self.sort_params.get(&ident.name) {
            Ok(fhir::Sort::Var(*idx))
        } else if self.map.sort_decls().get(&ident.name).is_some() {
            let ctor = fhir::SortCtor::User { name: ident.name };
            Ok(fhir::Sort::App(ctor, self.map.alloc_slice(&[])))
        } else {
            Err(self.sess.emit_err(errors::UnresolvedSort::new(*ident)))
        }
    }
}

pub(crate) struct Sorts {
    pub(crate) int: Symbol,
    pub(crate) real: Symbol,
    pub(crate) set: Symbol,
    pub(crate) map: Symbol,
}

pub(crate) static SORTS: std::sync::LazyLock<Sorts> = std::sync::LazyLock::new(|| {
    Sorts {
        int: Symbol::intern("int"),
        real: Symbol::intern("real"),
        set: Symbol::intern("Set"),
        map: Symbol::intern("Map"),
    }
});
