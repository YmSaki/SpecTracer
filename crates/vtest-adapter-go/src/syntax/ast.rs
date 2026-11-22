//! The Go syntax tree the static audit reads.
//!
//! Responsibility: the shapes of a Go function (declarations, statements,
//! expressions, and the type expressions that appear among them). Boundary: no
//! meaning; what a name resolves to and what a value derives from belong to
//! `static_audit`. A type that no rule reads (a struct or interface body) is
//! kept as an opaque node, so the tree never claims more than the parser read.
//! Realizes DES-619.

/// A byte range of the scanned text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Span {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// An identifier and where it is written.
#[derive(Clone, Debug)]
pub(crate) struct Ident {
    pub(crate) name: String,
    pub(crate) span: Span,
}

/// The kind of a basic literal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LitKind {
    Int,
    Float,
    Imag,
    Char,
    Str,
}

/// An expression; Go parses a type as an expression, so a type is one too.
#[derive(Clone, Debug)]
pub(crate) struct Expr {
    pub(crate) span: Span,
    pub(crate) kind: ExprKind,
}

#[derive(Clone, Debug)]
pub(crate) enum ExprKind {
    Ident(String),
    BasicLit {
        kind: LitKind,
        text: String,
    },
    /// `ty` is `None` for the elided type of an element (`{1, 2}`).
    CompositeLit {
        ty: Option<Box<Expr>>,
        elts: Vec<Expr>,
    },
    FuncLit(Box<FuncLit>),
    Paren(Box<Expr>),
    Selector(Box<Expr>, Ident),
    /// `x[i]`, or `f[A, B]` with several type arguments.
    Index(Box<Expr>, Vec<Expr>),
    Slice(Box<Expr>, Vec<Option<Expr>>),
    /// `x.(T)`; `None` is `x.(type)`.
    TypeAssert(Box<Expr>, Option<Box<Expr>>),
    Call {
        fun: Box<Expr>,
        args: Vec<Expr>,
        ellipsis: bool,
    },
    /// `*x`: a dereference or a pointer type.
    Star(Box<Expr>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    KeyValue(Box<Expr>, Box<Expr>),
    /// `[N]T`, or `[]T` when `len` is `None`.
    ArrayType {
        len: Option<Box<Expr>>,
        elt: Box<Expr>,
    },
    /// `...T` (or `[...]` as an array length).
    Ellipsis(Option<Box<Expr>>),
    /// A struct type; the fields are read but not kept.
    StructType,
    FuncType(FuncType),
    /// An interface type; the elements are read but not kept.
    InterfaceType,
    MapType(Box<Expr>, Box<Expr>),
    ChanType(Box<Expr>),
}

/// A function literal.
#[derive(Clone, Debug)]
pub(crate) struct FuncLit {
    pub(crate) sig: FuncType,
    pub(crate) body: Block,
}

/// The parameters and results of a function type.
#[derive(Clone, Debug)]
pub(crate) struct FuncType {
    pub(crate) params: Vec<Field>,
    pub(crate) results: Vec<Field>,
}

/// One parameter or result group: `names` is empty for an unnamed one.
#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub(crate) names: Vec<Ident>,
    pub(crate) ty: Expr,
}

/// A top-level function declaration.
#[derive(Clone, Debug)]
pub(crate) struct FuncDecl {
    pub(crate) name: Ident,
    pub(crate) has_receiver: bool,
    pub(crate) sig: FuncType,
    pub(crate) body: Option<Block>,
}

#[derive(Clone, Debug)]
pub(crate) struct Block {
    pub(crate) span: Span,
    pub(crate) stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub(crate) struct Stmt {
    pub(crate) span: Span,
    pub(crate) kind: StmtKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BranchKind {
    Break,
    Continue,
    Goto,
    Fallthrough,
}

#[derive(Clone, Debug)]
pub(crate) enum StmtKind {
    Empty,
    Decl(Vec<DeclSpec>),
    Labeled(Ident, Box<Stmt>),
    Expr(Expr),
    Send(Expr, Expr),
    IncDec(Expr),
    Assign {
        lhs: Vec<Expr>,
        /// `=`, `:=`, or a compound operator such as `+=`.
        op: &'static str,
        rhs: Vec<Expr>,
    },
    Go(Expr),
    Defer(Expr),
    Return(Vec<Expr>),
    Branch {
        kind: BranchKind,
        label: Option<Ident>,
    },
    Block(Block),
    If {
        init: Option<Box<Stmt>>,
        cond: Expr,
        then: Block,
        /// A block or an `if` statement.
        els: Option<Box<Stmt>>,
    },
    Switch {
        init: Option<Box<Stmt>>,
        tag: Option<Expr>,
        clauses: Vec<CaseClause>,
    },
    TypeSwitch {
        init: Option<Box<Stmt>>,
        bind: Option<Ident>,
        x: Expr,
        clauses: Vec<CaseClause>,
    },
    Select {
        clauses: Vec<CommClause>,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        post: Option<Box<Stmt>>,
        body: Block,
    },
    Range {
        key: Option<Expr>,
        value: Option<Expr>,
        define: bool,
        x: Expr,
        body: Block,
    },
}

/// A `case` or `default` clause of an expression or type switch.
#[derive(Clone, Debug)]
pub(crate) struct CaseClause {
    pub(crate) span: Span,
    pub(crate) default: bool,
    pub(crate) exprs: Vec<Expr>,
    pub(crate) body: Vec<Stmt>,
}

/// A `case` or `default` clause of a `select` statement.
#[derive(Clone, Debug)]
pub(crate) struct CommClause {
    pub(crate) span: Span,
    pub(crate) comm: Option<Box<Stmt>>,
    pub(crate) body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub(crate) enum DeclSpec {
    Var {
        names: Vec<Ident>,
        ty: Option<Expr>,
        values: Vec<Expr>,
    },
    Const {
        names: Vec<Ident>,
        values: Vec<Expr>,
    },
    Type {
        name: Ident,
    },
}

/// An import of the file: the name it is bound to (`None` when the path's last
/// element is the name), and the path.
#[derive(Clone, Debug)]
pub(crate) struct Import {
    pub(crate) name: Option<String>,
    pub(crate) path: String,
}

/// The header of a Go source file.
#[derive(Clone, Debug)]
pub(crate) struct FileHeader {
    pub(crate) package: String,
    pub(crate) imports: Vec<Import>,
}
