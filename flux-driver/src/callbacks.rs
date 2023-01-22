use std::path::PathBuf;

use flux_common::{cache::QueryCache, config, dbg, iter::IterExt};
use flux_desugar as desugar;
use flux_errors::FluxSession;
use flux_middle::{
    fhir::{self, ConstInfo},
    global_env::GlobalEnv,
    rustc,
};
use flux_refineck::{self as refineck, wf::Wf};
use flux_syntax::surface;
use rustc_driver::{Callbacks, Compilation};
use rustc_errors::ErrorGuaranteed;
use rustc_hir::{
    def::DefKind,
    def_id::{LocalDefId, LOCAL_CRATE},
};
use rustc_interface::{interface::Compiler, Queries};
use rustc_middle::ty::{
    query::{query_values, Providers},
    TyCtxt, WithOptConstParam,
};

use crate::{
    collector::{IgnoreKey, Ignores, SpecCollector, Specs},
    mir_storage,
};

pub(crate) struct FluxCallbacks {
    full_compilation: bool,
}

impl FluxCallbacks {
    pub(crate) fn new(full_compilation: bool) -> Self {
        FluxCallbacks { full_compilation }
    }
}

impl Callbacks for FluxCallbacks {
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        assert!(config.override_queries.is_none());

        config.override_queries = Some(|_, local, _| {
            local.mir_borrowck = mir_borrowck;
        });
    }

    fn after_analysis<'tcx>(
        &mut self,
        compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        if compiler.session().has_errors().is_some() {
            return Compilation::Stop;
        }

        queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
            if !is_tool_registered(tcx) {
                return;
            }
            let sess = FluxSession::new(&tcx.sess.opts, tcx.sess.parse_sess.clone_source_map());
            let _ = check_crate(tcx, &sess);
            sess.finish_diagnostics();
        });

        if self.full_compilation {
            Compilation::Continue
        } else {
            Compilation::Stop
        }
    }
}

fn check_crate(tcx: TyCtxt, sess: &FluxSession) -> Result<(), ErrorGuaranteed> {
    tracing::info_span!("check_crate").in_scope(|| {
        let mut specs = SpecCollector::collect(tcx, sess)?;

        // Ignore everything and go home
        if specs.ignores.contains(&IgnoreKey::Crate) {
            return Ok(());
        }

        // Do defn-expansion _after_ the WF check, so errors are given at user-specification level
        let map = build_fhir_map(tcx, sess, &mut specs)?;
        check_wf(tcx, sess, &map)?;

        tracing::info!("Callbacks::check_wf");

        let mut genv = GlobalEnv::new(tcx, sess, map)?;
        // Assert behavior from Crate config
        // TODO(atgeller) rest of settings from crate config
        if let Some(crate_config) = specs.crate_config {
            let assert_behavior = crate_config.check_asserts;
            genv.register_assert_behavior(assert_behavior);
        }

        let mut ck = CrateChecker::new(&mut genv, specs.ignores);

        let crate_items = tcx.hir_crate_items(());
        let items = crate_items.items().map(|item| item.owner_id.def_id);
        let impl_items = crate_items
            .impl_items()
            .map(|impl_item| impl_item.owner_id.def_id);

        let result = items
            .chain(impl_items)
            .try_for_each_exhaust(|def_id| ck.check_def(def_id));

        ck.cache.save().unwrap_or(());

        tracing::info!("Callbacks::check_crate");

        let crate_name = tcx.crate_name(LOCAL_CRATE);
        flux_metadata::encode_metadata(
            &genv,
            PathBuf::from(format!("{crate_name}.fluxmeta")).as_path(),
        );

        result
    })
}

struct CrateChecker<'a, 'genv, 'tcx> {
    genv: &'a mut GlobalEnv<'genv, 'tcx>,
    ignores: Ignores,
    cache: QueryCache,
}

impl<'a, 'genv, 'tcx> CrateChecker<'a, 'genv, 'tcx> {
    fn new(genv: &'a mut GlobalEnv<'genv, 'tcx>, ignores: Ignores) -> Self {
        CrateChecker { genv, ignores, cache: QueryCache::load() }
    }

    fn is_trusted(&self, def_id: LocalDefId) -> bool {
        self.genv.map().is_trusted(def_id.to_def_id())
    }

    /// `is_ignored` transitively follows the `def_id`'s parent-chain to check if
    /// any enclosing mod has been marked as `ignore`
    fn is_ignored(&self, def_id: LocalDefId) -> bool {
        let parent_def_id = self.genv.tcx.parent_module_from_def_id(def_id);
        if parent_def_id == def_id {
            false
        } else {
            self.ignores.contains(&IgnoreKey::Module(parent_def_id))
                || self.is_ignored(parent_def_id)
        }
    }

    fn matches_check_def(&self, def_id: LocalDefId) -> bool {
        let def_path = self.genv.tcx.def_path_str(def_id.to_def_id());
        def_path.contains(config::check_def())
    }

