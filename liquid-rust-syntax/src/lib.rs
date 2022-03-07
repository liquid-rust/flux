#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_span;

pub mod ast;
pub mod lexer;
pub mod surface;

use lalrpop_util::lalrpop_mod;
use lexer::{Cursor, Location, Token};
use rustc_ast::tokenstream::TokenStream;
use rustc_hir::def_id::LocalDefId;
use rustc_span::{BytePos, Span, SyntaxContext};

lalrpop_mod!(
    #[allow(warnings)]
    #[allow(clippy::all)]
    grammar
);

lalrpop_mod!(
    #[allow(warnings)]
    #[allow(clippy::all)]
    surface_grammar
);

macro_rules! parse {
    ($parser:path, $tokens:expr, $span:expr) => {{
        let offset = $span.lo();
        let ctx = $span.ctxt();
        let parent = $span.parent();
        let mk_span =
            |lo: Location, hi: Location| Span::new(lo.0 + offset, hi.0 + offset, ctx, parent);
        <$parser>::new()
            .parse(&mk_span, Cursor::new($tokens, $span.lo()))
            .map_err(|err| map_err(err, offset, ctx, parent))
    }};
}

pub fn parse_fn_sig(tokens: TokenStream, span: Span) -> ParseResult<ast::FnSig> {
    parse!(grammar::FnSigParser, tokens, span)
}

pub fn parse_refined_by(tokens: TokenStream, span: Span) -> ParseResult<ast::Generics> {
    parse!(grammar::RefinedByParser, tokens, span)
}

pub fn parse_fn_surface_sig(tokens: TokenStream, span: Span) -> ParseResult<ast::FnSig> {
    let res = parse!(surface_grammar::FnSigParser, tokens, span);
    res.map(surface::desugar)
}

pub fn parse_qualifier(tokens: TokenStream, span: Span) -> ParseResult<ast::Qualifier> {
    parse!(grammar::QualifierParser, tokens, span)
}

pub fn parse_ty(tokens: TokenStream, span: Span) -> ParseResult<ast::Ty> {
    parse!(grammar::TyParser, tokens, span)
}

pub enum UserParseError {
    UnsupportedLiteral(Location, Location),
}

type LalrpopError = lalrpop_util::ParseError<Location, Token, UserParseError>;

pub type ParseResult<T> = Result<T, ParseError>;

pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    UnexpectedEOF,
    UnexpectedToken,
    IntTooLarge,
}

impl ParseErrorKind {
    fn into_error(
        self,
        offset: BytePos,
        lo: Location,
        hi: Location,
        ctx: SyntaxContext,
        parent: Option<LocalDefId>,
    ) -> ParseError {
        ParseError { kind: self, span: Span::new(lo.0 + offset, hi.0 + offset, ctx, parent) }
    }
}

fn map_err(
    err: LalrpopError,
    offset: BytePos,
    ctx: SyntaxContext,
    parent: Option<LocalDefId>,
) -> ParseError {
    match err {
        LalrpopError::InvalidToken { .. } => unreachable!(),
        LalrpopError::User { error: UserParseError::UnsupportedLiteral(lo, hi) } => {
            ParseErrorKind::UnexpectedToken.into_error(offset, lo, hi, ctx, parent)
        }
        LalrpopError::UnrecognizedEOF { location, expected: _ } => {
            ParseErrorKind::UnexpectedEOF.into_error(offset, location, location, ctx, parent)
        }
        LalrpopError::UnrecognizedToken { token: (start, _, end), expected: _ }
        | LalrpopError::ExtraToken { token: (start, _, end) } => {
            ParseErrorKind::UnexpectedToken.into_error(offset, start, end, ctx, parent)
        }
    }
}
