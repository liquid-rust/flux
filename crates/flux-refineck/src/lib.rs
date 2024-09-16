//! Refinement type checking

#![feature(
    associated_type_defaults,
    box_patterns,
    extract_if,
    if_let_guard,
    let_chains,
    min_specialization,
    never_type,
    rustc_private,
    unwrap_infallible
)]

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_type_ir;

mod checker;
mod ghost_statements;
pub mod invariants;
mod primops;
mod queue;
mod type_env;

use checker::Checker;
pub use checker::CheckerConfig;
use flux_common::{cache::QueryCache, dbg, result::ResultExt as _};
use flux_config as config;
use flux_infer::{
    fixpoint_encoding::FixpointCtxt,
    infer::{ConstrReason, Tag},
};
use flux_macros::fluent_messages;
use flux_middle::{
    global_env::GlobalEnv,
    queries::QueryResult,
    rty::{self, ESpan},
};
use itertools::Itertools;
use rustc_errors::ErrorGuaranteed;
use rustc_hir::def_id::LocalDefId;
use rustc_span::Span;

use crate::{checker::errors::ResultExt as _, ghost_statements::compute_ghost_statements};

fluent_messages! { "../locales/en-US.ftl" }

pub fn check_fn(
    genv: GlobalEnv,
    cache: &mut QueryCache,
    def_id: LocalDefId,
    config: CheckerConfig,
) -> Result<(), ErrorGuaranteed> {
    dbg::check_fn_span!(genv.tcx(), def_id).in_scope(|| {
        let span = genv.tcx().def_span(def_id);

        // HACK(nilehmann) this will ignore any code generated by a macro. This is a temporary
        // workaround to deal with a `#[derive(..)]` that generates code that flux cannot handle.
        // Note that this is required because code generated by a `#[derive(..)]` cannot be
        // manually trusted or ignored.
        if !genv.tcx().def_span(def_id).ctxt().is_root() {
            return Ok(());
        }

        // Make sure we run conversion and report any errors even if the function is trusted or
        // doesn't have body.
        force_conv(genv, def_id).emit(&genv)?;

        // Skip trait methods without body
        if genv.tcx().hir_node_by_def_id(def_id).body_id().is_none() {
            return Ok(());
        }

        if genv.trusted(def_id) {
            return Ok(());
        }

        let ghost_stmts = compute_ghost_statements(genv, def_id)
            .with_span(span)
            .emit(&genv)?;

        // PHASE 1: infer shape of `TypeEnv` at the entry of join points
        let shape_result =
            Checker::run_in_shape_mode(genv, def_id, &ghost_stmts, config).emit(&genv)?;
        tracing::info!("check_fn::shape");

        // PHASE 2: generate refinement tree constraint
        let (mut refine_tree, kvars) =
            Checker::run_in_refine_mode(genv, def_id, &ghost_stmts, shape_result, config)
                .emit(&genv)?;
        tracing::info!("check_fn::refine");

        // PHASE 3: invoke fixpoint on the constraint
        if config::dump_constraint() {
            dbg::dump_item_info(genv.tcx(), def_id, "fluxc", &refine_tree).unwrap();
        }
        refine_tree.simplify();
        if config::dump_constraint() {
            dbg::dump_item_info(genv.tcx(), def_id, "simp.fluxc", &refine_tree).unwrap();
        }
        let mut fcx = FixpointCtxt::new(genv, def_id, kvars);
        let cstr = refine_tree.into_fixpoint(&mut fcx).emit(&genv)?;
        let errors = fcx.check(cache, cstr, config.scrape_quals).emit(&genv)?;

        tracing::info!("check_fn::fixpoint");
        #[expect(clippy::collapsible_else_if, reason = "it looks better")]
        if genv.should_fail(def_id) {
            if errors.is_empty() {
                report_expected_neg(genv, def_id)
            } else {
                Ok(())
            }
        } else {
            if errors.is_empty() {
                Ok(())
            } else {
                report_errors(genv, errors)
            }
        }
    })
}

fn force_conv(genv: GlobalEnv, def_id: LocalDefId) -> QueryResult {
    genv.generics_of(def_id)?;
    genv.refinement_generics_of(def_id)?;
    genv.predicates_of(def_id)?;
    genv.fn_sig(def_id)?;
    Ok(())
}