    fn check_def(&mut self, def_id: LocalDefId) -> Result<(), ErrorGuaranteed> {
        if self.is_ignored(def_id) || !self.matches_check_def(def_id) {
            return Ok(());
        }

        match self.genv.tcx.def_kind(def_id.to_def_id()) {
            DefKind::Fn | DefKind::AssocFn => self.check_fn(def_id),
            DefKind::Enum | DefKind::Struct => self.check_adt_invariants(def_id),
            _ => Ok(()),
        }
    }

    fn check_fn(&mut self, def_id: LocalDefId) -> Result<(), ErrorGuaranteed> {
        if self.is_trusted(def_id) {
            return Ok(());
        }

        let mir = unsafe { mir_storage::retrieve_mir_body(self.genv.tcx, def_id).body };

        // HACK(nilehmann) this will ignore any code generated by a macro. This is
        // a temporary workaround to allow `#[derive(PartialEq, Eq)]` and should be
        // removed.
        if mir.span.ctxt() > rustc_span::SyntaxContext::root() {
            return Ok(());
        }

        if config::dump_mir() {
            rustc_middle::mir::pretty::write_mir_fn(
                self.genv.tcx,
                &mir,
                &mut |_, _| Ok(()),
                &mut dbg::writer_for_item(self.genv.tcx, def_id.to_def_id(), "mir").unwrap(),
            )
            .unwrap();
        }

        let body =
            rustc::lowering::LoweringCtxt::lower_mir_body(self.genv.tcx, self.genv.sess, mir)?;

        refineck::check_fn(self.genv, &mut self.cache, def_id.to_def_id(), &body)
    }

    fn check_adt_invariants(&mut self, def_id: LocalDefId) -> Result<(), ErrorGuaranteed> {
        let adt_def = self.genv.adt_def(def_id.to_def_id());
        if adt_def.is_opaque() {
            return Ok(());
        }
        refineck::invariants::check_invariants(self.genv, &mut self.cache, &adt_def)
    }
}

fn build_fhir_map(
    tcx: TyCtxt,
    sess: &FluxSession,
    specs: &mut Specs,
) -> Result<fhir::Map, ErrorGuaranteed> {
    let mut map = fhir::Map::default();

    let mut err: Option<ErrorGuaranteed> = None;

    // Register Sorts
    for sort_decl in std::mem::take(&mut specs.sort_decls) {
        map.insert_sort_decl(desugar::desugar_sort_decl(sort_decl));
    }

    // Register Consts
    for (def_id, const_sig) in std::mem::take(&mut specs.consts) {
        let did = def_id.to_def_id();
        let sym = def_id_symbol(tcx, def_id);
        map.insert_const(ConstInfo { def_id: did, sym, val: const_sig.val });
    }

    // Register UIFs
    err = std::mem::take(&mut specs.uifs)
        .into_iter()
        .try_for_each_exhaust(|uif_def| {
            let name = uif_def.name;
            let uif_def = desugar::resolve_uif_def(sess, &map, uif_def)?;
            map.insert_uif(name.name, uif_def);
            Ok(())
        })
        .err()
        .or(err);

    // Register Defns as UIFs for sort-checking
    err = specs
        .dfns
        .iter()
        .try_for_each_exhaust(|defn| {
            let name = defn.name;
            let defn_uif = desugar::resolve_defn_uif(sess, &map, defn)?;
            map.insert_uif(name.name, defn_uif);
            Ok(())
        })
        .err()
        .or(err);

    // Register AdtDefs
    err = specs
        .structs
        .iter()
        .try_for_each_exhaust(|(def_id, def)| {
            let refined_by = def.refined_by.as_ref().unwrap_or(surface::RefinedBy::DUMMY);
            let adt_def = desugar::desugar_adt_def(
                tcx,
                sess,
                &map,
                def_id.to_def_id(),
                refined_by,
                &def.invariants,
                def.opaque,
            )?;
            map.insert_adt(*def_id, adt_def);
            Ok(())
        })
        .err()
        .or(err);
    err = specs
        .enums
        .iter()
        .try_for_each_exhaust(|(def_id, def)| {
            let refined_by = def.refined_by.as_ref().unwrap_or(surface::RefinedBy::DUMMY);
            let adt_def = desugar::desugar_adt_def(
                tcx,
                sess,
                &map,
                def_id.to_def_id(),
                refined_by,
                &def.invariants,
                false,
            )?;
            map.insert_adt(*def_id, adt_def);
            Ok(())
        })
        .err()
        .or(err);

    // Desugaring after this depends on the `fhir::Map` containing the information
    // collected before, so we bail out if there's any error at this point.
    if let Some(err) = err {
        return Err(err);
    }

    // Register Defns
    err = std::mem::take(&mut specs.dfns)
        .into_iter()
        .try_for_each_exhaust(|defn| {
            let name = defn.name;
            let defn = desugar::desugar_defn(tcx, sess, &map, defn)?;
            map.insert_defn(name.name, defn);
            Ok(())
        })
        .err()
        .or(err);

    // Qualifiers
    err = specs
        .qualifs
        .iter()
        .try_for_each_exhaust(|qualifier| {
            let qualifier = desugar::desugar_qualifier(tcx, sess, &map, qualifier)?;
            map.insert_qualifier(qualifier);
            Ok(())
        })
        .err()
        .or(err);

    // Variants
    err = std::mem::take(&mut specs.structs)
        .into_iter()
        .try_for_each_exhaust(|(def_id, struct_def)| {
            map.insert_struct(def_id, desugar::desugar_struct_def(tcx, sess, &map, struct_def)?);
            Ok(())
        })
        .err()
        .or(err);

    err = std::mem::take(&mut specs.enums)
        .into_iter()
        .try_for_each_exhaust(|(def_id, enum_def)| {
            map.insert_enum(def_id, desugar::desugar_enum_def(tcx, sess, &map, enum_def)?);
            Ok(())
        })
        .err()
        .or(err);

    // FnSigs
    let aliases = std::mem::take(&mut specs.aliases);
    err = std::mem::take(&mut specs.fns)
        .into_iter()
        .try_for_each_exhaust(|(def_id, spec)| {
            if spec.trusted {
                map.add_trusted(def_id);
            }
            if let Some(fn_sig) = spec.fn_sig {
                let fn_sig = surface::expand::expand_sig(&aliases, fn_sig)?;
                let fn_sig = desugar::desugar_fn_sig(tcx, sess, &map, def_id, fn_sig)?;
                map.insert_fn_sig(def_id, fn_sig);
            }
            if let Some(quals) = spec.qual_names {
                map.insert_fn_quals(def_id, quals.names);
            }
            Ok(())
        })
        .err()
        .or(err);

    if let Some(err) = err {
        Err(err)
    } else {
        Ok(map)
    }
}

