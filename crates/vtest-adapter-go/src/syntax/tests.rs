//! Tests of the Go reader: the shape of the tree for the places where the
//! grammar needs context, which the static audit's deciders depend on, and a
//! read of the whole Go standard library.

use super::*;

/// The tree of an expression as an S-expression, so a test states the shape
/// it expects and not only that the text was accepted.
fn show_expr(x: &Expr) -> String {
    let list = |items: &[Expr]| items.iter().map(show_expr).collect::<Vec<_>>().join(" ");
    match &x.kind {
        ExprKind::Ident(name) => name.clone(),
        ExprKind::BasicLit { text, .. } => text.clone(),
        ExprKind::CompositeLit { ty, elts } => format!(
            "(lit {} {})",
            ty.as_ref().map_or("_".to_owned(), |t| show_expr(t)),
            list(elts)
        ),
        ExprKind::FuncLit(lit) => format!(
            "(funclit {} {})",
            show_sig(&lit.sig),
            lit.body
                .stmts
                .iter()
                .map(show_stmt)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        ExprKind::Paren(x) => format!("(paren {})", show_expr(x)),
        ExprKind::Selector(x, sel) => format!("(. {} {})", show_expr(x), sel.name),
        ExprKind::Index(x, idx) => format!("(index {} {})", show_expr(x), list(idx)),
        ExprKind::Slice(x, parts) => format!(
            "(slice {} {})",
            show_expr(x),
            parts
                .iter()
                .map(|p| p.as_ref().map_or("_".to_owned(), show_expr))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        ExprKind::TypeAssert(x, t) => format!(
            "(assert {} {})",
            show_expr(x),
            t.as_ref().map_or("type".to_owned(), |t| show_expr(t))
        ),
        ExprKind::Call {
            fun,
            args,
            ellipsis,
        } => format!(
            "(call {} {}{})",
            show_expr(fun),
            list(args),
            if *ellipsis { " ..." } else { "" }
        ),
        ExprKind::Star(x) => format!("(* {})", show_expr(x)),
        ExprKind::Unary(op, x) => format!("({op} {})", show_expr(x)),
        ExprKind::Binary(op, a, b) => format!("({op} {} {})", show_expr(a), show_expr(b)),
        ExprKind::KeyValue(k, v) => format!("(: {} {})", show_expr(k), show_expr(v)),
        ExprKind::ArrayType { len, elt } => format!(
            "(array {} {})",
            len.as_ref().map_or("_".to_owned(), |l| show_expr(l)),
            show_expr(elt)
        ),
        ExprKind::Ellipsis(t) => format!(
            "(... {})",
            t.as_ref().map_or("_".to_owned(), |t| show_expr(t))
        ),
        ExprKind::StructType => "struct".to_owned(),
        ExprKind::FuncType(sig) => format!("(functype {})", show_sig(sig)),
        ExprKind::InterfaceType => "interface".to_owned(),
        ExprKind::MapType(k, v) => format!("(map {} {})", show_expr(k), show_expr(v)),
        ExprKind::ChanType(t) => format!("(chan {})", show_expr(t)),
    }
}

fn show_fields(fields: &[Field]) -> String {
    fields
        .iter()
        .map(|f| {
            let names = f
                .names
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            format!("[{names}:{}]", show_expr(&f.ty))
        })
        .collect::<Vec<_>>()
        .join("")
}

fn show_sig(sig: &FuncType) -> String {
    format!(
        "({})->({})",
        show_fields(&sig.params),
        show_fields(&sig.results)
    )
}

fn show_block(stmts: &[Stmt]) -> String {
    format!(
        "{{{}}}",
        stmts.iter().map(show_stmt).collect::<Vec<_>>().join(" ")
    )
}

fn show_opt(stmt: Option<&Stmt>) -> String {
    stmt.map_or("_".to_owned(), show_stmt)
}

fn show_stmt(s: &Stmt) -> String {
    let exprs = |items: &[Expr]| items.iter().map(show_expr).collect::<Vec<_>>().join(" ");
    match &s.kind {
        StmtKind::Empty => "empty".to_owned(),
        StmtKind::Decl(specs) => format!("(decl {})", specs.len()),
        StmtKind::Labeled(l, inner) => format!("(label {} {})", l.name, show_stmt(inner)),
        StmtKind::Expr(x) => format!("(expr {})", show_expr(x)),
        StmtKind::Send(a, b) => format!("(send {} {})", show_expr(a), show_expr(b)),
        StmtKind::IncDec(x) => format!("(incdec {})", show_expr(x)),
        StmtKind::Assign { lhs, op, rhs } => {
            format!("(assign {op} [{}] [{}])", exprs(lhs), exprs(rhs))
        }
        StmtKind::Go(x) => format!("(go {})", show_expr(x)),
        StmtKind::Defer(x) => format!("(defer {})", show_expr(x)),
        StmtKind::Return(xs) => format!("(return {})", exprs(xs)),
        StmtKind::Branch { kind, label } => format!(
            "({:?} {})",
            kind,
            label.as_ref().map_or("_", |l| l.name.as_str())
        ),
        StmtKind::Block(b) => show_block(&b.stmts),
        StmtKind::If {
            init,
            cond,
            then,
            els,
        } => format!(
            "(if {} {} {} {})",
            show_opt(init.as_deref()),
            show_expr(cond),
            show_block(&then.stmts),
            show_opt(els.as_deref())
        ),
        StmtKind::Switch { init, tag, clauses } => format!(
            "(switch {} {} {})",
            show_opt(init.as_deref()),
            tag.as_ref().map_or("_".to_owned(), show_expr),
            show_clauses(clauses)
        ),
        StmtKind::TypeSwitch {
            init,
            bind,
            x,
            clauses,
        } => format!(
            "(typeswitch {} {} {} {})",
            show_opt(init.as_deref()),
            bind.as_ref().map_or("_", |b| b.name.as_str()),
            show_expr(x),
            show_clauses(clauses)
        ),
        StmtKind::Select { clauses } => format!(
            "(select {})",
            clauses
                .iter()
                .map(|c| format!(
                    "(comm {} {})",
                    show_opt(c.comm.as_deref()),
                    show_block(&c.body)
                ))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        StmtKind::For {
            init,
            cond,
            post,
            body,
        } => format!(
            "(for {} {} {} {})",
            show_opt(init.as_deref()),
            cond.as_ref().map_or("_".to_owned(), show_expr),
            show_opt(post.as_deref()),
            show_block(&body.stmts)
        ),
        StmtKind::Range {
            key,
            value,
            define,
            x,
            body,
        } => format!(
            "(range {} {} {} {} {})",
            key.as_ref().map_or("_".to_owned(), show_expr),
            value.as_ref().map_or("_".to_owned(), show_expr),
            define,
            show_expr(x),
            show_block(&body.stmts)
        ),
    }
}

fn show_clauses(clauses: &[CaseClause]) -> String {
    clauses
        .iter()
        .map(|c| {
            format!(
                "(case {} {})",
                if c.default {
                    "default".to_owned()
                } else {
                    c.exprs.iter().map(show_expr).collect::<Vec<_>>().join(",")
                },
                show_block(&c.body)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The statements of `body` read as the body of a function.
fn body_of(body: &str) -> String {
    let src = format!("func f() {{\n{body}\n}}");
    let decl = parse_func_decl(&src).unwrap_or_else(|e| panic!("{e}: {src}"));
    let block = decl.body.expect("a body");
    block
        .stmts
        .iter()
        .map(show_stmt)
        .collect::<Vec<_>>()
        .join(" ")
}

fn expr_of(text: &str) -> String {
    let body = body_of(&format!("_ = {text}"));
    // `(assign = [_] [X])`
    body.strip_prefix("(assign = [_] [")
        .and_then(|rest| rest.strip_suffix("])"))
        .unwrap_or_else(|| panic!("{body}"))
        .to_owned()
}

fn rejects(src: &str) -> bool {
    parse_func_decl(src).is_err()
}

/// DES-619, DS-615: operator precedence and associativity are those of the Go
/// specification, so a condition that mixes them is read with the shape the
/// deciders (DS-1887) and the absorption rule (DS-1948) expect.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-OPERATOR-PRECEDENCE
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::Parser::parse_binary
/// @vtest.intent operators group with the five Go precedence levels, left to right, with unary operators binding tightest and the receive and address-of operators as unary operators
#[test]
fn operators_group_by_the_five_precedence_levels() {
    assert_eq!(
        expr_of("a + b*c == d && e || f"),
        "(|| (&& (== (+ a (* b c)) d) e) f)"
    );
    assert_eq!(expr_of("a - b - c"), "(- (- a b) c)");
    assert_eq!(expr_of("a < b == c"), "(== (< a b) c)");
    assert_eq!(expr_of("a | b ^ c & d"), "(^ (| a b) (& c d))");
    assert_eq!(expr_of("a &^ b << c"), "(<< (&^ a b) c)");
    assert_eq!(expr_of("-a * !b"), "(* (- a) (! b))");
    assert_eq!(expr_of("*p + <-ch"), "(+ (* p) (<- ch))");
    assert_eq!(expr_of("&x.f"), "(& (. x f))");
    assert_eq!(
        expr_of("f(a)(b)[c].d"),
        "(. (index (call (call f a) b) c) d)"
    );
}

/// DES-619: a composite literal is told from a block in a statement header the
/// way the Go specification does, so `if x == T {` opens a block while
/// `[]int{..}` and a parenthesized `T{..}` are values.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-COMPOSITE-LITERAL-AGAINST-BLOCK
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::Parser::parse_primary
/// @vtest.intent a type name before an open brace in an if, for or switch header is the end of the header, an array, slice, map or struct type before it is a composite literal, a parenthesized one is a composite literal, and outside a header a type name before the brace is a composite literal
#[test]
fn a_composite_literal_is_told_from_a_block_in_a_header() {
    assert_eq!(body_of("if x == T {\n}"), "(if _ (== x T) {} _)");
    assert_eq!(
        body_of("if x == (T{1}) {\n}"),
        "(if _ (== x (paren (lit T 1))) {} _)"
    );
    assert_eq!(
        body_of("for _, v := range []int{1, 2} {\n}"),
        "(range _ v true (lit (array _ int) 1 2) {})"
    );
    assert_eq!(
        body_of("switch x {\ncase 1:\n}"),
        "(switch _ x (case 1 {}))"
    );
    assert_eq!(
        body_of("if x := (S{a: 1}); x.a == 1 {\n}"),
        "(if (assign := [x] [(paren (lit S (: a 1)))]) (== (. x a) 1) {} _)"
    );
    assert_eq!(
        body_of("v := T{1}\n_ = v"),
        "(assign := [v] [(lit T 1)]) (assign = [_] [v])"
    );
    assert_eq!(body_of("if m[k] {\n}"), "(if _ (index m k) {} _)");
    assert_eq!(
        body_of("_ = struct{ a int }{a: 1}"),
        "(assign = [_] [(lit struct (: a 1))])"
    );
    // A function literal in a header has a body whose literals are values.
    assert_eq!(
        body_of("if f := func() T { return T{1} }; f() == nil {\n}"),
        "(if (assign := [f] [(funclit ()->([:T]) (return (lit T 1)))]) (== (call f ) nil) {} _)"
    );
}

/// DES-619: a statement ends at a semicolon, written or inserted at a line end
/// after an identifier, a literal, a closing bracket, `++`, `--`, `return`,
/// `break`, `continue` or `fallthrough`, so the same statements split across
/// lines read the same.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-SEMICOLON-INSERTION
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/lexer.rs::tokenize
/// @vtest.intent a line end after an identifier, a literal, a closing bracket, an increment, return, break, continue or fallthrough ends the statement, one after an operator, an opening bracket or a comma does not, a general comment holding a newline ends it as a newline does, and a trailing comma before a closing bracket on its own line is read
#[test]
fn a_line_end_ends_a_statement_where_the_language_says() {
    assert_eq!(
        body_of("x := a +\n\tb\ny := f(\n\ta,\n\tb,\n)"),
        "(assign := [x] [(+ a b)]) (assign := [y] [(call f a b)])"
    );
    assert_eq!(
        body_of("x++\ny--\nreturn\n"),
        "(incdec x) (incdec y) (return )"
    );
    assert_eq!(
        body_of("a := 1 /* a\ncomment */ b := 2"),
        "(assign := [a] [1]) (assign := [b] [2])"
    );
    assert_eq!(
        body_of("a := 1 // tail\nb := 2"),
        "(assign := [a] [1]) (assign := [b] [2])"
    );
    assert_eq!(
        body_of("s := []int{\n\t1,\n\t2,\n}"),
        "(assign := [s] [(lit (array _ int) 1 2)])"
    );
    assert_eq!(body_of("for {\n\tbreak\n}"), "(for _ _ _ {(Break _)})");
    assert_eq!(
        body_of("x := `raw\nstring`\n_ = x"),
        "(assign := [x] [`raw\nstring`]) (assign = [_] [x])"
    );
    assert!(rejects("func f() {\n\tx :=\n}"));
    assert!(rejects("func f() {\n\tx := 1 2\n}"));
}

/// DES-619: an `else if` chain nests in the else branch of the earlier `if`,
/// which is how the deciders of DS-1887 (1) are collected, and a switch keeps
/// the order of its clauses and the `fallthrough` that ends one.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-ELSE-IF-AND-SWITCH-CLAUSES
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::Parser::parse_if
/// @vtest.intent an else if chain reads as an if in the else branch of the earlier if with the final else last, a switch with an init and a tag, a tagless switch and a type switch with a bound name read their clauses in source order, and a clause that ends with fallthrough keeps it
#[test]
fn an_else_if_chain_and_the_clauses_of_a_switch_are_read_in_order() {
    assert_eq!(
        body_of("if a {\n} else if b {\n\tx()\n} else {\n\ty()\n}"),
        "(if _ a {} (if _ b {(expr (call x ))} {(expr (call y ))}))"
    );
    assert_eq!(
        body_of("if a := f(); a > 1 {\n}"),
        "(if (assign := [a] [(call f )]) (> a 1) {} _)"
    );
    assert_eq!(
        body_of("switch x := f(); x {\ncase 1, 2:\n\ta()\n\tfallthrough\ndefault:\n\tb()\n}"),
        "(switch (assign := [x] [(call f )]) x (case 1,2 {(expr (call a )) (Fallthrough _)}) (case default {(expr (call b ))}))"
    );
    assert_eq!(
        body_of("switch {\ncase a > 1:\n}"),
        "(switch _ _ (case (> a 1) {}))"
    );
    assert_eq!(
        body_of("switch v := x.(type) {\ncase int, string:\ncase nil:\ndefault:\n}"),
        "(typeswitch _ v x (case int,string {}) (case nil {}) (case default {}))"
    );
    assert_eq!(
        body_of("select {\ncase v := <-ch:\n\t_ = v\ncase ch <- 1:\ndefault:\n}"),
        "(select (comm (assign := [v] [(<- ch)]) {(assign = [_] [v])}) (comm (send ch 1) {}) (comm _ {}))"
    );
}

/// DES-619: a function literal is told from a function type, a parameter name
/// from a parameter type, and a type argument list from an index, because the
/// first decides what is a counted position (DS-1970) and the others decide
/// which names a scope holds.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-FUNCTION-LITERAL-PARAMETERS-AND-TYPE-ARGUMENTS
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::Parser::parse_params
/// @vtest.intent a func followed by a body is a function literal and one without is a type, grouped parameter names share the type that follows them, unnamed parameters are types, a variadic parameter keeps its ellipsis, a slice or array parameter type after a name is read as that type and not as an index, and f[T](x) is an index of f called
#[test]
fn function_literals_parameters_and_type_arguments_are_told_apart() {
    assert_eq!(
        expr_of("func(a, b int, c ...string) (n int, err error) { return }"),
        "(funclit ([a,b:int][c:(... string)])->([n:int][err:error]) (return ))"
    );
    assert_eq!(
        expr_of("func(int, string) bool { return true }"),
        "(funclit ([:int][:string])->([:bool]) (return true))"
    );
    assert_eq!(
        body_of("var f func(int) int\n_ = f"),
        "(decl 1) (assign = [_] [f])"
    );
    assert_eq!(
        body_of("fs := []func(){func() {}, nil}\n_ = fs"),
        "(assign := [fs] [(lit (array _ (functype ()->())) (funclit ()->() ) nil)]) (assign = [_] [fs])"
    );
    assert_eq!(
        expr_of("func(xs []int, ys [3]int, m map[string]int, p *T, c <-chan int) {}"),
        "(funclit ([xs:(array _ int)][ys:(array 3 int)][m:(map string int)][p:(* T)][c:(chan int)])->() )"
    );
    assert_eq!(expr_of("f[int](x)"), "(call (index f int) x)");
    assert_eq!(
        expr_of("Map[int, string](x)"),
        "(call (index Map int string) x)"
    );
    assert_eq!(expr_of("a[i:j:k]"), "(slice a i j k)");
    assert_eq!(expr_of("a[:]"), "(slice a _ _ _)");
    assert_eq!(expr_of("x.(T)"), "(assert x T)");
    assert_eq!(expr_of("(*T).M"), "(. (paren (* T)) M)");
    assert_eq!(expr_of("[]byte(s)"), "(call (array _ byte) s)");
    assert_eq!(expr_of("f(a, b...)"), "(call f a b ...)");
}

/// DES-619: statements the audit reads as exits and as scopes keep their parts.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-LOOPS-LABELS-AND-DEFERS
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::Parser::parse_for
/// @vtest.intent three-clause, condition-only, infinite and range for statements read their parts, range reads its variables and whether it defines them, a labeled statement keeps its label, break continue and goto keep a label, and go and defer keep the call they start
#[test]
fn loops_labels_exits_and_goroutines_keep_their_parts() {
    assert_eq!(
        body_of("for i := 0; i < n; i++ {\n}"),
        "(for (assign := [i] [0]) (< i n) (incdec i) {})"
    );
    assert_eq!(body_of("for ; ; {\n}"), "(for _ _ _ {})");
    assert_eq!(body_of("for x < 3 {\n}"), "(for _ (< x 3) _ {})");
    assert_eq!(body_of("for {\n}"), "(for _ _ _ {})");
    assert_eq!(body_of("for k, v := range m {\n}"), "(range k v true m {})");
    assert_eq!(body_of("for range ch {\n}"), "(range _ _ false ch {})");
    assert_eq!(body_of("for i := range 10 {\n}"), "(range i _ true 10 {})");
    assert_eq!(body_of("for k = range m {\n}"), "(range k _ false m {})");
    assert_eq!(
        body_of("L:\n\tfor {\n\t\tbreak L\n\t\tcontinue L\n\t}\ngoto L"),
        "(label L (for _ _ _ {(Break L) (Continue L)})) (Goto L)"
    );
    assert_eq!(
        body_of("go f(1)\ndefer func() { recover() }()"),
        "(go (call f 1)) (defer (call (funclit ()->() (expr (call recover ))) ))"
    );
}

/// DES-619, DS-615: text that is not valid Go is refused, never read into a
/// tree, so the audit reports UNKNOWN for it.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-REFUSES-INVALID-TEXT
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax.rs::ParseError
/// @vtest.intent an unterminated string, comment or block, a missing operand, a statement after the function, a for header with two conditions and a stray character are refused as parse errors
#[test]
fn invalid_text_is_refused() {
    assert!(rejects("func f() {\n\tx := \"abc\n}"));
    assert!(rejects("func f() {\n\t/* open\n}"));
    assert!(rejects("func f() {\n\tif x {\n}"));
    assert!(rejects("func f() {\n\tx := 1 +\n}"));
    assert!(rejects("func f() {}\nfunc g() {}"));
    assert!(rejects("func f() {\n\tfor a; b; c; d {\n\t}\n}"));
    assert!(rejects("func f() {\n\tx := 1 # 2\n}"));
    assert!(rejects("func f() {\n\tx := 'ab\n}"));
    assert!(rejects("func f() {\n\tif x == 1 {\n\t} else x = 2\n}"));
    assert!(parse_func_decl("func f() {}").is_ok());
    assert!(parse_func_decl("func f() {}\n").is_ok());
}

/// DES-619: the header of a file gives the package and the imports with the
/// names they are bound to.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-FILE-HEADER
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::parse_file_header
/// @vtest.intent a file header gives its package name and every import with its explicit name, blank and dot names included, and no name for an import without one, from a single import and from a parenthesized group
#[test]
fn the_header_of_a_file_gives_the_package_and_the_imports() {
    let header = parse_file_header(
        "// Package doc.\npackage calc_test\n\nimport \"testing\"\nimport (\n\t\"errors\"\n\tc \"example.com/proj/calc\"\n\t_ \"embed\"\n\t. \"fmt\"\n)\n\nfunc TestX(t *testing.T) {}\n",
    )
    .expect("a header");
    assert_eq!(header.package, "calc_test");
    let imports: Vec<(Option<&str>, &str)> = header
        .imports
        .iter()
        .map(|i| (i.name.as_deref(), i.path.as_str()))
        .collect();
    assert_eq!(
        imports,
        vec![
            (None, "testing"),
            (None, "errors"),
            (Some("c"), "example.com/proj/calc"),
            (Some("_"), "embed"),
            (Some("."), "fmt"),
        ]
    );
    assert!(parse_file_header("import \"x\"\n").is_err());
}

/// DES-619: the reader accepts every file of the Go standard library, which is
/// valid Go that uses the whole grammar (generics, labels, select, struct tags,
/// raw strings, every literal form). A file it refuses fails this test, so a
/// grammar gap shows here and not as an UNKNOWN in a user's project.
/// @vtest.id TEST-ADAPTER-GO-SYNTAX-READS-THE-STANDARD-LIBRARY
/// @vtest.covers VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/syntax/parser.rs::parse_file
/// @vtest.intent every .go file under the src directory of the installed Go distribution, except test data directories, is read by the reader without a parse error
#[test]
fn the_whole_standard_library_is_read() {
    let output = std::process::Command::new("go")
        .args(["env", "GOROOT"])
        .output()
        .expect("`go` must be on PATH: this test reads the Go standard library");
    assert!(output.status.success(), "go env GOROOT failed");
    let goroot = String::from_utf8(output.stdout).expect("UTF-8");
    let src = std::path::Path::new(goroot.trim()).join("src");
    let mut files = Vec::new();
    collect_go_files(&src, &mut files);
    assert!(files.len() > 1000, "too few files: {}", files.len());
    let mut failures = Vec::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        if let Err(error) = parse_file(&text) {
            failures.push(format!("{}: {error}", file.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} files refused:\n{}",
        failures.len(),
        files.len(),
        failures
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn collect_go_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "testdata" && name != "vendor" {
                collect_go_files(&path, out);
            }
        } else if name.ends_with(".go") {
            out.push(path);
        }
    }
}