fn call_error(genv: GlobalEnv, span: Span, dst_span: Option<ESpan>) -> ErrorGuaranteed {
    genv.sess()
        .emit_err(errors::RefineError::call(span, dst_span))
}

fn ret_error(genv: GlobalEnv, span: Span, dst_span: Option<ESpan>) -> ErrorGuaranteed {
    genv.sess()
        .emit_err(errors::RefineError::ret(span, dst_span))
}

fn report_errors(genv: GlobalEnv, errors: Vec<Tag>) -> Result<(), ErrorGuaranteed> {
    let mut e = None;
    for err in errors {
        let span = err.src_span;
        e = Some(match err.reason {
            ConstrReason::Call => call_error(genv, span, err.dst_span),
            ConstrReason::Assign => genv.sess().emit_err(errors::AssignError { span }),
            ConstrReason::Ret => ret_error(genv, span, err.dst_span),
            ConstrReason::Div => genv.sess().emit_err(errors::DivError { span }),
            ConstrReason::Rem => genv.sess().emit_err(errors::RemError { span }),
            ConstrReason::Goto(_) => genv.sess().emit_err(errors::GotoError { span }),
            ConstrReason::Assert(msg) => genv.sess().emit_err(errors::AssertError { span, msg }),
            ConstrReason::Fold => genv.sess().emit_err(errors::FoldError { span }),
            ConstrReason::Overflow => genv.sess().emit_err(errors::OverflowError { span }),
            ConstrReason::Other => genv.sess().emit_err(errors::UnknownError { span }),
        });
    }

    if let Some(e) = e {
        Err(e)
    } else {
        Ok(())
    }
}

fn report_expected_neg(genv: GlobalEnv, def_id: LocalDefId) -> Result<(), ErrorGuaranteed> {
    Err(genv.sess().emit_err(errors::ExpectedNeg {
        span: genv.tcx().def_span(def_id),
        def_descr: genv.tcx().def_descr(def_id.to_def_id()),
    }))
}

mod errors {
    use flux_errors::E0999;
    use flux_macros::{Diagnostic, Subdiagnostic};
    use flux_middle::rty::ESpan;
    use rustc_span::Span;

    #[derive(Diagnostic)]
    #[diag(refineck_goto_error, code = E0999)]
    pub struct GotoError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_assign_error, code = E0999)]
    pub struct AssignError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Subdiagnostic)]
    #[note(refineck_condition_span_note)]
    pub(crate) struct ConditionSpanNote {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Subdiagnostic)]
    #[note(refineck_call_span_note)]
    pub(crate) struct CallSpanNote {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_refine_error, code = E0999)]
    pub struct RefineError {
        #[primary_span]
        #[label]
        pub span: Span,
        cond: &'static str,
        #[subdiagnostic]
        span_note: Option<ConditionSpanNote>,
        #[subdiagnostic]
        call_span_note: Option<CallSpanNote>,
    }

    impl RefineError {
        pub fn call(span: Span, espan: Option<ESpan>) -> Self {
            RefineError::new("precondition", span, espan)
        }

        pub fn ret(span: Span, espan: Option<ESpan>) -> Self {
            RefineError::new("postcondition", span, espan)
        }

        fn new(cond: &'static str, span: Span, espan: Option<ESpan>) -> RefineError {
            match espan {
                Some(dst_span) => {
                    let span_note = Some(ConditionSpanNote { span: dst_span.span });
                    let call_span_note = dst_span.base.map(|span| CallSpanNote { span });
                    RefineError { span, cond, span_note, call_span_note }
                }
                None => RefineError { span, cond, span_note: None, call_span_note: None },
            }
        }
    }

    #[derive(Diagnostic)]
    #[diag(refineck_div_error, code = E0999)]
    pub struct DivError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_rem_error, code = E0999)]
    pub struct RemError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_assert_error, code = E0999)]
    pub struct AssertError {
        #[primary_span]
        pub span: Span,
        pub msg: &'static str,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_fold_error, code = E0999)]
    pub struct FoldError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_overflow_error, code = E0999)]
    pub struct OverflowError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_unknown_error, code = E0999)]
    pub struct UnknownError {
        #[primary_span]
        pub span: Span,
    }

    #[derive(Diagnostic)]
    #[diag(refineck_expected_neg, code = E0999)]
    pub struct ExpectedNeg {
        #[primary_span]
        pub span: Span,
        pub def_descr: &'static str,
    }
}