fn check_wf(tcx: TyCtxt, sess: &FluxSession, map: &fhir::Map) -> Result<(), ErrorGuaranteed> {
    let mut err: Option<ErrorGuaranteed> = None;

    for defn in map.defns() {
        err = Wf::check_defn(tcx, sess, map, defn).err().or(err);
    }

    for adt_def in map.adts() {
        err = Wf::check_adt_def(tcx, sess, map, adt_def).err().or(err);
    }

    for qualifier in map.qualifiers() {
        err = Wf::check_qualifier(tcx, sess, map, qualifier).err().or(err);
    }

    for struct_def in map.structs() {
        let refined_by = map.refined_by(struct_def.def_id).unwrap();
        err = Wf::check_struct_def(tcx, sess, map, refined_by, struct_def)
            .err()
            .or(err);
    }

    for enum_def in map.enums() {
        err = Wf::check_enum_def(tcx, sess, map, enum_def).err().or(err);
    }

    for (_, fn_sig) in map.fn_sigs() {
        err = Wf::check_fn_sig(tcx, sess, map, fn_sig).err().or(err);
    }

    let qualifiers = map.qualifiers().map(|q| q.name.clone()).collect();
    for (_, fn_quals) in map.fn_quals() {
        err = Wf::check_fn_quals(sess, &qualifiers, fn_quals)
            .err()
            .or(err);
    }

    if let Some(err) = err {
        Err(err)
    } else {
        Ok(())
    }
}

fn def_id_symbol(tcx: TyCtxt, def_id: LocalDefId) -> rustc_span::Symbol {
    let did = def_id.to_def_id();
    // TODO(RJ) use fully qualified names: Symbol::intern(&tcx.def_path_str(did))
    let def_path = tcx.def_path(did);
    if let Some(dp) = def_path.data.last() {
        if let rustc_hir::definitions::DefPathData::ValueNs(sym) = dp.data {
            return sym;
        }
    }
    panic!("def_id_symbol fails on {did:?}")
}

#[allow(clippy::needless_lifetimes)]
fn mir_borrowck<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> query_values::mir_borrowck<'tcx> {
    let body_with_facts = rustc_borrowck::consumers::get_body_with_borrowck_facts(
        tcx,
        WithOptConstParam::unknown(def_id),
    );
    // SAFETY: This is safe because we are feeding in the same `tcx` that is
    // going to be used as a witness when pulling out the data.
    unsafe {
        mir_storage::store_mir_body(tcx, def_id, body_with_facts);
    }
    let mut providers = Providers::default();
    rustc_borrowck::provide(&mut providers);
    let original_mir_borrowck = providers.mir_borrowck;
    original_mir_borrowck(tcx, def_id)
}

fn is_tool_registered(tcx: TyCtxt) -> bool {
    for attr in tcx.hir().krate_attrs() {
        if rustc_ast_pretty::pprust::attribute_to_string(attr) == "#![register_tool(flux)]" {
            return true;
        }
    }
    false
}
