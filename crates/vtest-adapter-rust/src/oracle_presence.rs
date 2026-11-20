//! `rust-cargo`'s `oracle_presence` static-analysis capability (DS-605-627,
//! DS-1947, DS-1950-DS-1954, DS-1963, DS-1983).
//!
//! `oracle_presence`'s question (REQ-073/077/079/080, verified present this
//! session) is answered by composing five deterministic, adapter-owned
//! rules — DA-001, DA-003, DA-004, DA-005, DA-006 (DS-605) — over the raw
//! bytes of one Test's construct. Core (`vtest-verify`) never interprets
//! Rust syntax itself (本冊:685-703, AGENTS.md "core owns nothing
//! language-specific"); it only asks this adapter capability for a verdict
//! and composes the five verdicts per DS-606/607/608/609/1680 (DS-1680
//! replaces the retired DS-610 as of the canon's fa63065 merge — verified
//! this session).
//!
//! **Disclosed scope narrowing** (the message that requested this capability
//! explicitly scoped it to "標準assert構文とtest frameworkのfailure
//! semanticsを検出する範囲でよい" — standard assert syntax and framework
//! failure semantics; not full delegation-chain analysis):
//! - DA-003's dataflow tracking is bounded to two shapes: a target call
//!   appearing lexically *inside* an assert-equivalent macro's argument
//!   list, or bound by a single `let NAME = ...;` whose `NAME` then appears
//!   as a single identifier in expression position of a *checked* argument of
//!   an assert-equivalent macro (`NAME`, `NAME.field`, `NAME.method()`, `&NAME`,
//!   `(NAME, ..)`; not `x.NAME`, not a string literal). Checked arguments are
//!   `assert!`'s condition and `assert_eq!`/`assert_ne!`'s two compared values;
//!   the message arguments after them only print on failure and do not count.
//!   The same two shapes also reach a position that decides whether a failure
//!   divergence runs (DS-1947; see the divergence bullets below).
//!   `NAME` is matched as a whole identifier read from the argument list's
//!   tokens, not as a substring. When it cannot be told to mean the binding DA-003
//!   reports `Unknown` (#281; derivations from DS-623 / DS-615 / DS-1680, not
//!   stated in them): the name is bound again in the function (a second `let`, a
//!   closure parameter, a match arm, an assignment `NAME = ..`); the macro is an
//!   `assertion_macros` entry of the config, whose argument grammar vtest cannot
//!   be told (`assert_matches!(expr, pat)`); or the name appears in a nested macro
//!   other than `matches!` (its first expression), `format!`, `vec!` and
//!   `assert!`/`assert_eq!`/`assert_ne!` (read as above), or one of those that does
//!   not parse as expressions (a `debug_assert*!` is not read there: DS-1954,
//!   DS-1962); or the `let mut` binding is
//!   changed through a place other than a projection assignment read below (a
//!   dereference assigned to, a projection whose path is not fixed by syntax, `&mut r`,
//!   `&mut r.p`, `&raw mut r`, a `ref mut` pattern over `r`, `r` as a method receiver
//!   outside an assert, an assignment in a closure, an `async` block or a macro's
//!   tokens, or `r` named in the tokens of a macro that is not read), since the assert
//!   may then read a value that is not the target's result (DS-1959, DS-615). This is a
//!   real subset of
//!   DS-628 ("let束縛、メソッドチェーン、フィールドアクセス"), not its full
//!   extent — a target call several bindings removed, or reached only
//!   through a closure, is not tracked and reports `Unknown` rather than
//!   `NoViolation` (fail-closed: DS-615 "違反なしと推測しない").
//! - DA-003 finds a target's call as a function call (`name(..)`, `Type::name(..)`)
//!   or as a method call (`receiver.name(..)`, #279). The callee is matched by the
//!   final name only; a method call's receiver type is not resolved, so a method
//!   call cannot be told from a same-named method of another type. When every
//!   call of the target that is not verified is such a method call, DA-003 reports
//!   `Unknown` (DS-1680: neither the call's identity nor the result's non-
//!   verification is proven), not `FAIL`. A verified method call is no violation
//!   as before, but only when no function call of the target stays unverified
//!   beside it; otherwise it is `Unknown` too, since the verifying call may be
//!   another type's same-named method. (These are derivations from DS-1680, not
//!   stated in it.) A target declared by `SrcId` has no call symbol, so `RustCargoStaticAnalysisAdapter::
//!   analyze` reports `Unknown` instead of a PASS composed without it.
//!   A call written inside a macro invocation's tokens is not a syntax-tree node
//!   (#282; derivations from DS-615 / DS-651 / DS-1680): in the checked arguments of
//!   `assert!` / `assert_eq!` / `assert_ne!` (read as expressions, with nested
//!   `matches!`, `format!` and `vec!`) it verifies the target only in the positions
//!   `CheckedCallVisitor` lists as carrying its value to the compared value, and a
//!   call in any other position (a block statement, `let _ =`, a closure body, a
//!   loop, a branch condition, ...) makes DA-003 `Unknown` (#289). A `let` name
//!   reaches an assert by the same positions (#289); a call-shaped use of
//!   the target's name (`name(..)`, `name::<T>(..)`) in any other macro, in an
//!   assert's message argument, or in tokens that do not parse as expressions
//!   makes DA-003 `Unknown`. A bare reference or a path prefix (`name::item`) is
//!   not a call there, as it is not a call site outside a macro.
//!   Outside macros, the target's name as an expression path that is not a direct
//!   call's callee (`(name)(..)`, `let f = name;`, `apply(name)`) may be an indirect
//!   call that is not followed, so it makes DA-003 `Unknown` too (#288; derivation
//!   from DS-615 / DS-1680).
//! - §7.2.1's cross-Test delegation-chain search (DA-003/DA-006's "照合の
//!   委譲先") is not implemented: this capability never confirms that a
//!   delegation target terminates (DS-643). For DA-003 it recognizes a delegation
//!   target where the target's result flows into an argument of a call in an
//!   assert's checked argument or in a `let` binding that reaches one (DS-641,
//!   DS-1960): a function call other than a value conversion (`Some` / `Ok` / `Err`,
//!   `Box::new`, `String::from`, DS-1961) or a declared target, or a method call
//!   whose argument is the result (not DS-619's `.expect(..)` / `.expect_err(..)`,
//!   whose argument is a message, DS-1935). When no result of the target's calls
//!   reaches an assert-equivalent other than through such a call, DA-003 is
//!   `Unknown` (DS-648, DS-652); one result that reaches directly decides it, as
//!   derived from DS-623 and DS-1962. A delegation target that the result
//!   does not flow into (`assert_eq!(double(2), expected())`) is not recognized,
//!   which DS-641 reads more widely ("assert相当の引数…に現れる関数呼出"). For DA-006 it only recognizes
//!   that a delegation target exists: a Test body with no assert-equivalent
//!   construct that calls a function or a method has those calls as its
//!   delegation targets (DS-642), and since their termination is never
//!   confirmed here, DA-006 reports `Unknown` for it, not `FAIL` (DS-626,
//!   DS-652, DS-1345). It therefore never reports `NoViolation` through
//!   delegation. A call written inside a macro invocation is not seen
//!   (the macro's tokens are not parsed), so a body whose only calls are
//!   there still reports `FAIL`. DA-003 treats the same way a body with no
//!   assert-equivalent macro and a call other than the declared targets and
//!   the assert-equivalent methods: `Unknown`, not `FAIL` (DS-642, DS-640).
//!   This deviates from DS-616: the "no assert-equivalent construct" premise
//!   looks at assert macros only, not at DS-619/620's `.unwrap()`/`.expect()`/
//!   `?` and Result signature, so it differs from DA-006's reading. The
//!   verdict only moves FAIL -> UNKNOWN. The target's own call is not counted
//!   as a delegation target (a derivation; DA-006 counts it, which differs
//!   from DS-650's self-delegation reading).
//! - A projection assignment `r.p = e` / `r.p op= e` to a `let mut` binding (DS-1956)
//!   keeps the binding's other positions derived when the path `p` is made only of
//!   field names, tuple numbers and integer-literal indexes, the assignment is outside
//!   a closure, an `async` block and macro tokens, and the binding's initializer is the
//!   target's result as a whole (a call of the target, through a method chain, `?`,
//!   `.await` or parentheses, whose positions are all derived: DS-1939). Any other
//!   initializer with such an assignment keeps the whole binding `Unknown`, which is
//!   narrower than DS-1939. A read in an assert's checked argument is then derived
//!   when no assigned path is a prefix of the path it reads (DS-1956). A read of a
//!   value that holds an assigned position is derived only when that value is
//!   compared whole by `assert_eq!` / `assert_ne!` or `==` / `!=` (`assert_eq!(r,
//!   R { .. })`; the false PASS DS-1939 and DS-1956 disclose). As a method receiver
//!   (DS-1939) and in any other use (an operator, `matches!`, `format!`, a
//!   constructor) it is not, nor is a read through an index that is not an integer
//!   literal (DS-1939); the uses other than a method receiver are a narrowing, as
//!   they may read the assigned position alone. A read that leaves an assigned path
//!   at a step of the other kind (an integer-literal index against a field or a
//!   tuple number: `r.0 = 5; assert_eq!(r[0], 5)`, `r[0] = 5; assert_eq!(r.g, 4)`) is
//!   not derived either, as an index of a type other than an array or a slice calls
//!   its `Index` / `IndexMut` method on the whole value; DS-1956 does not tell the two
//!   kinds apart, so this is a narrowing too. Statement order is not read: an assigned
//!   position counts as assigned for every read, before the assignment too, and on
//!   every path, in a branch, a loop or the right side of `&&` / `||` too (DS-1966).
//!   Such a position is never read as derived: the value `e` is not followed, so
//!   `r.f = double(3); assert_eq!(r.f, 6)` is `Unknown` where DS-1956 reads `r.f` as
//!   derived. A compound assignment keeps its position derived (DS-1956) unless a
//!   plain assignment is made to a path that is a prefix of its path or has it as a
//!   prefix, where the order would decide. A compound assignment whose value
//!   absorbs (`r.b &= false`, `r.b |= true`) counts as a plain assignment (DS-1948).
//!   After a projection assignment, a read of the binding in an arm of a branch
//!   expression (`if`, `if let`, `match`) or in the value of a `break` (which joins a
//!   labeled block's value) in an assert is `Unknown` whether or not the arms agree
//!   (narrower than DS-1940). Each of these narrowings only keeps a verdict
//!   `Unknown`. Field names are compared unraw (`r.r#f` and `r.f` are one position).
//!   Not covered: a binding name spelled raw in one place and plain in another
//!   (`let r#r = ..; r.f`), which is matched by its spelling.
//! - DS-1940 is read at every value an assert's checked argument, a deciding position
//!   of a failure divergence or a `let` initializer reaches: an `if` / `if let` /
//!   `match`, and a labeled block, whose exits are its body and the values of the
//!   `break`s that leave it (a derivation: DS-1940 names the branches only). Each arm
//!   other than a failure divergence (DS-1951) is classified at the position read (a
//!   projection of the branch is applied to each arm's value) as derived when a call of
//!   any declared target, a binding derived from one, or the probed binding flows
//!   there. Unless every such arm is derived, what flows through the branch is not
//!   counted as reaching (`Unknown`, never `NoViolation`); a `let` bound to it is
//!   `Unknown` wherever it is used. The value itself is read as before, so this only
//!   moves verdicts toward `Unknown`. Narrower than DS-1940 (each keeps `Unknown`): a
//!   `return Err(..)` arm is not read as a failure divergence; an arm that leaves by
//!   `return` / `break` / `continue`, a missing `else` and a labeled block's body that
//!   always leaves count as not derived; a binding counts as derived only when it is
//!   bound once and not `mut` (its positions are not told apart, as in #315), and a
//!   chain of such bindings is followed one step for a `let` initializer; and an
//!   `assert!` whose condition is `false` in one arm (`if c { x == 4 } else { false }`)
//!   is `Unknown`, as DS-1940 reads the value. A remaining false PASS: an arm read
//!   whole (not through a projection) counts as derived when a derived value flows
//!   anywhere in it, so arms whose positions cross (`if c { (x, 1) } else { (5, x) }`
//!   compared whole) are `NoViolation`, where DS-1940 reads each position; a branch
//!   read through a reference (`(&if ..).1`) is read whole in the same way. The arms
//!   of a pattern's scrutinee
//!   (`ShapeReader`) and of a closure's tail (DS-1942, `tail_is_join`) stay `Unknown` as
//!   before.
//! - DS-1939's per-position derivation is not applied to a binding without a
//!   projection assignment: a binding of a literal that holds the target's result
//!   in one element reaches the assert through any element (`let t = (double(2),
//!   1); assert_eq!(t.1, 1)` is `NoViolation`), a remaining false PASS (#315).
//! - DS-1964 is recognized only where the receiver's variant is fixed by syntax
//!   in the narrowest form: a constructor call (`Some(..).unwrap()`,
//!   `Err(..).unwrap_err()`, `Ok(..)?`), or a name bound exactly once by a plain,
//!   non-`mut`, non-`ref` `let` whose initializer is that constructor. An
//!   `.unwrap()` / `.expect(..)` / `?` on a `let mut` binding, on a name bound by
//!   a pattern (`let (o, _) = (Some(x), 1)`, `if let`, `match`), or on a name bound
//!   again (shadowed or rebound) stays an assert-equivalent (DS-619). DA-006 then
//!   stays `NoViolation` for such a body, e.g. `let mut o = Some(f()); o.unwrap();`,
//!   `let (o, _) = (Some(f()), 1); o.unwrap();`, or
//!   `let o = Some(f()); let o = o; o.unwrap();`.
//! - DA-001's constant-expression classifier only recognizes literals
//!   (numeric/string/bool/char), simple arithmetic/comparison
//!   combinations of literals, and an argument that is a logical expression whose
//!   value literals fix (DS-1949: `X || true`, `true || X`, `X && false`,
//!   `false && X`, the same with `|` / `&`, their negations, and such expressions
//!   nested in one another, parenthesized or not), and a `matches!` whose value
//!   DS-1965 fixes: `true` for a pattern that is irrefutable (DS-1952) with no guard
//!   or a guard fixed to `true`, `false` for a guard fixed to `false`, with their
//!   negations (`assert!(matches!(double(2), _))` is constant, as is
//!   `X || matches!(x, _)`, which absorbs like `X || true`). Syntax does not tell a
//!   struct from an enum variant, nor a binding from a constant or a unit struct or
//!   variant: `S { .. }` and `DaVerdict::Fail(_)` have one form, as do `x` and a
//!   lowercase `const`, and `U` and `E::A`. DS-1952 reads a tuple-struct or struct
//!   pattern of irrefutable parts and a single binding as irrefutable, and a path
//!   pattern or a macro as refutable, but for each of them only the type shows
//!   whether it is (a unit struct `U` is irrefutable for its type). Such a
//!   `matches!` is DS-621's "定数性を確定できない式", so DA-001 escapes to
//!   `Unknown` (DS-621) rather than `Fail` (a real check of a variant) or
//!   `NoViolation` (a pattern no type refutes, which checks nothing). This covers a
//!   tuple-struct, struct or path pattern, a lone identifier as the whole pattern
//!   (`x`, `ref x`, `mut x`, `Kind`), an identifier written like a variant below it
//!   (`(Kind, _)`), a slice of fixed length, a macro, and a tuple, reference or
//!   `or` pattern holding one with no part refutable whatever the type
//!   (`PatternClass::Undecidable`); DA-001 reads such a `matches!` both as `true`
//!   and as its scrutinee's value, and the constancy is undecided where the two
//!   differ (`da_001_argument_constancy`), so `matches!(f(), S { .. }) && g()` stays
//!   non-constant. A non-constant assert-equivalent beside it still makes DA-001
//!   `NoViolation`. `x @ _`, `_` and the like are fixed. A pattern refutable whatever
//!   the type (a literal, a range, `Some` / `None` / `Ok` / `Err` read as the
//!   prelude's variants, or a pattern holding one) and a guard that is not fixed stay
//!   non-constant. Reading an undecidable pattern as `true` is DA-001's only; the
//!   value `fixed_value` gives the other analyses reads it as its scrutinee's value,
//!   as before. One change reaches them through `fixed_value`: an `or` pattern below
//!   the top with an irrefutable alternative (`(S { .. } | _, _)`,
//!   `x @ (E::B(_) | _)`, `&(S { .. } | _)`, `(Some(_) | _, _)`) is irrefutable, as
//!   DS-1952 names it, where `may_be_refuted_by_type` had left it unfixed for the
//!   struct or variant in the other alternative. Its `matches!` is now `true`, so
//!   DA-001 is `Fail` for an assert of it alone, and DA-003 reads
//!   `X || matches!(v, (S { .. } | _, _))` as absorbing (`Unknown`) as for
//!   `X || true`. Whether a divergence is reached after a pattern
//!   (`may_be_refuted_by_type` in DA-003 / DA-006) is read as before. Remaining
//!   limits: a user type named `Some` / `None` / `Ok` / `Err` that shadows the
//!   prelude's is read as the prelude's variant, so a `matches!` over it stays
//!   non-constant; a lowercase identifier below the top of the pattern
//!   (`(four, _)`, `&four`, `x @ four`) is read as a binding, as DS-1952 reads it
//!   for DA-003 and DA-006 too, so a lowercase `const` there makes DA-001 `Fail` on
//!   a real check; and DA-001 is `Unknown`, not `Fail`, for an undecidable pattern
//!   that its type does make irrefutable (a struct, a single-variant enum, a unit
//!   struct, a binding). An operand made only of literals
//!   counts as the value it has: `!`, `^`, `==` / `!=` of `bool` literals and the
//!   comparisons of two literals of one kind (integer, string, char) are
//!   evaluated (`X || 1 == 1` is constant). Float literals are not compared, since
//!   the meaning of the comparison (`f32` or `f64`) is not fixed by syntax. A `bool`
//!   operand made only of literals whose value is not evaluated (`X || 1 + 1 == 2`,
//!   `X || 0.1f32 == 0.100000001f32`) may absorb, so DA-001 is `Unknown` for it
//!   (DS-621's escape). Another form with no variable, call, struct literal, closure
//!   or identifier in macro tokens (`if true { .. }`, `{ true }`, `1 as u8 == 1`,
//!   `matches!(1, 1)`) is built from literals only and is not evaluated, so it may
//!   absorb in the same way. A macro other than `matches!` (`m!()`, `env!("X")`,
//!   `double_m!(2)`) may expand to any code (DS-629), so a form holding one is not
//!   literal-only: as an operand of `||` / `|` / `&&` / `&` it may absorb, so
//!   DA-003 does not read its sibling as flowing, and DA-001 does not decide its
//!   constancy (`Unknown`, never `Fail`). A macro whose tokens hold an identifier
//!   (`cfg!(test)`, `m!(x)`) counts as non-literal like a call, which DS-1948
//!   leaves as a false PASS.
//!   Any other identifier
//!   reference (a `const` item included) is treated as non-constant rather than
//!   resolved, which can only make DA-001 report `NoViolation` where a deeper
//!   analysis might report `Fail`. The classifier reads every argument of an
//!   assert macro, its message arguments included, and `panic!`'s arguments too.
//!   A `.unwrap()` / `.expect(..)` / `?` that DS-619 counts (DS-1964 aside) is a
//!   non-constant assert-equivalent, so DA-001 has no violation beside it
//!   (DS-616, DS-621), but only outside a closure, an `async` block and a nested
//!   item. A position in a branch (`if c { x.unwrap(); }`) counts, as in DS-1962
//!   (derivation from the intent of DS-1962, which names those positions for
//!   DA-002 / DA-003; the canon does not name DA-001's positions). One only in a
//!   closure, an `async` block or a nested item is not counted, and
//!   DA-001 is `Unknown` where it would decide the verdict. A counted one also
//!   clears a constant assert under `#[should_panic]` (`let r = f().unwrap();
//!   assert!(false);`), which widens the false PASS DS-1950 / DS-1964 disclose.
//! - DA-003 does not count an operand of `||` / `|` / `&&` / `&` as a position the
//!   value flows from when the other operand fixes the result to the absorbing value
//!   (DS-1948: `true` for `||` / `|`, `false` for `&&` / `&`), or is made only of
//!   literals whose value is not evaluated (as above). A call there is in a position
//!   that drops its value, so DA-003 is `Unknown` for it. That is a derivation that
//!   follows the other dropping positions here: read plainly, DS-1948 with DS-623
//!   could make it `Fail`, and `Unknown` is the safe side. An operand
//!   beside any other expression flows (`X || false`, `X || ok`); an absorption
//!   that is not by literals (`X || always_true()`, `X * 0`) is not seen, which
//!   DS-1948 leaves as a false PASS.
//! - Failure divergences (DS-1951: `panic!` / `unreachable!` / `todo!` /
//!   `unimplemented!`, `assert!` of a fixed `false`, and `return Err(..)` from a Test
//!   that returns `Result`) are collected with the positions that decide whether
//!   they run (DS-1947, DS-1983): enclosing conditions and guards, the pattern
//!   checks of a scrutinee, `while` conditions, `for` iterated values, and the
//!   positions deciding an earlier `return` / `exit`. A target result that
//!   flows to one of a divergence that is reached counts as verified for DA-003.
//!   A divergence is not reached after an irrefutable arm (DS-1952) or under a
//!   constant `false` (DS-1953), nor after an exit that only constants decide.
//!   Its reach is undecided, and it counts for nothing, in a closure, an `async`
//!   block or a nested item (DS-1962), in the cases of DS-1963 (1) / (2) (also for
//!   the pattern of an `if let` and of a `let ... else` (each with the patterns
//!   refuted on the path to it) and of a `matches!` in a condition, and counting
//!   an arm whose guard is constant `true` as one without a guard), after a `?`, a
//!   `return` / `break` / `continue` in a loop or a macro whose expansion is not
//!   read (DS-1983), after a statement that leaves on every path or whose leaving
//!   is not decided by syntax (`if c { return } else { return }`, a block that ends
//!   in `return`), in a loop whose body assigns a name its condition or iterated
//!   value reads, on the right side of `&&` / `||`, and beside an exit in one
//!   expression.
//! - These narrow DS-1947 / DS-1963 / DS-1983 (each a derivation, toward `Unknown`):
//!   a pattern identifier that starts with an uppercase letter may name a constant
//!   or a variant, so it is undecided; a tuple-struct, struct or fixed-length slice
//!   pattern read as irrefutable may be refuted by its type, so the arms after it
//!   are undecided; a condition or guard made only of literals whose value is not
//!   evaluated (`1 + 1 == 2`) is undecided; a `for` over anything but a range of two
//!   integer literals or an array literal is undecided (the value may be empty); a
//!   pattern that checks a literal element of the scrutinee (`(_, 1)` over `(x, 1)`)
//!   or elements of more than one scrutinee element together (`(4, 2)` over
//!   `(double(2), k)`) leaves its arm and the later ones undecided; a guard that
//!   reads a name its arm binds is a decider whose name the flow does not track;
//!   any method call in a loop body is read as a change of what the condition
//!   reads; a `?` is not a deciding position and whether it leaves is undecided;
//!   and the variant check of a sub-position of a binding that holds the result
//!   only in part is not read.
//! - DA-003 does not count as reached an assert-equivalent (an assert macro, a
//!   `.unwrap()` / `.expect(..)` / `.expect_err(..)` / `.unwrap_err()` / `?` chain, a
//!   deciding position of a failure divergence) that syntax fixes never to run: after
//!   a statement that leaves on every path (`return`, `break`, `continue`, a failure
//!   divergence, both arms of an `if` that leave, also inside a loop body), in a
//!   branch or a `match` arm body that a condition or guard DA-001's classifier fixes
//!   to `false` never takes, in the arms after an irrefutable arm, in the `else` of an
//!   irrefutable `if let` / `let ... else`, and in a `while` / `for` that a constant
//!   never runs. This is a derivation: DS-1952, DS-1953 and DS-1983 state it for
//!   failure divergences only, and DS-1935 reads reach as the flow of a value, which
//!   does not happen where nothing runs. A result that would reach only such an
//!   assert-equivalent makes DA-003 `Unknown`, never `Fail` (DS-615, DS-1680); one that
//!   reaches an assert-equivalent that may run decides as before. Only signals that
//!   would verify are dropped there; one that keeps DA-003 `Unknown` (a branch whose
//!   arms differ, a delegation target, a macro not read) still counts. A statement
//!   holding a labeled block is not read as leaving, since a `break` to that block
//!   ends the block only. An assert under a condition decided at run time
//!   (`if flag() { .. }`) stays a destination, and so do a guard that only a bound
//!   name fixes (`(y, _) if y == 4` over `(4, ..)`), a pattern that earlier arms
//!   cover only pairwise (`Some(5)` after `Some(_)`), a `match` arm after a pattern
//!   that its type may refute, and code after a call of a function named `exit`:
//!   remaining false PASSes. So does code after a form the DS-1983 narrowings above
//!   leave undecided, which is not read as never running: a `return` / `break` /
//!   `continue` in a loop or in a macro whose expansion is not read, a `for` over
//!   anything but a range of two integer literals or an array literal
//!   (`for _ in [0; 0]`), a `macro_rules!` macro that expands to `return`,
//!   `continue 'o`, and `let _: i32 = loop { return; };`.
//! - Over-wide `Unknown` of the rule above: DS-1952 reads a single binding as
//!   irrefutable, so a lowercase pattern identifier that names a constant
//!   (`match 5 { four => {}, _ => assert_eq!(..) }`, `if let four = 5 { return; }`)
//!   is read as irrefutable, the later arms and the statements after it are read as
//!   never running, and DA-003 is `Unknown` where the assert may run.
//! - Remaining false PASSes of DS-1947 / DS-1983 read as written: conditions that
//!   complement each other (`if c {} else if !c {} else { panic!() }`) are not
//!   compared, so the last `else` counts as reached; and an exit decided by the
//!   target's result beside an unrelated non-constant condition
//!   (`if double(2) == 99 { return } if flag() { panic!() }`) makes the divergence
//!   count as reached, though a wrong result may skip it. So do: a pattern refuted
//!   by a statement that leaves for it, and a later check of the value through
//!   another name (`if let Some(_) = o { return }` then `let p = o;` and
//!   `if let None = p`); a check through a method or a macro (`o.is_some()`,
//!   `matches!(o, Some(_))` in place of an `if let`), which refutes nothing; a
//!   pattern whose values several refuted patterns cover only together
//!   (`Some(4) | Some(5)` after arms `Some(4)` and `Some(5)`), or that repeats
//!   values matched before it (inside the `then` of `if let Some(_)`), so that its
//!   own branch never runs; and an arm or an exit at a non-constant position that
//!   is not derived (`let k = 1;`) and always matches or always runs.
//! - The pattern of an `if let` and of a `let ... else` is read by DS-1963 (2) too,
//!   which the canon names for the arms of a `match` only, and so are the patterns
//!   refuted on the path to an `if let`, a `let ... else` or a `match` at any depth
//!   (inside the `else` of an `if let` or a `let ... else`, inside an arm after
//!   earlier arms without a guard, or after an `if let` whose `then` block or a
//!   `match` whose unguarded arms leave on every path): where they may together
//!   match every value at a position, the `else` or the later arms are undecided,
//!   and a branch whose pattern one refuted pattern over the same scrutinee covers
//!   (read pairwise: `Some(5)` after `Some(_)`) is undecided too, also where the value is
//!   not exhausted (`let Some(Kind::A(x)) = f() else { .. }`, whose inner variant
//!   check is opaque). A pattern refuted by an enclosing `else` or arm over a
//!   scrutinee not shown to be the same expression (after parentheses, references
//!   and a block's tail are removed) leaves them undecided too; one refuted by a
//!   statement that leaves takes part only over the same scrutinee.
//! - DA-001 reads a failure divergence by its deciding positions, not by its
//!   arguments (DS-1950): one with no deciding position, or whose deciding positions
//!   DA-001's classifier all fixes, is a constant assertion (`if true { .. }`,
//!   `match r { _ => unreachable!() }`, `let x: i32 = todo!();`, `{ unreachable!()
//!   }`, an unconditional `panic!("{}", x)`). One with a non-constant deciding
//!   position of a reached divergence is not constant. One whose reach is
//!   undecided, or that is not reached while a deciding position is not constant
//!   (`while false { if r != 4 { panic!() } }`), is not decided, so DA-001 is
//!   `Unknown` when it decides the verdict (DS-621's escape). Under
//!   `#[should_panic]` a non-constant deciding position does not decide it either
//!   (`Unknown`): a divergence on every path passes whatever the target returns,
//!   which DS-1950 read alone would leave as a false PASS. DA-001 still counts
//!   only assert macros: an unconditional `return Err(..)` alone is not a constant
//!   assertion here, and `assert_eq!(1, 2)` is classified by its arguments.
//! - `debug_assert!` / `debug_assert_eq!` / `debug_assert_ne!` are not
//!   assert-equivalents, also when the config lists them (DS-1954): they make
//!   DA-003 and DA-006 `Unknown` where they are the only check.
//! - DA-003 counts a target call and an assert-equivalent only at DS-1962's counted
//!   positions: outside the closures, `async` blocks and nested items of the Test
//!   body. An assert, a `.unwrap()` / `.expect(..)` / `?` chain or a binding in one of
//!   those never verifies the target. A target call there makes DA-003 `Unknown`
//!   when no result of a counted call reaches an assert-equivalent, and an
//!   assert-equivalent there keeps DA-003 from `Fail`, since a derived value may flow
//!   into it (DS-1944). DA-001 and DA-006 still read every assert macro.
//! - A name bound by a `match` arm, `if let` / `while let`, `let ... else` or a
//!   destructuring `let` takes the derivation of the position it extracts from the
//!   scrutinee (DS-1938, DS-1939). The scrutinee is read by syntax: a target call's
//!   result is derived at every position, a tuple, array, struct literal and
//!   `Some` / `Ok` / `Err` keep one derivation per position, and a name bound once by a
//!   plain `let` has its initializer's. A name whose position syntax does not fix
//!   over positions of different derivations, or that an or-pattern binds from such
//!   positions, makes DA-003 `Unknown` where it is used.
//! - DA-003 reads a call `f(..)` of a closure bound by `let f = |..| body;` at a
//!   counted position as the execution of its body (DS-1941): before the rules run,
//!   such a call is replaced by a block that binds the parameters to the arguments and
//!   runs the body (`inline_tracked_closures`). A call whose value is used is replaced
//!   only when the body's value comes from its tail alone, not from a `return` or a
//!   branch (DS-1942). Any other closure stays as it is and DS-1944 holds for it. The
//!   other rules read the Test as written.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::{spanned::Spanned, visit::Visit};
use vtest_adapter_api::{
    RuleOutcome, RuleVerdict, StaticAnalysisAdapter, StaticAnalysisInput, StaticAnalysisVerdict,
    TargetReach,
};
use vtest_model::{Diagnostic, Locator, TargetRef};

use crate::ADAPTER_ID;

/// One DA rule's verdict, before DS-606/607/608 composes the five into one
/// `oracle_presence` state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaVerdict {
    /// The rule found no violation (DS-606's "違反なし").
    NoViolation,
    /// The rule's own FAIL condition (verbatim DS id cited per rule) holds.
    Fail(&'static str),
    /// The rule could not decide (its own disclosed UNKNOWN escape, or this
    /// capability's own disclosed narrowing above).
    Unknown(String),
}

/// The five DA rules `oracle_presence` composes (DS-605), each independently
/// evaluated over one Test construct.
#[derive(Clone, Debug)]
pub struct OraclePresenceAnalysis {
    /// DA-001 (DS-621): constant assertion.
    pub da_001: DaVerdict,
    /// DA-003 (DS-623): result unverified.
    pub da_003: DaVerdict,
    /// DA-004 (DS-624): self comparison.
    pub da_004: DaVerdict,
    /// DA-005 (DS-625): empty test.
    pub da_005: DaVerdict,
    /// DA-006 (DS-626): no verification syntax.
    pub da_006: DaVerdict,
}

/// DS-617/618/619/620: `rust-cargo`'s assert-equivalent syntax, shared by
/// all six DA rules (DS-616). DS-617 closes the list of standard macros; any
/// other macro counts only through DS-617's `rust-cargo` config
/// `assertion_macros` list (`extra_assertion_macros`; this capability's caller
/// supplies it; this module does not read `config.yaml` itself).
const STANDARD_ASSERTION_MACROS: &[&str] = &[
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "panic!",
    "unreachable!",
    "todo!",
    "unimplemented!",
];

/// DS-1951's failure divergences among DS-617's macros other than `panic!`. DA-001
/// judges their constancy by their deciding positions (DS-1950), not by their
/// message arguments (`da_001_constant_assertion`).
const DIVERGING_MACROS: &[&str] = &["unreachable", "todo", "unimplemented"];

/// DS-1954: not assert-equivalents, even when the config lists them.
const DEBUG_ASSERT_MACROS: &[&str] = &["debug_assert", "debug_assert_eq", "debug_assert_ne"];

fn is_debug_assert(name: &str) -> bool {
    DEBUG_ASSERT_MACROS.contains(&name)
}

fn is_diverging_macro(name: &str) -> bool {
    DIVERGING_MACROS.contains(&name)
}

/// Runs the five DA rules over `construct_text` (the Test function's raw
/// source bytes — the same bytes `vtest-scan` hashes into the Test subject,
/// read by byte range so this capability never re-parses or re-locates
/// anything core has already located).
///
/// `target_symbols` are the bare identifier names (the final `::`-segment)
/// of the Test's declared targets — DA-003 is evaluated per DS-776 "複数
/// target Testでは…各targetへ個別適用する" (that statement is about
/// DA-002/DA-003; this function folds per-target DA-003 verdicts with the
/// same FAIL-then-UNKNOWN-then-NoViolation priority DS-607/608 use for the
/// outer composition, since DS-605 does not separately name a per-target
/// aggregation rule for DA-003 within one Test — an implementation choice,
/// disclosed here rather than left silent).
pub fn analyze(
    construct_text: &str,
    target_symbols: &[String],
    extra_assertion_macros: &[String],
) -> OraclePresenceAnalysis {
    let macros = assertion_macros(extra_assertion_macros);
    let assert_spans = find_assert_spans(construct_text, &macros);
    let syntax = RustFunctionSyntax::parse(construct_text);
    let has_should_panic = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::has_should_panic);
    let method_or_try = syntax
        .as_ref()
        .map_or(MethodOrTry::None, RustFunctionSyntax::method_or_try);
    let has_assert_equivalent_method_or_try = method_or_try != MethodOrTry::None;
    let has_result_return = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::returns_result);
    let has_verification_syntax = !assert_spans.is_empty()
        || has_should_panic
        || has_assert_equivalent_method_or_try
        || has_result_return;
    let has_call = syntax.as_ref().is_some_and(RustFunctionSyntax::has_call);
    let has_debug_assert = !debug_assert_macros(construct_text).is_empty();

    OraclePresenceAnalysis {
        da_001: da_001_constant_assertion(
            construct_text,
            &assert_spans,
            method_or_try,
            has_should_panic,
        ),
        da_003: da_003_result_unverified(
            construct_text,
            target_symbols,
            target_symbols,
            &macros,
            has_should_panic,
        ),
        da_004: da_004_self_comparison(construct_text),
        da_005: da_005_empty_test(construct_text),
        da_006: da_006_no_verification_syntax(has_verification_syntax, has_debug_assert, has_call),
    }
}

/// DA-003 for one declared target `symbol` of a Test whose declared targets are
/// `all_targets` (DS-630's per-target report): the other targets' calls are not
/// delegation targets (DS-1960), as in `analyze`.
fn analyze_da_003_for(
    construct_text: &str,
    symbol: &str,
    all_targets: &[String],
    extra_assertion_macros: &[String],
) -> DaVerdict {
    let has_should_panic = RustFunctionSyntax::parse(construct_text)
        .as_ref()
        .is_some_and(RustFunctionSyntax::has_should_panic);
    da_003_result_unverified(
        construct_text,
        &[symbol.to_owned()],
        all_targets,
        &assertion_macros(extra_assertion_macros),
        has_should_panic,
    )
}

/// DS-617's macros. A config entry naming a `debug_assert*` macro (by its last
/// path segment, with or without `!`) is left out (DS-1954).
fn assertion_macros(extra_assertion_macros: &[String]) -> Vec<&str> {
    STANDARD_ASSERTION_MACROS
        .iter()
        .copied()
        .chain(
            extra_assertion_macros
                .iter()
                .map(String::as_str)
                .filter(|name| {
                    let last = name.rsplit("::").next().unwrap_or(name);
                    !is_debug_assert(last.trim_end_matches('!'))
                }),
        )
        .collect()
}

/// `vtest_adapter_api::StaticAnalysisAdapter` 実装（DES-408、本冊 §5.2
/// `StaticAnalysisAdapter`）。core（`vtest-verify`）はこの capability を
/// registry 経由で解決し、`analyze` を呼ぶだけで Rust 構文を直接解釈しない
/// （AGENTS.md「core owns nothing language-specific」）。関数本体は、
/// 以前 `vtest-verify::evaluate_oracle_presence` が直接呼んでいた
/// `analyze`/`compose`/`target_symbol` への委譲そのもの — 解析ロジック自体は
/// この PR で変更しない（本 PR は capability trait を通した dispatch への
/// 置き換えのみ）。
#[derive(Clone, Copy, Debug, Default)]
pub struct RustCargoStaticAnalysisAdapter;

impl RustCargoStaticAnalysisAdapter {
    pub const fn new() -> Self {
        Self
    }
}

impl StaticAnalysisAdapter for RustCargoStaticAnalysisAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn analyze(&self, input: StaticAnalysisInput<'_>) -> StaticAnalysisVerdict {
        // DS-628-shaped disclosed narrowing（このmodule冒頭のdoc comment
        // 参照）: `TargetRef::Locator` targetだけがDA-003へsymbol名を渡す。
        // `SrcId` targetはsymbol名を持たないのでDA-003へは渡さない（違反を
        // 捏造しない側への narrowing）。ただし除外したままでは他の規則だけで
        // PASSに合成されるので、下で UNKNOWN に引き上げる。以前は
        // `vtest-verify::evaluate_oracle_presence` がこのfilter_mapを行っていた。
        let target_symbols: Vec<String> = input
            .test
            .targets
            .iter()
            .filter_map(|target| match target {
                TargetRef::Locator(locator) => Some(target_symbol(locator)),
                TargetRef::SrcId(_) => None,
            })
            .collect();
        let analysis = analyze(
            input.construct_text,
            &target_symbols,
            input.extra_assertion_macros,
        );
        let (is_fail, mut is_unknown, mut basis) = compose(&analysis);
        // Derivation (not stated in DS-1680 / DS-615): a SrcId target has no
        // call symbol here, so DA-003 cannot look for its call. That is neither
        // a proof of non-verification nor of a verifying device, which DS-1680
        // reads as UNKNOWN: a would-be PASS becomes UNKNOWN. A FAIL from
        // another rule is unaffected.
        let has_unidentified_target = input
            .test
            .targets
            .iter()
            .any(|target| matches!(target, TargetRef::SrcId(_)));
        if has_unidentified_target && !is_fail && !is_unknown {
            is_unknown = true;
            basis = vec![
                "DA-003: a SrcId target is not resolved to a call symbol by this analysis, so \
                 its call cannot be found and the result being unverified is neither proven \
                 nor refuted (DS-1680)"
                    .to_owned(),
            ];
        }
        if is_fail {
            StaticAnalysisVerdict::Fail(basis)
        } else if is_unknown {
            StaticAnalysisVerdict::Unknown(basis)
        } else {
            StaticAnalysisVerdict::Pass(basis)
        }
    }

    /// DA-002 (DS-622) per declared target, from the Test body and the functions
    /// of its own file that it calls (`crate::target_reach`).
    fn target_reach(&self, input: StaticAnalysisInput<'_>) -> Vec<TargetReach> {
        crate::target_reach::target_reach(input)
    }

    /// SPEC-387/DS-1208: DA-001/004/005/006 are Test-scoped; DA-002 and
    /// DA-003 are reported once per declared target (DS-630). DA-002 comes
    /// from `target_reach`, the verdict `target_binding` reads (DS-675).
    /// `SrcId` targets are not resolved to a symbol here, so their DA-003 is
    /// `UNKNOWN`.
    fn audit_rules(&self, input: StaticAnalysisInput<'_>) -> Vec<RuleVerdict> {
        let symbols: Vec<String> = input
            .test
            .targets
            .iter()
            .filter_map(|target| match target {
                TargetRef::Locator(locator) => Some(target_symbol(locator)),
                TargetRef::SrcId(_) => None,
            })
            .collect();
        let whole = analyze(input.construct_text, &symbols, input.extra_assertion_macros);
        let mut out = Vec::new();
        let push = |out: &mut Vec<RuleVerdict>,
                    rule: &str,
                    target: Option<String>,
                    verdict: &DaVerdict| {
            let (outcome, reason) = match verdict {
                DaVerdict::Fail(reason) => (RuleOutcome::Fail, (*reason).to_owned()),
                DaVerdict::Unknown(reason) => (RuleOutcome::Unknown, reason.clone()),
                DaVerdict::NoViolation => (RuleOutcome::NoViolation, String::new()),
            };
            out.push(RuleVerdict {
                rule: rule.to_owned(),
                target,
                outcome,
                reason,
            });
        };
        push(&mut out, "DA-001", None, &whole.da_001);
        for TargetReach {
            target,
            outcome,
            reason,
        } in self.target_reach(input)
        {
            let label = match &target {
                TargetRef::Locator(locator) => locator.value.clone(),
                TargetRef::SrcId(src_id) => src_id.as_str().to_owned(),
            };
            out.push(RuleVerdict {
                rule: "DA-002".to_owned(),
                target: Some(label.clone()),
                outcome: outcome.into(),
                reason,
            });
            let da_003 = match &target {
                TargetRef::Locator(locator) => analyze_da_003_for(
                    input.construct_text,
                    &target_symbol(locator),
                    &symbols,
                    input.extra_assertion_macros,
                ),
                TargetRef::SrcId(_) => DaVerdict::Unknown(
                    "a SrcId target is not resolved to a call symbol by this analysis".to_owned(),
                ),
            };
            push(&mut out, "DA-003", Some(label), &da_003);
        }
        push(&mut out, "DA-004", None, &whole.da_004);
        push(&mut out, "DA-005", None, &whole.da_005);
        push(&mut out, "DA-006", None, &whole.da_006);
        out
    }
}

/// DS-606/607/608: compose the five DA verdicts into one `oracle_presence`
/// outcome. Returns `(is_fail, is_unknown, basis)` — `vtest-verify` (core)
/// maps that into `VerificationState`/`DiagnosticLabel`, since this crate
/// does not construct core's verification types (core owns that vocabulary).
pub fn compose(analysis: &OraclePresenceAnalysis) -> (bool, bool, Vec<String>) {
    let rules: [(&str, &DaVerdict); 5] = [
        ("DA-001", &analysis.da_001),
        ("DA-003", &analysis.da_003),
        ("DA-004", &analysis.da_004),
        ("DA-005", &analysis.da_005),
        ("DA-006", &analysis.da_006),
    ];
    let mut failures = Vec::new();
    let mut unknowns = Vec::new();
    for (name, verdict) in rules {
        match verdict {
            DaVerdict::Fail(reason) => failures.push(format!("{name}: {reason}")),
            DaVerdict::Unknown(reason) => unknowns.push(format!("{name}: {reason}")),
            DaVerdict::NoViolation => {}
        }
    }
    if !failures.is_empty() {
        return (true, false, failures);
    }
    if !unknowns.is_empty() {
        return (false, true, unknowns);
    }
    (
        false,
        false,
        vec!["DA-001/003/004/005/006 all report no violation".to_owned()],
    )
}

/// Extracts the bare final-segment symbol name from a declared target
/// `Locator` (`rust-cargo`'s own opaque `<path>::<item_path>` value shape —
/// this adapter owns that syntax, core does not, per the crate's own
/// `locator_parts`-style precedent in `vtest-exec`).
pub fn target_symbol(locator: &Locator) -> String {
    locator
        .value
        .rsplit("::")
        .next()
        .unwrap_or(&locator.value)
        .to_owned()
}

// ---------------------------------------------------------------------------
// DA-001 (DS-621): constant assertion
// ---------------------------------------------------------------------------

fn da_001_constant_assertion(
    text: &str,
    assert_spans: &[(usize, usize)],
    method_or_try: MethodOrTry,
    has_should_panic: bool,
) -> DaVerdict {
    if assert_spans.is_empty() {
        // No assert-equivalent macro call at all: DA-001's own subject
        // (an assert whose arguments are constant) does not occur, so
        // there is nothing to fail on. `#[should_panic]`/`.unwrap()`-only
        // verification is DA-003/DA-006's territory, not DA-001's.
        return DaVerdict::NoViolation;
    }
    // DS-1950: the constancy of a DS-1951 divergence is decided by the positions
    // that decide whether it runs (DS-1947, DS-1983), never by its message
    // arguments (`divergence_constancy`).
    //
    // DS-621 fails only when every assert-equivalent is constant, and DS-616 counts
    // DS-619's `.unwrap()` / `.expect(..)` / `?` among them. Such a method or `?` is
    // a non-constant assert-equivalent (this classifier reads any name as
    // non-constant), so DA-001 has no violation beside it, whichever assert macros
    // there are. Derivation from the intent of DS-1962: only one in a position that
    // is outside a closure, an `async` block and a nested item counts, a branch
    // included as in DS-1962 (`MethodOrTry::Counted`).
    if method_or_try == MethodOrTry::Counted {
        return DaVerdict::NoViolation;
    }
    let function = syn::parse_str::<syn::ItemFn>(text).ok();
    let divergences = function
        .as_ref()
        .map(|function| collect_divergences(text, function))
        .unwrap_or_default();
    let mut undetermined = false;
    let mut undetermined_argument = false;
    let mut should_panic_divergence = false;
    for &(start, end) in assert_spans {
        let constancy = if let Some(divergence) = divergences
            .iter()
            .find(|divergence| divergence.start == Some(start))
        {
            match divergence_constancy(divergence) {
                // Under `#[should_panic]` a divergence is the expected outcome, so a
                // non-constant deciding position fixes the result only if some path
                // avoids every divergence, which this analysis does not decide
                // (DS-615, DS-1680): `if double(2) != 4 { panic!() } else { panic!() }`
                // passes whatever `double` returns.
                Constancy::NotAllLiteral if has_should_panic => {
                    should_panic_divergence = true;
                    Constancy::Undetermined
                }
                other => other,
            }
        } else if macro_name_before(text, start)
            .is_some_and(|name| name == "panic" || is_diverging_macro(name))
        {
            // A divergence that the syntax tree walk does not reach (in the tokens of
            // another macro) has deciding positions that are not read (DS-1950).
            Constancy::Undetermined
        } else {
            match classify_constancy(&text[start..end]) {
                Constancy::Undetermined => {
                    undetermined_argument = true;
                    Constancy::AllLiteral
                }
                other => other,
            }
        };
        match constancy {
            Constancy::NotAllLiteral => return DaVerdict::NoViolation,
            Constancy::Undetermined => undetermined = true,
            Constancy::AllLiteral => {}
        }
    }
    if should_panic_divergence {
        return DaVerdict::Unknown(
            "every other assert-equivalent call is constant, and under #[should_panic] a \
             failure divergence is the expected outcome, so whether a non-constant position \
             deciding it fixes the result is not decided (DS-621, DS-1950, DS-1680)"
                .to_owned(),
        );
    }
    if undetermined {
        return DaVerdict::Unknown(
            "every other assert-equivalent call is constant, and whether a failure divergence \
             runs is not fixed by syntax (an exit before it whose condition is not fixed, a \
             loop that may change its condition, a pattern that may name a variant or a \
             constant, a branch of `&&` / `||`, a closure, an `async` block or a nested item), \
             so its constancy is not decided (DS-621, DS-1950, DS-1983)"
                .to_owned(),
        );
    }
    if undetermined_argument {
        return DaVerdict::Unknown(
            "every other assert-equivalent call is constant, and an argument holds a macro \
             whose expansion is not read, an operand of `||` / `|` / `&&` / `&` made only \
             of literals whose value this analysis does not evaluate, or a `matches!` whose \
             pattern (a struct, tuple-struct or path pattern, a lone identifier, a slice of \
             fixed length) only its type shows to be irrefutable or not, so whether it is \
             constant is not decided (DS-621, DS-629, DS-1949, DS-1952, DS-1965)"
                .to_owned(),
        );
    }
    if method_or_try == MethodOrTry::Uncounted {
        return DaVerdict::Unknown(
            "every assert macro's arguments are constant, and a `.unwrap()` / `.expect(..)` / `?` \
             appears only in a closure, an `async` block or a nested item, where whether it runs \
             is not decided, so it is neither counted as a non-constant assert-equivalent nor \
             ruled out (DS-621; positions derived from the intent of DS-1962)"
                .to_owned(),
        );
    }
    // Any identifier reference resolves to `NotAllLiteral` (module doc — this can
    // only under-report `Fail`, never fabricate one).
    DaVerdict::Fail("every assert-equivalent call's arguments are all literal/constant (DS-621)")
}

// ---------------------------------------------------------------------------
// Failure divergences and the positions that decide whether they run
// (DS-1947, DS-1950-DS-1953, DS-1963, DS-1983)
// ---------------------------------------------------------------------------

/// One step from a position to a part of its value, as a pattern reads it.
#[derive(Clone, Debug)]
enum PatternStep {
    Member(syn::Member),
    Index(usize),
}

/// How a deciding position is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeciderKind {
    /// A condition, a guard, a literal pattern, or a `Some` / `None` / `Ok` / `Err`
    /// pattern over a value whose variant is not fixed by syntax.
    Value,
    /// Only a range pattern, or a path or variant pattern other than `Some` /
    /// `None` / `Ok` / `Err` (DS-1963 (1)).
    Opaque,
    /// The variant of a value fixed by syntax (a constructor, or a name bound by a
    /// `let` to one: DS-1947, DS-1964), which no derived value decides.
    FixedVariant,
}

/// A position that decides whether a failure divergence runs (DS-1947, DS-1983):
/// the value of `base` projected by `steps`.
#[derive(Clone, Debug)]
struct Decider<'a> {
    base: &'a syn::Expr,
    steps: Vec<PatternStep>,
    kind: DeciderKind,
}

impl<'a> Decider<'a> {
    fn value(base: &'a syn::Expr) -> Self {
        Self {
            base,
            steps: Vec::new(),
            kind: DeciderKind::Value,
        }
    }
}

/// Whether a failure divergence is reached. The order is the order in which one
/// state overrides another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum DivergenceReach {
    /// It runs when its deciding positions say so.
    Counted,
    /// It never runs (DS-1952, DS-1953).
    NotCounted,
    /// Whether it runs is not fixed by syntax (DS-1962, DS-1963 (2), DS-1983).
    Undecided,
}

/// A failure divergence of a Test body (DS-1951) with its deciding positions.
#[derive(Clone, Debug)]
struct Divergence<'a> {
    /// The start of the macro's arguments, as `find_assert_spans` records it;
    /// `None` for a `return Err(..)`.
    start: Option<usize>,
    deciders: Vec<Decider<'a>>,
    reach: DivergenceReach,
    /// Syntax fixes that it never runs (`DivergenceContext::dead`).
    dead: bool,
}

/// An exit that is not a failure divergence (`return`, `?`, `break`, `continue`, a
/// call of `exit`, a macro whose expansion is not read: DS-1951, DS-1983).
#[derive(Debug)]
struct Exit<'a> {
    deciders: Vec<Decider<'a>>,
    reach: DivergenceReach,
}

#[derive(Clone, Debug)]
struct DivergenceContext<'a> {
    deciders: Vec<Decider<'a>>,
    reach: DivergenceReach,
    in_loop: bool,
    /// The patterns that did not match on the path to here (the `else` of an
    /// `if let`, the `else` of a `let ... else`, the arms of a `match` after its
    /// earlier arms), with their scrutinees as `scrutinee_key` writes them.
    refuted: Vec<(String, &'a syn::Pat)>,
    /// The patterns an earlier statement of an enclosing block refuted by leaving
    /// for them (`exit_refuted`). Only those over the same scrutinee take part.
    left_for: Vec<(String, &'a syn::Pat)>,
    /// Syntax fixes that the code here never runs: after a statement that leaves on
    /// every path, in a branch or an arm that a constant condition (DS-1953) or an
    /// earlier irrefutable arm (DS-1952) never takes, or in a loop that a constant
    /// never runs. Unlike `reach`, it is not raised to `Undecided` by an exit whose
    /// condition is not decided, and it stays set inside everything nested here.
    dead: bool,
}

/// The failure divergences of `function` (DS-1951) with what decides whether they
/// run.
///
/// Derivation from DS-1947, DS-1952, DS-1953, DS-1963 and DS-1983: the walk descends
/// only through statements, `if` / `if let` chains, `match`, `let ... else`,
/// `while` / `while let` / `for` / `loop` and blocks, and through expressions that
/// evaluate their parts in any order. Every other place makes a divergence in it
/// `Undecided`: the right side of `&&` / `||`, a closure, an `async` block and a
/// nested item (DS-1962), an expression that holds both an exit and a divergence
/// (its order is not read), and a pattern with an identifier that may name a unit
/// variant or a constant (`A`: Rust names bindings in lower case, so DS-1952's
/// binding is not told from a path by syntax).
fn collect_divergences<'a>(text: &'a str, function: &'a syn::ItemFn) -> Vec<Divergence<'a>> {
    run_divergence_collector(text, function).0
}

/// The byte spans of the statements and expressions of the Test body that syntax
/// fixes never to run (`DivergenceContext::dead`), outermost only.
pub(crate) fn never_run_regions(text: &str) -> Vec<(usize, usize)> {
    syn::parse_str::<syn::ItemFn>(text)
        .ok()
        .map(|function| run_divergence_collector(text, &function).1)
        .unwrap_or_default()
}

fn run_divergence_collector<'a>(
    text: &'a str,
    function: &'a syn::ItemFn,
) -> (Vec<Divergence<'a>>, Vec<(usize, usize)>) {
    let mut constructed = ConstructorLets(Vec::new());
    constructed.visit_item_fn(function);
    let returns_result = matches!(
        &function.sig.output,
        syn::ReturnType::Type(_, ty) if matches!(
            ty.as_ref(),
            syn::Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "Result")
        )
    );
    let mut collector = DivergenceCollector {
        text,
        returns_result,
        constructed: constructed.0,
        context: DivergenceContext {
            deciders: Vec::new(),
            reach: DivergenceReach::Counted,
            in_loop: false,
            refuted: Vec::new(),
            left_for: Vec::new(),
            dead: false,
        },
        nested: false,
        divergences: Vec::new(),
        exits: Vec::new(),
        dead_spans: Vec::new(),
        in_dead_span: false,
    };
    collector.visit_block(&function.block);
    (collector.divergences, collector.dead_spans)
}

/// The names bound by any `let` to `Some(..)`, `Ok(..)`, `Err(..)` or `None`.
struct ConstructorLets(Vec<String>);

impl<'ast> Visit<'ast> for ConstructorLets {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        let mut pattern = &node.pat;
        while let syn::Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        if let (syn::Pat::Ident(binding), Some(init)) = (pattern, &node.init) {
            if literal_variant(&init.expr).is_some() {
                self.0.push(binding.ident.to_string());
            }
        }
        syn::visit::visit_local(self, node);
    }
}

/// The variant a constructor call `Some(..)` / `Ok(..)` / `Err(..)` or the path
/// `None` builds, through parentheses and references.
fn literal_variant(expression: &syn::Expr) -> Option<String> {
    match strip_place(expression) {
        syn::Expr::Call(call) => constructor_name(call),
        syn::Expr::Path(path)
            if path.qself.is_none()
                && path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "None") =>
        {
            Some("None".to_owned())
        }
        _ => None,
    }
}

/// The expression inside parentheses and references.
fn strip_place(expression: &syn::Expr) -> &syn::Expr {
    match expression {
        syn::Expr::Paren(inner) => strip_place(&inner.expr),
        syn::Expr::Group(inner) => strip_place(&inner.expr),
        syn::Expr::Reference(inner) => strip_place(&inner.expr),
        other => other,
    }
}

struct DivergenceCollector<'a> {
    text: &'a str,
    returns_result: bool,
    constructed: Vec<String>,
    context: DivergenceContext<'a>,
    /// Inside a closure, an `async` block or a nested item, whose `return` does
    /// not leave the Test function.
    nested: bool,
    divergences: Vec<Divergence<'a>>,
    exits: Vec<Exit<'a>>,
    /// The outermost spans visited while `context.dead` holds.
    dead_spans: Vec<(usize, usize)>,
    in_dead_span: bool,
}

/// A scrutinee as compared with the patterns refuted before it: without
/// parentheses, references and a block around a tail expression.
fn scrutinee_key(expression: &syn::Expr) -> String {
    match expression {
        syn::Expr::Paren(inner) => scrutinee_key(&inner.expr),
        syn::Expr::Group(inner) => scrutinee_key(&inner.expr),
        syn::Expr::Reference(inner) => scrutinee_key(&inner.expr),
        syn::Expr::Block(block) if block.label.is_none() => match block.block.stmts.as_slice() {
            [syn::Stmt::Expr(tail, None)] => scrutinee_key(tail),
            _ => expression.to_token_stream().to_string(),
        },
        other => other.to_token_stream().to_string(),
    }
}

impl<'a> DivergenceCollector<'a> {
    fn within(
        &mut self,
        deciders: Vec<Decider<'a>>,
        reach: DivergenceReach,
        in_loop: bool,
        visit: impl FnOnce(&mut Self),
    ) {
        let outer = self.context.clone();
        self.context.deciders.extend(deciders);
        self.context.reach = self.context.reach.max(reach);
        self.context.in_loop |= in_loop;
        visit(self);
        self.context = outer;
    }

    /// Runs `visit` with `refuted` added to the patterns that did not match on the
    /// path to here.
    fn refuting(&mut self, refuted: Vec<(String, &'a syn::Pat)>, visit: impl FnOnce(&mut Self)) {
        let outer = self.context.refuted.len();
        self.context.refuted.extend(refuted);
        visit(self);
        self.context.refuted.truncate(outer);
    }

    /// Whether one pattern refuted on the path to here (or in `earlier`) over
    /// `scrutinee` matches every value `pattern` matches, so that its branch never
    /// runs (derivation from DS-1952 / DS-1963 toward `Unknown`). An irrefutable one
    /// is left to DS-1952, which does not count what follows it.
    fn covered_by_refuted(
        &self,
        scrutinee: &str,
        earlier: &[(String, &'a syn::Pat)],
        pattern: &syn::Pat,
    ) -> bool {
        self.context
            .refuted
            .iter()
            .chain(&self.context.left_for)
            .chain(earlier)
            .any(|(other, refuted)| {
                other == scrutinee && !is_irrefutable(refuted) && pattern_covers(refuted, pattern)
            })
    }

    /// DS-1963 (2): whether `patterns` over `scrutinee`, together with the patterns
    /// that did not match on the path to here (at any depth: an `if let`, a
    /// `let ... else` or a `match` inside the `else` or a later arm of another, or
    /// after a statement that leaves for them), may match every value, so that a
    /// branch for the values they leave may never run. A pattern refuted by an
    /// enclosing `else` or arm over a scrutinee not shown to be this one may take
    /// part too, so it counts as one that may.
    fn may_exhaust_with_refuted(
        &self,
        scrutinee: &str,
        patterns: impl IntoIterator<Item = &'a syn::Pat>,
    ) -> bool {
        self.context
            .refuted
            .iter()
            .any(|(other, _)| other != scrutinee)
            || patterns_may_exhaust(
                self.context
                    .refuted
                    .iter()
                    .chain(
                        self.context
                            .left_for
                            .iter()
                            .filter(|(other, _)| other == scrutinee),
                    )
                    .map(|(_, pattern)| *pattern)
                    .chain(patterns),
            )
    }

    fn nesting(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.nested;
        self.nested = true;
        self.within(Vec::new(), DivergenceReach::Undecided, false, visit);
        self.nested = outer;
    }

    fn record_divergence(&mut self, start: Option<usize>) {
        self.divergences.push(Divergence {
            start,
            deciders: self.context.deciders.clone(),
            reach: self.context.reach,
            dead: self.context.dead,
        });
    }

    /// Runs `visit` with the code it visits marked as never running when `dead`.
    fn dead_if(&mut self, dead: bool, visit: impl FnOnce(&mut Self)) {
        let outer = self.context.dead;
        self.context.dead |= dead;
        visit(self);
        self.context.dead = outer;
    }

    /// Visits a statement or an expression, recording its span when it never runs.
    fn spanned(&mut self, span: proc_macro2::Span, visit: impl FnOnce(&mut Self)) {
        if self.context.dead && !self.in_dead_span {
            self.dead_spans.push((
                span_offset(span.start(), self.text),
                span_offset(span.end(), self.text),
            ));
            self.in_dead_span = true;
            visit(self);
            self.in_dead_span = false;
        } else {
            visit(self);
        }
    }

    /// Records an exit taken where the current context says. DS-1983: one in a loop
    /// body is not decided by syntax.
    fn record_exit(&mut self) {
        if self.nested {
            return;
        }
        let deciders = self.context.deciders.clone();
        let reach = if self.context.in_loop {
            DivergenceReach::Undecided
        } else {
            self.context.reach
        };
        self.exits.push(Exit { deciders, reach });
    }

    fn record_undecided_exit(&mut self) {
        if !self.nested {
            self.exits.push(Exit {
                deciders: Vec::new(),
                reach: DivergenceReach::Undecided,
            });
        }
    }

    /// What the exits recorded from `mark` on add to the code that runs after them
    /// (DS-1983). An exit that never runs adds nothing.
    fn exits_since(&self, mark: usize) -> (Vec<Decider<'a>>, DivergenceReach) {
        let mut deciders = Vec::new();
        let mut reach = DivergenceReach::Counted;
        for exit in &self.exits[mark..] {
            match exit.reach {
                DivergenceReach::Counted => deciders.extend(exit.deciders.iter().cloned()),
                DivergenceReach::NotCounted => {}
                DivergenceReach::Undecided => reach = DivergenceReach::Undecided,
            }
        }
        (deciders, reach)
    }

    /// The positions a pattern's refutable parts check in `scrutinee` (DS-1947).
    fn pattern_deciders(
        &self,
        pattern: &syn::Pat,
        scrutinee: &'a syn::Expr,
    ) -> (Vec<Decider<'a>>, DivergenceReach) {
        let mut deciders = Vec::new();
        self.pattern_checks(pattern, scrutinee, Vec::new(), &mut deciders);
        // A check of a literal element of the scrutinee (`(_, 1)` over `(x, 1)`) may
        // always or never match, which is not evaluated; checks of several elements
        // of a literal scrutinee (`(4, 2)` over `(double(2), k)`) match only together,
        // and one of them may not be derived. Whether the pattern matches is then not
        // decided by a derived value (DS-1947, DS-615), so it is undecided.
        let literal_element = deciders.iter().any(|decider| {
            decider.kind != DeciderKind::FixedVariant
                && decider.steps.is_empty()
                && !matches!(argument_constancy(decider.base), Constancy::NotAllLiteral)
        });
        let mut bases: Vec<*const syn::Expr> = deciders
            .iter()
            .filter(|decider| decider.kind != DeciderKind::FixedVariant)
            .map(|decider| std::ptr::from_ref(decider.base))
            .collect();
        bases.sort();
        bases.dedup();
        let reach = if has_variant_like_ident(pattern)
            || (is_irrefutable(pattern) && may_be_refuted_by_type(pattern))
            || literal_element
            || bases.len() > 1
        {
            DivergenceReach::Undecided
        } else {
            DivergenceReach::Counted
        };
        (deciders, reach)
    }

    fn pattern_checks(
        &self,
        pattern: &syn::Pat,
        base: &'a syn::Expr,
        steps: Vec<PatternStep>,
        out: &mut Vec<Decider<'a>>,
    ) {
        if is_irrefutable(pattern) {
            return;
        }
        let opaque = |out: &mut Vec<Decider<'a>>, steps: Vec<PatternStep>| {
            out.push(Decider {
                base,
                steps,
                kind: DeciderKind::Opaque,
            });
        };
        match pattern {
            syn::Pat::Lit(_) => out.push(Decider {
                base,
                steps,
                kind: DeciderKind::Value,
            }),
            syn::Pat::Ident(ident) => match &ident.subpat {
                Some((_, sub)) => self.pattern_checks(sub, base, steps, out),
                None if ident.ident == "None" => {
                    self.variant_check("None", base, steps, out);
                }
                None => opaque(out, steps),
            },
            syn::Pat::Path(path)
                if path.qself.is_none()
                    && path
                        .path
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == "None") =>
            {
                self.variant_check("None", base, steps, out);
            }
            syn::Pat::Or(or) => {
                for case in &or.cases {
                    self.pattern_checks(case, base, steps.clone(), out);
                }
            }
            syn::Pat::Paren(inner) => self.pattern_checks(&inner.pat, base, steps, out),
            syn::Pat::Reference(inner) => self.pattern_checks(&inner.pat, base, steps, out),
            syn::Pat::Type(inner) => self.pattern_checks(&inner.pat, base, steps, out),
            syn::Pat::Tuple(tuple) => self.element_checks(&tuple.elems, true, base, &steps, out),
            syn::Pat::TupleStruct(node) => {
                match option_or_result_variant(&node.path) {
                    Some(name) => {
                        if !self.variant_check(name, base, steps.clone(), out) {
                            return;
                        }
                    }
                    None => opaque(out, steps.clone()),
                }
                self.element_checks(&node.elems, true, base, &steps, out);
            }
            syn::Pat::Struct(node) => {
                opaque(out, steps.clone());
                for field in &node.fields {
                    let (base, steps) =
                        descend(base, &steps, PatternStep::Member(field.member.clone()));
                    self.pattern_checks(&field.pat, base, steps, out);
                }
            }
            syn::Pat::Slice(slice) => self.element_checks(&slice.elems, false, base, &steps, out),
            // A range, a path other than `None`, a constant block, a macro: DS-1963 (1).
            _ => opaque(out, steps),
        }
    }

    /// The checks of the elements of a tuple, tuple-struct or slice pattern. An
    /// element after `..` is at a position fixed only when the value is a literal.
    fn element_checks(
        &self,
        elements: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>,
        tuple: bool,
        base: &'a syn::Expr,
        steps: &[PatternStep],
        out: &mut Vec<Decider<'a>>,
    ) {
        let rest = elements
            .iter()
            .position(|element| matches!(element, syn::Pat::Rest(_)));
        let literal_length = steps.is_empty().then(|| match strip_place(base) {
            syn::Expr::Tuple(literal) if tuple => Some(literal.elems.len()),
            syn::Expr::Array(literal) if !tuple => Some(literal.elems.len()),
            _ => None,
        });
        for (index, element) in elements.iter().enumerate() {
            if Some(index) == rest || is_irrefutable(element) {
                continue;
            }
            let position = match rest {
                Some(rest) if index > rest => literal_length
                    .flatten()
                    .and_then(|length| length.checked_sub(elements.len() - index)),
                _ => Some(index),
            };
            let Some(position) = position else {
                out.push(Decider {
                    base,
                    steps: steps.to_vec(),
                    kind: DeciderKind::Opaque,
                });
                continue;
            };
            let step = if tuple {
                PatternStep::Member(syn::Member::Unnamed(syn::Index {
                    index: u32::try_from(position).unwrap_or(u32::MAX),
                    span: proc_macro2::Span::call_site(),
                }))
            } else {
                PatternStep::Index(position)
            };
            let (base, steps) = descend(base, steps, step);
            self.pattern_checks(element, base, steps, out);
        }
    }

    /// Records the check of the variant `name` at a position. Returns whether the
    /// sub-patterns may match: not when a constructor of another variant is there.
    fn variant_check(
        &self,
        name: &str,
        base: &'a syn::Expr,
        steps: Vec<PatternStep>,
        out: &mut Vec<Decider<'a>>,
    ) -> bool {
        if steps.is_empty() {
            let fixed = literal_variant(base).or_else(|| match strip_place(base) {
                syn::Expr::Path(path) if path.qself.is_none() => path
                    .path
                    .get_ident()
                    .filter(|ident| self.constructed.iter().any(|bound| *ident == bound))
                    .map(|_| String::new()),
                _ => None,
            });
            if let Some(variant) = fixed {
                out.push(Decider {
                    base,
                    steps,
                    kind: DeciderKind::FixedVariant,
                });
                return variant.is_empty() || variant == name;
            }
        }
        out.push(Decider {
            base,
            steps,
            kind: DeciderKind::Value,
        });
        true
    }
}

/// `Some`, `Ok` or `Err` when a path names that variant by its final segment
/// (DS-1963, DS-1967).
fn option_or_result_variant(path: &syn::Path) -> Option<&'static str> {
    let last = path.segments.last()?;
    ["Some", "Ok", "Err"]
        .into_iter()
        .find(|name| last.ident == name)
}

/// The position one `step` below (`base`, `steps`), through a literal tuple, array,
/// struct or `Some` / `Ok` / `Err` constructor when it is one.
fn descend<'a>(
    base: &'a syn::Expr,
    steps: &[PatternStep],
    step: PatternStep,
) -> (&'a syn::Expr, Vec<PatternStep>) {
    if steps.is_empty() {
        let element = match (strip_place(base), &step) {
            (syn::Expr::Tuple(literal), PatternStep::Member(syn::Member::Unnamed(index))) => {
                literal.elems.iter().nth(index.index as usize)
            }
            (syn::Expr::Array(literal), PatternStep::Index(index)) => {
                literal.elems.iter().nth(*index)
            }
            (syn::Expr::Struct(literal), PatternStep::Member(member)) => literal
                .fields
                .iter()
                .find(|field| field.member == *member)
                .map(|field| &field.expr),
            (syn::Expr::Call(call), PatternStep::Member(syn::Member::Unnamed(index)))
                if index.index == 0 && call.args.len() == 1 && constructor_name(call).is_some() =>
            {
                call.args.first()
            }
            _ => None,
        };
        if let Some(element) = element {
            return (element, Vec::new());
        }
    }
    let mut steps = steps.to_vec();
    steps.push(step);
    (base, steps)
}

/// DS-1952's irrefutable pattern. An identifier written like a unit variant or a
/// constant (`None`, `A`) is not read as a binding (`has_variant_like_ident`).
fn is_irrefutable(pattern: &syn::Pat) -> bool {
    match pattern {
        syn::Pat::Wild(_) | syn::Pat::Rest(_) => true,
        syn::Pat::Ident(ident) => match &ident.subpat {
            Some((_, sub)) => is_irrefutable(sub),
            None => !is_variant_like(&ident.ident),
        },
        syn::Pat::Or(or) => or.cases.iter().any(is_irrefutable),
        syn::Pat::Paren(inner) => is_irrefutable(&inner.pat),
        syn::Pat::Reference(inner) => is_irrefutable(&inner.pat),
        syn::Pat::Type(inner) => is_irrefutable(&inner.pat),
        syn::Pat::Tuple(tuple) => tuple.elems.iter().all(is_irrefutable),
        syn::Pat::Slice(slice) => slice.elems.iter().all(is_irrefutable),
        syn::Pat::TupleStruct(node) => {
            option_or_result_variant(&node.path).is_none() && node.elems.iter().all(is_irrefutable)
        }
        syn::Pat::Struct(node) => {
            option_or_result_variant(&node.path).is_none()
                && node.fields.iter().all(|field| is_irrefutable(&field.pat))
        }
        _ => false,
    }
}

fn is_variant_like(ident: &syn::Ident) -> bool {
    ident == "None"
        || ident
            .to_string()
            .starts_with(|c: char| c.is_ascii_uppercase())
}

/// Whether a pattern holds an identifier other than `None` written like a unit
/// variant or a constant, which may be a path pattern or a binding.
fn has_variant_like_ident(pattern: &syn::Pat) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            self.0 |= node.subpat.is_none() && node.ident != "None" && is_variant_like(&node.ident);
            syn::visit::visit_pat_ident(self, node);
        }
    }
    let mut finder = Finder(false);
    finder.visit_pat(pattern);
    finder.0
}

/// Whether a pattern that DS-1952 reads as irrefutable may still be refuted by its
/// type: a tuple-struct or struct pattern whose path may name an enum variant, or a
/// slice pattern that fixes a length. DS-1952 reads them as irrefutable so that the
/// divergence after them is not reached; DA-001 would then read one in their scope
/// as constant, which their type may make false (DS-1680), so it is undecided.
fn may_be_refuted_by_type(pattern: &syn::Pat) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_pat_tuple_struct(&mut self, node: &'ast syn::PatTupleStruct) {
            self.0 = true;
            syn::visit::visit_pat_tuple_struct(self, node);
        }
        fn visit_pat_struct(&mut self, node: &'ast syn::PatStruct) {
            self.0 = true;
            syn::visit::visit_pat_struct(self, node);
        }
        fn visit_pat_slice(&mut self, node: &'ast syn::PatSlice) {
            let rest_only = node.elems.len() == 1
                && node.elems.iter().all(|element| match element {
                    syn::Pat::Rest(_) => true,
                    syn::Pat::Ident(ident) => {
                        matches!(&ident.subpat, Some((_, sub)) if matches!(sub.as_ref(), syn::Pat::Rest(_)))
                    }
                    _ => false,
                });
            self.0 |= !rest_only;
            syn::visit::visit_pat_slice(self, node);
        }
    }
    let mut finder = Finder(false);
    finder.visit_pat(pattern);
    finder.0
}

/// Which branches of a condition may run: (the `true` branch, the `false` branch).
/// DS-1953: a branch that a constant condition never takes is not reached. A
/// condition made only of literals whose value is not evaluated here (`1 + 1 == 3`,
/// `Fixed::Literal`), or holding a macro whose expansion is not read, may be either,
/// and a `let` chain is not read, so both are undecided.
fn condition_reach(condition: &syn::Expr) -> (DivergenceReach, DivergenceReach) {
    if contains_let(condition) || holds_matches_that_may_always_match(condition) {
        return (DivergenceReach::Undecided, DivergenceReach::Undecided);
    }
    match fixed_value(condition) {
        Fixed::Value(true) => (DivergenceReach::Counted, DivergenceReach::NotCounted),
        Fixed::Value(false) => (DivergenceReach::NotCounted, DivergenceReach::Counted),
        Fixed::Literal | Fixed::Opaque => (DivergenceReach::Undecided, DivergenceReach::Undecided),
        Fixed::Constant | Fixed::Open => (DivergenceReach::Counted, DivergenceReach::Counted),
    }
}

/// Whether control may leave a statement or an expression at its end (DS-1983),
/// read from syntax only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Completion {
    /// Some path reaches the end.
    Falls,
    /// Every path leaves (an exit or a failure divergence).
    Leaves,
    /// Not decided by syntax.
    Unknown,
}

impl Completion {
    /// Either of two paths runs.
    fn either(self, other: Self) -> Self {
        match (self, other) {
            (Self::Falls, _) | (_, Self::Falls) => Self::Falls,
            (Self::Leaves, Self::Leaves) => Self::Leaves,
            _ => Self::Unknown,
        }
    }

    /// One runs, then the other.
    fn then(self, next: impl FnOnce() -> Self) -> Self {
        match self {
            Self::Leaves => Self::Leaves,
            Self::Falls => next(),
            Self::Unknown => match next() {
                Self::Leaves => Self::Leaves,
                Self::Falls | Self::Unknown => Self::Unknown,
            },
        }
    }
}

fn block_completion(block: &syn::Block) -> Completion {
    block
        .stmts
        .iter()
        .fold(Completion::Falls, |done, statement| {
            done.then(|| statement_completion(statement))
        })
}

fn statement_completion(statement: &syn::Stmt) -> Completion {
    match statement {
        syn::Stmt::Local(local) => local
            .init
            .as_ref()
            .map_or(Completion::Falls, |init| expression_completion(&init.expr)),
        syn::Stmt::Item(_) => Completion::Falls,
        syn::Stmt::Expr(expression, _) => expression_completion(expression),
        syn::Stmt::Macro(node) => macro_completion(&node.mac),
    }
}

/// Whether a statement holds, outside closures, `async` blocks and items, a labeled
/// block (`statement_completion` reads a `break` to it as leaving the statement) or a
/// `cfg` / `cfg_attr` attribute (the part it marks may not be compiled, which syntax
/// does not decide).
fn holds_labeled_block_or_cfg(statement: &syn::Stmt) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_expr_block(&mut self, node: &'ast syn::ExprBlock) {
            self.0 |= node.label.is_some();
            syn::visit::visit_expr_block(self, node);
        }
        fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
            self.0 |= is_cfg_attribute(node);
        }
        fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _node: &'ast syn::ExprAsync) {}
        fn visit_item(&mut self, _node: &'ast syn::Item) {}
    }
    let mut finder = Finder(false);
    finder.visit_stmt(statement);
    finder.0
}

fn is_cfg_attribute(attribute: &syn::Attribute) -> bool {
    attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
}

fn macro_completion(node: &syn::Macro) -> Completion {
    if is_failure_divergence_macro(node) {
        Completion::Leaves
    } else if macro_may_exit(node) {
        Completion::Unknown
    } else {
        Completion::Falls
    }
}

/// Derivation from DS-1983: an expression falls through when some path of its
/// structure reaches its end. A part that may hold an exit in an order this does
/// not read, or an arm that may not be reachable (DS-1952, DS-1963 (2)), is unknown.
fn expression_completion(expression: &syn::Expr) -> Completion {
    let open = |part: &syn::Expr| {
        if expression_may_exit(part) {
            Completion::Unknown
        } else {
            Completion::Falls
        }
    };
    match expression {
        syn::Expr::Return(_) | syn::Expr::Break(_) | syn::Expr::Continue(_) => Completion::Leaves,
        syn::Expr::Paren(inner) => expression_completion(&inner.expr),
        syn::Expr::Group(inner) => expression_completion(&inner.expr),
        syn::Expr::Block(block) => block_completion(&block.block),
        syn::Expr::Unsafe(block) => block_completion(&block.block),
        syn::Expr::Macro(node) => macro_completion(&node.mac),
        syn::Expr::If(node) => {
            let condition = match unwrap_parens(&node.cond) {
                syn::Expr::Let(binding) => open(&binding.expr),
                condition => open(condition),
            };
            let then = block_completion(&node.then_branch);
            let other = node
                .else_branch
                .as_ref()
                .map_or(Completion::Falls, |(_, other)| expression_completion(other));
            let taken = match unwrap_parens(&node.cond) {
                syn::Expr::Let(binding) if is_irrefutable(&binding.pat) => then,
                syn::Expr::Let(_) => then.either(other),
                condition => match fixed_value(condition) {
                    Fixed::Value(true) => then,
                    Fixed::Value(false) => other,
                    Fixed::Literal | Fixed::Opaque | Fixed::Constant | Fixed::Open => {
                        then.either(other)
                    }
                },
            };
            condition.then(|| taken)
        }
        syn::Expr::Match(node) => {
            let mut arms = Completion::Leaves;
            for (index, arm) in node.arms.iter().enumerate() {
                let body = expression_completion(&arm.body);
                let body = if body == Completion::Falls && arms_may_exhaust(&node.arms[..index]) {
                    Completion::Unknown
                } else {
                    body
                };
                arms = arms.either(body);
                if arm.guard.is_none() && is_irrefutable(&arm.pat) {
                    break;
                }
            }
            open(&node.expr).then(|| arms)
        }
        syn::Expr::While(node) => match unwrap_parens(&node.cond) {
            syn::Expr::Let(binding) => open(&binding.expr),
            condition => open(condition),
        },
        syn::Expr::ForLoop(node) => open(&node.expr),
        syn::Expr::Loop(_) => Completion::Unknown,
        syn::Expr::Closure(_) | syn::Expr::Async(_) => Completion::Falls,
        other => open(other),
    }
}

/// Whether an expression holds, outside closures, `async` blocks and items, an exit
/// or a failure divergence: `return`, `break`, `continue`, `?`, a call of `exit`, a
/// failure divergence macro or a macro that may exit.
fn expression_may_exit(expression: &syn::Expr) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_expr_return(&mut self, _node: &'ast syn::ExprReturn) {
            self.0 = true;
        }
        fn visit_expr_break(&mut self, _node: &'ast syn::ExprBreak) {
            self.0 = true;
        }
        fn visit_expr_continue(&mut self, _node: &'ast syn::ExprContinue) {
            self.0 = true;
        }
        fn visit_expr_try(&mut self, _node: &'ast syn::ExprTry) {
            self.0 = true;
        }
        fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = node.func.as_ref() {
                self.0 |= path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "exit");
            }
            syn::visit::visit_expr_call(self, node);
        }
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            self.0 |= macro_completion(node) != Completion::Falls;
        }
        fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _node: &'ast syn::ExprAsync) {}
        fn visit_item(&mut self, _node: &'ast syn::Item) {}
    }
    let mut finder = Finder(false);
    finder.visit_expr(expression);
    finder.0
}

/// Whether a condition holds a `matches!(value, pattern)` whose pattern (without a
/// guard) is irrefutable (DS-1952) or may match every value (DS-1963 (2)): it is
/// then `true` whatever the value, so the branches it decides are undecided. A
/// guarded one is not held here: `fixed_value` fixes it only where DS-1965 does.
fn holds_matches_that_may_always_match(condition: &syn::Expr) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            match parse_matches(node) {
                None | Some(Ok((_, Some(_)))) => {}
                Some(Ok((pattern, None))) => {
                    self.0 |= is_irrefutable(&pattern) || patterns_may_exhaust([&pattern]);
                }
                Some(Err(_)) => self.0 = true,
            }
        }
        fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}
    }
    let mut finder = Finder(false);
    finder.visit_expr(condition);
    finder.0
}

/// Whether the body of `for .. in iterated` may run. A range of two integer
/// literals and an array literal are evaluated; any other value may be empty or not
/// (a target result that is empty skips the body whatever its checks say), so it is
/// undecided. Derivation from DS-1953 and DS-1983: a loop that a constant never runs
/// does not run its body.
fn loop_reach(iterated: &syn::Expr) -> DivergenceReach {
    let integer =
        |expression: &Option<Box<syn::Expr>>| match expression.as_deref().map(unwrap_parens) {
            Some(syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(literal),
                ..
            })) => literal.base10_parse::<i128>().ok(),
            _ => None,
        };
    match unwrap_parens(iterated) {
        syn::Expr::Range(range) => match (integer(&range.start), integer(&range.end)) {
            (Some(start), Some(end)) => {
                let empty = match range.limits {
                    syn::RangeLimits::HalfOpen(_) => start >= end,
                    syn::RangeLimits::Closed(_) => start > end,
                };
                if empty {
                    DivergenceReach::NotCounted
                } else {
                    DivergenceReach::Counted
                }
            }
            _ => DivergenceReach::Undecided,
        },
        syn::Expr::Array(array) if array.elems.is_empty() => DivergenceReach::NotCounted,
        syn::Expr::Array(_) => DivergenceReach::Counted,
        _ => DivergenceReach::Undecided,
    }
}

/// What a pattern checks at one position, for DS-1963 (2).
#[derive(PartialEq, Eq)]
enum PatternAtom {
    Opaque,
    Bool(bool),
    Variant(&'static str),
    Literal(String),
}

/// DS-1963 (2): whether the arms without a guard before a failure divergence's arm
/// may already match every value at one position. An arm whose guard is constant
/// `true` counts as one without a guard.
fn arms_may_exhaust(arms: &[syn::Arm]) -> bool {
    patterns_may_exhaust(
        arms.iter()
            .filter(|arm| {
                arm.guard
                    .as_ref()
                    .is_none_or(|(_, guard)| fixed_value(guard) == Fixed::Value(true))
            })
            .map(|arm| &arm.pat),
    )
}

/// The patterns a statement refutes for the statements after it: those of an
/// `if let` whose `then` block leaves on every path, and of the unguarded arms of
/// a `match` whose bodies do, also inside a block statement or a `let`
/// initializer (DS-1983).
fn exit_refuted<'a>(statement: &'a syn::Stmt, refuted: &mut Vec<(String, &'a syn::Pat)>) {
    match statement {
        syn::Stmt::Expr(expression, _) => expression_exit_refuted(expression, refuted),
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                expression_exit_refuted(&init.expr, refuted);
            }
        }
        syn::Stmt::Item(_) | syn::Stmt::Macro(_) => {}
    }
}

fn expression_exit_refuted<'a>(
    expression: &'a syn::Expr,
    refuted: &mut Vec<(String, &'a syn::Pat)>,
) {
    match expression {
        syn::Expr::Paren(inner) => expression_exit_refuted(&inner.expr, refuted),
        syn::Expr::Group(inner) => expression_exit_refuted(&inner.expr, refuted),
        syn::Expr::Block(block) if block.label.is_none() => {
            for statement in &block.block.stmts {
                exit_refuted(statement, refuted);
            }
        }
        syn::Expr::If(node) => {
            if let syn::Expr::Let(binding) = unwrap_parens(&node.cond) {
                if block_completion(&node.then_branch) == Completion::Leaves {
                    refuted.push((scrutinee_key(&binding.expr), &binding.pat));
                }
            }
        }
        syn::Expr::Match(node) => {
            for arm in &node.arms {
                if arm.guard.is_none() && expression_completion(&arm.body) == Completion::Leaves {
                    refuted.push((scrutinee_key(&node.expr), &arm.pat));
                }
            }
        }
        _ => {}
    }
}

/// Whether every value `pattern` matches is matched by `by`, read pairwise from
/// syntax; any pair this does not read is not covered.
fn pattern_covers(by: &syn::Pat, pattern: &syn::Pat) -> bool {
    let strip = |pattern: &syn::Pat| -> syn::Pat {
        let mut pattern = pattern;
        loop {
            pattern = match pattern {
                syn::Pat::Paren(inner) => &inner.pat,
                syn::Pat::Reference(inner) => &inner.pat,
                syn::Pat::Type(inner) => &inner.pat,
                other => return other.clone(),
            };
        }
    };
    let (by, pattern) = (strip(by), strip(pattern));
    let tokens = |pattern: &syn::Pat| pattern.to_token_stream().to_string();
    let all = |by: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>,
               pattern: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>| {
        by.len() == pattern.len()
            && by
                .iter()
                .chain(pattern)
                .all(|item| !matches!(item, syn::Pat::Rest(_)))
            && by
                .iter()
                .zip(pattern)
                .all(|(by, item)| pattern_covers(by, item))
    };
    if is_irrefutable(&by) && !may_be_refuted_by_type(&by) {
        return true;
    }
    match (&by, &pattern) {
        (syn::Pat::Or(by), _) => by.cases.iter().any(|case| pattern_covers(case, &pattern)),
        (_, syn::Pat::Or(or)) => or.cases.iter().all(|case| pattern_covers(&by, case)),
        (_, syn::Pat::Ident(ident)) if ident.subpat.is_some() => ident
            .subpat
            .as_ref()
            .is_some_and(|(_, sub)| pattern_covers(&by, sub)),
        (syn::Pat::Ident(ident), _) if ident.subpat.is_some() => ident
            .subpat
            .as_ref()
            .is_some_and(|(_, sub)| pattern_covers(sub, &pattern)),
        (syn::Pat::TupleStruct(by), syn::Pat::TupleStruct(other)) => {
            by.path.to_token_stream().to_string() == other.path.to_token_stream().to_string()
                && all(&by.elems, &other.elems)
        }
        (syn::Pat::Tuple(by), syn::Pat::Tuple(other)) => all(&by.elems, &other.elems),
        (syn::Pat::Lit(_) | syn::Pat::Path(_) | syn::Pat::Range(_), _) => {
            tokens(&by) == tokens(&pattern)
        }
        _ => false,
    }
}

/// DS-1963 (2) over patterns: whether they may together match every value at one
/// position. It also reads the pattern of an `if let` and of a `let ... else`
/// (with the patterns refuted on the path to it), whose `else` then may never run.
fn patterns_may_exhaust<'p>(patterns: impl IntoIterator<Item = &'p syn::Pat>) -> bool {
    let mut atoms: Vec<(Vec<String>, PatternAtom)> = Vec::new();
    for pattern in patterns {
        pattern_atoms(pattern, &mut Vec::new(), &mut atoms);
    }
    let mut positions: Vec<&Vec<String>> = atoms.iter().map(|(position, _)| position).collect();
    positions.sort();
    positions.dedup();
    positions.into_iter().any(|position| {
        let at: Vec<&PatternAtom> = atoms
            .iter()
            .filter(|(other, _)| other == position)
            .map(|(_, atom)| atom)
            .collect();
        let has = |atom: &PatternAtom| at.contains(&atom);
        let mut literals: Vec<&String> = at
            .iter()
            .filter_map(|atom| match atom {
                PatternAtom::Literal(literal) => Some(literal),
                _ => None,
            })
            .collect();
        literals.sort();
        literals.dedup();
        has(&PatternAtom::Opaque)
            || (has(&PatternAtom::Bool(true)) && has(&PatternAtom::Bool(false)))
            || (has(&PatternAtom::Variant("Some")) && has(&PatternAtom::Variant("None")))
            || (has(&PatternAtom::Variant("Ok")) && has(&PatternAtom::Variant("Err")))
            || literals.len() >= 256
    })
}

fn pattern_atoms(
    pattern: &syn::Pat,
    position: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, PatternAtom)>,
) {
    let mut push = |atom: PatternAtom, position: &Vec<String>| out.push((position.clone(), atom));
    match pattern {
        syn::Pat::Wild(_) | syn::Pat::Rest(_) => {}
        syn::Pat::Lit(literal) => match &literal.lit {
            syn::Lit::Bool(value) => push(PatternAtom::Bool(value.value), position),
            other => push(
                PatternAtom::Literal(other.to_token_stream().to_string()),
                position,
            ),
        },
        syn::Pat::Ident(ident) => match &ident.subpat {
            Some((_, sub)) => pattern_atoms(sub, position, out),
            None if ident.ident == "None" => push(PatternAtom::Variant("None"), position),
            None if is_variant_like(&ident.ident) => push(PatternAtom::Opaque, position),
            None => {}
        },
        syn::Pat::Path(path)
            if path.qself.is_none()
                && path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "None") =>
        {
            push(PatternAtom::Variant("None"), position);
        }
        syn::Pat::Or(or) => {
            for case in &or.cases {
                pattern_atoms(case, position, out);
            }
        }
        syn::Pat::Paren(inner) => pattern_atoms(&inner.pat, position, out),
        syn::Pat::Reference(inner) => pattern_atoms(&inner.pat, position, out),
        syn::Pat::Type(inner) => pattern_atoms(&inner.pat, position, out),
        syn::Pat::Tuple(tuple) => element_atoms(&tuple.elems, ".", position, out),
        syn::Pat::TupleStruct(node) => {
            match option_or_result_variant(&node.path) {
                Some(name) => push(PatternAtom::Variant(name), position),
                None => push(PatternAtom::Opaque, position),
            }
            element_atoms(&node.elems, ".", position, out);
        }
        syn::Pat::Struct(node) => {
            push(PatternAtom::Opaque, position);
            for field in &node.fields {
                position.push(format!(".{}", field.member.to_token_stream()));
                pattern_atoms(&field.pat, position, out);
                position.pop();
            }
        }
        syn::Pat::Slice(slice) => element_atoms(&slice.elems, "[", position, out),
        _ => push(PatternAtom::Opaque, position),
    }
}

fn element_atoms(
    elements: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>,
    kind: &str,
    position: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, PatternAtom)>,
) {
    let mut after_rest = false;
    for (index, element) in elements.iter().enumerate() {
        if matches!(element, syn::Pat::Rest(_)) {
            after_rest = true;
            continue;
        }
        // An element after `..` is keyed by its place from the end.
        let from_end = elements.len() - index;
        position.push(if after_rest {
            format!("{kind}-{from_end}")
        } else {
            format!("{kind}{index}")
        });
        pattern_atoms(element, position, out);
        position.pop();
    }
}

/// Whether `body` assigns to, takes `&mut` of, or calls a method on a variable that
/// `read` names (DS-1983: the loop's repeating is then not fixed by syntax). A
/// method call is counted because it may take `&mut self`.
fn assigns_read_names(body: &syn::Block, read: &syn::Expr) -> bool {
    struct Names(Vec<syn::Ident>);
    impl<'ast> Visit<'ast> for Names {
        fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
            if let (None, Some(ident)) = (&node.qself, node.path.get_ident()) {
                self.0.push(ident.clone());
            }
        }
    }
    struct Changes<'n> {
        names: &'n [syn::Ident],
        found: bool,
    }
    impl Changes<'_> {
        fn roots_at_name(&self, place: &syn::Expr) -> bool {
            self.names
                .iter()
                .any(|name| place_roots_at(place, &name.to_string()))
        }
    }
    impl<'ast> Visit<'ast> for Changes<'_> {
        fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
            self.found |= self.roots_at_name(&node.left);
            syn::visit::visit_expr_assign(self, node);
        }
        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            self.found |= is_compound_assignment(node.op) && self.roots_at_name(&node.left);
            syn::visit::visit_expr_binary(self, node);
        }
        fn visit_expr_reference(&mut self, node: &'ast syn::ExprReference) {
            self.found |= node.mutability.is_some() && self.roots_at_name(&node.expr);
            syn::visit::visit_expr_reference(self, node);
        }
        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            self.found |= self.roots_at_name(&node.receiver);
            syn::visit::visit_expr_method_call(self, node);
        }
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            self.found |= self
                .names
                .iter()
                .any(|name| tokens_mention_ident(node.tokens.clone(), &name.to_string()));
        }
    }
    let mut names = Names(Vec::new());
    names.visit_expr(read);
    if names.0.is_empty() {
        return false;
    }
    let mut changes = Changes {
        names: &names.0,
        found: false,
    };
    changes.visit_block(body);
    changes.found
}

/// Whether an expression holds a `let` (a `let` chain in a condition).
fn contains_let(expression: &syn::Expr) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_expr_let(&mut self, _node: &'ast syn::ExprLet) {
            self.0 = true;
        }
        fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}
    }
    let mut finder = Finder(false);
    finder.visit_expr(expression);
    finder.0
}

/// DS-1951's failure divergence among macros: `panic!` / `unreachable!` / `todo!` /
/// `unimplemented!`, an `assert!` whose condition literals fix to `false`, and an
/// `assert_eq!` / `assert_ne!` whose comparison of its two values they fix to
/// `false` (DS-1951 counts every assert-equivalent the DA-001 classifier fixes).
fn is_failure_divergence_macro(node: &syn::Macro) -> bool {
    let Some(name) = node
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
    else {
        return false;
    };
    let comparison = match name.as_str() {
        "assert_eq" => Some(syn::BinOp::Eq(Default::default())),
        "assert_ne" => Some(syn::BinOp::Ne(Default::default())),
        _ => None,
    };
    if let Some(op) = comparison {
        return macro_arguments(node).is_some_and(|arguments| match &arguments.checked[..] {
            [left, right] => {
                fixed_value(&syn::Expr::Binary(syn::ExprBinary {
                    attrs: Vec::new(),
                    left: Box::new(left.clone()),
                    op,
                    right: Box::new(right.clone()),
                })) == Fixed::Value(false)
            }
            _ => false,
        });
    }
    name == "panic"
        || is_diverging_macro(&name)
        || (name == "assert"
            && macro_arguments(node)
                .and_then(|arguments| arguments.checked.first().map(fixed_value))
                == Some(Fixed::Value(false)))
}

/// Whether a macro may leave the function: one whose expansion is not read here, or
/// one of those named here whose tokens hold an exit (DS-1983, DS-615).
fn macro_may_exit(node: &syn::Macro) -> bool {
    let known = node.path.segments.last().is_some_and(|segment| {
        let name = segment.ident.to_string();
        STANDARD_ASSERTION_MACROS.contains(&format!("{name}!").as_str())
            || is_debug_assert(&name)
            || matches!(
                name.as_str(),
                "format" | "print" | "println" | "eprint" | "eprintln" | "vec" | "matches"
            )
    });
    !known || tokens_may_exit(node.tokens.clone())
}

impl<'a> Visit<'a> for DivergenceCollector<'a> {
    fn visit_block(&mut self, node: &'a syn::Block) {
        let outer = self.context.clone();
        for statement in &node.stmts {
            let mark = self.exits.len();
            let known = self.context.deciders.len();
            self.visit_stmt(statement);
            // DS-1983: an exit in a statement decides whether the later ones run. One
            // that is reached and that only constant conditions of its own decide
            // (DS-1953) leaves the rest of the block unreached.
            let unconditional = self.exits[mark..].iter().any(|exit| {
                exit.reach == DivergenceReach::Counted
                    && exit.deciders.iter().skip(known).all(|decider| {
                        decider.steps.is_empty()
                            && matches!(fixed_value(decider.base), Fixed::Value(_))
                    })
            });
            let (deciders, reach) = self.exits_since(mark);
            self.context.deciders.extend(deciders);
            self.context.reach = self.context.reach.max(reach);
            if unconditional {
                self.context.reach = self.context.reach.max(DivergenceReach::NotCounted);
            } else if self.exits[mark..]
                .iter()
                .any(|exit| exit.reach == DivergenceReach::Counted)
                && statement_completion(statement) != Completion::Falls
            {
                // A statement that leaves on every path, or whose leaving is not
                // decided by syntax, leaves the later ones undecided, however its
                // positions are decided (DS-1983, DS-615).
                self.context.reach = self.context.reach.max(DivergenceReach::Undecided);
            }
            // A statement after one that leaves on every path never runs (derivation
            // from DS-1983, which reads the exits before a construct). A `break` to a
            // labeled block that the statement holds may end that block only, and a
            // part under `cfg` may not be compiled, so such a statement is not read.
            if statement_completion(statement) == Completion::Leaves
                && !holds_labeled_block_or_cfg(statement)
            {
                self.context.dead = true;
            }
            exit_refuted(statement, &mut self.context.left_for);
        }
        self.context = outer;
    }

    fn visit_stmt(&mut self, node: &'a syn::Stmt) {
        self.spanned(node.span(), |visitor| syn::visit::visit_stmt(visitor, node));
    }

    fn visit_expr(&mut self, node: &'a syn::Expr) {
        self.spanned(node.span(), |visitor| {
            let structured = matches!(
            node,
            syn::Expr::If(_)
                | syn::Expr::Match(_)
                | syn::Expr::While(_)
                | syn::Expr::ForLoop(_)
                | syn::Expr::Loop(_)
                | syn::Expr::Block(_)
                | syn::Expr::Unsafe(_)
                | syn::Expr::Const(_)
                | syn::Expr::Closure(_)
                | syn::Expr::Async(_)
                | syn::Expr::Return(_)
                | syn::Expr::Break(_)
                | syn::Expr::Continue(_)
                | syn::Expr::Try(_)
                | syn::Expr::Macro(_)
                | syn::Expr::Paren(_)
                | syn::Expr::Group(_)
        ) || matches!(node, syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_) | syn::BinOp::Or(_)));
            if structured {
                return syn::visit::visit_expr(visitor, node);
            }
            // The order in which an expression evaluates its parts is not read, so a
            // divergence beside an exit in it is not decided.
            let (divergences, exits) = (visitor.divergences.len(), visitor.exits.len());
            syn::visit::visit_expr(visitor, node);
            if visitor.exits[exits..]
                .iter()
                .any(|exit| exit.reach != DivergenceReach::NotCounted)
            {
                for divergence in &mut visitor.divergences[divergences..] {
                    divergence.reach = DivergenceReach::Undecided;
                }
            }
        });
    }

    fn visit_expr_if(&mut self, node: &'a syn::ExprIf) {
        let mark = self.exits.len();
        let (deciders, then_reach, else_reach, refuted) = match unwrap_parens(&node.cond) {
            syn::Expr::Let(binding) => {
                self.visit_expr(&binding.expr);
                let (deciders, reach) = self.pattern_deciders(&binding.pat, &binding.expr);
                let scrutinee = scrutinee_key(&binding.expr);
                let reach = if self.covered_by_refuted(&scrutinee, &[], &binding.pat) {
                    DivergenceReach::Undecided
                } else {
                    reach
                };
                let else_reach = if is_irrefutable(&binding.pat) {
                    reach.max(DivergenceReach::NotCounted)
                } else if self.may_exhaust_with_refuted(&scrutinee, [&*binding.pat]) {
                    DivergenceReach::Undecided
                } else {
                    reach
                };
                (
                    deciders,
                    reach,
                    else_reach,
                    vec![(scrutinee, &*binding.pat)],
                )
            }
            condition => {
                self.visit_expr(condition);
                let (then_reach, else_reach) = condition_reach(condition);
                (
                    vec![Decider::value(condition)],
                    then_reach,
                    else_reach,
                    Vec::new(),
                )
            }
        };
        let (mut exit_deciders, exit_reach) = self.exits_since(mark);
        exit_deciders.extend(deciders);
        // A branch that a constant condition (DS-1953) or an irrefutable pattern
        // (DS-1952) never takes never runs.
        self.within(
            exit_deciders.clone(),
            then_reach.max(exit_reach),
            false,
            |visitor| {
                visitor.dead_if(then_reach == DivergenceReach::NotCounted, |visitor| {
                    visitor.visit_block(&node.then_branch);
                });
            },
        );
        if let Some((_, other)) = &node.else_branch {
            self.within(
                exit_deciders,
                else_reach.max(exit_reach),
                false,
                |visitor| {
                    visitor.dead_if(else_reach == DivergenceReach::NotCounted, |visitor| {
                        visitor.refuting(refuted, |visitor| visitor.visit_expr(other));
                    });
                },
            );
        }
    }

    fn visit_expr_match(&mut self, node: &'a syn::ExprMatch) {
        let mark = self.exits.len();
        self.visit_expr(&node.expr);
        let scrutinee = scrutinee_key(&node.expr);
        let (mut deciders, mut reach) = self.exits_since(mark);
        // The patterns of the earlier arms that a value reaching this arm did not
        // match: those without a guard, or with a guard that is always `true`.
        let mut refuted: Vec<(String, &'a syn::Pat)> = Vec::new();
        // The arms after an irrefutable arm (DS-1952) never run.
        let mut after_irrefutable = false;
        for (index, arm) in node.arms.iter().enumerate() {
            let (checks, pattern_reach) = self.pattern_deciders(&arm.pat, &node.expr);
            deciders.extend(checks);
            reach = reach.max(pattern_reach);
            let mut arm_reach = reach;
            if let Some((_, guard)) = &arm.guard {
                deciders.push(Decider::value(guard));
                arm_reach = arm_reach.max(condition_reach(guard).0);
            }
            if (index > 0
                && self.may_exhaust_with_refuted(
                    &scrutinee,
                    refuted.iter().map(|(_, pattern)| *pattern),
                ))
                || self.covered_by_refuted(&scrutinee, &refuted, &arm.pat)
            {
                arm_reach = DivergenceReach::Undecided;
            }
            // A guard that is constant `false` (DS-1953) runs, but the body never does.
            let guard_false = arm
                .guard
                .as_ref()
                .is_some_and(|(_, guard)| condition_reach(guard).0 == DivergenceReach::NotCounted);
            self.within(deciders.clone(), arm_reach, false, |visitor| {
                visitor.dead_if(after_irrefutable, |visitor| {
                    visitor.refuting(refuted.clone(), |visitor| {
                        if let Some((_, guard)) = &arm.guard {
                            visitor.visit_expr(guard);
                        }
                        visitor.dead_if(guard_false, |visitor| visitor.visit_expr(&arm.body));
                    });
                });
            });
            if arm
                .guard
                .as_ref()
                .is_none_or(|(_, guard)| fixed_value(guard) == Fixed::Value(true))
            {
                refuted.push((scrutinee.clone(), &arm.pat));
            }
            // DS-1952: the arms after an irrefutable arm never run. A pattern read as
            // irrefutable that the type may refute, or a guard made of literals whose
            // value is not evaluated, leaves the later arms undecided.
            if is_irrefutable(&arm.pat) {
                let later = match arm.guard.as_ref().map(|(_, guard)| fixed_value(guard)) {
                    None | Some(Fixed::Value(true)) => DivergenceReach::NotCounted,
                    Some(Fixed::Literal | Fixed::Opaque) => DivergenceReach::Undecided,
                    Some(Fixed::Value(false) | Fixed::Constant | Fixed::Open) => {
                        DivergenceReach::Counted
                    }
                };
                let later =
                    if later == DivergenceReach::NotCounted && may_be_refuted_by_type(&arm.pat) {
                        DivergenceReach::Undecided
                    } else {
                        later
                    };
                // An arm under `cfg` may not be compiled.
                after_irrefutable |=
                    later == DivergenceReach::NotCounted && !arm.attrs.iter().any(is_cfg_attribute);
                reach = reach.max(later);
            }
        }
    }

    fn visit_expr_while(&mut self, node: &'a syn::ExprWhile) {
        let mark = self.exits.len();
        let (deciders, reach, read) = match unwrap_parens(&node.cond) {
            syn::Expr::Let(binding) => {
                self.visit_expr(&binding.expr);
                let (deciders, reach) = self.pattern_deciders(&binding.pat, &binding.expr);
                (deciders, reach, binding.expr.as_ref())
            }
            condition => {
                self.visit_expr(condition);
                let (reach, _) = condition_reach(condition);
                (vec![Decider::value(condition)], reach, condition)
            }
        };
        // A body that a constant condition never enters never runs (DS-1953).
        let never = reach == DivergenceReach::NotCounted;
        let reach = if assigns_read_names(&node.body, read) {
            DivergenceReach::Undecided
        } else {
            reach
        };
        let (mut exit_deciders, exit_reach) = self.exits_since(mark);
        exit_deciders.extend(deciders);
        self.within(exit_deciders, reach.max(exit_reach), true, |visitor| {
            visitor.dead_if(never, |visitor| visitor.visit_block(&node.body));
        });
    }

    fn visit_expr_for_loop(&mut self, node: &'a syn::ExprForLoop) {
        let mark = self.exits.len();
        self.visit_expr(&node.expr);
        // A body over a literal range or array that is empty never runs.
        let never = loop_reach(&node.expr) == DivergenceReach::NotCounted;
        let reach = if assigns_read_names(&node.body, &node.expr) {
            DivergenceReach::Undecided
        } else {
            loop_reach(&node.expr)
        };
        let (mut deciders, exit_reach) = self.exits_since(mark);
        deciders.push(Decider::value(&node.expr));
        self.within(deciders, reach.max(exit_reach), true, |visitor| {
            visitor.dead_if(never, |visitor| visitor.visit_block(&node.body));
        });
    }

    fn visit_expr_loop(&mut self, node: &'a syn::ExprLoop) {
        self.within(Vec::new(), DivergenceReach::Counted, true, |visitor| {
            visitor.visit_block(&node.body);
        });
    }

    fn visit_expr_closure(&mut self, node: &'a syn::ExprClosure) {
        self.nesting(|visitor| syn::visit::visit_expr_closure(visitor, node));
    }

    fn visit_expr_async(&mut self, node: &'a syn::ExprAsync) {
        self.nesting(|visitor| syn::visit::visit_expr_async(visitor, node));
    }

    fn visit_item(&mut self, node: &'a syn::Item) {
        self.nesting(|visitor| syn::visit::visit_item(visitor, node));
    }

    fn visit_local(&mut self, node: &'a syn::Local) {
        let Some(init) = &node.init else {
            return;
        };
        let mark = self.exits.len();
        self.visit_expr(&init.expr);
        if let Some((_, diverge)) = &init.diverge {
            let (mut deciders, reach) = self.pattern_deciders(&node.pat, &init.expr);
            let scrutinee = scrutinee_key(&init.expr);
            // DS-1952: the `else` of an irrefutable pattern never runs; that of a
            // pattern that may match every value may not (DS-1963 (2)).
            let reach = if is_irrefutable(&node.pat) {
                reach.max(DivergenceReach::NotCounted)
            } else if self.may_exhaust_with_refuted(&scrutinee, [&node.pat]) {
                DivergenceReach::Undecided
            } else {
                reach
            };
            let (exit_deciders, exit_reach) = self.exits_since(mark);
            deciders.extend(exit_deciders);
            let never = reach == DivergenceReach::NotCounted;
            self.within(deciders, reach.max(exit_reach), false, |visitor| {
                visitor.dead_if(never, |visitor| {
                    visitor.refuting(vec![(scrutinee, &node.pat)], |visitor| {
                        visitor.visit_expr(diverge);
                    });
                });
            });
        }
    }

    fn visit_expr_binary(&mut self, node: &'a syn::ExprBinary) {
        if matches!(node.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            self.visit_expr(&node.left);
            self.within(Vec::new(), DivergenceReach::Undecided, false, |visitor| {
                visitor.visit_expr(&node.right);
            });
        } else {
            syn::visit::visit_expr_binary(self, node);
        }
    }

    fn visit_expr_return(&mut self, node: &'a syn::ExprReturn) {
        if let Some(value) = &node.expr {
            self.visit_expr(value);
        }
        // DS-1951 (DS-620): `return Err(..)` from a Test that returns `Result` is a
        // failure divergence; any other `return` is an exit (DS-1983).
        let returns_err = self.returns_result
            && node.expr.as_deref().is_some_and(|value| {
                matches!(unwrap_parens(value), syn::Expr::Call(call) if constructor_name(call).as_deref() == Some("Err"))
            });
        if returns_err && !self.nested {
            self.record_divergence(None);
        } else {
            self.record_exit();
        }
    }

    fn visit_expr_break(&mut self, node: &'a syn::ExprBreak) {
        syn::visit::visit_expr_break(self, node);
        self.record_undecided_exit();
    }

    fn visit_expr_continue(&mut self, _node: &'a syn::ExprContinue) {
        self.record_undecided_exit();
    }

    /// `?` leaves the function when its operand is an `Err` / `None`, and may also
    /// convert it (`From`), so its operand is not a deciding position and whether
    /// it leaves is not decided (DS-1983).
    fn visit_expr_try(&mut self, node: &'a syn::ExprTry) {
        self.visit_expr(&node.expr);
        self.record_undecided_exit();
    }

    // DS-1951 names `process::exit` among the exits; any callee named `exit` is read
    // as one.
    fn visit_expr_call(&mut self, node: &'a syn::ExprCall) {
        syn::visit::visit_expr_call(self, node);
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "exit")
            {
                self.record_exit();
            }
        }
    }

    fn visit_macro(&mut self, node: &'a syn::Macro) {
        if is_failure_divergence_macro(node) {
            if let Some(start) = macro_argument_start(self.text, node) {
                self.record_divergence(Some(start));
            }
        } else if macro_may_exit(node) {
            self.record_undecided_exit();
        }
    }
}

/// DS-1950: a failure divergence is constant when DA-001's classifier fixes every
/// position that decides whether it runs, none included. One that never runs
/// (DS-1952, DS-1953) is constant only then; a non-constant position of it is not
/// shown to make it non-constant, as it does not run. One whose running is not
/// fixed by syntax is not decided (DS-1983).
fn divergence_constancy(divergence: &Divergence<'_>) -> Constancy {
    if divergence.reach == DivergenceReach::Undecided {
        return Constancy::Undetermined;
    }
    let mut constancy = Constancy::AllLiteral;
    for decider in &divergence.deciders {
        match da_001_argument_constancy(decider.base) {
            Constancy::AllLiteral => {}
            Constancy::Undetermined => {
                if matches!(constancy, Constancy::AllLiteral) {
                    constancy = Constancy::Undetermined;
                }
            }
            Constancy::NotAllLiteral => {
                return if divergence.reach == DivergenceReach::Counted {
                    Constancy::NotAllLiteral
                } else {
                    Constancy::Undetermined
                };
            }
        }
    }
    constancy
}

/// Visits the value at a deciding position with `visitor`.
fn visit_decider(visitor: &mut CheckedCallVisitor<'_>, decider: &Decider<'_>) {
    if decider.steps.is_empty() {
        visitor.visit_expr(decider.base);
        return;
    }
    let steps: Vec<Projection<'_>> = decider
        .steps
        .iter()
        .map(|step| match step {
            PatternStep::Member(member) => Projection::Member(member),
            PatternStep::Index(index) => Projection::Index(Some(*index)),
        })
        .collect();
    visitor.visit_projected(decider.base, &steps);
}

/// The deciding positions of the divergences that may run when they say
/// (DS-1947), checked so that a derived value there decides: `Value` positions.
fn counted_deciders<'d, 'a>(
    divergences: &'d [Divergence<'a>],
) -> impl Iterator<Item = &'d Decider<'a>> {
    divergences
        .iter()
        .filter(|divergence| divergence.reach == DivergenceReach::Counted)
        .flat_map(|divergence| &divergence.deciders)
        .filter(|decider| decider.kind == DeciderKind::Value)
}

/// How calls of the target reach the deciding positions of the failure divergences
/// (DS-1947), read as `CheckedCallVisitor` reads an assert's checked argument.
#[derive(Default)]
struct DecidedCalls {
    path: bool,
    method: bool,
    delegated: bool,
    /// A call flows there only through a branch whose arms differ (DS-1940).
    mixed: bool,
}

fn divergence_target_reach(
    divergences: &[Divergence<'_>],
    names: &NameUse<'_>,
    targets: &[String],
    sources: &JoinSources,
) -> DecidedCalls {
    let mut reach = DecidedCalls::default();
    for decider in counted_deciders(divergences) {
        let mut visitor = CheckedCallVisitor::new(Some(names), None, targets, sources);
        visit_decider(&mut visitor, decider);
        reach.path |= visitor.path;
        reach.method |= visitor.method;
        reach.delegated |= visitor.delegated;
        reach.mixed |= visitor.mixed;
    }
    reach
}

/// How a binding reaches the deciding positions of the failure divergences
/// (DS-1947), one outcome per position. A pattern that checks a part of a binding
/// that is not the target's result as a whole is not shown to read a derived part
/// (DS-1939), so it is `Uncertain`.
fn divergence_binding_reach(
    divergences: &[Divergence<'_>],
    binding: &DerivedBinding,
    targets: &[String],
    sources: &JoinSources,
    overwrites: &[Vec<PlaceStep>],
) -> Vec<BindingReach> {
    let name = binding.name.as_str();
    counted_deciders(divergences)
        .map(|decider| {
            let mut visitor = CheckedCallVisitor::new(None, Some(name), targets, sources);
            visitor.probe_overwrites = overwrites;
            visit_decider(&mut visitor, decider);
            let outcome = probe_reach(&visitor);
            let projects_binding = !decider.steps.is_empty()
                && matches!(strip_place(decider.base), syn::Expr::Path(path) if path.qself.is_none() && path.path.is_ident(name));
            if projects_binding
                && !binding.whole_result
                && !matches!(outcome, BindingReach::NotReached)
            {
                BindingReach::Uncertain
            } else {
                outcome
            }
        })
        .collect()
}

/// Whether macro tokens hold `?`, `return`, `break` or `continue` outside literals.
fn tokens_may_exit(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Group(group) => tokens_may_exit(group.stream()),
        proc_macro2::TokenTree::Punct(punct) => punct.as_char() == '?',
        proc_macro2::TokenTree::Ident(ident) => {
            ident == "return" || ident == "break" || ident == "continue"
        }
        proc_macro2::TokenTree::Literal(_) => false,
    })
}

enum Constancy {
    AllLiteral,
    NotAllLiteral,
    /// DS-621's "定数性を確定できない式".
    Undetermined,
}

/// Whether DS-619's `.unwrap()` / `.expect(..)` / `?` (DS-1964 aside) appears in a
/// Test body, for DA-001.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MethodOrTry {
    None,
    /// Outside a closure, an `async` block and a nested item; a position in a
    /// branch counts, as in DS-1962.
    Counted,
    /// Only in a closure, an `async` block or a nested item.
    Uncounted,
}

/// A conservative literal/constant-expression classifier: recognizes
/// numeric/string/bool/char literals and their combination with arithmetic/
/// comparison operators and parentheses/commas/whitespace, and a logical
/// expression whose value literals fix (`fixed_value`, DS-1949) as one argument.
/// Any other identifier (a variable, a function call, a `const` item reference)
/// makes the whole argument list `NotAllLiteral` — this can only under-report
/// DA-001's `Fail` (treat a truly-constant `const` reference as non-constant),
/// never over-report it, per this module's doc comment.
fn classify_constancy(arguments: &str) -> Constancy {
    if let Constancy::AllLiteral = classify_literal_tokens(arguments) {
        return Constancy::AllLiteral;
    }
    let Ok(parsed) = syn::parse::Parser::parse_str(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        arguments,
    ) else {
        return Constancy::NotAllLiteral;
    };
    let mut undetermined = false;
    for argument in &parsed {
        if let Constancy::AllLiteral =
            classify_literal_tokens(&argument.to_token_stream().to_string())
        {
            continue;
        }
        match da_001_argument_constancy(argument) {
            Constancy::AllLiteral => {}
            Constancy::Undetermined => undetermined = true,
            Constancy::NotAllLiteral => return Constancy::NotAllLiteral,
        }
    }
    if undetermined {
        Constancy::Undetermined
    } else {
        Constancy::AllLiteral
    }
}

fn classify_literal_tokens(arguments: &str) -> Constancy {
    let mut chars = arguments.chars().peekable();
    let mut saw_identifier_like = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                // Skip a string literal body.
                while let Some(next) = chars.next() {
                    if next == '\\' {
                        chars.next();
                        continue;
                    }
                    if next == '"' {
                        break;
                    }
                }
            }
            '\'' => {
                // A char literal or a lifetime — either way, not an
                // identifier reference to a runtime value; skip to the
                // closing quote if present.
                while let Some(next) = chars.next() {
                    if next == '\\' {
                        chars.next();
                        continue;
                    }
                    if next == '\'' {
                        break;
                    }
                }
            }
            c if c.is_ascii_digit() => {
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '.' || *c == '_')
                {
                    chars.next();
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                saw_identifier_like = true;
                break;
            }
            _ => {}
        }
    }
    if saw_identifier_like {
        // `true`/`false` are the only bare identifiers this classifier
        // must not misclassify as a variable reference.
        let trimmed = arguments.trim();
        if trimmed == "true" || trimmed == "false" {
            return Constancy::AllLiteral;
        }
        Constancy::NotAllLiteral
    } else {
        Constancy::AllLiteral
    }
}

// ---------------------------------------------------------------------------
// DA-003 (DS-623): result unverified
// ---------------------------------------------------------------------------

/// DA-003 for the calls of `target_symbols`. `all_targets` is every declared
/// target of the Test, whose calls are not delegation targets (DS-1960).
fn da_003_result_unverified(
    text: &str,
    target_symbols: &[String],
    all_targets: &[String],
    macros: &[&str],
    has_should_panic: bool,
) -> DaVerdict {
    // DS-1941: a call of a tracked closure is read as the execution of its body.
    let inlined = inline_tracked_closures(text);
    let text = inlined.as_deref().unwrap_or(text);
    let every_assert = find_assert_spans(text, macros);
    // DS-1962: only an assert-equivalent at a counted position is a destination of
    // DA-003. One in a closure, an `async` block or a nested item still keeps DA-003
    // from failing below, since a derived value may flow there (DS-1944).
    let regions = non_counted_regions(text);
    // An assert-equivalent that syntax fixes never to run is not reached either
    // (derivation from DS-1935, DS-1953 and DS-1983; `DivergenceContext::dead`). It
    // still keeps DA-003 from failing below, and a result that would reach only it
    // makes DA-003 UNKNOWN (DS-615, DS-1680).
    let never_run = never_run_regions(text);
    let (dead_asserts, counted_asserts): (Vec<_>, Vec<_>) = every_assert
        .iter()
        .copied()
        .filter(|&(start, _)| !in_regions(start, &regions))
        .partition(|&(start, _)| in_regions(start, &never_run));
    let assert_spans = counted_asserts.as_slice();
    let never_run = NeverRun {
        regions: &never_run,
        asserts: &dead_asserts,
    };
    if target_symbols.is_empty() {
        // No declared target to check a call against; DA-003 has no
        // subject. (This capability only reaches Tests with at least one
        // resolved target — `evaluate_oracle_presence`'s caller in
        // `vtest-verify` already holds DS-1664's "no target" case at
        // `NO_EVIDENCE` before ever calling this analysis.)
        return DaVerdict::NoViolation;
    }
    if has_should_panic {
        return DaVerdict::NoViolation;
    }
    let mut any_call_found = false;
    let mut any_ambiguous = false;
    let mut any_unidentified = false;
    let mut any_unverified = false;
    let mut any_binding_uncertain = false;
    let mut any_macro_opaque = false;
    let mut any_delegated = false;
    let mut any_into_target = false;
    let mut any_debug_assert = false;
    let mut any_not_counted = false;
    let mut any_branch_unknown = false;
    let mut any_never_run = false;
    for symbol in target_symbols {
        match target_call_result_is_verified(
            text,
            symbol,
            all_targets,
            assert_spans,
            &regions,
            &never_run,
        ) {
            TargetCallVerification::NotCalled => {}
            TargetCallVerification::NeverRun => {
                any_call_found = true;
                any_never_run = true;
            }
            TargetCallVerification::NotCounted => {
                any_call_found = true;
                any_not_counted = true;
            }
            TargetCallVerification::Delegated => {
                any_call_found = true;
                any_delegated = true;
            }
            TargetCallVerification::Verified => {
                any_call_found = true;
            }
            TargetCallVerification::Unverified => {
                any_call_found = true;
                any_unverified = true;
            }
            TargetCallVerification::Ambiguous => {
                any_call_found = true;
                any_ambiguous = true;
            }
            TargetCallVerification::Unidentified => {
                any_call_found = true;
                any_unidentified = true;
            }
            TargetCallVerification::BindingUncertain => {
                any_call_found = true;
                any_binding_uncertain = true;
            }
            TargetCallVerification::IntoTargetArgument => {
                any_call_found = true;
                any_into_target = true;
            }
            TargetCallVerification::MacroOpaque => {
                any_call_found = true;
                any_macro_opaque = true;
            }
            TargetCallVerification::DebugAssertOnly => {
                any_call_found = true;
                any_debug_assert = true;
            }
            TargetCallVerification::BranchUnknown => {
                any_call_found = true;
                any_branch_unknown = true;
            }
        }
    }
    if !any_call_found {
        // No declared target is called at all in this construct — that is
        // DA-002's ("対象未呼出") territory (target_binding, not
        // oracle_presence). DA-003 only fires once a call exists.
        return DaVerdict::NoViolation;
    }
    // A FAIL proven for one target decides before another target's UNKNOWN, as
    // `analyze` composes (DS-623, DS-1680): with no assert-equivalent construct and
    // no other call to delegate to, an unverified result reaches none. A target
    // whose call or result this analysis cannot follow (a method call matched by
    // name only, a binding or macro it does not read, a result handed to another
    // target, a mutable reference) may still carry a result there, so those keep
    // their UNKNOWN (DS-1680).
    if any_unverified
        && !(any_unidentified
            || any_ambiguous
            || any_binding_uncertain
            || any_into_target
            || any_macro_opaque
            || any_not_counted
            || any_branch_unknown
            || any_never_run)
        && every_assert.is_empty()
        && !has_non_target_call(text, target_symbols)
    {
        return DaVerdict::Fail(
            "a declared target's call result reaches no assert-equivalent construct, \
             and no #[should_panic] is present (DS-623)",
        );
    }
    if any_delegated {
        // DS-641 / DS-1960: no result of a declared target's call reaches an
        // assert-equivalent other than through a delegation target (see
        // `target_call_result_is_verified`). This capability does not confirm a
        // delegation target's termination (DS-643), so DA-003 is UNKNOWN by §7.2.1
        // (DS-648, DS-651, DS-652).
        return DaVerdict::Unknown(
            "a declared target's result flows into an argument of a function call that is \
             not a value conversion, or of a method call, in an assert-equivalent's checked \
             argument or a binding that reaches one: that call is a delegation target of \
             this Test (DS-641, DS-1960) whose termination this analysis does not confirm, \
             so the result is neither a violation nor its absence (DS-648, DS-652)"
                .to_owned(),
        );
    }
    if any_ambiguous {
        return DaVerdict::Unknown(
            "a target call's result may be verified through a mutable reference or \
             global state, which this capability does not track (DS-623)"
                .to_owned(),
        );
    }
    if any_binding_uncertain {
        // Derivation (see `binding_reach`): the name of the binding that holds the
        // result appears in an assert, but it cannot be told to mean that binding.
        return DaVerdict::Unknown(
            "a declared target's result is bound to a name that an assert-equivalent \
             construct mentions, but the name cannot be shown to mean that binding (it is \
             bound again, a `let mut` binding is changed through a dereference, a path not \
             fixed by syntax, `&mut`, a `ref mut` pattern, a method call, a closure or a \
             macro (DS-1959), the assert reads a position that a projection assignment \
             may have overwritten (DS-1956, DS-1966), a value holding one other than as a \
             value compared whole or through an index that is not an integer literal \
             (DS-1939), the binding is read in, or bound to, a branch expression or a labeled \
             block whose arms differ in derivation (DS-1940), or it \
             appears in tokens this capability does not parse), so the result reaching the \
             assert is neither proven nor refuted (DS-1680)"
                .to_owned(),
        );
    }
    if any_branch_unknown {
        return DaVerdict::Unknown(
            "a declared target's result reaches an assert-equivalent or a position deciding a \
             failure divergence only through a branch expression (`if`, `if let`, `match`) or a \
             labeled block whose arms or exits, other than failure divergences (DS-1951), are \
             not all derived from a target's result at the position read, so the value is \
             neither shown derived nor shown underived (DS-1940, DS-1680)"
                .to_owned(),
        );
    }
    if any_into_target {
        // DS-1960 excludes a declared target's call from the delegation targets
        // and does not carry its argument to its value, so a result that reaches
        // an assert only as such an argument is not shown verified (DS-615).
        return DaVerdict::Unknown(
            "a declared target's result is bound to a name that reaches an \
             assert-equivalent only as an argument of another declared target's call: \
             that call is not a delegation target and does not carry the argument to its \
             value (DS-1960), so the result is not compared directly, and its being \
             unverified is neither proven nor refuted (DS-615)"
                .to_owned(),
        );
    }
    if any_macro_opaque {
        // Derivation (see `macro_target_use`): a call hidden in a macro's tokens may
        // or may not exist and may or may not be verified.
        return DaVerdict::Unknown(
            "a declared target is called in the tokens of a macro invocation that is not read \
             as a call whose result an assert checks (a macro other than an assert, an \
             assert's message argument, or tokens that do not parse as expressions), or its \
             name is used as a value (an indirect call), so whether it is called and whether \
             its result is verified is neither proven nor refuted (DS-615, DS-651, DS-1680)"
                .to_owned(),
        );
    }
    if any_not_counted {
        // DS-1962, DS-1944: a call in a closure that is not tracked, an `async` block
        // or a nested item may run or not, and its result flows nowhere counted.
        return DaVerdict::Unknown(
            "a declared target is called only where DS-1962 does not count a call (in a \
             closure that DS-1941 does not track, an `async` block or a nested item), and no \
             result of a call at a counted position reaches an assert-equivalent, so the \
             result is neither shown verified nor shown unverified (DS-1944, DS-1962)"
                .to_owned(),
        );
    }
    if any_never_run {
        return DaVerdict::Unknown(
            "a declared target's result reaches an assert-equivalent, or a position deciding a \
             failure divergence, only where syntax fixes that the code never runs (after a \
             statement that leaves on every path, in a branch or an arm that a constant \
             condition or an earlier irrefutable arm never takes, or in a loop that a constant \
             never runs), so it is not counted as reached, and the result is neither shown \
             verified nor shown unverified (DS-1935, DS-1953, DS-1983, DS-1680)"
                .to_owned(),
        );
    }
    if any_unidentified {
        // Derivation (not stated in DS-1680 / DS-615 / DS-623): a method call is
        // matched to the target by name only; the receiver's type is not
        // resolved. A same-named method of another type cannot be told from the
        // target, so neither "the target's result is not verified" (DS-623's
        // FAIL condition) nor "it is verified" (DS-1680's verifying device) can
        // be said deterministically, which DS-1680 reads as UNKNOWN. It arises
        // when every unverified call is a method call, or when the only verified
        // call is a method call and a function call of the target stays
        // unverified.
        return DaVerdict::Unknown(
            "a declared target's call is identified by name only (a method call \
             `receiver.name(..)`, whose receiver type is not resolved): it may be another \
             type's same-named method, or the only verified call may be one, so the result \
             being unverified is neither proven nor refuted (DS-1680)"
                .to_owned(),
        );
    }
    if any_debug_assert {
        return DaVerdict::Unknown(
            "a declared target's result reaches no assert-equivalent, but a binding of it is \
             used in a checked argument of a `debug_assert*`, which is not an assert-equivalent \
             but checks in a debug build (DS-1954)"
                .to_owned(),
        );
    }
    if any_unverified {
        // REQ-074「不成立が構造から証明できる場合のみFAIL」/DS-1680（旧
        // DS-610の退役後の再掲。verified against the merged canon this
        // session, fa63065: "静的解析は不成立の証明であり、不成立を証明
        // できないことだけを理由にUNKNOWNとはしない。照合装置の存在が
        // 決定論的に確認できる場合は違反なしとし、不成立の証明も照合装置
        // の存在の確認もいずれも決定論的に言えない場合に限りUNKNOWNとす
        // る」）/DS-615「adapterが不完全、解析限界…を報告した場合は
        // UNKNOWNとし、違反なしと推測しない」: this capability's dataflow
        // tracking is bounded (module doc — a direct in-assert call or one
        // `let` binding). When at least one assert-equivalent construct
        // exists in the function (`assert_spans` non-empty) but the call
        // cannot be traced into any of them, neither "the result does not
        // reach one" (DS-623's FAIL condition) nor "the result reaches
        // one" (DS-1680's "照合装置の存在が確認できる" -> no violation) can
        // be said deterministically by this capability's bounded tracker —
        // exactly DS-1680's own UNKNOWN condition ("いずれも決定論的に
        //言えない場合"), not a structural proof of non-verification.
        //
        // Only when `assert_spans` is empty is non-reaching a call
        // structurally certain: there is no assert-equivalent construct in
        // the function at all for any path to reach, tracked or not — a
        // genuine DS-623 `Fail`.
        //
        // DS-642: with no assert-equivalent construct, every other call in the
        // body is a delegation target of the Test. Its termination (§7.2.1,
        // #242) is not confirmed here, so DA-003 is neither a violation nor
        // its absence (DS-640, DS-652).
        // Deviation from DS-616 (disclosed): `assert_spans` holds assert
        // macros only, not the full assert-equivalent set of DS-616/619/620
        // (`.unwrap()`/`.expect()`/`?`, Result signature), so "no
        // assert-equivalent construct" is read more widely than DA-006 reads
        // it. This only moves a verdict FAIL -> UNKNOWN, never toward PASS.
        // Derivation (DS-642 does not exclude it): the target's own call is
        // not counted as a delegation target, so a body that only calls and
        // discards the target stays FAIL. DA-006 counts the target as a
        // delegation target, which differs from this reading (cf. DS-650's
        // self-delegation).
        // The case with no other call returned FAIL above.
        if every_assert.is_empty() {
            return DaVerdict::Unknown(
                "a call other than the declared target is a delegation target of this Test \
                 (DS-642) and its termination (§7.2.1) is not confirmed, so the result is \
                 neither a violation nor its absence (DS-652)"
                    .to_owned(),
            );
        }
        return DaVerdict::Unknown(
            "a declared target's call could not be traced into any of the function's \
             assert-equivalent constructs by this capability's bounded dataflow \
             tracking (a method chain, a second let-binding, or a field access this \
             capability does not follow could still carry the result there) — neither \
             reaching nor non-reaching can be said deterministically, which is DS-1680's \
             own UNKNOWN condition, not a structural proof of non-verification"
                .to_owned(),
        );
    }
    DaVerdict::NoViolation
}

/// `true` when the body calls a function or method that is neither a
/// declared target nor an assert-equivalent method (DS-642 delegation).
fn has_non_target_call(text: &str, target_symbols: &[String]) -> bool {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return false;
    };
    let mut visitor = NonTargetCallVisitor {
        symbols: target_symbols,
        found: false,
    };
    visitor.visit_block(&function.block);
    visitor.found
}

struct NonTargetCallVisitor<'a> {
    symbols: &'a [String],
    found: bool,
}

impl<'ast> Visit<'ast> for NonTargetCallVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        let is_target =
            match node.func.as_ref() {
                syn::Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
                    self.symbols.iter().any(|symbol| segment.ident == symbol)
                }),
                _ => false,
            };
        if !is_target {
            self.found = true;
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let name = node.method.to_string();
        let is_excluded = matches!(
            name.as_str(),
            "unwrap" | "expect" | "unwrap_err" | "expect_err"
        ) || self.symbols.contains(&name);
        if !is_excluded {
            self.found = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

enum TargetCallVerification {
    NotCalled,
    /// No call at a counted position reaches an assert-equivalent, and the target is
    /// called where DS-1962 does not count a call.
    NotCounted,
    Verified,
    Unverified,
    Ambiguous,
    /// Every unverified call is a method call matched by name only (#279).
    Unidentified,
    /// The binding that holds a call's result is named in an assert, but the name
    /// cannot be shown to mean that binding (#281).
    BindingUncertain,
    /// The target is called in macro tokens that are not read as a call whose
    /// result is checked (#282).
    MacroOpaque,
    /// The binding that holds a call's result reaches an assert only as an
    /// argument of a declared target's call (DS-1960).
    IntoTargetArgument,
    /// The result flows into a delegation target in an assert-equivalent's
    /// checked argument or in a binding that reaches one (DS-641, DS-1960).
    Delegated,
    /// A call flows to an assert-equivalent or a deciding position only through a
    /// branch whose arms are not all derived (DS-1940).
    BranchUnknown,
    /// No assert-equivalent is reached, but a binding of the result is used in a
    /// checked argument of a `debug_assert*` (DS-1954).
    DebugAssertOnly,
    /// No assert-equivalent is reached, but a result would reach one, or a position
    /// deciding a failure divergence, that syntax fixes never to run.
    NeverRun,
}

/// What syntax fixes never to run in a Test body (`never_run_regions`), and the
/// assert macros at counted positions there.
struct NeverRun<'a> {
    regions: &'a [(usize, usize)],
    asserts: &'a [(usize, usize)],
}

/// Whether a `let` binding's name reaches an assert-equivalent argument list.
enum BindingReach {
    Reaches,
    NotReached,
    Uncertain,
    /// The name flows into an argument of a delegation target there (DS-1960).
    Delegated,
    /// The name is used there only in an argument of a declared target's call,
    /// which carries no value to the assert (DS-1960).
    IntoTarget,
}

/// Derivation (not stated in DS-623 / DS-628 / DS-1680 / DS-615): a binding's name
/// reaches an assert only as a whole identifier in expression position, read from
/// the tokens of the assert's argument list. A field or method named like the
/// binding (`x.r`, `x.r()`) and a string literal are not the binding. Only the
/// arguments that are checked count (`assert!`: the condition; `assert_eq!` /
/// `assert_ne!`: the two compared values); the message arguments after them only
/// print on failure and verify nothing. The name cannot be told to mean the
/// binding (`Uncertain`, which DS-1680 reads as UNKNOWN) when the name is bound
/// again anywhere in the function (a second `let`, a closure parameter, a match
/// arm, a parameter, an assignment `name = ..` or `name += ..`), when the
/// argument list is not a list of expressions, when the macro is an
/// `assertion_macros` entry of the project config (its argument grammar is not
/// known), or when the name appears in a nested macro other than `matches!`,
/// `format!`, `vec!` and the assert macros, or in one of those whose tokens do not
/// parse as expressions.
///
/// A `let mut` binding that is changed through a place (`place_changes`) cannot be
/// told to mean the result (DS-1959, DS-615), except through a projection assignment
/// whose path is fixed by syntax, after which only the assigned positions stop being
/// derived (DS-1956): the assert's reads are then judged by the path they read
/// (`read_is_derived`). Only a `let mut` binding is checked. A binding of a target
/// that returns `&mut T` can be changed without `let mut` and is not covered.
fn binding_reach(
    text: &str,
    binding: &DerivedBinding,
    targets: &[String],
    sources: &JoinSources,
    assert_spans: &[(usize, usize)],
    divergences: &[Divergence<'_>],
) -> BindingReach {
    let name = binding.name.as_str();
    let changes = if binding.mutable {
        place_changes(text, name, assert_spans)
    } else {
        PlaceChanges::default()
    };
    // DS-1956 keeps the other positions derived only for a value whose positions
    // are all derived to begin with (DS-1939); a binding of a literal that holds the
    // result in one element is not followed through an assignment, plain or compound.
    let tracked = !changes.whole && (!changes.projected || binding.whole_result);
    let overwrites: &[Vec<PlaceStep>] = if tracked { &changes.overwrites } else { &[] };
    let reaches = |outcome: &BindingReach| matches!(outcome, BindingReach::Reaches);
    let mut outcomes: Vec<BindingReach> = assert_spans
        .iter()
        .map(|&(start, end)| {
            span_binding_reach(
                &text[start..end],
                macro_name_before(text, start),
                name,
                targets,
                sources,
                overwrites,
            )
        })
        .collect();
    // DS-1947: a deciding position of a failure divergence is reached as an
    // assert's checked argument is.
    outcomes.extend(divergence_binding_reach(
        divergences,
        binding,
        targets,
        sources,
        overwrites,
    ));
    // A name bound again or changed in place is not shown to mean this binding,
    // whether it reaches an assert directly or through a delegation target.
    let name_uncertain = || count_rebindings(text, name) > 1 || !tracked;
    // DS-623 / DS-1962: an assert the name reaches directly decides, and a
    // delegation target in another assert does not make it UNKNOWN.
    if outcomes.iter().any(reaches) {
        return if name_uncertain() {
            BindingReach::Uncertain
        } else {
            BindingReach::Reaches
        };
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::Delegated))
    {
        return if name_uncertain() {
            BindingReach::Uncertain
        } else {
            BindingReach::Delegated
        };
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::Uncertain))
    {
        return BindingReach::Uncertain;
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::IntoTarget))
    {
        return if name_uncertain() {
            BindingReach::Uncertain
        } else {
            BindingReach::IntoTarget
        };
    }
    BindingReach::NotReached
}

/// The name of the macro whose argument list starts at `args_start` (the byte after
/// its opening delimiter), read from the text before it.
fn macro_name_before(text: &str, args_start: usize) -> Option<&str> {
    let before = text[..args_start]
        .trim_end()
        .strip_suffix(['(', '[', '{'])?
        .trim_end()
        .strip_suffix('!')?
        .trim_end();
    let name_start = before
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map_or(0, |index| index + 1);
    let name = &before[name_start..];
    (!name.is_empty()).then_some(name)
}

/// How many leading arguments of a macro are checked (the rest is message), or
/// `None` for a macro whose argument grammar is not known. `usize::MAX`: every
/// argument is a value.
fn checked_argument_count(macro_name: &str) -> Option<usize> {
    match macro_name {
        "assert" | "debug_assert" => Some(1),
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Some(2),
        "panic" | "unreachable" | "todo" | "unimplemented" => Some(0),
        "format" | "vec" => Some(usize::MAX),
        _ => None,
    }
}

fn span_binding_reach(
    span_text: &str,
    macro_name: Option<&str>,
    binding: &str,
    targets: &[String],
    sources: &JoinSources,
    overwrites: &[Vec<PlaceStep>],
) -> BindingReach {
    let Ok(tokens) = span_text.parse::<proc_macro2::TokenStream>() else {
        return BindingReach::Uncertain;
    };
    if !tokens_mention_ident(tokens, binding) {
        return BindingReach::NotReached;
    }
    // An `assertion_macros` entry of the config is not a built-in assert: its
    // argument grammar (`assert_matches!(expr, pat)`) is not known (DS-615).
    let Some(checked) = macro_name
        .filter(|name| STANDARD_ASSERTION_MACROS.contains(&format!("{name}!").as_str()))
        .and_then(checked_argument_count)
    else {
        return BindingReach::Uncertain;
    };
    let Ok(arguments) = syn::parse::Parser::parse_str(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        span_text,
    ) else {
        return BindingReach::Uncertain;
    };
    let mut visitor = CheckedCallVisitor::new(None, Some(binding), targets, sources);
    visitor.probe_overwrites = overwrites;
    // `assert_eq!` / `assert_ne!` compare their two checked arguments whole.
    let compares = matches!(macro_name, Some("assert_eq" | "assert_ne"));
    for argument in arguments.iter().take(checked) {
        visitor.in_compared = compares;
        visitor.visit_expr(argument);
    }
    probe_reach(&visitor)
}

/// How the probed name reaches the value `visitor` has visited.
///
/// A use of the name where its value is dropped (`{ r; true }`, `let _ = r`, a
/// closure body, a `match` scrutinee) does not carry it to the compared value,
/// which is not shown to be verified either: UNKNOWN (DS-615). So does a read of
/// an assigned position, whose value is not followed (DS-1956, DS-1966), and a read
/// through a branch whose arms differ in derivation (DS-1940).
/// A use that flows to the compared value decides before a use handed to a
/// delegation target (DS-623, DS-1962).
fn probe_reach(visitor: &CheckedCallVisitor<'_>) -> BindingReach {
    if visitor.opaque || visitor.probe_rebound || visitor.probe_in_branch {
        BindingReach::Uncertain
    } else if visitor.probe_flows {
        BindingReach::Reaches
    } else if visitor.probe_delegated {
        BindingReach::Delegated
    } else if visitor.probe_dropped || visitor.probe_overwritten || visitor.mixed {
        BindingReach::Uncertain
    } else if visitor.probe_into_target {
        BindingReach::IntoTarget
    } else {
        BindingReach::NotReached
    }
}

fn tokens_mention_ident(tokens: proc_macro2::TokenStream, name: &str) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident == name,
        proc_macro2::TokenTree::Group(group) => tokens_mention_ident(group.stream(), name),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// The expressions of a macro whose argument grammar is known, split into the
/// arguments that are checked or computed and those that are not.
struct MacroArguments {
    checked: Vec<syn::Expr>,
    unchecked: Vec<proc_macro2::TokenStream>,
}

/// Derivation (not stated in DS-615 / DS-1680): `matches!`, `format!`, `vec!` and
/// the assert macros take expressions in a fixed grammar, so a name in an
/// expression position of a checked argument is read as in an assert's own
/// argument list; message arguments (and `matches!`'s pattern, in which a name is
/// bound by the arm) are returned as `unchecked`. `None` for any other macro, and
/// for one of these whose tokens do not parse that way.
fn macro_arguments(node: &syn::Macro) -> Option<MacroArguments> {
    let name = node.path.segments.last()?.ident.to_string();
    if name == "matches" {
        return syn::parse::Parser::parse2(
            |input: syn::parse::ParseStream<'_>| {
                let first: syn::Expr = input.parse()?;
                let rest: proc_macro2::TokenStream = input.parse()?;
                Ok(MacroArguments {
                    checked: vec![first],
                    unchecked: vec![rest],
                })
            },
            node.tokens.clone(),
        )
        .ok();
    }
    let checked_count = checked_argument_count(&name)?;
    let parsed = syn::parse::Parser::parse2(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        node.tokens.clone(),
    )
    .ok()?;
    let mut arguments = MacroArguments {
        checked: Vec::new(),
        unchecked: Vec::new(),
    };
    for (index, argument) in parsed.into_iter().enumerate() {
        if index >= checked_count {
            arguments.unchecked.push(argument.into_token_stream());
            continue;
        }
        arguments.checked.push(match argument {
            // `format!("{r}", r = value)`: a named argument. The left side names
            // the placeholder, not a binding; only the value is read.
            syn::Expr::Assign(named) if name == "format" => *named.right,
            other => other,
        });
    }
    Some(arguments)
}

/// How a target is called in the tokens of the macro invocations of a Test.
#[derive(Default)]
struct MacroUse {
    /// A call `name(..)` / `Type::name(..)` in a checked argument of an assert.
    verified_path: bool,
    /// A call `receiver.name(..)` in a checked argument of an assert.
    verified_method: bool,
    /// A call-shaped use of the name where it is not read as a call whose result
    /// is checked: in a macro other than an assert, in an assert's message
    /// argument or in tokens that do not parse as expressions; or the name used as
    /// a value outside a macro (`ValueUseVisitor`).
    opaque: bool,
    /// A call whose value flows into a delegation target in a checked argument of
    /// an assert (DS-1960).
    delegated: bool,
    /// A use of the name in the tokens of a macro where DS-1962 does not count a call
    /// (in a closure, an `async` block or a nested item).
    non_counted: bool,
    /// A call in a checked argument of an assert flows only through a branch whose
    /// arms differ (DS-1940).
    mixed: bool,
    /// A call in a checked argument of an assert that syntax fixes never to run,
    /// which would otherwise set `verified_path` or `verified_method`.
    never_run: bool,
}

/// Derivation (not stated in DS-615 / DS-651 / DS-1680): a macro's tokens are not
/// syntax-tree nodes, so a call written there is found only by reading the
/// tokens. In `assert!` / `assert_eq!` / `assert_ne!` the checked arguments are
/// read as expressions, and a call found there verifies the target as a call
/// outside a macro that reaches an assert does. A call hidden anywhere else (any
/// other macro, a message argument, tokens that do not parse) may or may not exist
/// and may or may not be verified, and DS-651 reads a call hidden in a macro
/// expansion as UNKNOWN, so a use of the name there that is not read as a call
/// whose result an assert checks is `opaque` (`NameUse::tokens_use_name`). Reading
/// macro tokens only moves a verdict toward UNKNOWN or turns "not called" into
/// verified: `target_call_result_is_verified` takes a call read in an assert as
/// verifying only when the Test has no call outside a macro, so it never clears
/// a call outside a macro that stays unverified, and a verified method call
/// (matched by name only) does not clear an `opaque` name that may be a
/// function call.
fn macro_target_use(
    text: &str,
    symbol: &str,
    targets: &[String],
    sources: &JoinSources,
    regions: &[(usize, usize)],
    never_run: &[(usize, usize)],
) -> MacroUse {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return MacroUse::default();
    };
    let names = NameUse::new(text, symbol, &function);
    let mut visitor = MacroUseVisitor {
        names: &names,
        targets,
        sources,
        regions,
        never_run,
        found: MacroUse::default(),
    };
    visitor.visit_item_fn(&function);
    let mut values = ValueUseVisitor {
        names: &names,
        found: false,
    };
    values.visit_item_fn(&function);
    visitor.found.opaque |= values.found;
    visitor.found
}

/// Which uses of the target's name are the target's own, for `ValueUseVisitor`
/// and the macro readers.
struct NameUse<'a> {
    symbol: &'a str,
    text: &'a str,
    /// Derivation (not stated in DS-615 / DS-1680): a constant or static is not
    /// callable, and Rust names them in upper case, so its value is not an
    /// indirect call.
    is_constant: bool,
    /// The byte ranges in which the name is bound by a `let` statement: from the
    /// end of the `let` to the end of its block. A single-segment path inside one
    /// names that local, not the target (derivation: lexical scope). A binding in
    /// a closure, a match arm, a nested fn or an assignment is no exception, and
    /// neither is a use before the `let` or outside its block.
    local_ranges: Vec<(usize, usize)>,
}

impl<'a> NameUse<'a> {
    fn new(text: &'a str, symbol: &'a str, function: &syn::ItemFn) -> Self {
        let mut scopes = LetScopeVisitor {
            text,
            symbol,
            ranges: Vec::new(),
        };
        scopes.visit_item_fn(function);
        Self {
            symbol,
            text,
            is_constant: is_constant_name(symbol),
            local_ranges: scopes.ranges,
        }
    }

    fn is_local_at(&self, location: proc_macro2::LineColumn) -> bool {
        let offset = span_offset(location, self.text);
        self.local_ranges
            .iter()
            .any(|&(from, to)| offset >= from && offset < to)
    }

    /// An expression path that is a value use of the target.
    fn path_is_value_use(&self, node: &syn::ExprPath) -> bool {
        let names_symbol = node
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == self.symbol);
        let names_local = node.qself.is_none()
            && node.path.get_ident().is_some()
            && self.is_local_at(node.span().start());
        names_symbol && !self.is_constant && !names_local
    }

    /// Whether the tokens of a macro use the name where it is not a direct call
    /// read as an expression: followed by an argument list (`name(..)`) or by
    /// `::<`, or standing alone (a value). Not a use: after `.` (a method or a
    /// field), before `::` (a path prefix), before `!` (a macro name), before a
    /// single `:` (a field label), a constant, or a local bound by a `let` earlier in the same block.
    fn tokens_use_name(&self, tokens: proc_macro2::TokenStream) -> bool {
        use proc_macro2::{Delimiter, TokenTree};
        let trees: Vec<TokenTree> = tokens.into_iter().collect();
        trees.iter().enumerate().any(|(index, tree)| match tree {
            TokenTree::Group(group) => self.tokens_use_name(group.stream()),
            TokenTree::Ident(ident) if ident == self.symbol => {
                let next = trees.get(index + 1);
                let punct_after = |offset: usize, expected: char| {
                    matches!(trees.get(index + offset), Some(TokenTree::Punct(p)) if p.as_char() == expected)
                };
                if matches!(next, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
                    || (punct_after(1, ':') && punct_after(2, ':') && punct_after(3, '<'))
                {
                    return true;
                }
                let after_dot = index > 0
                    && matches!(&trees[index - 1], TokenTree::Punct(p) if p.as_char() == '.');
                let path_prefix = punct_after(1, ':') && punct_after(2, ':');
                let field_label = punct_after(1, ':') && !punct_after(2, ':');
                let macro_name = punct_after(1, '!') && !punct_after(2, '=');
                !(after_dot
                    || path_prefix
                    || field_label
                    || macro_name
                    || self.is_constant
                    || self.is_local_at(ident.span().start()))
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        })
    }
}

struct LetScopeVisitor<'a> {
    text: &'a str,
    symbol: &'a str,
    ranges: Vec<(usize, usize)>,
}

impl<'ast> Visit<'ast> for LetScopeVisitor<'_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        let block_end = span_offset(node.brace_token.span.join().end(), self.text);
        for statement in &node.stmts {
            if let syn::Stmt::Local(local) = statement {
                if pattern_binds(&local.pat, self.symbol) {
                    self.ranges
                        .push((span_offset(local.span().end(), self.text), block_end));
                }
            }
        }
        syn::visit::visit_block(self, node);
    }
}

/// Whether a pattern binds `name` as its own identifier (`name`, `mut name`,
/// `name: T`).
fn pattern_binds(pattern: &syn::Pat, name: &str) -> bool {
    match pattern {
        syn::Pat::Ident(ident) => ident.ident == name,
        syn::Pat::Type(typed) => pattern_binds(&typed.pat, name),
        _ => false,
    }
}

fn is_constant_name(name: &str) -> bool {
    name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Derivation (not stated in DS-615 / DS-623 / DS-1680): the target's name as an
/// expression path that is not the callee of a direct call (`(name)(..)`,
/// `let f = name;`, `apply(name)`, `Type::name` passed as a value) may be called
/// later through another name, which this capability does not follow. It is
/// `opaque`, like a call hidden in macro tokens: UNKNOWN, not "not called". A
/// `use`, a type, a path prefix, a field, a method name and a string are not
/// expression paths of the target. Inside macros the same uses are read from the
/// tokens (`NameUse::tokens_use_name`) or, in an assert's checked arguments, from
/// the parsed expressions (`CheckedCallVisitor`).
struct ValueUseVisitor<'a> {
    names: &'a NameUse<'a>,
    found: bool,
}

impl<'ast> Visit<'ast> for ValueUseVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        // A path callee is a direct call, read by `find_call_sites`; any other
        // callee expression (`(name)`) is visited as a value.
        if !matches!(node.func.as_ref(), syn::Expr::Path(_)) {
            self.visit_expr(&node.func);
        }
        for argument in &node.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if self.names.path_is_value_use(node) {
            self.found = true;
        }
    }
}

struct MacroUseVisitor<'a> {
    names: &'a NameUse<'a>,
    targets: &'a [String],
    sources: &'a JoinSources,
    /// DS-1962's regions that are not counted.
    regions: &'a [(usize, usize)],
    /// The regions that syntax fixes never to run (`never_run_regions`).
    never_run: &'a [(usize, usize)],
    found: MacroUse,
}

impl<'ast> Visit<'ast> for MacroUseVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if !self.names.tokens_use_name(node.tokens.clone()) {
            return;
        }
        if in_regions(
            span_offset(node.span().start(), self.names.text),
            self.regions,
        ) {
            self.found.non_counted = true;
            return;
        }
        let is_assert = node.path.segments.last().is_some_and(|segment| {
            ["assert", "assert_eq", "assert_ne"].contains(&segment.ident.to_string().as_str())
        });
        let Some(arguments) = is_assert.then(|| macro_arguments(node)).flatten() else {
            self.found.opaque = true;
            return;
        };
        let mut calls = CheckedCallVisitor::new(Some(self.names), None, self.targets, self.sources);
        for expression in &arguments.checked {
            calls.visit_expr(expression);
        }
        self.found.mixed |= calls.mixed;
        if in_regions(
            span_offset(node.span().start(), self.names.text),
            self.never_run,
        ) {
            self.found.never_run |= calls.path || calls.method;
        } else {
            self.found.verified_path |= calls.path;
            self.found.verified_method |= calls.method;
        }
        self.found.delegated |= calls.delegated;
        self.found.opaque |= calls.opaque
            || arguments
                .unchecked
                .iter()
                .any(|tokens| self.names.tokens_use_name(tokens.clone()));
    }
}

/// Finds calls of the target in the checked arguments of an assert, reading a
/// nested macro as `macro_arguments` does. The target's name as a value there is
/// `opaque`, as outside a macro.
///
/// Derivation (not stated in DS-615 / DS-623 / DS-628 / DS-1680): a call verifies
/// the target only where its value can reach the compared value, which is `flows`,
/// and only the positions listed here flow; any other position is `dropping` and a
/// call there is `opaque` (UNKNOWN, DS-615: that the value is verified is not
/// shown). The listed positions are the argument expression itself; the operand of
/// a unary or binary operator (not of an assignment, nor an operand of `||` / `|` /
/// `&&` / `&` whose other operand fixes the result, DS-1948); the receiver of a method call (DS-1937); the
/// arguments of a value conversion (DS-1960, DS-1961: a call whose callee's final
/// segment is `Some`, `Ok` or `Err`, or `Box::new` / `String::from`); a reference, a cast, `?`,
/// `.await` and parentheses; the base of a field access and the base and the index
/// of an index expression (the index decides the element, as an argument does); the tail
/// expression of a block; the tail expression of each branch of an `if` or a
/// `match`, and the tail of a labeled block, only where every arm or exit other than
/// a failure divergence is derived (DS-1940, `visit_join`: otherwise `mixed`); the
/// initializer of a `let` whose single name the tail expression of the
/// block uses where it flows; and the elements of a tuple, array or struct literal,
/// except that a literal projected directly (`(a, b).1`, `S { a, b }.b`,
/// `[a, b][1]`, `vec![a, b][1]`) passes only the element that is projected, and none
/// when that cannot be told. A closure body is not a flowing position (DS-629 reads
/// dataflow inside a closure as UNKNOWN) and neither is a `match` scrutinee (outside
/// the range DS-628 gives the tracking, so DS-615 reads it as UNKNOWN). A chain of projections (`((a, b), c).0.1`) is applied
/// step by step. The same test applied to the initializer of a `let` decides whether
/// its binding is derived from the target's result (`find_let_bindings_containing_call`).
/// A `let` name is followed only through the tail expression of its own
/// block, one binding deep (DS-628's `let` binding, bounded as the rest of DA-003
/// is). The same positions decide whether the name of a `let` outside the assert
/// reaches it (`probe`, `binding_reach`).
///
/// DS-1960: any other function call (a callee that is not a path, or a path that
/// is neither a value conversion nor a declared target, `Foo(x)` and `T::new(x)`
/// included, as a constructor is not told from the name's shape, DS-1967) and a
/// method call that is not a declared target take a value passed as an argument
/// as the Test's delegation target (DS-641): a value that flows into such an
/// argument is `delegated`, not carried to the compared value. The arguments of a
/// declared target's call are neither a value conversion nor a delegation target
/// (DS-1960), so they are not a flowing position either.
struct CheckedCallVisitor<'a> {
    /// The target whose calls are looked for; `None` while probing a `let` name.
    names: Option<&'a NameUse<'a>>,
    /// The name of a `let` whose use is probed, instead of the target's calls.
    probe: Option<&'a str>,
    /// Every declared target's symbol, whose calls are not delegation targets.
    targets: &'a [String],
    /// The values derived from the target's result, read to join the arms of a
    /// branch (DS-1940).
    sources: &'a JoinSources,
    /// This visitor classifies one arm of a branch: it looks for any of `sources`
    /// (and `probe`) where it flows, recorded in `source_flows`.
    classifying: bool,
    /// A value of `sources` is used where it flows (`classifying` only).
    source_flows: bool,
    /// What is looked for flows only through a branch whose arms are not all
    /// derived (DS-1940), so it is neither shown derived nor shown underived.
    mixed: bool,
    flows: bool,
    /// Inside an argument of a delegation target (DS-1960).
    delegating: bool,
    path: bool,
    method: bool,
    opaque: bool,
    /// A call of the target flows into an argument of a delegation target.
    delegated: bool,
    /// `probe` is used where it flows.
    probe_flows: bool,
    /// `probe` flows into an argument of a delegation target.
    probe_delegated: bool,
    /// `probe` is used where it does not flow.
    probe_dropped: bool,
    /// Inside the argument of a declared target's call (DS-1960).
    into_target: bool,
    /// `probe` is used only inside the argument of a declared target's call.
    probe_into_target: bool,
    /// `probe` is bound or assigned again.
    probe_rebound: bool,
    /// The paths of `probe` assigned by projection assignments (DS-1956).
    probe_overwrites: &'a [Vec<PlaceStep>],
    /// `probe` is read where it flows, but the read value is not shown derived
    /// (`read_is_derived`).
    probe_overwritten: bool,
    /// `probe`, after a projection assignment, is read where it flows in an arm of
    /// a branch expression (`if`, `if let`, `match`), whose arms may differ in
    /// derivation (DS-1940).
    probe_in_branch: bool,
    /// The expression is in an arm of a branch expression.
    in_branch: bool,
    /// The expression is in the value of a `break`.
    in_break: bool,
    /// The expression is a value that `assert_eq!` / `assert_ne!` or `==` / `!=`
    /// compares whole, through parentheses, references and projections only.
    in_compared: bool,
    /// The expression is inside the base of a projection that does not start at
    /// `probe` itself (`(&r).f`, `{ r }.f`), or inside a block-local `let`
    /// initializer, so the path that a read of `probe` there leads to is not known.
    path_unknown: bool,
    /// The expression is the initializer of a `let` (`find_let_bindings_containing_call`), where
    /// calls in macro tokens are not read.
    init_mode: bool,
}

/// One step of a field access or an index expression, applied to a literal.
enum Projection<'e> {
    Member(&'e syn::Member),
    /// An index that is an integer literal, or `None` when it is not one.
    Index(Option<usize>),
}

impl Projection<'_> {
    fn place_step(&self) -> PlaceStep {
        match self {
            Projection::Member(syn::Member::Named(name)) => PlaceStep::Field(field_name(name)),
            Projection::Member(syn::Member::Unnamed(index)) => PlaceStep::Tuple(index.index),
            Projection::Index(Some(index)) => PlaceStep::Index(*index),
            Projection::Index(None) => PlaceStep::Undetermined,
        }
    }
}

/// A field name as a position: `r#f` and `f` name the same field (DS-1956).
fn field_name(ident: &syn::Ident) -> String {
    use syn::ext::IdentExt;
    ident.unraw().to_string()
}

/// One step of the path of a place or a read (DS-1956): a field name, a tuple
/// number, an integer-literal index, or a step not fixed by syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaceStep {
    Field(String),
    Tuple(u32),
    Index(usize),
    Undetermined,
}

/// Whether a read of the path `read` from a binding whose whole value was derived
/// is derived, after projection assignments to the paths `overwrites` (DS-1956).
///
/// A read under an assigned path is not shown derived: the assigned value is not
/// followed, and statement order and branches are not read, so the read may see
/// the assigned value (DS-1956, DS-1966). A read whose value holds an assigned
/// position and other positions is derived only where it is the value compared
/// whole (`compared`: the false PASS DS-1939 and DS-1956 disclose). Anywhere else
/// (a method receiver, an operand of another operator, a pattern match, a macro or a
/// constructor argument) it is not shown derived: DS-1939 reads a method over such
/// a value as UNKNOWN, and the other uses may take the assigned position alone. A
/// read through a step not fixed by syntax may take an assigned position or not
/// (DS-1939: UNKNOWN unless every position is derived). An index and a field of one
/// value are not told apart by syntax as positions: an index of a type other than an
/// array or a slice is a call of its `Index` / `IndexMut` method on the whole value,
/// which may read or write any field (DS-1939's method over a value whose positions
/// differ), so a read that leaves an assigned path at a step of the other kind is not
/// derived either.
fn read_is_derived(overwrites: &[Vec<PlaceStep>], read: &[PlaceStep], compared: bool) -> bool {
    let fixed = read
        .iter()
        .position(|step| *step == PlaceStep::Undetermined)
        .unwrap_or(read.len());
    let prefix = &read[..fixed];
    let is_index = |step: &PlaceStep| matches!(step, PlaceStep::Index(_));
    let leaves_by_other_kind = |path: &Vec<PlaceStep>| {
        let common = path.iter().zip(prefix).take_while(|(a, b)| a == b).count();
        path.get(common)
            .zip(prefix.get(common))
            .is_some_and(|(assigned, read)| is_index(assigned) != is_index(read))
    };
    if overwrites
        .iter()
        .any(|path| prefix.starts_with(path) || leaves_by_other_kind(path))
    {
        return false;
    }
    let holds_assigned = overwrites.iter().any(|path| path.starts_with(prefix));
    !(holds_assigned && (!compared || fixed < read.len()))
}

/// How the value of a call's argument goes on (DS-1960).
#[derive(Clone, Copy)]
enum ArgumentFlow {
    /// A value conversion: the argument flows as the call's value does.
    Flows,
    /// A message argument (DS-1935): the argument is neither carried nor delegated.
    Dropped,
    /// A declared target's call: the argument is neither carried nor delegated
    /// (DS-1960).
    IntoTarget,
    /// A delegation target: the argument is handed to it (DS-641).
    Delegated,
}

impl<'a> CheckedCallVisitor<'a> {
    fn new(
        names: Option<&'a NameUse<'a>>,
        probe: Option<&'a str>,
        targets: &'a [String],
        sources: &'a JoinSources,
    ) -> Self {
        Self {
            names,
            probe,
            targets,
            sources,
            classifying: false,
            source_flows: false,
            mixed: false,
            flows: true,
            delegating: false,
            path: false,
            method: false,
            opaque: false,
            delegated: false,
            probe_flows: false,
            probe_delegated: false,
            probe_dropped: false,
            into_target: false,
            probe_into_target: false,
            probe_rebound: false,
            probe_overwrites: &[],
            probe_overwritten: false,
            probe_in_branch: false,
            in_branch: false,
            in_break: false,
            in_compared: false,
            path_unknown: false,
            init_mode: false,
        }
    }

    /// Visits `visit` as an arm of a branch expression.
    fn branch_arm(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.in_branch;
        self.in_branch = true;
        visit(self);
        self.in_branch = outer;
    }

    /// Whether every arm of `arms` that is not a failure divergence (DS-1951) carries,
    /// at the position `steps` select, a value derived from the target's result
    /// (DS-1940). Each arm is read by a classifying visitor that counts a call of any
    /// declared target, a binding of `sources` and `probe`, so an arm derived through
    /// another of them than the one looked for counts as derived too. With no arm
    /// left, the value is not shown derived.
    fn arms_all_derived(&self, arms: &[Arm<'_>], steps: &[Projection<'_>]) -> bool {
        let mut counted = false;
        for arm in arms {
            if arm.always_fails() {
                continue;
            }
            let mut classifier =
                CheckedCallVisitor::new(None, self.probe, self.targets, self.sources);
            classifier.classifying = true;
            classifier.probe_overwrites = self.probe_overwrites;
            classifier.path_unknown = self.path_unknown;
            classifier.init_mode = self.init_mode;
            match arm {
                Arm::Expr(expression) => classifier.visit_projected(expression, steps),
                Arm::Block(block) => classifier.visit_block_projected(block, steps),
                Arm::Unit => {}
            }
            if !(classifier.source_flows || classifier.probe_flows) {
                return false;
            }
            counted = true;
        }
        counted
    }

    /// Visits a value that joins `arms` with `visit`, which reads it as any other
    /// value. DS-1940: unless every arm is derived at `steps`, the joined value is not
    /// shown derived, so what `visit` finds flowing is recorded as `mixed` instead.
    fn visit_join(
        &mut self,
        arms: &[Arm<'_>],
        steps: &[Projection<'_>],
        visit: impl FnOnce(&mut Self),
    ) {
        let derived = self.arms_all_derived(arms, steps);
        // A classifying read needs only whether the joined value is derived, which the
        // arms just told; reading them again would cost twice per nested join.
        if self.classifying {
            if derived {
                self.found_source();
            }
            return;
        }
        if derived {
            return visit(self);
        }
        let outer = (
            std::mem::take(&mut self.path),
            std::mem::take(&mut self.method),
            std::mem::take(&mut self.probe_flows),
            std::mem::take(&mut self.source_flows),
        );
        visit(self);
        if self.path || self.method || self.probe_flows || self.source_flows {
            self.mixed = true;
        }
        (self.path, self.method, self.probe_flows, self.source_flows) = outer;
    }

    /// Records a use of one of `sources` where `self.flows` says (`classifying` only).
    fn found_source(&mut self) {
        if self.classifying && self.flows && !self.delegating {
            self.source_flows = true;
        }
    }

    /// The arms of an `if` (a missing `else` is a unit arm).
    fn if_arms(node: &syn::ExprIf) -> Vec<Arm<'_>> {
        vec![
            Arm::Block(&node.then_branch),
            node.else_branch
                .as_ref()
                .map_or(Arm::Unit, |(_, other)| Arm::Expr(other)),
        ]
    }

    /// The exits of a labeled block: its body and the value of each `break` that
    /// leaves it (a `break` without a value is a unit exit).
    fn labeled_exits<'e>(node: &'e syn::ExprBlock, label: &syn::Label) -> Vec<Arm<'e>> {
        let mut breaks = LabeledBreaks {
            label: &label.name,
            found: Vec::new(),
        };
        breaks.visit_block(&node.block);
        std::iter::once(Arm::Block(&node.block))
            .chain(
                breaks
                    .found
                    .into_iter()
                    .map(|value| value.map_or(Arm::Unit, Arm::Expr)),
            )
            .collect()
    }

    /// Visits a block whose value `steps` project: they are applied to its tail
    /// expression.
    fn visit_block_projected(&mut self, node: &syn::Block, steps: &[Projection<'_>]) {
        let outer = self.flows;
        let outer_delegating = self.delegating;
        for (index, statement) in node.stmts.iter().enumerate() {
            let mut delegates = false;
            let is_tail = index + 1 == node.stmts.len();
            self.flows = outer
                && match statement {
                    syn::Stmt::Expr(_, None) => is_tail,
                    syn::Stmt::Local(local) => {
                        match self.local_reaches_tail(local, &node.stmts[index + 1..]) {
                            TailReach::Flows => true,
                            TailReach::Delegated => {
                                delegates = true;
                                true
                            }
                            TailReach::No => false,
                        }
                    }
                    syn::Stmt::Expr(_, Some(_)) | syn::Stmt::Item(_) | syn::Stmt::Macro(_) => false,
                };
            self.delegating = outer_delegating || delegates;
            match statement {
                // The initializer's value goes on under another name, whose
                // projections are not followed back to `probe` (DS-1939).
                syn::Stmt::Local(_) => self.path_unknown(|visitor| visitor.visit_stmt(statement)),
                syn::Stmt::Expr(tail, None) if is_tail && !steps.is_empty() => {
                    self.visit_projected(tail, steps);
                }
                _ => self.visit_stmt(statement),
            }
        }
        self.flows = outer;
        self.delegating = outer_delegating;
    }

    /// Visits `node` as an `if` whose value `steps` project.
    fn visit_if_projected(&mut self, node: &syn::ExprIf, steps: &[Projection<'_>]) {
        let arms = Self::if_arms(node);
        self.visit_join(&arms, steps, |visitor| {
            visitor.dropping(|visitor| visitor.visit_expr(&node.cond));
            visitor.branch_arm(|visitor| {
                visitor.visit_block(&node.then_branch);
                if let Some((_, other)) = &node.else_branch {
                    visitor.visit_expr(other);
                }
            });
        });
    }

    /// Visits `node` as a `match` whose value `steps` project.
    fn visit_match_projected(&mut self, node: &syn::ExprMatch, steps: &[Projection<'_>]) {
        let arms: Vec<Arm<'_>> = node.arms.iter().map(|arm| Arm::Expr(&arm.body)).collect();
        self.visit_join(&arms, steps, |visitor| {
            visitor.dropping(|visitor| visitor.visit_expr(&node.expr));
            for arm in &node.arms {
                visitor.visit_pat(&arm.pat);
                if let Some((_, guard)) = &arm.guard {
                    visitor.dropping(|visitor| visitor.visit_expr(guard));
                }
                visitor.branch_arm(|visitor| visitor.visit_expr(&arm.body));
            }
        });
    }

    /// Records a read of `probe` that projects the path `steps` (innermost first)
    /// where `self.flows` says.
    fn probe_read(&mut self, steps: &[Projection<'_>]) {
        let mut path: Vec<PlaceStep> = steps.iter().map(Projection::place_step).collect();
        if self.path_unknown {
            path.push(PlaceStep::Undetermined);
        }
        let derived = read_is_derived(self.probe_overwrites, &path, self.in_compared);
        // DS-1940 reads each arm's derivation; it is not compared across arms here,
        // so after a projection assignment a read in an arm is not shown derived. A
        // `break` value joins its labeled block's value the same way.
        if ((self.in_branch && self.flows) || self.in_break) && !self.probe_overwrites.is_empty() {
            self.probe_in_branch = true;
        }
        if !derived && self.flows && !self.delegating {
            self.probe_overwritten = true;
        } else if self.flows && self.delegating {
            self.probe_delegated = true;
        } else if self.flows {
            self.probe_flows = true;
        } else if self.into_target {
            self.probe_into_target = true;
        } else {
            self.probe_dropped = true;
        }
    }

    /// Visits `visit` with `self.path_unknown` set.
    fn path_unknown(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.path_unknown;
        self.path_unknown = true;
        visit(self);
        self.path_unknown = outer;
    }

    /// Records a call of the looked-for target found where `self.flows` says.
    fn found_target_call(&mut self, by_method: bool) {
        if !self.flows {
            self.opaque = true;
        } else if self.delegating {
            self.delegated = true;
        } else if by_method {
            self.method = true;
        } else {
            self.path = true;
        }
    }

    fn visit_argument(&mut self, argument: &syn::Expr, flow: ArgumentFlow) {
        match flow {
            ArgumentFlow::Flows => self.visit_expr(argument),
            ArgumentFlow::Dropped => self.dropping(|visitor| visitor.visit_expr(argument)),
            ArgumentFlow::IntoTarget => {
                let outer = self.into_target;
                self.into_target = true;
                self.dropping(|visitor| visitor.visit_expr(argument));
                self.into_target = outer;
            }
            ArgumentFlow::Delegated => {
                let outer = self.delegating;
                self.delegating = true;
                self.visit_expr(argument);
                self.delegating = outer;
            }
        }
    }

    fn is_declared_target(&self, name: &syn::Ident) -> bool {
        self.targets.iter().any(|target| name == target)
    }

    /// Visits the index of `node` as flowing and returns the step it makes.
    ///
    /// Derivation (from DS-623): the value of the index decides which element is
    /// compared (`function[range]`), so it flows.
    fn index_projection(&mut self, node: &syn::ExprIndex) -> Projection<'static> {
        let outer = self.in_compared;
        self.in_compared = false;
        self.visit_expr(&node.index);
        self.in_compared = outer;
        Projection::Index(match node.index.as_ref() {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(index),
                ..
            }) => index.base10_parse::<usize>().ok(),
            _ => None,
        })
    }

    /// Visits a chain of field accesses and index expressions over `base` (`first`
    /// is the outermost step). Derivation (DS-623: the value of a projection is the
    /// projected element's): the steps are applied to a literal one after another, so
    /// only the element they select flows (`((a, b), c).0.1` passes `b`).
    fn visit_projections<'e>(&mut self, base: &'e syn::Expr, mut steps: Vec<Projection<'e>>) {
        let mut base = unwrap_parens(base);
        loop {
            match base {
                syn::Expr::Field(field) => {
                    steps.push(Projection::Member(&field.member));
                    base = unwrap_parens(&field.base);
                }
                syn::Expr::Index(index) => {
                    let step = self.index_projection(index);
                    steps.push(step);
                    base = unwrap_parens(&index.expr);
                }
                _ => break,
            }
        }
        steps.reverse();
        self.visit_projected(base, &steps);
    }

    /// Visits `base` with the elements that `steps` (innermost first) select as
    /// flowing, and the others as dropped. A base that is not a literal passes its
    /// value whole.
    fn visit_projected(&mut self, base: &syn::Expr, steps: &[Projection<'_>]) {
        let base = unwrap_parens(base);
        if let (Some(name), syn::Expr::Path(path)) = (self.probe, base) {
            if path.qself.is_none() && path.path.is_ident(name) && !steps.is_empty() {
                return self.probe_read(steps);
            }
        }
        let Some((first, rest)) = steps.split_first() else {
            return self.visit_expr(base);
        };
        // DS-1940: the steps select a position of each arm's value (of a block's tail,
        // read as one arm, so that a branch in it is joined there too). The value is
        // read whole as before, and its arms are joined at the selected position.
        let whole = |visitor: &mut Self| visitor.path_unknown(|visitor| visitor.visit_expr(base));
        match base {
            syn::Expr::If(node) => return self.visit_join(&Self::if_arms(node), steps, whole),
            syn::Expr::Match(node) => {
                let arms: Vec<Arm<'_>> = node.arms.iter().map(|arm| Arm::Expr(&arm.body)).collect();
                return self.visit_join(&arms, steps, whole);
            }
            syn::Expr::Block(node) => {
                let arms = match &node.label {
                    Some(label) => Self::labeled_exits(node, label),
                    None => vec![Arm::Block(&node.block)],
                };
                return self.visit_join(&arms, steps, whole);
            }
            syn::Expr::Unsafe(node) => {
                return self.visit_join(&[Arm::Block(&node.block)], steps, whole);
            }
            _ => {}
        }
        let selected = match first {
            Projection::Index(index) => *index,
            Projection::Member(syn::Member::Unnamed(member)) => Some(member.index as usize),
            Projection::Member(syn::Member::Named(_)) => None,
        };
        match base {
            syn::Expr::Tuple(tuple) => {
                let by_member = matches!(first, Projection::Member(_));
                for (index, element) in tuple.elems.iter().enumerate() {
                    self.visit_selected(element, by_member && selected == Some(index), rest);
                }
            }
            syn::Expr::Array(array) => {
                let by_index = matches!(first, Projection::Index(_));
                for (index, element) in array.elems.iter().enumerate() {
                    self.visit_selected(element, by_index && selected == Some(index), rest);
                }
            }
            syn::Expr::Struct(literal) => {
                for field in &literal.fields {
                    let projected = match (first, &field.member) {
                        (Projection::Member(syn::Member::Named(a)), syn::Member::Named(b)) => {
                            a == b
                        }
                        (Projection::Member(syn::Member::Unnamed(a)), syn::Member::Unnamed(b)) => {
                            a.index == b.index
                        }
                        _ => false,
                    };
                    self.visit_selected(&field.expr, projected, rest);
                }
                if let Some(base) = &literal.rest {
                    self.visit_element(base, false);
                }
            }
            syn::Expr::Macro(node) if node.mac.path.is_ident("vec") => {
                if self.init_mode || !self.macro_relevant(&node.mac) {
                    return;
                }
                let Some(arguments) = macro_arguments(&node.mac) else {
                    self.opaque = true;
                    return;
                };
                let by_index = matches!(first, Projection::Index(_));
                for (index, element) in arguments.checked.iter().enumerate() {
                    self.visit_selected(element, by_index && selected == Some(index), rest);
                }
            }
            syn::Expr::Repeat(_) => self.dropping(|visitor| visitor.visit_expr(base)),
            // The steps are not applied to a value read here, so a read of `probe`
            // in it does not know its path (DS-1939).
            _ => self.path_unknown(|visitor| visitor.visit_expr(base)),
        }
    }

    /// Visits an element of a literal: the selected one goes on with the remaining
    /// steps, any other is dropped.
    fn visit_selected(&mut self, element: &syn::Expr, selected: bool, rest: &[Projection<'_>]) {
        if selected {
            self.visit_projected(element, rest);
        } else {
            self.visit_element(element, false);
        }
    }

    /// Whether the tokens of a macro mention what is looked for.
    fn macro_relevant(&self, node: &syn::Macro) -> bool {
        if self.classifying
            && self
                .targets
                .iter()
                .chain(&self.sources.names)
                .any(|name| tokens_mention_ident(node.tokens.clone(), name))
        {
            return true;
        }
        match (self.probe, self.names) {
            (Some(name), _) => tokens_mention_ident(node.tokens.clone(), name),
            (None, Some(names)) => names.tokens_use_name(node.tokens.clone()),
            (None, None) => false,
        }
    }

    /// Visits `element` as `flows` says, for a literal's elements.
    fn visit_element(&mut self, element: &syn::Expr, flows: bool) {
        if flows {
            self.visit_expr(element);
        } else {
            self.dropping(|visitor| visitor.visit_expr(element));
        }
    }

    /// Visits the qualified self type and the generic arguments of a path, where a
    /// call (`f::<{ g() }>`) is not carried to the path's value.
    fn visit_path_generics(&mut self, node: &syn::ExprPath) {
        self.dropping(|visitor| {
            if let Some(qself) = &node.qself {
                visitor.visit_qself(qself);
            }
            for segment in &node.path.segments {
                visitor.visit_path_arguments(&segment.arguments);
            }
        });
    }

    fn dropping(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.flows;
        self.flows = false;
        visit(self);
        self.flows = outer;
    }

    /// Whether the value of `local`'s initializer reaches the tail expression of
    /// its block: a single name bound once, used where the tail expression flows,
    /// and not assigned or bound again in the statements after it. `Delegated`
    /// when the tail hands the name to a delegation target (DS-1960).
    fn local_reaches_tail(&self, local: &syn::Local, rest: &[syn::Stmt]) -> TailReach {
        let mut pattern = &local.pat;
        while let syn::Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        let syn::Pat::Ident(binding) = pattern else {
            return TailReach::No;
        };
        let Some((tail, statements)) = rest.split_last() else {
            return TailReach::No;
        };
        let syn::Stmt::Expr(tail, None) = tail else {
            return TailReach::No;
        };
        // A mutable binding may be changed before the tail expression reads it.
        if binding.by_ref.is_some()
            || binding.mutability.is_some()
            || binding.subpat.is_some()
            || local
                .init
                .as_ref()
                .is_none_or(|init| init.diverge.is_some())
        {
            return TailReach::No;
        }
        let name = binding.ident.to_string();
        let mut probe = CheckedCallVisitor::new(None, Some(&name), self.targets, self.sources);
        probe.flows = false;
        for statement in statements {
            probe.visit_stmt(statement);
        }
        probe.flows = true;
        probe.visit_expr(tail);
        if probe.probe_rebound {
            TailReach::No
        } else if probe.probe_flows {
            // DS-623 / DS-1962: a use that flows decides before a delegated one.
            TailReach::Flows
        } else if probe.probe_delegated {
            TailReach::Delegated
        } else {
            TailReach::No
        }
    }
}

/// How the name of a block-local `let` reaches the tail expression of its block.
enum TailReach {
    No,
    Flows,
    /// The tail hands the name to a delegation target (DS-1960).
    Delegated,
}

/// The values derived from a target's result that a branch's arms are read against
/// (DS-1940): a call of any declared target (`CheckedCallVisitor::targets`), and the
/// names of bindings that hold such a value.
#[derive(Default)]
struct JoinSources {
    /// Bindings derived from the target's result, bound once, not `mut`, and neither
    /// `Unknown` (DS-1939) nor joined from arms that differ (DS-1940).
    names: Vec<String>,
}

impl JoinSources {
    fn new<'b>(text: &str, bindings: impl IntoIterator<Item = &'b DerivedBinding>) -> Self {
        let mut names: Vec<String> = bindings
            .into_iter()
            .filter(|binding| {
                (binding.has_path_call || binding.has_method_call)
                    && !binding.uncertain
                    && !binding.mutable
                    && count_rebindings(text, &binding.name) == 1
            })
            .map(|binding| binding.name.clone())
            .collect();
        names.sort();
        names.dedup();
        Self { names }
    }
}

/// One arm of a branch expression, or one exit of a labeled block (DS-1940).
#[derive(Clone, Copy)]
enum Arm<'e> {
    Expr(&'e syn::Expr),
    Block(&'e syn::Block),
    /// A missing `else`, or a `break` without a value: the unit value.
    Unit,
}

impl Arm<'_> {
    /// DS-1940 leaves out an arm that is a failure divergence (DS-1951).
    fn always_fails(&self) -> bool {
        match self {
            Arm::Expr(expression) => expression_always_fails(expression),
            Arm::Block(block) => block_always_fails(block),
            Arm::Unit => false,
        }
    }
}

/// Whether every path through `expression` ends in a failure divergence (DS-1951),
/// read from syntax: a failure divergence macro, reached after statements that fall
/// through, in a block, or in every arm of an `if` with an `else` or of a `match`.
/// Narrower than DS-1951: a `return Err(..)` is not read, as whether the Test returns
/// `Result` is not known here, so such an arm counts as not derived.
fn expression_always_fails(expression: &syn::Expr) -> bool {
    match expression {
        syn::Expr::Macro(node) => is_failure_divergence_macro(&node.mac),
        syn::Expr::Paren(inner) => expression_always_fails(&inner.expr),
        syn::Expr::Group(inner) => expression_always_fails(&inner.expr),
        // A labeled block may be left by a `break` to its label.
        syn::Expr::Block(node) => node.label.is_none() && block_always_fails(&node.block),
        syn::Expr::Unsafe(node) => block_always_fails(&node.block),
        syn::Expr::If(node) => {
            let condition = match unwrap_parens(&node.cond) {
                syn::Expr::Let(binding) => &binding.expr,
                condition => condition,
            };
            !expression_may_exit(condition)
                && block_always_fails(&node.then_branch)
                && node
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, other)| expression_always_fails(other))
        }
        syn::Expr::Match(node) => {
            !node.arms.is_empty()
                && !expression_may_exit(&node.expr)
                && node.arms.iter().all(|arm| {
                    arm.guard
                        .as_ref()
                        .is_none_or(|(_, guard)| !expression_may_exit(guard))
                        && expression_always_fails(&arm.body)
                })
        }
        _ => false,
    }
}

fn block_always_fails(block: &syn::Block) -> bool {
    for statement in &block.stmts {
        let fails = match statement {
            syn::Stmt::Macro(node) => is_failure_divergence_macro(&node.mac),
            syn::Stmt::Expr(expression, _) => expression_always_fails(expression),
            syn::Stmt::Local(local) => local
                .init
                .as_ref()
                .is_some_and(|init| init.diverge.is_none() && expression_always_fails(&init.expr)),
            syn::Stmt::Item(_) => false,
        };
        if fails {
            return true;
        }
        // A statement before the divergence that may leave on some path (`if c {
        // return; }`) lets the arm end without failing.
        let may_exit = match statement {
            syn::Stmt::Macro(node) => macro_may_exit(&node.mac),
            syn::Stmt::Expr(expression, _) => expression_may_exit(expression),
            syn::Stmt::Local(local) => local
                .init
                .as_ref()
                .is_some_and(|init| init.diverge.is_some() || expression_may_exit(&init.expr)),
            syn::Stmt::Item(_) => false,
        };
        if may_exit || statement_completion(statement) != Completion::Falls {
            return false;
        }
    }
    false
}

/// The values of the `break`s that leave the block labeled `label`, outside closures,
/// `async` blocks, nested items and an inner expression with the same label.
struct LabeledBreaks<'e, 'l> {
    label: &'l syn::Lifetime,
    found: Vec<Option<&'e syn::Expr>>,
}

impl<'e> Visit<'e> for LabeledBreaks<'e, '_> {
    fn visit_expr_break(&mut self, node: &'e syn::ExprBreak) {
        if node
            .label
            .as_ref()
            .is_some_and(|label| label.ident == self.label.ident)
        {
            self.found.push(node.expr.as_deref());
        }
        syn::visit::visit_expr_break(self, node);
    }

    fn visit_expr_block(&mut self, node: &'e syn::ExprBlock) {
        if !self.shadows(node.label.as_ref()) {
            syn::visit::visit_expr_block(self, node);
        }
    }

    fn visit_expr_loop(&mut self, node: &'e syn::ExprLoop) {
        if !self.shadows(node.label.as_ref()) {
            syn::visit::visit_expr_loop(self, node);
        }
    }

    fn visit_expr_while(&mut self, node: &'e syn::ExprWhile) {
        if !self.shadows(node.label.as_ref()) {
            syn::visit::visit_expr_while(self, node);
        }
    }

    fn visit_expr_for_loop(&mut self, node: &'e syn::ExprForLoop) {
        if !self.shadows(node.label.as_ref()) {
            syn::visit::visit_expr_for_loop(self, node);
        }
    }

    fn visit_expr_closure(&mut self, _node: &'e syn::ExprClosure) {}
    fn visit_expr_async(&mut self, _node: &'e syn::ExprAsync) {}
    fn visit_item(&mut self, _node: &'e syn::Item) {}
}

impl LabeledBreaks<'_, '_> {
    fn shadows(&self, label: Option<&syn::Label>) -> bool {
        label.is_some_and(|label| label.name.ident == self.label.ident)
    }
}

impl<'ast> Visit<'ast> for CheckedCallVisitor<'_> {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        let outer_compared = self.in_compared;
        self.in_compared &= matches!(
            node,
            syn::Expr::Paren(_)
                | syn::Expr::Group(_)
                | syn::Expr::Reference(_)
                | syn::Expr::Field(_)
                | syn::Expr::Index(_)
                | syn::Expr::Path(_)
        );
        match node {
            syn::Expr::Path(_)
            | syn::Expr::Lit(_)
            | syn::Expr::Paren(_)
            | syn::Expr::Group(_)
            | syn::Expr::Reference(_)
            | syn::Expr::Cast(_)
            | syn::Expr::Try(_)
            | syn::Expr::Await(_)
            | syn::Expr::Unary(_)
            | syn::Expr::Binary(_)
            | syn::Expr::Call(_)
            | syn::Expr::MethodCall(_)
            | syn::Expr::Field(_)
            | syn::Expr::Index(_)
            | syn::Expr::Block(_)
            | syn::Expr::Unsafe(_)
            | syn::Expr::Const(_)
            | syn::Expr::If(_)
            | syn::Expr::Match(_)
            | syn::Expr::Tuple(_)
            | syn::Expr::Array(_)
            | syn::Expr::Struct(_)
            | syn::Expr::Macro(_) => syn::visit::visit_expr(self, node),
            syn::Expr::Break(_) => {
                let outer = self.in_break;
                self.in_break = true;
                self.dropping(|visitor| syn::visit::visit_expr(visitor, node));
                self.in_break = outer;
            }
            // Every other position (a closure, a loop, a range, `let`, `return`,
            // an assignment, `async`, ...) drops or holds the value.
            _ => self.dropping(|visitor| syn::visit::visit_expr(visitor, node)),
        }
        self.in_compared = outer_compared;
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        let flow = if let syn::Expr::Path(path) = node.func.as_ref() {
            let last = path.path.segments.last().map(|segment| &segment.ident);
            if self
                .names
                .is_some_and(|names| last.is_some_and(|name| name == names.symbol))
            {
                self.found_target_call(false);
            }
            if last.is_some_and(|name| self.is_declared_target(name)) {
                self.found_source();
            }
            // A probed `let` name may be the callee (`let f = |x| ..; f(2)`).
            if self.probe.is_some() {
                self.visit_expr(&node.func);
            } else {
                self.visit_path_generics(path);
            }
            if is_value_conversion(&path.path) {
                ArgumentFlow::Flows
            } else if last.is_some_and(|name| self.is_declared_target(name)) {
                ArgumentFlow::IntoTarget
            } else {
                ArgumentFlow::Delegated
            }
        } else {
            self.visit_expr(&node.func);
            ArgumentFlow::Delegated
        };
        for argument in &node.args {
            self.visit_argument(argument, flow);
        }
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.names.is_some_and(|names| node.method == names.symbol) {
            self.found_target_call(true);
        }
        if self.is_declared_target(&node.method) {
            self.found_source();
        }
        // DS-1937: the receiver flows to the method's value.
        self.visit_expr(&node.receiver);
        if let Some(turbofish) = &node.turbofish {
            self.dropping(|visitor| visitor.visit_angle_bracketed_generic_arguments(turbofish));
        }
        // DS-619's `.expect(..)` / `.expect_err(..)` is an assert-equivalent, not a
        // delegation target (DS-641), and its argument is a message (DS-1935).
        let is_assert_equivalent = node.method == "expect" || node.method == "expect_err";
        let flow = if is_assert_equivalent {
            ArgumentFlow::Dropped
        } else if self.is_declared_target(&node.method) {
            ArgumentFlow::IntoTarget
        } else {
            ArgumentFlow::Delegated
        };
        for argument in &node.args {
            self.visit_argument(argument, flow);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        self.visit_path_generics(node);
        if node.qself.is_none()
            && node
                .path
                .get_ident()
                .is_some_and(|ident| self.sources.names.iter().any(|name| ident == name))
        {
            self.found_source();
        }
        if let Some(name) = self.probe {
            if node.qself.is_none() && node.path.is_ident(name) {
                self.probe_read(&[]);
            }
            return;
        }
        if self
            .names
            .is_some_and(|names| names.path_is_value_use(node))
        {
            self.opaque = true;
        }
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        self.visit_projections(&node.base, vec![Projection::Member(&node.member)]);
    }

    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        let projection = self.index_projection(node);
        self.visit_projections(&node.expr, vec![projection]);
    }

    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        for field in &node.fields {
            self.visit_expr(&field.expr);
        }
        if let Some(rest) = &node.rest {
            self.visit_element(rest, false);
        }
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        if self.probe.is_some_and(|name| node.ident == name) {
            self.probe_rebound = true;
        }
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.visit_block_projected(node, &[]);
    }

    fn visit_expr_block(&mut self, node: &'ast syn::ExprBlock) {
        let Some(label) = &node.label else {
            return self.visit_block(&node.block);
        };
        let exits = Self::labeled_exits(node, label);
        self.visit_join(&exits, &[], |visitor| {
            syn::visit::visit_expr_block(visitor, node);
        });
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.visit_if_projected(node, &[]);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.visit_match_projected(node, &[]);
    }

    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        if self
            .probe
            .is_some_and(|name| is_assignment_to(&node.left, name))
        {
            self.probe_rebound = true;
        }
        self.dropping(|visitor| syn::visit::visit_expr_assign(visitor, node));
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if is_compound_assignment(node.op) {
            if self
                .probe
                .is_some_and(|name| is_assignment_to(&node.left, name))
            {
                self.probe_rebound = true;
            }
            self.dropping(|visitor| syn::visit::visit_expr_binary(visitor, node));
        } else if let Some(absorbing) = absorbing_literal(node.op) {
            // DS-1948: an operand whose sibling fixes the result is not a position
            // the value flows from.
            for (operand, sibling) in [(&node.left, &node.right), (&node.right, &node.left)] {
                self.visit_element(operand, operand_flows(sibling, absorbing));
            }
        } else if matches!(node.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_)) {
            // `==` / `!=` compares each operand whole, as `assert_eq!` does.
            for operand in [&node.left, &node.right] {
                let outer = self.in_compared;
                self.in_compared = true;
                self.visit_expr(operand);
                self.in_compared = outer;
            }
        } else {
            syn::visit::visit_expr_binary(self, node);
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.init_mode || !self.macro_relevant(node) {
            return;
        }
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        // `unreachable!` / `todo!` / `unimplemented!` inside an assert are not read:
        // a use of the value there is opaque (DS-615).
        let Some(arguments) = macro_arguments(node).filter(|_| !is_diverging_macro(&name)) else {
            self.opaque = true;
            return;
        };
        // DS-1954, DS-1962: a `debug_assert*` is not one of DS-617's macros, so a
        // value in its arguments is in no position that reaches the assert.
        let reaches = !is_debug_assert(&name);
        for expression in &arguments.checked {
            self.visit_element(expression, reaches);
        }
        if let Some(names) = self.names {
            self.opaque |= arguments
                .unchecked
                .iter()
                .any(|tokens| names.tokens_use_name(tokens.clone()));
        }
    }
}

/// Whether a callee path is a value conversion of DS-1960: a constructor known by
/// name (`Some`, `Ok`, `Err`, DS-1961) or the standard conversion `Box::new` /
/// `String::from` (DS-1961). Any other path, a tuple struct or a variant included,
/// is a function call, since a constructor is not told from the shape of a name
/// (DS-1960, DS-1967).
fn is_value_conversion(path: &syn::Path) -> bool {
    let mut segments = path.segments.iter().rev();
    let Some(last) = segments.next() else {
        return false;
    };
    if last.ident == "Some" || last.ident == "Ok" || last.ident == "Err" {
        return true;
    }
    let owner = segments.next();
    owner.is_some_and(|owner| {
        (owner.ident == "Box" && last.ident == "new")
            || (owner.ident == "String" && last.ident == "from")
    })
}

/// The expression inside any parentheses around `expression`.
fn unwrap_parens(expression: &syn::Expr) -> &syn::Expr {
    match expression {
        syn::Expr::Paren(inner) => unwrap_parens(&inner.expr),
        syn::Expr::Group(inner) => unwrap_parens(&inner.expr),
        other => other,
    }
}

/// DS-1948, DS-1949: the literal that fixes the result of `rust-cargo`'s `||` / `|`
/// (`true`) or `&&` / `&` (`false`) whatever the other operand is.
fn absorbing_literal(op: syn::BinOp) -> Option<bool> {
    match op {
        syn::BinOp::Or(_) | syn::BinOp::BitOr(_) => Some(true),
        syn::BinOp::And(_) | syn::BinOp::BitAnd(_) => Some(false),
        _ => None,
    }
}

/// Whether literals fix the value of an expression whatever its other operands are.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fixed {
    /// Fixed to this `bool`.
    Value(bool),
    /// A `bool` made only of literals, whose value is not evaluated here.
    Literal,
    /// Made only of literals, but not a `bool` by syntax (`0`, `1 + 1`).
    Constant,
    /// Holds a macro other than `matches!` whose tokens are literals only
    /// (`m!()`, `four_m!() == 4`): its expansion may run any code (DS-629), so it
    /// may absorb and its constancy is not decided.
    Opaque,
    /// Depends on something other than literals.
    Open,
}

/// The value literals fix (DS-1949): a `bool` literal; `!` of a fixed value;
/// `||` / `|` / `&&` / `&` with an absorbing operand (DS-1948) or with both
/// operands fixed; `^`, `==`, `!=` of fixed `bool`s; a comparison of two integer,
/// string or char literals of one kind (not floats). Parentheses do not change it.
/// A `matches!` is evaluated where DS-1965 fixes it (`matches_value`).
/// Only literals, `!`, binary operators and parentheses are evaluated; another form
/// built from literals only (`if true { .. }`, `{ true }`, `matches!(1, 1)`) is
/// `Fixed::Literal`; one holding another macro is `Fixed::Opaque`.
///
/// Derivation from DS-621 and DS-1949: an operand fixed this way (`!false`,
/// `1 == 1`, `(true || ok)`) absorbs as the literal it equals, so `X || 1 == 1` is
/// not read as `X` flowing. An operand made only of literals whose value is not
/// evaluated here (`1 + 1 == 2`) may absorb (`Fixed::Literal`).
pub(crate) fn fixed_value(expression: &syn::Expr) -> Fixed {
    fixed_value_read(expression, UndecidedMatches::Open)
}

/// How `fixed_value_read` reads a `matches!` whose pattern syntax shows neither
/// irrefutable nor refutable (`PatternClass::Undecidable`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum UndecidedMatches {
    /// As a value that depends on its scrutinee, as `fixed_value` reads it.
    Open,
    /// As the value `true` its pattern has when its type makes it irrefutable.
    True,
}

/// `fixed_value` with a reading of an undecidable `matches!` (DA-001 only).
fn fixed_value_read(expression: &syn::Expr, reading: UndecidedMatches) -> Fixed {
    if let syn::Expr::Macro(node) = unwrap_parens(expression) {
        if let Some(fixed) = matches_value(&node.mac, reading) {
            return fixed;
        }
    }
    let fixed_value = |inner: &syn::Expr| fixed_value_read(inner, reading);
    match unwrap_parens(expression) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(literal),
            ..
        }) => Fixed::Value(literal.value),
        syn::Expr::Lit(_) => Fixed::Constant,
        syn::Expr::Unary(unary) => match (unary.op, fixed_value(&unary.expr)) {
            (syn::UnOp::Not(_), Fixed::Value(value)) => Fixed::Value(!value),
            (syn::UnOp::Not(_), Fixed::Literal) => Fixed::Literal,
            (_, inner @ (Fixed::Open | Fixed::Opaque)) => inner,
            _ => Fixed::Constant,
        },
        syn::Expr::Binary(binary) => {
            let left = fixed_value(&binary.left);
            let right = fixed_value(&binary.right);
            if let Some(absorbing) = absorbing_literal(binary.op) {
                if left == Fixed::Value(absorbing) || right == Fixed::Value(absorbing) {
                    return Fixed::Value(absorbing);
                }
                if let (Fixed::Value(_), Fixed::Value(_)) = (left, right) {
                    return Fixed::Value(!absorbing);
                }
            }
            if let (Fixed::Value(a), Fixed::Value(b)) = (left, right) {
                match binary.op {
                    syn::BinOp::BitXor(_) | syn::BinOp::Ne(_) => return Fixed::Value(a != b),
                    syn::BinOp::Eq(_) => return Fixed::Value(a == b),
                    _ => {}
                }
            }
            if let Some(value) = compare_literals(binary) {
                return Fixed::Value(value);
            }
            if left == Fixed::Open || right == Fixed::Open {
                return Fixed::Open;
            }
            if left == Fixed::Opaque || right == Fixed::Opaque {
                return Fixed::Opaque;
            }
            let is_bool = |side: Fixed| matches!(side, Fixed::Value(_) | Fixed::Literal);
            match binary.op {
                syn::BinOp::Eq(_)
                | syn::BinOp::Ne(_)
                | syn::BinOp::Lt(_)
                | syn::BinOp::Le(_)
                | syn::BinOp::Gt(_)
                | syn::BinOp::Ge(_)
                | syn::BinOp::And(_)
                | syn::BinOp::Or(_) => Fixed::Literal,
                syn::BinOp::BitOr(_) | syn::BinOp::BitAnd(_) | syn::BinOp::BitXor(_)
                    if is_bool(left) || is_bool(right) =>
                {
                    Fixed::Literal
                }
                _ => Fixed::Constant,
            }
        }
        // Any other form with no variable, call or identifier in macro tokens
        // (`if true { true } else { false }`, `{ true }`, `matches!(1, 1)`) is built
        // from literals only, so it may absorb (DS-1948 leaves only non-literal
        // constants as a false PASS). A macro other than `matches!` may expand to
        // any code (DS-629), so a form holding one is `Opaque`, not a literal.
        other => {
            let mut finder = NonLiteralFinder {
                found: false,
                opaque_macro: false,
            };
            finder.visit_expr(other);
            if finder.found {
                Fixed::Open
            } else if finder.opaque_macro {
                Fixed::Opaque
            } else {
                Fixed::Literal
            }
        }
    }
}

/// The pattern and the guard of `matches!(value, pattern)` or
/// `matches!(value, pattern if guard)`; `None` for another macro, `Some(Err)` when
/// its tokens are not that grammar.
fn parse_matches(node: &syn::Macro) -> Option<syn::Result<(syn::Pat, Option<syn::Expr>)>> {
    if node
        .path
        .segments
        .last()
        .is_none_or(|segment| segment.ident != "matches")
    {
        return None;
    }
    Some(node.parse_body_with(|input: syn::parse::ParseStream<'_>| {
        let _value: syn::Expr = input.parse()?;
        let _comma: syn::Token![,] = input.parse()?;
        let pattern = syn::Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(syn::Token![if]) {
            let _if: syn::Token![if] = input.parse()?;
            Some(input.parse::<syn::Expr>()?)
        } else {
            None
        };
        let _trailing: Option<syn::Token![,]> = input.parse()?;
        Ok((pattern, guard))
    }))
}

/// The value of a `matches!` that DS-1965 fixes: `true` for a pattern that is
/// irrefutable (DS-1952) with no guard or a guard fixed to `true`, `false` for a
/// guard fixed to `false` (DA-001's `fixed_value`), each as `Fixed::Value`.
///
/// A pattern whose irrefutability its type decides (`PatternClass::Undecidable`:
/// `S { .. }`, `E::V(_)`, `E::A`, `x`) is read as `reading` says: `None` (as any
/// other macro, which its identifiers make `Fixed::Open`) under
/// `UndecidedMatches::Open`, `true` under `UndecidedMatches::True`. DA-001 reads
/// both and leaves the constancy undecided where they differ (DS-621's escape).
/// Any other pattern not fixed is `None`.
fn matches_value(node: &syn::Macro, reading: UndecidedMatches) -> Option<Fixed> {
    let (pattern, guard) = parse_matches(node)?.ok()?;
    let class = pattern_class(&pattern);
    match guard.as_ref().map(|guard| fixed_value_read(guard, reading)) {
        Some(Fixed::Value(false)) => Some(Fixed::Value(false)),
        Some(Fixed::Value(true)) | None if class == PatternClass::Irrefutable => {
            Some(Fixed::Value(true))
        }
        Some(Fixed::Value(true)) | None
            if class == PatternClass::Undecidable && reading == UndecidedMatches::True =>
        {
            Some(Fixed::Value(true))
        }
        _ => None,
    }
}

/// What syntax shows of a `matches!` pattern for DA-001 (DS-1952, DS-1965).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PatternClass {
    /// Irrefutable whatever the type: `_`, `..`, `x @ _`, a slice of a rest only,
    /// and tuples, references and parentheses of these.
    Irrefutable,
    /// Irrefutable for some type and refutable for another, which syntax does not
    /// tell: a lone identifier as the whole pattern (a binding, a constant, a unit
    /// struct or a unit variant) or one written like a variant below it, a path, a tuple-struct or struct pattern (a struct or an enum
    /// variant), a slice that fixes a length, or a macro.
    Undecidable,
    /// Refutable whatever the type: a literal, a range, `Some` / `None` / `Ok` /
    /// `Err` (the prelude's variants, as `option_or_result_variant` reads them),
    /// or a pattern holding one in a position every value reaches.
    Refutable,
}

impl PatternClass {
    /// The class of patterns that must all match (tuple elements, fields).
    fn all(classes: impl IntoIterator<Item = Self>) -> Self {
        classes
            .into_iter()
            .fold(Self::Irrefutable, |all, class| match (all, class) {
                (Self::Refutable, _) | (_, Self::Refutable) => Self::Refutable,
                (Self::Undecidable, _) | (_, Self::Undecidable) => Self::Undecidable,
                (Self::Irrefutable, Self::Irrefutable) => Self::Irrefutable,
            })
    }

    /// The class of alternatives one of which must match (an `or` pattern).
    fn any(classes: impl IntoIterator<Item = Self>) -> Self {
        classes
            .into_iter()
            .fold(Self::Refutable, |any, class| match (any, class) {
                (Self::Irrefutable, _) | (_, Self::Irrefutable) => Self::Irrefutable,
                (Self::Undecidable, _) | (_, Self::Undecidable) => Self::Undecidable,
                (Self::Refutable, Self::Refutable) => Self::Refutable,
            })
    }

    /// A pattern with these elements whose path or length a type may refute.
    fn refutable_by_type(self) -> Self {
        match self {
            Self::Irrefutable | Self::Undecidable => Self::Undecidable,
            Self::Refutable => Self::Refutable,
        }
    }
}

/// The class of a `matches!` pattern (DS-1952 read for DA-001, DS-621's escape).
/// `Irrefutable` is exactly what DS-1965 fixes to `true` whatever the type.
///
/// A lone identifier as the whole pattern (`x`, `ref x`, `mut x`, `Kind`, through
/// parentheses and `or` alternatives) is `Undecidable`: it may name a constant or a
/// unit struct or variant. Below a tuple, a reference, a struct or an `@`, a
/// lowercase one is read as a binding (`nested_pattern_class`), as DS-1952 and
/// `is_irrefutable` read it.
fn pattern_class(pattern: &syn::Pat) -> PatternClass {
    match pattern {
        syn::Pat::Ident(ident) if ident.subpat.is_none() && ident.ident == "None" => {
            PatternClass::Refutable
        }
        syn::Pat::Ident(ident) if ident.subpat.is_none() => PatternClass::Undecidable,
        syn::Pat::Or(or) => PatternClass::any(or.cases.iter().map(pattern_class)),
        syn::Pat::Paren(inner) => pattern_class(&inner.pat),
        other => nested_pattern_class(other),
    }
}

/// `pattern_class` of a pattern below the top: a lone lowercase identifier is a
/// binding (`Irrefutable`), one written like a variant or a constant (`Kind`) is
/// `Undecidable`, and `None` is `Refutable`.
fn nested_pattern_class(pattern: &syn::Pat) -> PatternClass {
    match pattern {
        syn::Pat::Wild(_) | syn::Pat::Rest(_) => PatternClass::Irrefutable,
        syn::Pat::Ident(ident) => match &ident.subpat {
            Some((_, sub)) => nested_pattern_class(sub),
            None if ident.ident == "None" => PatternClass::Refutable,
            None if is_variant_like(&ident.ident) => PatternClass::Undecidable,
            None => PatternClass::Irrefutable,
        },
        syn::Pat::Or(or) => PatternClass::any(or.cases.iter().map(nested_pattern_class)),
        syn::Pat::Paren(inner) => nested_pattern_class(&inner.pat),
        syn::Pat::Reference(inner) => nested_pattern_class(&inner.pat),
        syn::Pat::Type(inner) => nested_pattern_class(&inner.pat),
        syn::Pat::Tuple(tuple) => PatternClass::all(tuple.elems.iter().map(nested_pattern_class)),
        syn::Pat::Slice(slice) if may_be_refuted_by_type(pattern) => {
            PatternClass::all(slice.elems.iter().map(nested_pattern_class)).refutable_by_type()
        }
        syn::Pat::Slice(slice) => PatternClass::all(slice.elems.iter().map(nested_pattern_class)),
        syn::Pat::TupleStruct(node) if option_or_result_variant(&node.path).is_some() => {
            PatternClass::Refutable
        }
        syn::Pat::TupleStruct(node) => {
            PatternClass::all(node.elems.iter().map(nested_pattern_class)).refutable_by_type()
        }
        syn::Pat::Struct(node) if option_or_result_variant(&node.path).is_some() => {
            PatternClass::Refutable
        }
        syn::Pat::Struct(node) => PatternClass::all(
            node.fields
                .iter()
                .map(|field| nested_pattern_class(&field.pat)),
        )
        .refutable_by_type(),
        syn::Pat::Path(path)
            if path.qself.is_none()
                && path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "None") =>
        {
            PatternClass::Refutable
        }
        syn::Pat::Lit(_) | syn::Pat::Range(_) => PatternClass::Refutable,
        // A path (`E::A`, a unit struct or a constant), a macro, a `const` block or a
        // form syn does not name may be irrefutable for some type.
        _ => PatternClass::Undecidable,
    }
}

/// Whether an expression refers to a variable, calls something (a struct literal
/// and a closure included, whose comparison or call runs user code), or has an
/// identifier other than `true` / `false` in macro tokens.
struct NonLiteralFinder {
    found: bool,
    /// A macro other than `matches!` with literal-only tokens.
    opaque_macro: bool,
}

impl<'ast> Visit<'ast> for NonLiteralFinder {
    fn visit_expr_path(&mut self, _node: &'ast syn::ExprPath) {
        self.found = true;
    }

    fn visit_expr_struct(&mut self, _node: &'ast syn::ExprStruct) {
        self.found = true;
    }

    fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {
        self.found = true;
    }

    fn visit_expr_call(&mut self, _node: &'ast syn::ExprCall) {
        self.found = true;
    }

    fn visit_expr_method_call(&mut self, _node: &'ast syn::ExprMethodCall) {
        self.found = true;
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        fn has_ident(tokens: proc_macro2::TokenStream) -> bool {
            tokens.into_iter().any(|tree| match tree {
                proc_macro2::TokenTree::Ident(ident) => ident != "true" && ident != "false",
                proc_macro2::TokenTree::Group(group) => has_ident(group.stream()),
                proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
            })
        }
        self.found |= has_ident(node.tokens.clone());
        self.opaque_macro |= !node.path.is_ident("matches");
    }
}

/// The value of a comparison of two literals of one kind (integer, string, char),
/// or `None` when it is not one. Float literals are not compared: whether the
/// comparison is in `f32` or `f64` is not fixed by syntax, so such a comparison is
/// left unevaluated (`Fixed::Literal`).
fn compare_literals(binary: &syn::ExprBinary) -> Option<bool> {
    use std::cmp::Ordering;
    let literal = |expression: &syn::Expr| match unwrap_parens(expression) {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => Some(lit.clone()),
        _ => None,
    };
    let ordering = match (literal(&binary.left)?, literal(&binary.right)?) {
        (syn::Lit::Int(a), syn::Lit::Int(b)) => a
            .base10_parse::<i128>()
            .ok()?
            .cmp(&b.base10_parse::<i128>().ok()?),
        (syn::Lit::Str(a), syn::Lit::Str(b)) => a.value().cmp(&b.value()),
        (syn::Lit::Char(a), syn::Lit::Char(b)) => a.value().cmp(&b.value()),
        _ => return None,
    };
    match binary.op {
        syn::BinOp::Eq(_) => Some(ordering == Ordering::Equal),
        syn::BinOp::Ne(_) => Some(ordering != Ordering::Equal),
        syn::BinOp::Lt(_) => Some(ordering == Ordering::Less),
        syn::BinOp::Le(_) => Some(ordering != Ordering::Greater),
        syn::BinOp::Gt(_) => Some(ordering == Ordering::Greater),
        syn::BinOp::Ge(_) => Some(ordering != Ordering::Less),
        _ => None,
    }
}

/// Whether an operand of `||` / `|` / `&&` / `&` beside `sibling` is a position
/// the value flows from (DS-1948): not when `sibling` is fixed to the absorbing
/// value, nor when it is made only of literals whose value is not evaluated.
fn operand_flows(sibling: &syn::Expr, absorbing: bool) -> bool {
    match fixed_value(sibling) {
        Fixed::Value(value) => value != absorbing,
        Fixed::Literal | Fixed::Opaque => false,
        Fixed::Constant | Fixed::Open => true,
    }
}

/// DA-001's constancy of one argument (DS-621, DS-1949): `AllLiteral` when literals
/// fix it; `Undetermined` when it holds a macro whose expansion is not read
/// (DS-629), or when an operand of `||` / `|` / `&&` / `&` in it is made only of
/// literals whose value is not evaluated or holds such a macro, so it may absorb
/// the rest.
fn argument_constancy(argument: &syn::Expr) -> Constancy {
    argument_constancy_read(argument, UndecidedMatches::Open)
}

/// DA-001's constancy of one argument, with a `matches!` whose pattern syntax shows
/// neither irrefutable nor refutable (`PatternClass::Undecidable`) read both ways:
/// as `true`, the value its type makes it when the pattern is irrefutable, and as
/// a value its scrutinee decides. Where the two readings differ, the constancy is
/// not decided (DS-621's "定数性を確定できない式", DS-1952, DS-1965): `S { .. }` and
/// `E::V(_)` look the same, and only the type tells whether one is constant.
fn da_001_argument_constancy(argument: &syn::Expr) -> Constancy {
    match (
        argument_constancy_read(argument, UndecidedMatches::Open),
        argument_constancy_read(argument, UndecidedMatches::True),
    ) {
        (Constancy::AllLiteral, Constancy::AllLiteral) => Constancy::AllLiteral,
        (Constancy::NotAllLiteral, Constancy::NotAllLiteral) => Constancy::NotAllLiteral,
        _ => Constancy::Undetermined,
    }
}

fn argument_constancy_read(argument: &syn::Expr, reading: UndecidedMatches) -> Constancy {
    match fixed_value_read(argument, reading) {
        Fixed::Open => {}
        Fixed::Opaque => return Constancy::Undetermined,
        Fixed::Value(_) | Fixed::Literal | Fixed::Constant => return Constancy::AllLiteral,
    }
    let mut finder = UnevaluatedAbsorberFinder {
        found: false,
        reading,
    };
    finder.visit_expr(argument);
    if finder.found {
        Constancy::Undetermined
    } else {
        Constancy::NotAllLiteral
    }
}

struct UnevaluatedAbsorberFinder {
    found: bool,
    reading: UndecidedMatches,
}

impl<'ast> Visit<'ast> for UnevaluatedAbsorberFinder {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if absorbing_literal(node.op).is_some() {
            let may_absorb = |side: &syn::Expr| {
                matches!(
                    fixed_value_read(side, self.reading),
                    Fixed::Literal | Fixed::Opaque
                )
            };
            self.found |= may_absorb(&node.left) || may_absorb(&node.right);
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

fn is_assignment_to(left: &syn::Expr, name: &str) -> bool {
    matches!(left, syn::Expr::Path(path) if path.qself.is_none() && path.path.is_ident(name))
}

fn is_compound_assignment(op: syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Number of places in the function that bind `name` again: patterns (`let`,
/// closure parameter, match arm, parameter, `for`) and assignments `name = ..` /
/// `name += ..`.
fn count_rebindings(text: &str, name: &str) -> usize {
    syn::parse_str::<syn::ItemFn>(text).map_or(0, |function| count_rebindings_in(&function, name))
}

fn count_rebindings_in(function: &syn::ItemFn, name: &str) -> usize {
    struct Counter<'a> {
        name: &'a str,
        count: usize,
    }
    impl<'ast> Visit<'ast> for Counter<'_> {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            if node.ident == self.name {
                self.count += 1;
            }
            syn::visit::visit_pat_ident(self, node);
        }

        // The cases of an or-pattern bind the same names once (DS-1938).
        fn visit_pat_or(&mut self, node: &'ast syn::PatOr) {
            let before = self.count;
            let mut most = 0;
            for case in &node.cases {
                self.count = 0;
                self.visit_pat(case);
                most = most.max(self.count);
            }
            self.count = before + most;
        }

        fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
            if is_assignment_to(&node.left, self.name) {
                self.count += 1;
            }
            syn::visit::visit_expr_assign(self, node);
        }

        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            if is_compound_assignment(node.op) && is_assignment_to(&node.left, self.name) {
                self.count += 1;
            }
            syn::visit::visit_expr_binary(self, node);
        }
    }
    let mut counter = Counter { name, count: 0 };
    counter.visit_item_fn(function);
    counter.count
}

/// How the function changes the value of a `let mut` binding through something other
/// than the name itself (`name = ..` is `count_rebindings`').
#[derive(Default)]
struct PlaceChanges {
    /// The whole binding is not shown to keep the target's result (DS-1959).
    whole: bool,
    /// The paths assigned by projection assignments whose path is fixed by syntax
    /// (DS-1956), and those of compound assignments whose order against such an
    /// assignment would decide their derivation.
    overwrites: Vec<Vec<PlaceStep>>,
    /// A projection assignment or a compound assignment with a path fixed by
    /// syntax is made, whether or not it is listed in `overwrites`.
    projected: bool,
}

/// The changes the function makes to the `let mut` binding `name` through a place.
///
/// A projection assignment `name.p = e` or `name.p op= e` whose path `p` is made of
/// field names, tuple numbers and integer-literal indexes only, outside a closure, an
/// `async` block and macro tokens, changes only the position `p` (DS-1956). A
/// compound assignment keeps `p` derived, as the original value is, unless a plain
/// assignment is made to a path that is a prefix of `p` or has `p` as a prefix: with
/// statement order not read, which one comes first is not known, so `p` is listed
/// as assigned too. Every other change below makes the `whole` binding not shown to
/// be the target's result (DS-1959, DS-615) when the binding is
/// - the base of an assigned place whose path is not fixed by syntax (`r[i] = ..`,
///   `*r = ..`, `(*r).x = ..`, or an element of a destructuring assignment), or a
///   projection assignment in a closure, an `async` block or macro tokens,
/// - borrowed mutably (`&mut r`, `&mut r.0`, `&raw mut r`, or a `ref mut` pattern
///   whose matched value is `r`: `let (ref mut a, _) = r`, `match r { .. }`,
///   `if let .. = r`),
/// - the receiver of a method call (`r.clear()`), also inside the arguments of an
///   assert (DS-1959): whether the method takes `&mut self` is not decided by this
///   capability, so every method call on the `let mut` binding counts, or
/// - assigned as the plain name (`r = ..`, `r += ..`, also in a closure) inside a
///   macro's arguments, which `count_rebindings` does not read.
///
/// Macro arguments of `matches!`, `format!`, `vec!` and the assert macros are read
/// as expressions. DS-629 reads dataflow inside a macro expansion as UNKNOWN, so
/// the name in the tokens of any other macro outside an assert (`set!{ r.0 = 1 }`,
/// `reset!(r)`, `addr_of_mut!(r)`) counts as a change too, as its tokens are not
/// read.
///
/// Not covered: a binding of a target that returns `&mut T` need not be `let mut`
/// to be changed.
fn place_changes(text: &str, name: &str, assert_spans: &[(usize, usize)]) -> PlaceChanges {
    struct Changes<'a> {
        text: &'a str,
        name: &'a str,
        assert_spans: &'a [(usize, usize)],
        /// Depth of macro arguments being visited, where `count_rebindings` does not
        /// see an assignment to the plain name.
        in_macro: usize,
        /// Depth of closures and `async` blocks being visited, which capture the
        /// binding (DS-1959).
        in_closure: usize,
        found: bool,
        assigned: Vec<Vec<PlaceStep>>,
        compound: Vec<Vec<PlaceStep>>,
    }
    impl Changes<'_> {
        /// The path of a projection assignment to `name` that DS-1956 reads, or
        /// `None` when the left side changes the binding otherwise or not at all.
        fn fixed_projection(&self, left: &syn::Expr) -> Option<Vec<PlaceStep>> {
            if self.in_macro > 0 || self.in_closure > 0 {
                return None;
            }
            let path = fixed_place_path(left, self.name)?;
            (!path.is_empty()).then_some(path)
        }
    }
    impl<'ast> Visit<'ast> for Changes<'_> {
        fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
            if let Some(path) = self.fixed_projection(&node.left) {
                self.assigned.push(path);
            } else {
                self.found |= assigned_place_roots_at(&node.left, self.name, false)
                    || (self.in_macro > 0 && is_assignment_to(&node.left, self.name));
            }
            syn::visit::visit_expr_assign(self, node);
        }

        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            if is_compound_assignment(node.op) {
                if let Some(path) = self.fixed_projection(&node.left) {
                    // DS-1948: `p &= false` / `p |= true` fixes `p` whatever its
                    // original value, so `p` holds the literal, as after `p = e`.
                    let absorbed = match node.op {
                        syn::BinOp::BitAndAssign(_) => Some(false),
                        syn::BinOp::BitOrAssign(_) => Some(true),
                        _ => None,
                    }
                    .is_some_and(|absorbing| !operand_flows(&node.right, absorbing));
                    if absorbed {
                        self.assigned.push(path);
                    } else {
                        self.compound.push(path);
                    }
                } else {
                    self.found |= assigned_place_roots_at(&node.left, self.name, false)
                        || (self.in_macro > 0 && is_assignment_to(&node.left, self.name));
                }
            }
            syn::visit::visit_expr_binary(self, node);
        }

        fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
            self.in_closure += 1;
            syn::visit::visit_expr_closure(self, node);
            self.in_closure -= 1;
        }

        fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
            self.in_closure += 1;
            syn::visit::visit_expr_async(self, node);
            self.in_closure -= 1;
        }

        fn visit_expr_reference(&mut self, node: &'ast syn::ExprReference) {
            self.found |= node.mutability.is_some() && place_roots_at(&node.expr, self.name);
            syn::visit::visit_expr_reference(self, node);
        }

        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            // DS-1959: a receiver counts inside the arguments of an assert too.
            self.found |= place_roots_at(&node.receiver, self.name);
            syn::visit::visit_expr_method_call(self, node);
        }

        fn visit_expr_raw_addr(&mut self, node: &'ast syn::ExprRawAddr) {
            self.found |= matches!(node.mutability, syn::PointerMutability::Mut(_))
                && place_roots_at(&node.expr, self.name);
            syn::visit::visit_expr_raw_addr(self, node);
        }

        fn visit_local(&mut self, node: &'ast syn::Local) {
            self.found |= node.init.as_ref().is_some_and(|init| {
                place_roots_at(&init.expr, self.name) && pattern_borrows_mutably(&node.pat)
            });
            syn::visit::visit_local(self, node);
        }

        fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
            self.found |=
                place_roots_at(&node.expr, self.name) && pattern_borrows_mutably(&node.pat);
            syn::visit::visit_expr_let(self, node);
        }

        fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
            self.found |= place_roots_at(&node.expr, self.name)
                && node
                    .arms
                    .iter()
                    .any(|arm| pattern_borrows_mutably(&arm.pat));
            syn::visit::visit_expr_match(self, node);
        }

        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            self.in_macro += 1;
            self.visit_macro_tokens(node);
            self.in_macro -= 1;
        }
    }
    impl Changes<'_> {
        fn visit_macro_tokens(&mut self, node: &syn::Macro) {
            let offset = span_offset(node.path.span().start(), self.text);
            let in_assert = self
                .assert_spans
                .iter()
                .any(|&(start, end)| offset >= start && offset < end);
            let read = macro_arguments(node);
            self.found |= !in_assert
                && read.is_none()
                && tokens_mention_ident(node.tokens.clone(), self.name);
            if node.path.is_ident("matches") {
                // Only the first argument is an expression; the pattern and its guard
                // may bind `ref mut` or mutate the value, so the name there is a change.
                if let Some(arguments) = read {
                    for expression in &arguments.checked {
                        self.visit_expr(expression);
                    }
                    self.found |= arguments
                        .unchecked
                        .iter()
                        .any(|tokens| tokens_mention_ident(tokens.clone(), self.name));
                    let parsed = syn::parse::Parser::parse2(
                        |input: syn::parse::ParseStream<'_>| {
                            let scrutinee: syn::Expr = input.parse()?;
                            input.parse::<syn::Token![,]>()?;
                            let pattern = syn::Pat::parse_multi_with_leading_vert(input)?;
                            let guard = if input.peek(syn::Token![if]) {
                                input.parse::<syn::Token![if]>()?;
                                Some(input.parse::<syn::Expr>()?)
                            } else {
                                None
                            };
                            input.parse::<Option<syn::Token![,]>>()?;
                            Ok((scrutinee, pattern, guard))
                        },
                        node.tokens.clone(),
                    );
                    match parsed {
                        Ok((scrutinee, pattern, guard)) => {
                            self.found |= place_roots_at(&scrutinee, self.name)
                                && pattern_borrows_mutably(&pattern);
                            if let Some(guard) = guard {
                                self.visit_expr(&guard);
                            }
                        }
                        Err(_) => {
                            self.found |= tokens_mention_ident(node.tokens.clone(), self.name);
                        }
                    }
                } else {
                    self.found |= tokens_mention_ident(node.tokens.clone(), self.name);
                }
                return;
            }
            if let Ok(arguments) = syn::parse::Parser::parse2(
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
                node.tokens.clone(),
            ) {
                for argument in &arguments {
                    self.visit_expr(argument);
                }
            }
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return PlaceChanges::default();
    };
    // The arguments of `panic!` / `unreachable!` / `todo!` / `unimplemented!` are
    // checked like the body: a method receiver or an unread macro there counts as a
    // change, as these macros compare nothing (DS-1959, DS-615).
    let assert_spans: Vec<(usize, usize)> = assert_spans
        .iter()
        .copied()
        .filter(|&(start, _)| {
            !macro_name_before(text, start)
                .is_some_and(|name| name == "panic" || is_diverging_macro(name))
        })
        .collect();
    let mut changes = Changes {
        text,
        name,
        assert_spans: &assert_spans,
        in_macro: 0,
        in_closure: 0,
        found: false,
        assigned: Vec::new(),
        compound: Vec::new(),
    };
    changes.visit_item_fn(&function);
    let projected = !changes.assigned.is_empty() || !changes.compound.is_empty();
    let related = |a: &[PlaceStep], b: &[PlaceStep]| a.starts_with(b) || b.starts_with(a);
    let mut overwrites = changes.assigned.clone();
    overwrites.extend(changes.compound.into_iter().filter(|path| {
        changes
            .assigned
            .iter()
            .any(|assigned| related(path, assigned))
    }));
    PlaceChanges {
        whole: changes.found,
        overwrites,
        projected,
    }
}

/// The path of the place `expression` when it is `name` followed by field names,
/// tuple numbers and integer-literal indexes only (DS-1956), innermost step first;
/// `None` for any other place.
fn fixed_place_path(expression: &syn::Expr, name: &str) -> Option<Vec<PlaceStep>> {
    match unwrap_parens(expression) {
        syn::Expr::Path(path) => (path.qself.is_none() && path.path.is_ident(name)).then(Vec::new),
        syn::Expr::Field(field) => {
            let mut path = fixed_place_path(&field.base, name)?;
            path.push(match &field.member {
                syn::Member::Named(ident) => PlaceStep::Field(field_name(ident)),
                syn::Member::Unnamed(index) => PlaceStep::Tuple(index.index),
            });
            Some(path)
        }
        syn::Expr::Index(index) => {
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(literal),
                ..
            }) = unwrap_parens(&index.index)
            else {
                return None;
            };
            let mut path = fixed_place_path(&index.expr, name)?;
            path.push(PlaceStep::Index(literal.base10_parse().ok()?));
            Some(path)
        }
        _ => None,
    }
}

/// Whether a pattern binds a part of the matched value as `ref mut`.
fn pattern_borrows_mutably(pattern: &syn::Pat) -> bool {
    struct RefMut(bool);
    impl<'ast> Visit<'ast> for RefMut {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            self.0 |= node.by_ref.is_some() && node.mutability.is_some();
            syn::visit::visit_pat_ident(self, node);
        }
    }
    let mut found = RefMut(false);
    found.visit_pat(pattern);
    found.0
}

/// Whether `expression` is `name` or a field, index or dereference of a place that is.
fn place_roots_at(expression: &syn::Expr, name: &str) -> bool {
    match unwrap_parens(expression) {
        syn::Expr::Path(path) => path.qself.is_none() && path.path.is_ident(name),
        syn::Expr::Field(field) => place_roots_at(&field.base, name),
        syn::Expr::Index(index) => place_roots_at(&index.expr, name),
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            place_roots_at(&unary.expr, name)
        }
        _ => false,
    }
}

/// Whether the left side of an assignment changes `name` other than as the plain
/// name at its top (`nested` is true inside a destructuring pattern, where the plain
/// name is assigned as well).
fn assigned_place_roots_at(left: &syn::Expr, name: &str, nested: bool) -> bool {
    match unwrap_parens(left) {
        syn::Expr::Tuple(tuple) => tuple
            .elems
            .iter()
            .any(|element| assigned_place_roots_at(element, name, true)),
        syn::Expr::Array(array) => array
            .elems
            .iter()
            .any(|element| assigned_place_roots_at(element, name, true)),
        syn::Expr::Struct(literal) => literal
            .fields
            .iter()
            .any(|field| assigned_place_roots_at(&field.expr, name, true)),
        syn::Expr::Call(call) => call
            .args
            .iter()
            .any(|argument| assigned_place_roots_at(argument, name, true)),
        other => place_roots_at(other, name) && (nested || !is_assignment_to(other, name)),
    }
}

/// One call of a declared target found in the Test body.
struct CallSite {
    /// Byte offset where the callee name starts (the `ExprCall` function path,
    /// or the method name of an `ExprMethodCall`). `call_open_position` reads
    /// the argument list from here.
    callee_start: usize,
    /// Byte offset where the whole call expression starts (for a method call,
    /// its receiver).
    expr_start: usize,
    /// `true` for `receiver.name(..)`: identified by name only.
    is_method: bool,
}

fn target_call_result_is_verified(
    text: &str,
    symbol: &str,
    targets: &[String],
    assert_spans: &[(usize, usize)],
    regions: &[(usize, usize)],
    never_run: &NeverRun<'_>,
) -> TargetCallVerification {
    // DS-1962: a call in a closure that is not tracked, an `async` block or a nested
    // item is not counted. It never verifies the target, and when no counted call
    // does, it makes DA-003 UNKNOWN rather than FAIL.
    let (non_counted_sites, call_sites): (Vec<CallSite>, Vec<CallSite>) =
        find_call_sites(text, symbol)
            .into_iter()
            .partition(|site| in_regions(site.callee_start, regions));
    // DS-1940: the arms of a branch are read against every value derived from the
    // target's result. The bindings are found once without the names, whose arms
    // may then read as differing, and again with the names those found.
    let pattern_bindings = find_pattern_bindings(text, symbol, regions);
    let first =
        find_let_bindings_containing_call(text, symbol, targets, &JoinSources::default(), regions);
    let first_sources = JoinSources::new(text, first.iter().chain(&pattern_bindings));
    let let_bindings =
        find_let_bindings_containing_call(text, symbol, targets, &first_sources, regions);
    let sources = JoinSources::new(text, let_bindings.iter().chain(&pattern_bindings));
    let macro_use = macro_target_use(text, symbol, targets, &sources, regions, never_run.regions);
    let function = syn::parse_str::<syn::ItemFn>(text).ok();
    // A divergence that never runs is not reached (`NeverRun`).
    let (dead_divergences, divergences): (Vec<Divergence<'_>>, Vec<Divergence<'_>>) = function
        .as_ref()
        .map(|function| collect_divergences(text, function))
        .unwrap_or_default()
        .into_iter()
        .partition(|divergence| divergence.dead);
    // DS-1947: a call whose value flows to a position that decides whether a
    // failure divergence runs reaches it, as a call in an assert's checked argument
    // reaches the assert.
    let decide = |divergences: &[Divergence<'_>]| {
        function
            .as_ref()
            .map_or_else(DecidedCalls::default, |function| {
                divergence_target_reach(
                    divergences,
                    &NameUse::new(text, symbol, function),
                    targets,
                    &sources,
                )
            })
    };
    let decided = decide(&divergences);
    let dead_decided = decide(&dead_divergences);
    // Only a signal that would verify is dropped where the code never runs; one that
    // keeps DA-003 UNKNOWN (a branch, a delegation target, a macro not read) is kept.
    let mut never_run_only = macro_use.never_run || dead_decided.path || dead_decided.method;
    let mixed = macro_use.mixed || decided.mixed || dead_decided.mixed;
    let every_counted_assert: Vec<(usize, usize)> = assert_spans
        .iter()
        .chain(never_run.asserts)
        .copied()
        .collect();
    let every_divergence: Vec<Divergence<'_>> = divergences
        .iter()
        .chain(&dead_divergences)
        .cloned()
        .collect();
    let bindings: Vec<(DerivedBinding, BindingReach)> = let_bindings
        .into_iter()
        .chain(pattern_bindings)
        .map(|binding| {
            let reach = binding_reach(
                text,
                &binding,
                targets,
                &sources,
                &every_counted_assert,
                &every_divergence,
            );
            // A binding that reaches only what never runs is not verified by it; the
            // other outcomes are those read with everything, as `Reaches` decides
            // over them there (`binding_reach`).
            let reach = if matches!(reach, BindingReach::Reaches) {
                let live = binding_reach(
                    text,
                    &binding,
                    targets,
                    &sources,
                    assert_spans,
                    &divergences,
                );
                if !matches!(live, BindingReach::Reaches) {
                    never_run_only = true;
                }
                live
            } else {
                reach
            };
            // DS-1939: a binding whose position is not fixed by syntax over a value
            // whose positions differ is UNKNOWN wherever it is used.
            let reach = if binding.uncertain && !matches!(reach, BindingReach::NotReached) {
                BindingReach::Uncertain
            } else {
                reach
            };
            (binding, reach)
        })
        .collect();
    // Derivation from DS-623 and DS-1962: DA-003 fails only when the result is not
    // verified at all (DS-623), and one result that reaches an assert-equivalent
    // keeps the other calls from making it UNKNOWN (DS-1962). A delegation target
    // (DS-641, DS-1960) is read the same way: a result that reaches an
    // assert-equivalent directly decides DA-003, and only when none does is a path
    // through a delegation target judged by §7.2.1.
    let delegated = macro_use.delegated
        || decided.delegated
        || dead_decided.delegated
        || bindings.iter().any(|(binding, reach)| {
            matches!(reach, BindingReach::Delegated)
                || (binding.delegated
                    && matches!(reach, BindingReach::Reaches | BindingReach::Uncertain))
        });
    if call_sites.is_empty() {
        // Derivation (#282): a call read inside an assert verifies the target only
        // when no call outside a macro exists, so reading macro tokens never
        // clears a call that stays unverified (DS-615: it only moves toward UNKNOWN).
        return if macro_use.verified_path
            || decided.path
            || ((macro_use.verified_method || decided.method) && !macro_use.opaque)
        {
            TargetCallVerification::Verified
        } else if mixed {
            TargetCallVerification::BranchUnknown
        } else if delegated {
            TargetCallVerification::Delegated
        } else if macro_use.opaque {
            // A verified method call is matched by name only: it does not clear a
            // call that a macro may make as a function.
            TargetCallVerification::MacroOpaque
        } else if !non_counted_sites.is_empty() || macro_use.non_counted {
            TargetCallVerification::NotCounted
        } else if never_run_only {
            TargetCallVerification::NeverRun
        } else {
            TargetCallVerification::NotCalled
        };
    }
    // A call whose result is consumed by an assert-equivalent construct
    // (directly, through DS-626's method chain, or through the single `let`
    // binding below) verifies the target wherever it appears; the order of
    // call sites in the function does not change that. Only when no call is
    // verified this way does a call inside a loop make the outcome
    // ambiguous.
    let mut path_verified = decided.path;
    let mut method_verified = decided.method;
    for site in &call_sites {
        let call_start = site.callee_start;
        if every_counted_assert
            .iter()
            .any(|&(start, end)| call_start >= start && call_start < end)
            || target_call_has_assert_method_chain(text, call_start, symbol)
        {
            if in_regions(call_start, never_run.regions) {
                never_run_only = true;
            } else if site.is_method {
                method_verified = true;
            } else {
                path_verified = true;
            }
        }
    }
    // DS-628's disclosed bound: a single `let NAME = ...<call>...;` binding,
    // where `NAME` (optionally followed by `.field`/`.method()`) later
    // appears inside an assert span as an identifier of its own (#281).
    let mut binding_uncertain = false;
    let mut binding_into_target = false;
    for (binding, reach) in &bindings {
        match reach {
            BindingReach::Reaches => {
                if binding.has_path_call {
                    path_verified = true;
                } else if binding.has_method_call {
                    method_verified = true;
                }
            }
            BindingReach::Uncertain => binding_uncertain = true,
            BindingReach::IntoTarget => binding_into_target = true,
            BindingReach::NotReached | BindingReach::Delegated => {}
        }
    }
    if path_verified {
        return TargetCallVerification::Verified;
    }
    if binding_uncertain {
        return TargetCallVerification::BindingUncertain;
    }
    if mixed {
        return TargetCallVerification::BranchUnknown;
    }
    if binding_into_target {
        return TargetCallVerification::IntoTargetArgument;
    }
    if method_verified {
        // #279: a method call is matched by name only, so a verified method call
        // is evidence for the target only when no function call of the target
        // is left unverified beside it. Otherwise the verifying device may
        // belong to another type's same-named method, and the function call's
        // result is neither shown verified nor shown unverified (DS-1680).
        // A name in a macro that is not read may be a function call (#282).
        return if macro_use.opaque || call_sites.iter().any(|site| !site.is_method) {
            TargetCallVerification::Unidentified
        } else {
            TargetCallVerification::Verified
        };
    }
    if delegated {
        return TargetCallVerification::Delegated;
    }
    if macro_use.opaque {
        return TargetCallVerification::MacroOpaque;
    }
    if !non_counted_sites.is_empty() || macro_use.non_counted {
        return TargetCallVerification::NotCounted;
    }
    if never_run_only {
        return TargetCallVerification::NeverRun;
    }
    let loop_bodies = loop_body_spans(text);
    if call_sites.iter().any(|site| {
        loop_bodies
            .iter()
            .any(|&(start, end)| site.callee_start >= start && site.callee_start < end)
    }) {
        return TargetCallVerification::Ambiguous;
    }
    // A mutable-reference or global-state argument to the call is this
    // capability's disclosed ambiguity escape (DS-623's own UNKNOWN
    // example). Scoped to the call's own argument list (the parenthesized
    // span right after `symbol`), not the rest of the function text —
    // scanning past the call site would flag an unrelated `&mut ` written
    // anywhere later in the function as if it belonged to this call.
    if call_sites.iter().any(|site| {
        call_argument_list(text, site.callee_start, symbol)
            .is_some_and(|arguments| arguments.contains("&mut "))
    }) {
        return TargetCallVerification::Ambiguous;
    }
    if call_sites.iter().any(|site| {
        target_call_is_passed_to_other_function(text, site.expr_start, site.callee_start, symbol)
    }) {
        return TargetCallVerification::Ambiguous;
    }
    if call_sites.iter().all(|site| site.is_method) {
        return TargetCallVerification::Unidentified;
    }
    if bindings
        .iter()
        .any(|(binding, _)| reaches_debug_assert(text, &binding.name, targets, &sources))
    {
        return TargetCallVerification::DebugAssertOnly;
    }
    TargetCallVerification::Unverified
}

/// Whether the name of a binding derived from the target's result is used in a
/// checked argument of a `debug_assert*` where it may flow (DS-1954). A use whose
/// value is dropped there, or one in the message arguments (DS-1935), is not.
fn reaches_debug_assert(text: &str, name: &str, targets: &[String], sources: &JoinSources) -> bool {
    debug_assert_macros(text).iter().any(|node| {
        let Some(arguments) = macro_arguments(node) else {
            return tokens_mention_ident(node.tokens.clone(), name);
        };
        let mut probe = CheckedCallVisitor::new(None, Some(name), targets, sources);
        for expression in &arguments.checked {
            probe.visit_expr(expression);
        }
        probe.probe_flows
            || probe.probe_delegated
            || probe.probe_into_target
            || probe.probe_rebound
            || probe.opaque
    })
}

/// A target result consumed by one of DS-626's assertion-equivalent methods is
/// verified even when it is not wrapped in an assert macro. This is deliberately
/// limited to the method chain immediately following the target call.
fn target_call_has_assert_method_chain(text: &str, call_start: usize, symbol: &str) -> bool {
    let Some(close) = call_close_position(text, call_start, symbol) else {
        return false;
    };
    let suffix = text[close + 1..].trim_start();
    [
        ".expect(",
        ".unwrap()",
        ".expect_err(",
        ".unwrap_err()",
        "?",
    ]
    .iter()
    .any(|token| suffix.starts_with(token))
}

fn call_close_position(text: &str, call_start: usize, symbol: &str) -> Option<usize> {
    let open = call_open_position(text, call_start, symbol)?;
    find_matching_close(text, open, '(', ')')
}

/// Finds the argument-list opener for a call whose AST span starts at the
/// beginning of its callee path. `find_call_sites` deliberately records the
/// `ExprCall` function span, so a qualified call such as `Type::from_yaml`
/// starts before the bare target symbol. Trim the callee path before matching
/// its final segment, then use the same opener for all call-site consumers.
fn call_open_position(text: &str, call_start: usize, symbol: &str) -> Option<usize> {
    let after = &text[call_start..];
    let open_offset = after.find('(')?;
    let callee = after[..open_offset].trim_end();
    callee.ends_with(symbol).then_some(call_start + open_offset)
}

/// Byte spans of every `for` / `while` / `loop` body in the function, found
/// from the syntax tree (not by searching the text, which would also match
/// the keywords inside string literals and identifiers such as
/// `default_for`). Iteration and control flow can determine whether every
/// result reaches a verifier, so a call inside one of these bodies is
/// outside this bounded analysis.
fn loop_body_spans(text: &str) -> Vec<(usize, usize)> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = LoopBodyVisitor {
        text,
        spans: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    visitor.spans
}

struct LoopBodyVisitor<'a> {
    text: &'a str,
    spans: Vec<(usize, usize)>,
}

/// The byte spans of the closures, `async` blocks and nested items of the Test body:
/// DS-1962's positions that are not counted.
fn non_counted_regions(text: &str) -> Vec<(usize, usize)> {
    struct Regions<'a> {
        text: &'a str,
        spans: Vec<(usize, usize)>,
    }
    impl Regions<'_> {
        fn record(&mut self, span: proc_macro2::Span) {
            self.spans.push((
                span_offset(span.start(), self.text),
                span_offset(span.end(), self.text),
            ));
        }
    }
    impl<'ast> Visit<'ast> for Regions<'_> {
        fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
            self.record(node.span());
        }
        fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
            self.record(node.span());
        }
        fn visit_item(&mut self, node: &'ast syn::Item) {
            self.record(node.span());
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut regions = Regions {
        text,
        spans: Vec::new(),
    };
    regions.visit_block(&function.block);
    regions.spans
}

pub(crate) fn in_regions(offset: usize, regions: &[(usize, usize)]) -> bool {
    regions
        .iter()
        .any(|&(start, end)| offset >= start && offset < end)
}

impl LoopBodyVisitor<'_> {
    fn record(&mut self, body: &syn::Block) {
        let span = body.brace_token.span.join();
        self.spans.push((
            span_offset(span.start(), self.text),
            span_offset(span.end(), self.text),
        ));
    }
}

impl<'ast> Visit<'ast> for LoopBodyVisitor<'_> {
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.record(&node.body);
        syn::visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.record(&node.body);
        syn::visit::visit_expr_while(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.record(&node.body);
        syn::visit::visit_expr_loop(self, node);
    }
}

/// Passing a target result to an ordinary function is a delegation/data-flow
/// shape this adapter cannot resolve, so DS-1680 requires UNKNOWN.
fn target_call_is_passed_to_other_function(
    text: &str,
    expr_start: usize,
    callee_start: usize,
    symbol: &str,
) -> bool {
    let Some(close) = call_close_position(text, callee_start, symbol) else {
        return false;
    };
    let before = text[..expr_start].trim_end();
    before.ends_with('(')
        || before.ends_with(',')
        || text[close + 1..].trim_start().starts_with(",")
}

/// Returns the argument-list text (inside the parens) of a call to `symbol`
/// whose name starts at byte offset `call_start` in `text`.
fn call_argument_list<'a>(text: &'a str, call_start: usize, symbol: &str) -> Option<&'a str> {
    let open_pos = call_open_position(text, call_start, symbol)?;
    let close_pos = find_matching_close(text, open_pos, '(', ')')?;
    Some(&text[open_pos + 1..close_pos])
}

fn find_call_sites(text: &str, symbol: &str) -> Vec<CallSite> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = CallSiteVisitor {
        text,
        symbol,
        sites: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    visitor.sites
}

struct CallSiteVisitor<'a> {
    text: &'a str,
    symbol: &'a str,
    sites: Vec<CallSite>,
}

impl<'ast> Visit<'ast> for CallSiteVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == self.symbol)
            {
                let start = span_offset(node.func.span().start(), self.text);
                self.sites.push(CallSite {
                    callee_start: start,
                    expr_start: start,
                    is_method: false,
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == self.symbol {
            self.sites.push(CallSite {
                callee_start: span_offset(node.method.span().start(), self.text),
                expr_start: span_offset(node.span().start(), self.text),
                is_method: true,
            });
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// Names bound by `let NAME = ...<call>...;` where the call's value can flow to
/// the initializer's value, each with whether such a call is a function call of
/// `symbol` (not only a method call).
///
/// Derivation (not stated in DS-615 / DS-623 / DS-628 / DS-1680): a binding is
/// derived from the target's result (DS-623) only where the call's value can reach
/// the initializer's value, which is the flow test of `CheckedCallVisitor` applied to
/// the initializer. A call whose value is dropped there (a block statement, an
/// operand that a literal absorbs (DS-1948), a `match` scrutinee, a projection that leaves it out, a closure
/// body) leaves the binding underived, so the call stays unverified, UNKNOWN
/// (DS-615). A closure body is UNKNOWN because DS-629 reads dataflow inside a
/// closure as UNKNOWN. A `match` scrutinee is outside the range DS-628 gives the
/// tracking (a `let` binding, a method chain, a field access), so DS-615 reads it as
/// UNKNOWN. Calls in macro tokens are not read here: `macro_target_use` reads them.
fn find_let_bindings_containing_call(
    text: &str,
    symbol: &str,
    targets: &[String],
    sources: &JoinSources,
    regions: &[(usize, usize)],
) -> Vec<DerivedBinding> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let names = NameUse::new(text, symbol, &function);
    let mut visitor = LetBindingVisitor {
        names: &names,
        targets,
        sources,
        bindings: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    // DS-1962: a `let` that is not at a counted position holds a call that is not
    // counted, which `target_call_result_is_verified` reads as such.
    visitor
        .bindings
        .retain(|binding| !in_regions(binding.offset, regions));
    visitor.bindings
}

/// A `let` binding whose initializer carries the target's result, or hands it to
/// a delegation target (DS-1960).
struct DerivedBinding {
    name: String,
    /// The result comes from a function call of the target, not a method call.
    has_path_call: bool,
    /// The result comes from a method call of the target.
    has_method_call: bool,
    /// The initializer hands the result to a delegation target (DS-641: a
    /// delegation target in a binding that reaches an assert-equivalent).
    delegated: bool,
    /// Declared `let mut`.
    mutable: bool,
    /// The initializer is the target's result as a whole, so every position of the
    /// binding is derived (DS-1939).
    whole_result: bool,
    /// Byte offset of the binding statement.
    offset: usize,
    /// The binding is neither shown derived nor shown underived (DS-1939).
    uncertain: bool,
}

/// Whether `expression` is a call of the target, or a method chain, `?`, `.await` or
/// parentheses over one: a value with no positions of its own, all derived (DS-1939).
fn is_whole_result(expression: &syn::Expr, symbol: &str) -> bool {
    match unwrap_parens(expression) {
        syn::Expr::Call(call) => matches!(
            call.func.as_ref(),
            syn::Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == symbol)
        ),
        syn::Expr::MethodCall(call) => {
            call.method == symbol || is_whole_result(&call.receiver, symbol)
        }
        syn::Expr::Try(node) => is_whole_result(&node.expr, symbol),
        syn::Expr::Await(node) => is_whole_result(&node.base, symbol),
        _ => false,
    }
}

struct LetBindingVisitor<'a> {
    names: &'a NameUse<'a>,
    targets: &'a [String],
    sources: &'a JoinSources,
    bindings: Vec<DerivedBinding>,
}

impl<'ast> Visit<'ast> for LetBindingVisitor<'_> {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let (syn::Pat::Ident(pattern), Some(init)) = (&node.pat, &node.init) {
            let mut calls =
                CheckedCallVisitor::new(Some(self.names), None, self.targets, self.sources);
            calls.init_mode = true;
            calls.visit_expr(init.expr.as_ref());
            // DS-1940: a value joined from arms that differ is neither shown derived
            // nor shown underived, wherever the binding is used.
            if calls.path || calls.method || calls.delegated || calls.mixed {
                self.bindings.push(DerivedBinding {
                    name: pattern.ident.to_string(),
                    has_path_call: calls.path,
                    has_method_call: calls.method,
                    delegated: calls.delegated,
                    mutable: pattern.mutability.is_some(),
                    whole_result: is_whole_result(&init.expr, self.names.symbol),
                    offset: span_offset(node.span().start(), self.names.text),
                    uncertain: calls.mixed,
                });
            }
        }
        syn::visit::visit_local(self, node);
    }
}

/// The derivation of the positions of a value (DS-1939), read from the syntax of a
/// pattern's scrutinee.
#[derive(Clone, Debug)]
enum Shape {
    /// No position is derived.
    Plain,
    /// Every position is derived: a target call's result, from a function call of the
    /// target (`path`) or a method call (`method`).
    Derived { path: bool, method: bool },
    /// A tuple or an array (`vec![..]` included) literal: one shape per element.
    Elements(Vec<Shape>),
    /// A struct literal: the last segment of its path, the shape of each named field,
    /// and of the fields its base expression gives.
    Struct(Option<String>, Vec<(String, Shape)>, Box<Shape>),
    /// A constructor that syntax fixes (`Some` / `Ok` / `Err`, DS-1960): its variant is
    /// not derived (DS-1939).
    Variant(String, Vec<Shape>),
    /// The positions are not fixed by syntax and are not all alike.
    Unknown,
}

impl Shape {
    /// One derivation for the value as a whole (DS-1939): derived when every
    /// position is, underived when none is, and `Unknown` otherwise.
    fn summary(&self) -> Shape {
        let parts: Vec<Shape> = match self {
            Shape::Elements(elements) => elements.iter().map(Shape::summary).collect(),
            Shape::Struct(_, fields, rest) => fields
                .iter()
                .map(|(_, shape)| shape.summary())
                .chain([rest.summary()])
                .collect(),
            Shape::Variant(_, elements) => elements
                .iter()
                .map(Shape::summary)
                .chain([Shape::Plain])
                .collect(),
            other => return other.clone(),
        };
        join_shapes(parts)
    }

    /// The shape of the element `index` of `count` elements, or of the field `name`.
    fn element(&self, index: usize) -> Shape {
        match self {
            Shape::Elements(elements) | Shape::Variant(_, elements) => {
                elements.get(index).cloned().unwrap_or(Shape::Unknown)
            }
            Shape::Plain | Shape::Derived { .. } => self.clone(),
            Shape::Struct(..) | Shape::Unknown => Shape::Unknown,
        }
    }

    fn field(&self, name: &str) -> Shape {
        match self {
            Shape::Struct(_, fields, rest) => fields
                .iter()
                .find(|(field, _)| field == name)
                .map_or_else(|| (**rest).clone(), |(_, shape)| shape.clone()),
            Shape::Elements(_) => name
                .parse::<usize>()
                .map_or(Shape::Unknown, |index| self.element(index)),
            Shape::Plain | Shape::Derived { .. } => self.clone(),
            Shape::Variant(..) | Shape::Unknown => Shape::Unknown,
        }
    }
}

/// Whether an identifier pattern may be a constant or a unit variant rather than a
/// binding: Rust reads a name in scope as an item, and `None`, constants and unit
/// variants are written like a variant (`is_variant_like`).
pub(crate) fn pattern_ident_may_be_path(ident: &syn::PatIdent) -> bool {
    ident.subpat.is_none()
        && ident.by_ref.is_none()
        && ident.mutability.is_none()
        && is_variant_like(&ident.ident)
}

/// One derivation for several summaries: `Plain` when none is derived (or there is
/// none), `Derived` when every one is, and `Unknown` otherwise.
fn join_shapes(parts: impl IntoIterator<Item = Shape>) -> Shape {
    let mut joined: Option<Shape> = None;
    for part in parts {
        joined = Some(match (joined, part) {
            (None, part) => part,
            (Some(Shape::Plain), Shape::Plain) => Shape::Plain,
            (Some(Shape::Derived { path, method }), Shape::Derived { path: p, method: m }) => {
                Shape::Derived {
                    path: path || p,
                    method: method || m,
                }
            }
            _ => Shape::Unknown,
        });
    }
    joined.unwrap_or(Shape::Plain)
}

/// How the derivation of a scrutinee is read: the target's symbol and the `let`
/// bindings whose name is followed to its initializer.
struct ShapeReader<'a> {
    symbol: &'a str,
    /// A name bound once in the function, by a plain `let` that is not `mut`, with its
    /// initializer (DS-1939: the variable bound to a value has its derivation).
    lets: Vec<(String, &'a syn::Expr)>,
    /// Every name bound by a `let` whose initializer mentions the target.
    mentioning: Vec<String>,
}

/// The deepest chain of `let` names `ShapeReader` follows.
const SHAPE_DEPTH: usize = 8;

impl<'a> ShapeReader<'a> {
    fn new(function: &'a syn::ItemFn, symbol: &'a str) -> Self {
        struct Lets<'a> {
            all: Vec<(String, &'a syn::Expr, bool)>,
        }
        impl<'a> Visit<'a> for Lets<'a> {
            fn visit_local(&mut self, node: &'a syn::Local) {
                let mut pattern = &node.pat;
                while let syn::Pat::Type(typed) = pattern {
                    pattern = &typed.pat;
                }
                if let (syn::Pat::Ident(ident), Some(init)) = (pattern, &node.init) {
                    let plain = ident.by_ref.is_none()
                        && ident.mutability.is_none()
                        && ident.subpat.is_none()
                        && init.diverge.is_none();
                    self.all
                        .push((ident.ident.to_string(), init.expr.as_ref(), plain));
                }
                syn::visit::visit_local(self, node);
            }
        }
        let mut lets = Lets { all: Vec::new() };
        lets.visit_block(&function.block);
        let mut reader = Self {
            symbol,
            lets: Vec::new(),
            mentioning: Vec::new(),
        };
        for (name, init, plain) in &lets.all {
            if *plain && count_rebindings_in(function, name) == 1 {
                reader.lets.push((name.clone(), init));
            }
        }
        // A name mentions the target when its initializer does, through other names
        // too; the chain is followed as deep as `SHAPE_DEPTH`.
        for _ in 0..SHAPE_DEPTH {
            for (name, init, _) in &lets.all {
                if !reader.mentioning.contains(name) && reader.mentions(init) {
                    reader.mentioning.push(name.clone());
                }
            }
        }
        reader
    }

    /// Whether `expression` holds a call of the target or a name whose initializer
    /// does, in its syntax tree or in macro tokens.
    fn mentions(&self, expression: &syn::Expr) -> bool {
        fn tokens_mention(reader: &ShapeReader<'_>, tokens: proc_macro2::TokenStream) -> bool {
            tokens_mention_ident(tokens.clone(), reader.symbol)
                || reader
                    .mentioning
                    .iter()
                    .any(|name| tokens_mention_ident(tokens.clone(), name))
        }
        struct Finder<'r, 'a> {
            reader: &'r ShapeReader<'a>,
            found: bool,
        }
        impl<'ast> Visit<'ast> for Finder<'_, '_> {
            fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
                self.found |= node.path.segments.last().is_some_and(|segment| {
                    segment.ident == self.reader.symbol
                        || self
                            .reader
                            .mentioning
                            .iter()
                            .any(|name| segment.ident == name)
                });
            }
            fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
                self.found |= node.method == self.reader.symbol;
                syn::visit::visit_expr_method_call(self, node);
            }
            fn visit_macro(&mut self, node: &'ast syn::Macro) {
                self.found |= tokens_mention(self.reader, node.tokens.clone());
            }
        }
        let mut finder = Finder {
            reader: self,
            found: false,
        };
        finder.visit_expr(expression);
        finder.found
    }

    /// The shape of `expression`.
    ///
    /// Derivation from DS-1939 and DS-1960: a target call's result, through a method
    /// chain, `?`, `.await`, a reference or parentheses, is derived at every position;
    /// a tuple, array, struct literal and `Some` / `Ok` / `Err` keep a shape per
    /// position; `Box::new` / `String::from` carry their argument's derivation as a
    /// whole; a name bound once by a plain `let` has its initializer's shape; a field
    /// access and an integer-literal index take one position. Anything else that
    /// mentions the target (a function call, which DS-1960 reads as a delegation
    /// target, a method with a derived argument, an operator) is `Unknown`, and an
    /// expression that does not mention it is `Plain`.
    fn shape(&self, expression: &syn::Expr, depth: usize) -> Shape {
        if depth > SHAPE_DEPTH {
            return Shape::Unknown;
        }
        let expression = strip_place(expression);
        if is_whole_result(expression, self.symbol) {
            return Shape::Derived {
                path: whole_result_is_path(expression, self.symbol),
                method: !whole_result_is_path(expression, self.symbol),
            };
        }
        let whole = |shape: Shape| match shape.summary() {
            summary @ (Shape::Plain | Shape::Derived { .. }) => summary,
            _ => Shape::Unknown,
        };
        match expression {
            syn::Expr::Tuple(tuple) => Shape::Elements(
                tuple
                    .elems
                    .iter()
                    .map(|element| self.shape(element, depth + 1))
                    .collect(),
            ),
            syn::Expr::Array(array) => Shape::Elements(
                array
                    .elems
                    .iter()
                    .map(|element| self.shape(element, depth + 1))
                    .collect(),
            ),
            syn::Expr::Macro(node) if node.mac.path.is_ident("vec") => {
                match macro_arguments(&node.mac) {
                    Some(arguments) => Shape::Elements(
                        arguments
                            .checked
                            .iter()
                            .map(|element| self.shape(element, depth + 1))
                            .collect(),
                    ),
                    None if self.mentions(expression) => Shape::Unknown,
                    None => Shape::Plain,
                }
            }
            syn::Expr::Struct(literal) => {
                let fields = literal
                    .fields
                    .iter()
                    .map(|field| {
                        let name = match &field.member {
                            syn::Member::Named(name) => field_name(name),
                            syn::Member::Unnamed(index) => index.index.to_string(),
                        };
                        (name, self.shape(&field.expr, depth + 1))
                    })
                    .collect();
                let rest = literal
                    .rest
                    .as_ref()
                    .map_or(Shape::Plain, |rest| whole(self.shape(rest, depth + 1)));
                let path = literal
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string());
                Shape::Struct(path, fields, Box::new(rest))
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(callee) = call.func.as_ref() else {
                    return self.unknown_if_mentioned(expression);
                };
                let last = callee
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string());
                match last.as_deref() {
                    Some(variant @ ("Some" | "Ok" | "Err")) => Shape::Variant(
                        variant.to_owned(),
                        call.args
                            .iter()
                            .map(|argument| self.shape(argument, depth + 1))
                            .collect(),
                    ),
                    _ if is_value_conversion(&callee.path) && call.args.len() == 1 => {
                        whole(self.shape(&call.args[0], depth + 1))
                    }
                    _ => self.unknown_if_mentioned(expression),
                }
            }
            syn::Expr::Path(path) if path.qself.is_none() => {
                let Some(name) = path.path.get_ident() else {
                    return self.unknown_if_mentioned(expression);
                };
                match self.lets.iter().find(|(bound, _)| name == bound) {
                    Some((_, init)) => self.shape(init, depth + 1),
                    None => self.unknown_if_mentioned(expression),
                }
            }
            syn::Expr::Field(field) => {
                let name = match &field.member {
                    syn::Member::Named(name) => field_name(name),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                };
                self.shape(&field.base, depth + 1).field(&name)
            }
            syn::Expr::Index(index) => match index.index.as_ref() {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(literal),
                    ..
                }) if !self.mentions(&index.index) => match literal.base10_parse::<usize>() {
                    Ok(position) => match self.shape(&index.expr, depth + 1) {
                        elements @ Shape::Elements(_) => elements.element(position),
                        whole @ (Shape::Plain | Shape::Derived { .. }) => whole,
                        _ => Shape::Unknown,
                    },
                    Err(_) => Shape::Unknown,
                },
                _ => self.unknown_if_mentioned(expression),
            },
            syn::Expr::Try(node) => whole(self.shape(&node.expr, depth + 1)),
            syn::Expr::Await(node) => whole(self.shape(&node.base, depth + 1)),
            // A block that is not labeled has the value of its tail expression.
            syn::Expr::Block(node) if node.label.is_none() => match node.block.stmts.last() {
                Some(syn::Stmt::Expr(tail, None)) => self.shape(tail, depth + 1),
                _ => self.unknown_if_mentioned(expression),
            },
            syn::Expr::MethodCall(call) if !call.args.iter().any(|arg| self.mentions(arg)) => {
                // DS-1937 / DS-1939: a method over a receiver whose positions are all
                // alike carries its derivation; any other receiver is UNKNOWN.
                whole(self.shape(&call.receiver, depth + 1))
            }
            _ => self.unknown_if_mentioned(expression),
        }
    }

    fn unknown_if_mentioned(&self, expression: &syn::Expr) -> Shape {
        if self.mentions(expression) {
            Shape::Unknown
        } else {
            Shape::Plain
        }
    }
}

/// Whether a value that `is_whole_result` accepts comes from a function call of the
/// target rather than from a method call.
fn whole_result_is_path(expression: &syn::Expr, symbol: &str) -> bool {
    match unwrap_parens(expression) {
        syn::Expr::Call(_) => true,
        syn::Expr::MethodCall(call) => {
            is_whole_result(&call.receiver, symbol) && whole_result_is_path(&call.receiver, symbol)
        }
        syn::Expr::Try(node) => whole_result_is_path(&node.expr, symbol),
        syn::Expr::Await(node) => whole_result_is_path(&node.base, symbol),
        _ => false,
    }
}

/// One name a pattern binds, with the derivation of the position it takes.
struct PatternBinding {
    name: String,
    shape: Shape,
    /// `mut` or `ref mut`: the binding may be changed afterwards (DS-1959).
    mutable: bool,
}

/// The names `pattern` binds from a value of shape `shape` (DS-1938, DS-1939): each
/// takes the position it extracts. A position that syntax does not fix (a variant or
/// struct pattern over a value whose positions are not all alike, an arity that does
/// not match) is `Unknown`. A name bound by every case of an or-pattern is derived
/// only when every case derives it, underived when none does, and `Unknown`
/// otherwise (as DS-1940 joins the arms of a branch).
fn pattern_bindings(pattern: &syn::Pat, shape: &Shape, out: &mut Vec<PatternBinding>) {
    let elements = |patterns: &syn::punctuated::Punctuated<syn::Pat, syn::Token![,]>,
                    shape: &Shape,
                    out: &mut Vec<PatternBinding>| {
        let count = patterns.len();
        let rest = patterns
            .iter()
            .position(|element| matches!(element, syn::Pat::Rest(_)));
        let known = match shape {
            Shape::Elements(values) | Shape::Variant(_, values) => Some(values.len()),
            _ => None,
        };
        for (index, element) in patterns.iter().enumerate() {
            let position = match (rest, known) {
                (Some(rest), Some(len)) if index > rest => (len + index).checked_sub(count),
                (None, Some(len)) if len != count => None,
                _ => Some(index),
            };
            let element_shape = position.map_or(Shape::Unknown, |position| shape.element(position));
            pattern_bindings(element, &element_shape, out);
        }
    };
    match pattern {
        // A name that may be a constant or a unit variant (`None`, `K`) compares the
        // value and binds nothing.
        syn::Pat::Ident(ident) if pattern_ident_may_be_path(ident) => {}
        syn::Pat::Ident(ident) => {
            out.push(PatternBinding {
                name: ident.ident.to_string(),
                shape: shape.summary(),
                // `mut x` and `ref mut x` both carry the `mut` token.
                mutable: ident.mutability.is_some(),
            });
            if let Some((_, sub)) = &ident.subpat {
                pattern_bindings(sub, shape, out);
            }
        }
        syn::Pat::Type(typed) => pattern_bindings(&typed.pat, shape, out),
        syn::Pat::Paren(inner) => pattern_bindings(&inner.pat, shape, out),
        syn::Pat::Reference(inner) => pattern_bindings(&inner.pat, shape, out),
        syn::Pat::Tuple(tuple) => {
            let shape = match shape {
                Shape::Elements(_) | Shape::Plain | Shape::Derived { .. } => shape.clone(),
                _ => Shape::Unknown,
            };
            elements(&tuple.elems, &shape, out);
        }
        syn::Pat::Slice(slice) => {
            let shape = match shape {
                Shape::Elements(_) | Shape::Plain | Shape::Derived { .. } => shape.clone(),
                _ => Shape::Unknown,
            };
            elements(&slice.elems, &shape, out);
        }
        syn::Pat::TupleStruct(node) => {
            let name = node
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            let shape = match shape {
                Shape::Variant(variant, _) if name.as_deref() == Some(variant.as_str()) => {
                    shape.clone()
                }
                // Another variant never matches the value, so its names take nothing.
                Shape::Variant(..) => Shape::Plain,
                Shape::Plain | Shape::Derived { .. } => shape.clone(),
                _ => Shape::Unknown,
            };
            elements(&node.elems, &shape, out);
        }
        syn::Pat::Struct(node) => {
            let name = node
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            // A struct pattern of another variant never matches the literal, so its
            // names take nothing.
            let other_variant = matches!(
                shape,
                Shape::Struct(Some(literal), ..) if name.as_deref() != Some(literal.as_str())
            );
            for field in &node.fields {
                if other_variant {
                    pattern_bindings(&field.pat, &Shape::Plain, out);
                    continue;
                }
                let name = match &field.member {
                    syn::Member::Named(name) => field_name(name),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                };
                let field_shape = match shape {
                    Shape::Struct(..) | Shape::Plain | Shape::Derived { .. } => shape.field(&name),
                    _ => Shape::Unknown,
                };
                pattern_bindings(&field.pat, &field_shape, out);
            }
        }
        syn::Pat::Or(node) => {
            let cases: Vec<Vec<PatternBinding>> = node
                .cases
                .iter()
                .map(|case| {
                    let mut bound = Vec::new();
                    pattern_bindings(case, shape, &mut bound);
                    bound
                })
                .collect();
            let Some((first, others)) = cases.split_first() else {
                return;
            };
            for binding in first {
                let mut joined = binding.shape.clone();
                let mut mutable = binding.mutable;
                for case in others {
                    let other = case.iter().find(|other| other.name == binding.name);
                    joined = match (joined, other.map(|other| &other.shape)) {
                        (Shape::Plain, Some(Shape::Plain)) => Shape::Plain,
                        (
                            Shape::Derived { path, method },
                            Some(Shape::Derived { path: p, method: m }),
                        ) => Shape::Derived {
                            path: path || *p,
                            method: method || *m,
                        },
                        _ => Shape::Unknown,
                    };
                    mutable |= other.is_some_and(|other| other.mutable);
                }
                out.push(PatternBinding {
                    name: binding.name.clone(),
                    shape: joined,
                    mutable,
                });
            }
        }
        // A literal, a range, a path, `_` and `..` bind nothing; a macro's pattern is not
        // read, so the names it may bind are not derived.
        _ => {}
    }
}

/// The names bound by the patterns of `match` arms, `if let` / `while let` and of
/// `let` statements other than a single name (`let ... else` included), whose
/// position in the scrutinee is derived from the target's result or not fixed by
/// syntax (DS-1938, DS-1939). Only those at DS-1962's counted positions are read.
fn find_pattern_bindings(
    text: &str,
    symbol: &str,
    regions: &[(usize, usize)],
) -> Vec<DerivedBinding> {
    struct Finder<'r, 'a> {
        reader: &'r ShapeReader<'a>,
        text: &'a str,
        regions: &'a [(usize, usize)],
        bindings: Vec<DerivedBinding>,
    }
    impl Finder<'_, '_> {
        fn bind(&mut self, pattern: &syn::Pat, scrutinee: &syn::Expr, at: proc_macro2::Span) {
            let offset = span_offset(at.start(), self.text);
            if in_regions(offset, self.regions) || !self.reader.mentions(scrutinee) {
                return;
            }
            let shape = self.reader.shape(scrutinee, 0);
            let mut bound = Vec::new();
            pattern_bindings(pattern, &shape, &mut bound);
            for binding in bound {
                let (path, method, uncertain) = match binding.shape {
                    Shape::Derived { path, method } => (path, method, false),
                    Shape::Plain => continue,
                    _ => (false, false, true),
                };
                self.bindings.push(DerivedBinding {
                    name: binding.name,
                    has_path_call: path,
                    has_method_call: method,
                    delegated: false,
                    mutable: binding.mutable,
                    whole_result: !uncertain,
                    offset,
                    uncertain,
                });
            }
        }
    }
    impl<'ast> Visit<'ast> for Finder<'_, '_> {
        fn visit_local(&mut self, node: &'ast syn::Local) {
            let mut pattern = &node.pat;
            while let syn::Pat::Type(typed) = pattern {
                pattern = &typed.pat;
            }
            // A single name is `LetBindingVisitor`'s.
            let single = matches!(pattern, syn::Pat::Ident(ident) if ident.subpat.is_none());
            if let (false, Some(init)) = (single, &node.init) {
                self.bind(&node.pat, &init.expr, node.span());
            }
            syn::visit::visit_local(self, node);
        }
        fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
            self.bind(&node.pat, &node.expr, node.span());
            syn::visit::visit_expr_let(self, node);
        }
        fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
            for arm in &node.arms {
                self.bind(&arm.pat, &node.expr, arm.span());
            }
            syn::visit::visit_expr_match(self, node);
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let reader = ShapeReader::new(&function, symbol);
    let mut finder = Finder {
        reader: &reader,
        text,
        regions,
        bindings: Vec::new(),
    };
    finder.visit_block(&function.block);
    finder.bindings
}

// ---------------------------------------------------------------------------
// DA-004 (DS-624): self comparison
// ---------------------------------------------------------------------------

fn da_004_self_comparison(text: &str) -> DaVerdict {
    let spans = find_macro_call_spans(text, "assert_eq!");
    for (start, end) in spans {
        let arguments = &text[start..end];
        if let Some((left, right)) = split_top_level_comma(arguments) {
            if normalize_tokens(left) == normalize_tokens(right)
                && !normalize_tokens(left).is_empty()
            {
                return DaVerdict::Fail("assert_eq!'s two arguments are token-identical (DS-624)");
            }
        }
    }
    DaVerdict::NoViolation
}

fn normalize_tokens(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_top_level_comma(arguments: &str) -> Option<(&str, &str)> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    for (index, ch) in arguments.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '(' | '[' | '{' if !in_string => depth += 1,
            ')' | ']' | '}' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                return Some((&arguments[..index], &arguments[index + 1..]));
            }
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// DA-005 (DS-625): empty test
// ---------------------------------------------------------------------------

fn da_005_empty_test(text: &str) -> DaVerdict {
    if syn::parse_str::<syn::ItemFn>(text)
        .ok()
        .is_some_and(|function| function.block.stmts.is_empty())
    {
        DaVerdict::Fail("the Test function body has no statements (DS-625)")
    } else {
        DaVerdict::NoViolation
    }
}

struct RustFunctionSyntax {
    function: syn::ItemFn,
}

impl RustFunctionSyntax {
    fn parse(text: &str) -> Option<Self> {
        syn::parse_str(text).ok().map(|function| Self { function })
    }

    fn has_should_panic(&self) -> bool {
        self.function
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("should_panic"))
    }

    /// Where DS-619's methods and `?` appear (DS-1964 aside). Counted only outside
    /// a closure, an `async` block and a nested item, the positions DS-1962 leaves
    /// out because they may not run (derivation from the intent of DS-1962 for
    /// DA-001).
    fn method_or_try(&self) -> MethodOrTry {
        let mut constructed = ConstructedBindings {
            variants: Vec::new(),
        };
        constructed.visit_item_fn(&self.function);
        constructed
            .variants
            .retain(|(name, _)| count_rebindings_in(&self.function, name) == 1);
        let mut visitor = VerificationSyntaxVisitor {
            constructed: &constructed.variants,
            nested: false,
            has_verifier: false,
            counted: false,
        };
        visitor.visit_item_fn(&self.function);
        if visitor.counted {
            MethodOrTry::Counted
        } else if visitor.has_verifier {
            MethodOrTry::Uncounted
        } else {
            MethodOrTry::None
        }
    }

    /// `true` when the function calls a function or a method anywhere in
    /// its body. In a body without an assert-equivalent construct, such a
    /// call is a delegation target of the Test (DS-642).
    fn has_call(&self) -> bool {
        let mut visitor = CallVisitor { has_call: false };
        visitor.visit_block(&self.function.block);
        visitor.has_call
    }

    fn returns_result(&self) -> bool {
        match &self.function.sig.output {
            syn::ReturnType::Type(_, ty) => match ty.as_ref() {
                syn::Type::Path(path) => path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "Result"),
                _ => false,
            },
            syn::ReturnType::Default => false,
        }
    }
}

struct CallVisitor {
    has_call: bool,
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, _node: &'ast syn::ExprCall) {
        self.has_call = true;
    }

    fn visit_expr_method_call(&mut self, _node: &'ast syn::ExprMethodCall) {
        self.has_call = true;
    }
}

/// DS-619's assert-equivalent methods and `?`, except where DS-1964 says they
/// cannot fail: a receiver whose variant is `Some` / `Ok` (for `.unwrap()`,
/// `.expect(..)`, `?`) or `Err` (for `.unwrap_err()`, `.expect_err(..)`) by syntax.
struct VerificationSyntaxVisitor<'a> {
    /// `let` names bound once to a constructor, with the constructor's name.
    constructed: &'a [(String, String)],
    /// Inside a closure, an `async` block or a nested item.
    nested: bool,
    has_verifier: bool,
    /// One is found outside a closure, an `async` block and a nested item.
    counted: bool,
}

impl VerificationSyntaxVisitor<'_> {
    /// Whether the variant of `receiver` is one of `variants` by syntax (DS-1964,
    /// DS-1939): a constructor call, or a name bound once by a `let` to one. A
    /// receiver whose variant is not told this way stays an assert-equivalent
    /// (DS-619), as DS-1964 covers only a variant fixed by syntax.
    fn is_constructed(&self, receiver: &syn::Expr, variants: &[&str]) -> bool {
        match unwrap_parens(receiver) {
            syn::Expr::Call(call) => {
                constructor_name(call).is_some_and(|name| variants.contains(&name.as_str()))
            }
            syn::Expr::Path(path) if path.qself.is_none() => {
                path.path.get_ident().is_some_and(|ident| {
                    self.constructed.iter().any(|(name, variant)| {
                        ident == name && variants.contains(&variant.as_str())
                    })
                })
            }
            _ => false,
        }
    }

    fn found(&mut self, counts: bool) {
        self.has_verifier |= counts;
        self.counted |= counts && !self.nested;
    }

    fn nesting(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.nested;
        self.nested = true;
        visit(self);
        self.nested = outer;
    }
}

impl<'ast> Visit<'ast> for VerificationSyntaxVisitor<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let counts = match node.method.to_string().as_str() {
            "unwrap" | "expect" => !self.is_constructed(&node.receiver, &["Some", "Ok"]),
            "unwrap_err" | "expect_err" => !self.is_constructed(&node.receiver, &["Err"]),
            _ => false,
        };
        self.found(counts);
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.found(!self.is_constructed(&node.expr, &["Some", "Ok"]));
        syn::visit::visit_expr_try(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.nesting(|visitor| syn::visit::visit_expr_closure(visitor, node));
    }

    fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
        self.nesting(|visitor| syn::visit::visit_expr_async(visitor, node));
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        self.nesting(|visitor| syn::visit::visit_item(visitor, node));
    }
}

/// The constructor name (`Some`, `Ok`, `Err`) a call's callee ends with, by name
/// (DS-1961, DS-1967).
fn constructor_name(call: &syn::ExprCall) -> Option<String> {
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    let name = path.path.segments.last()?.ident.to_string();
    matches!(name.as_str(), "Some" | "Ok" | "Err").then_some(name)
}

/// `let NAME = Some(..) / Ok(..) / Err(..);` bindings (not `mut`, not `ref`), with
/// the constructor's name.
struct ConstructedBindings {
    variants: Vec<(String, String)>,
}

impl<'ast> Visit<'ast> for ConstructedBindings {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        let mut pattern = &node.pat;
        while let syn::Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        if let (syn::Pat::Ident(binding), Some(init)) = (pattern, &node.init) {
            let plain = binding.by_ref.is_none()
                && binding.mutability.is_none()
                && binding.subpat.is_none()
                && init.diverge.is_none();
            if let (true, syn::Expr::Call(call)) = (plain, unwrap_parens(&init.expr)) {
                if let Some(variant) = constructor_name(call) {
                    self.variants.push((binding.ident.to_string(), variant));
                }
            }
        }
        syn::visit::visit_local(self, node);
    }
}

// ---------------------------------------------------------------------------
// DA-006 (DS-626): no verification syntax
// ---------------------------------------------------------------------------

fn da_006_no_verification_syntax(
    has_verification_syntax: bool,
    has_debug_assert: bool,
    has_call: bool,
) -> DaVerdict {
    if has_verification_syntax {
        DaVerdict::NoViolation
    } else if has_debug_assert {
        // DS-1954 (before DS-626 / DS-642 / DS-647): a `debug_assert*` checks in a
        // debug build only, so it is neither an assert-equivalent nor its absence.
        DaVerdict::Unknown(
            "the only checking construct is a `debug_assert*`, which is not an \
             assert-equivalent but checks in a debug build (DS-1954)"
                .to_owned(),
        )
    } else if has_call {
        // DS-642: with no assert-equivalent construct, the calls in the body
        // are the Test's delegation targets. This capability never confirms
        // that one terminates (module doc), so the result is neither a
        // violation nor its absence (DS-626, DS-652, DS-1345).
        DaVerdict::Unknown(
            "no assert-equivalent construct is present and the body delegates to a function \
             call whose termination (§7.2.1) this analysis does not confirm (DS-626, DS-652)"
                .to_owned(),
        )
    } else {
        DaVerdict::Fail(
            "no assert-equivalent construct (standard macro, #[should_panic], \
             .unwrap()/.expect()/.expect_err()/.unwrap_err()/?, or a Result-returning signature) is present, \
             and no §7.2.1 delegation target is identified (DS-626)",
        )
    }
}

// ---------------------------------------------------------------------------
// Shared lexical helpers
// ---------------------------------------------------------------------------

/// The `debug_assert*` invocations the function's syntax tree holds (not those
/// inside the tokens of another macro).
fn debug_assert_macros(text: &str) -> Vec<syn::Macro> {
    struct Finder(Vec<syn::Macro>);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if node
                .path
                .segments
                .last()
                .is_some_and(|segment| is_debug_assert(&segment.ident.to_string()))
            {
                self.0.push(node.clone());
            }
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut finder = Finder(Vec::new());
    finder.visit_item_fn(&function);
    finder.0
}

/// Finds every occurrence of any macro in `macros` and returns the byte
/// range of its argument-list contents (inside the outermost matching
/// parens/brackets/braces the macro invocation uses).
fn find_assert_spans(text: &str, macros: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for macro_name in macros {
        spans.extend(find_macro_call_spans(text, macro_name));
    }
    spans.sort_by_key(|&(start, _)| start);
    spans
}

fn find_macro_call_spans(text: &str, macro_name: &str) -> Vec<(usize, usize)> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = MacroVisitor {
        text,
        macro_name,
        spans: Vec::new(),
    };
    syn::visit::Visit::visit_item_fn(&mut visitor, &function);
    visitor.spans
}

struct MacroVisitor<'a> {
    text: &'a str,
    macro_name: &'a str,
    spans: Vec<(usize, usize)>,
}

impl<'ast> syn::visit::Visit<'ast> for MacroVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|segment| format!("{}!", segment.ident) == self.macro_name)
        {
            if let Some(span) = macro_argument_span(self.text, node) {
                self.spans.push(span);
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

/// The byte range of a macro invocation's argument-list contents (inside its
/// outermost delimiters).
fn macro_argument_span(text: &str, node: &syn::Macro) -> Option<(usize, usize)> {
    let start = span_offset(node.span().start(), text);
    let end = span_offset(node.span().end(), text);
    let invocation = &text[start..end];
    let open = invocation.find(['(', '[', '{'])?;
    let open_char = invocation.as_bytes()[open] as char;
    let close_char = match open_char {
        '(' => ')',
        '[' => ']',
        _ => '}',
    };
    let close = find_matching_close(invocation, open, open_char, close_char)?;
    Some((start + open + 1, start + close))
}

fn macro_argument_start(text: &str, node: &syn::Macro) -> Option<usize> {
    macro_argument_span(text, node).map(|(start, _)| start)
}

// ---------------------------------------------------------------------------
// DA-003 (DS-1941, DS-1942, DS-1944): calls of a closure bound by `let`
// ---------------------------------------------------------------------------

/// The macros whose checked arguments are DS-1962's counted positions.
const COUNTED_MACROS: [&str; 5] = ["assert", "assert_eq", "assert_ne", "vec", "matches"];

/// The most rounds of inlining: a closure called from the body of another tracked
/// closure is inlined one round after it.
const INLINE_ROUNDS: usize = 16;

/// `text` with every call that DS-1941 tracks replaced by the body of the closure
/// it runs, for DA-003 to read the call as the execution of that body.
///
/// A closure is tracked when it is bound by `let f = |..| body;` (a plain name, not
/// `mut`, a closure that is not `async`, `const`, `static` or `for<..>`) and called
/// as `f(..)` at a counted position, `f` meaning the nearest `let` of that name in
/// scope (DS-1941). A call `f(a, ..)` becomes `{ let p = a; ..; body }`: each
/// parameter pattern is bound to its argument once, before the body, as a call
/// evaluates them, and every name the closure binds is renamed per call so that no
/// name of the call site is captured. A literal or a name the closure does not bind
/// replaces a plain parameter directly. The `let f` is removed once every use of `f`
/// is an inlined call.
///
/// A closure is left as it is, and DS-1944 holds for it, when `f` is used other than
/// by a call (passed, rebound, referenced), bound by another pattern or assigned, or
/// never called at a counted position. A closure is also left as it is where the
/// inlined text could not mean the same execution, which DS-1680 reads as `UNKNOWN`:
/// its body uses a name of the call site that the function binds more than once, or
/// binds a name other than a parameter that is also bound outside it, may capture a
/// `mut` binding (DS-1959), or uses `?`; or a call whose value is used runs a body that has its own
/// `return` or whose tail is a branch, loop or labeled block (DS-1942 makes the value
/// derived only when every exit's value is; a `return` of a call whose value is
/// discarded becomes a `break` of the inlined block).
///
/// `None` when no call is inlined, and when the rounds do not settle.
pub(crate) fn inline_tracked_closures(text: &str) -> Option<String> {
    let mut current = text.to_owned();
    let mut fresh = 0;
    let mut inlined = false;
    for _ in 0..INLINE_ROUNDS {
        match inline_round(&current, &mut fresh) {
            Some(next) => {
                current = next;
                inlined = true;
            }
            None => return inlined.then_some(current),
        }
    }
    None
}

/// Per name, the places that bind it: patterns (the cases of an or-pattern once),
/// assignments and compound assignments, also in the checked arguments of a macro
/// whose grammar is known.
#[derive(Default)]
struct BindingCounter {
    counts: BTreeMap<String, usize>,
    /// The names bound by a `mut` or `ref mut` pattern.
    mutable: BTreeSet<String>,
    /// Where each name is bound.
    positions: BTreeMap<String, Vec<proc_macro2::LineColumn>>,
    /// The names of the items (`fn`, `const`, `static`, ..) declared in the body.
    items: BTreeSet<String>,
}

impl BindingCounter {
    fn get(&self, name: &str) -> usize {
        self.counts.get(name).copied().unwrap_or(0)
    }
}

impl<'ast> Visit<'ast> for BindingCounter {
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        if pattern_ident_may_be_path(node) {
            return;
        }
        *self.counts.entry(node.ident.to_string()).or_default() += 1;
        self.positions
            .entry(node.ident.to_string())
            .or_default()
            .push(node.ident.span().start());
        if node.mutability.is_some() {
            self.mutable.insert(node.ident.to_string());
        }
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        let name = match node {
            syn::Item::Fn(item) => Some(&item.sig.ident),
            syn::Item::Const(item) => Some(&item.ident),
            syn::Item::Static(item) => Some(&item.ident),
            syn::Item::Struct(item) => Some(&item.ident),
            syn::Item::Enum(item) => Some(&item.ident),
            syn::Item::Macro(item) => item.ident.as_ref(),
            _ => None,
        };
        if let Some(name) = name {
            self.items.insert(name.to_string());
        }
        syn::visit::visit_item(self, node);
    }

    fn visit_pat_or(&mut self, node: &'ast syn::PatOr) {
        let mut most: BTreeMap<String, usize> = BTreeMap::new();
        for case in &node.cases {
            let mut one = BindingCounter::default();
            one.visit_pat(case);
            self.mutable.extend(one.mutable);
            for (name, at) in one.positions {
                self.positions.entry(name).or_default().extend(at);
            }
            for (name, count) in one.counts {
                let entry = most.entry(name).or_default();
                *entry = (*entry).max(count);
            }
        }
        for (name, count) in most {
            *self.counts.entry(name).or_default() += count;
        }
    }

    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        if let syn::Expr::Path(path) = node.left.as_ref() {
            if let Some(name) = path.path.get_ident() {
                *self.counts.entry(name.to_string()).or_default() += 1;
            }
        }
        syn::visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if is_compound_assignment(node.op) {
            if let syn::Expr::Path(path) = node.left.as_ref() {
                if let Some(name) = path.path.get_ident() {
                    *self.counts.entry(name.to_string()).or_default() += 1;
                }
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(arguments) = macro_arguments(node) {
            for argument in &arguments.checked {
                self.visit_expr(argument);
            }
        }
    }
}

/// An identifier in macro tokens that may name a variable: not a method or field
/// after `.`, a lifetime, a macro name, a path segment or a field label.
struct TokenIdent {
    name: String,
    range: (usize, usize),
    /// Followed by a parenthesized group: written as a call.
    called: bool,
}

fn token_idents(
    tokens: proc_macro2::TokenStream,
    offset: &dyn Fn(proc_macro2::LineColumn) -> usize,
    out: &mut Vec<TokenIdent>,
) {
    use proc_macro2::{Delimiter, Spacing, TokenTree};
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let punct = |index: Option<usize>, ch: char| {
        index
            .and_then(|index| trees.get(index))
            .is_some_and(|tree| matches!(tree, TokenTree::Punct(p) if p.as_char() == ch))
    };
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Group(group) => token_idents(group.stream(), offset, out),
            TokenTree::Ident(ident) => {
                let previous = index.checked_sub(1);
                let after_dot_or_quote = punct(previous, '.') || punct(previous, '\'');
                let after_path = punct(previous, ':') && punct(index.checked_sub(2), ':');
                let before_bang_or_colon = matches!(
                    trees.get(index + 1),
                    Some(TokenTree::Punct(p))
                        if (p.as_char() == '!' && p.spacing() == Spacing::Alone) || p.as_char() == ':'
                );
                if after_dot_or_quote || after_path || before_bang_or_colon {
                    continue;
                }
                out.push(TokenIdent {
                    name: ident.to_string(),
                    range: (offset(ident.span().start()), offset(ident.span().end())),
                    called: matches!(
                        trees.get(index + 1),
                        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis
                    ),
                });
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
}

fn tokens_have_question_mark(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Punct(punct) => punct.as_char() == '?',
        proc_macro2::TokenTree::Group(group) => tokens_have_question_mark(group.stream()),
        proc_macro2::TokenTree::Ident(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// What inlining a call needs to know about a closure that may be tracked.
struct ClosureInfo {
    /// Per parameter: its pattern's range, and its name when it is a plain name.
    params: Vec<((usize, usize), Option<String>)>,
    body: (usize, usize),
    /// Every name the closure binds.
    bound: BTreeSet<String>,
    /// The names the closure binds in one place only.
    once: BTreeSet<String>,
    /// The ranges of those names, in the parameters and the body.
    renames: Vec<((usize, usize), String)>,
    /// The field shorthands among them, written out as `label: name` when renamed.
    shorthands: Vec<((usize, usize), String)>,
    /// The ranges of the `return` keywords of the closure's own returns.
    returns: Vec<(usize, usize)>,
    /// The tail of the body is a branch, a loop or a labeled block.
    tail_join: bool,
}

/// The `ClosureInfo` of `closure`, or `None` when inlining its calls could not mean
/// the same execution (see `inline_tracked_closures`).
fn closure_info(
    closure: &syn::ExprClosure,
    offset: &dyn Fn(proc_macro2::LineColumn) -> usize,
    function_counts: &BindingCounter,
) -> Option<ClosureInfo> {
    struct Sites<'o> {
        offset: &'o dyn Fn(proc_macro2::LineColumn) -> usize,
        bindings: Vec<((usize, usize), String)>,
        uses: Vec<((usize, usize), String)>,
        /// Field shorthands (`S { a }`), whose name is a field label as well.
        shorthands: Vec<((usize, usize), String)>,
        returns: Vec<(usize, usize)>,
        question: bool,
        /// Inside a closure, an `async` block or an item, whose `return` and `?` are
        /// not the closure's own.
        nested: usize,
    }
    impl Sites<'_> {
        fn range(&self, span: proc_macro2::Span) -> (usize, usize) {
            ((self.offset)(span.start()), (self.offset)(span.end()))
        }
    }
    impl<'ast> Visit<'ast> for Sites<'_> {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            if !pattern_ident_may_be_path(node) {
                self.bindings
                    .push((self.range(node.ident.span()), node.ident.to_string()));
            }
            syn::visit::visit_pat_ident(self, node);
        }
        fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
            if let (None, Some(name)) = (&node.qself, node.path.get_ident()) {
                self.uses.push((self.range(name.span()), name.to_string()));
            }
            syn::visit::visit_expr_path(self, node);
        }
        fn visit_field_value(&mut self, node: &'ast syn::FieldValue) {
            if let (None, syn::Member::Named(name)) = (&node.colon_token, &node.member) {
                self.shorthands
                    .push((self.range(name.span()), name.to_string()));
            }
            syn::visit::visit_field_value(self, node);
        }
        fn visit_field_pat(&mut self, node: &'ast syn::FieldPat) {
            if let (None, syn::Member::Named(name)) = (&node.colon_token, &node.member) {
                self.shorthands
                    .push((self.range(name.span()), name.to_string()));
            }
            syn::visit::visit_field_pat(self, node);
        }
        fn visit_expr_return(&mut self, node: &'ast syn::ExprReturn) {
            if self.nested == 0 {
                self.returns.push(self.range(node.return_token.span));
            }
            syn::visit::visit_expr_return(self, node);
        }
        fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
            self.question |= self.nested == 0;
            syn::visit::visit_expr_try(self, node);
        }
        fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
            self.nested += 1;
            syn::visit::visit_expr_closure(self, node);
            self.nested -= 1;
        }
        fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
            self.nested += 1;
            syn::visit::visit_expr_async(self, node);
            self.nested -= 1;
        }
        fn visit_item(&mut self, node: &'ast syn::Item) {
            self.nested += 1;
            syn::visit::visit_item(self, node);
            self.nested -= 1;
        }
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            let mut idents = Vec::new();
            let unparsed = match macro_arguments(node) {
                Some(arguments) => {
                    for argument in &arguments.checked {
                        self.visit_expr(argument);
                    }
                    arguments.unchecked
                }
                None => vec![node.tokens.clone()],
            };
            for tokens in unparsed {
                // A `?` or `return` in tokens that are not parsed is not seen as an exit:
                // such a closure is not inlined.
                if self.nested == 0
                    && (tokens_mention_ident(tokens.clone(), "return")
                        || tokens_have_question_mark(tokens.clone()))
                {
                    self.question = true;
                }
                token_idents(tokens, self.offset, &mut idents);
            }
            self.uses
                .extend(idents.into_iter().map(|ident| (ident.range, ident.name)));
        }
    }
    if closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.movability.is_some()
        || closure.asyncness.is_some()
    {
        return None;
    }
    let mut sites = Sites {
        offset,
        bindings: Vec::new(),
        uses: Vec::new(),
        shorthands: Vec::new(),
        returns: Vec::new(),
        question: false,
        nested: 0,
    };
    for input in &closure.inputs {
        sites.visit_pat(input);
    }
    // A parameter's scope is the whole body, so every use of its name in the closure
    // means a binding of the closure.
    let params_bound: BTreeSet<String> = sites
        .bindings
        .iter()
        .map(|(_, name)| name.clone())
        .collect();
    sites.visit_expr(&closure.body);
    if sites.question {
        return None;
    }
    let mut inside = BindingCounter::default();
    inside.visit_expr_closure(closure);
    let bound: BTreeSet<String> = sites
        .bindings
        .iter()
        .map(|(_, name)| name.clone())
        .collect();
    if bound.iter().any(|name| {
        !params_bound.contains(name) && function_counts.get(name) > inside.get(name)
    })
        // DS-1959: a closure that may capture a `mut` binding mutably changes it where
        // it runs; that change is not inlined.
        // DS-1941: a name the body captures must mean, at every call, the binding it
        // means at the closure: one binding made before the closure, or an item that
        // no `let` shadows.
        || sites.uses.iter().any(|(_, name)| {
            let count = function_counts.get(name);
            !bound.contains(name)
                && (count > 1
                    || function_counts.mutable.contains(name)
                    || (count == 1 && function_counts.items.contains(name))
                    || function_counts.positions.get(name).is_some_and(|at| {
                        at.iter().any(|position| *position >= closure.span().start())
                    }))
        })
    {
        return None;
    }
    let params = closure
        .inputs
        .iter()
        .map(|input| {
            let mut pattern = input;
            while let syn::Pat::Type(typed) = pattern {
                pattern = &typed.pat;
            }
            let plain = match pattern {
                syn::Pat::Ident(ident)
                    if ident.by_ref.is_none()
                        && ident.mutability.is_none()
                        && ident.subpat.is_none() =>
                {
                    Some(ident.ident.to_string())
                }
                _ => None,
            };
            // The type of a parameter is not written on its `let`: DA-003 reads the
            // binding of a plain name the same with and without one.
            let typeless = match input {
                syn::Pat::Type(typed) => typed.pat.as_ref(),
                other => other,
            };
            (sites.range(typeless.span()), plain)
        })
        .collect();
    let renames = sites
        .bindings
        .iter()
        .chain(sites.uses.iter().filter(|(_, name)| bound.contains(name)))
        .cloned()
        .collect();
    let once = bound
        .iter()
        .filter(|name| inside.get(name) == 1)
        .cloned()
        .collect();
    let shorthands = sites
        .shorthands
        .iter()
        .filter(|(_, name)| bound.contains(name))
        .cloned()
        .collect();
    Some(ClosureInfo {
        params,
        body: sites.range(closure.body.span()),
        bound,
        once,
        renames,
        shorthands,
        returns: sites.returns,
        tail_join: tail_is_join(&closure.body),
    })
}

/// Whether the value of `expression` comes from a branch, a loop or a labeled block,
/// whose exits DS-1942 requires to be read one by one.
fn tail_is_join(expression: &syn::Expr) -> bool {
    let block_tail = |block: &syn::Block| match block.stmts.last() {
        Some(syn::Stmt::Expr(tail, None)) => tail_is_join(tail),
        _ => false,
    };
    match expression {
        syn::Expr::If(_)
        | syn::Expr::Match(_)
        | syn::Expr::Loop(_)
        | syn::Expr::While(_)
        | syn::Expr::ForLoop(_) => true,
        syn::Expr::Block(node) => node.label.is_some() || block_tail(&node.block),
        syn::Expr::Unsafe(node) => block_tail(&node.block),
        syn::Expr::Paren(node) => tail_is_join(&node.expr),
        syn::Expr::Group(node) => tail_is_join(&node.expr),
        syn::Expr::Reference(node) => tail_is_join(&node.expr),
        syn::Expr::Unary(node) => tail_is_join(&node.expr),
        syn::Expr::Cast(node) => tail_is_join(&node.expr),
        syn::Expr::Field(node) => tail_is_join(&node.base),
        syn::Expr::Try(node) => tail_is_join(&node.expr),
        syn::Expr::MethodCall(node) => tail_is_join(&node.receiver),
        syn::Expr::Binary(node) => tail_is_join(&node.left) || tail_is_join(&node.right),
        _ => false,
    }
}

/// A `let NAME = ..;` of the function, with the range in which `NAME` means it.
struct LetDef {
    name: String,
    range: (usize, usize),
    scope_end: usize,
    counted: bool,
    closure: Option<ClosureInfo>,
}

/// A call `NAME(..)` of the function.
struct NameCall {
    name: String,
    range: (usize, usize),
    /// Per argument: its range, and whether it is a literal or a plain name.
    args: Vec<((usize, usize), bool)>,
    counted: bool,
    discarded: bool,
}

/// One round: the outermost calls that DS-1941 tracks are inlined. `None` when there
/// is none, or the result does not parse.
fn inline_round(text: &str, fresh: &mut usize) -> Option<String> {
    struct Scan<'o> {
        offset: &'o dyn Fn(proc_macro2::LineColumn) -> usize,
        counts: &'o BindingCounter,
        defs: Vec<LetDef>,
        calls: Vec<NameCall>,
        uses: Vec<(usize, String)>,
        discarded: BTreeSet<usize>,
        noncounted: usize,
    }
    impl Scan<'_> {
        fn range(&self, span: proc_macro2::Span) -> (usize, usize) {
            ((self.offset)(span.start()), (self.offset)(span.end()))
        }
        fn tokens(&mut self, tokens: proc_macro2::TokenStream, calls_allowed: bool) {
            let mut idents = Vec::new();
            token_idents(tokens, self.offset, &mut idents);
            for ident in idents {
                if calls_allowed && ident.called {
                    self.calls.push(NameCall {
                        name: ident.name,
                        range: ident.range,
                        args: Vec::new(),
                        counted: false,
                        discarded: false,
                    });
                } else {
                    self.uses.push((ident.range.0, ident.name));
                }
            }
        }
    }
    impl<'ast> Visit<'ast> for Scan<'_> {
        fn visit_block(&mut self, node: &'ast syn::Block) {
            let scope_end = self.range(node.span()).1;
            for statement in &node.stmts {
                match statement {
                    syn::Stmt::Local(local) => {
                        if let syn::Pat::Ident(ident) = &local.pat {
                            let closure = match &local.init {
                                Some(init)
                                    if init.diverge.is_none()
                                        && ident.by_ref.is_none()
                                        && ident.mutability.is_none()
                                        && ident.subpat.is_none() =>
                                {
                                    match init.expr.as_ref() {
                                        syn::Expr::Closure(closure) => {
                                            closure_info(closure, self.offset, self.counts)
                                        }
                                        _ => None,
                                    }
                                }
                                _ => None,
                            };
                            self.defs.push(LetDef {
                                name: ident.ident.to_string(),
                                range: self.range(local.span()),
                                scope_end,
                                counted: self.noncounted == 0,
                                closure,
                            });
                        }
                    }
                    syn::Stmt::Expr(syn::Expr::Call(call), Some(_)) => {
                        self.discarded.insert(self.range(call.span()).0);
                    }
                    _ => {}
                }
            }
            syn::visit::visit_block(self, node);
        }
        fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
            let syn::Expr::Path(path) = node.func.as_ref() else {
                return syn::visit::visit_expr_call(self, node);
            };
            let (None, Some(name)) = (&path.qself, path.path.get_ident()) else {
                return syn::visit::visit_expr_call(self, node);
            };
            let range = self.range(node.span());
            let args = node
                .args
                .iter()
                .map(|argument| {
                    let simple = match argument {
                        syn::Expr::Lit(_) => true,
                        syn::Expr::Path(path) => {
                            path.qself.is_none() && path.path.get_ident().is_some()
                        }
                        _ => false,
                    };
                    (self.range(argument.span()), simple)
                })
                .collect();
            self.calls.push(NameCall {
                name: name.to_string(),
                range,
                args,
                counted: self.noncounted == 0,
                discarded: self.discarded.contains(&range.0),
            });
            for argument in &node.args {
                self.visit_expr(argument);
            }
        }
        fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
            if let (None, Some(name)) = (&node.qself, node.path.get_ident()) {
                self.uses
                    .push((self.range(name.span()).0, name.to_string()));
            }
        }
        fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
            self.noncounted += 1;
            syn::visit::visit_expr_closure(self, node);
            self.noncounted -= 1;
        }
        fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
            self.noncounted += 1;
            syn::visit::visit_expr_async(self, node);
            self.noncounted -= 1;
        }
        fn visit_item(&mut self, node: &'ast syn::Item) {
            self.noncounted += 1;
            syn::visit::visit_item(self, node);
            self.noncounted -= 1;
        }
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            let name = node
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            let Some(arguments) = macro_arguments(node) else {
                return self.tokens(node.tokens.clone(), false);
            };
            let counted = name
                .as_deref()
                .is_some_and(|name| COUNTED_MACROS.contains(&name));
            if !counted {
                self.noncounted += 1;
            }
            for argument in &arguments.checked {
                self.visit_expr(argument);
            }
            if !counted {
                self.noncounted -= 1;
            }
            for tokens in arguments.unchecked {
                self.tokens(tokens, true);
            }
        }
    }

    let function = syn::parse_str::<syn::ItemFn>(text).ok()?;
    let lines = crate::line_offsets(text);
    let offset = |at: proc_macro2::LineColumn| {
        crate::line_column_byte_offset(text, &lines, at.line, at.column).unwrap_or(text.len())
    };
    let mut counts = BindingCounter::default();
    counts.visit_block(&function.block);
    for input in &function.sig.inputs {
        counts.visit_fn_arg(input);
    }
    let mut scan = Scan {
        offset: &offset,
        counts: &counts,
        defs: Vec::new(),
        calls: Vec::new(),
        uses: Vec::new(),
        discarded: BTreeSet::new(),
        noncounted: 0,
    };
    scan.visit_block(&function.block);
    let Scan {
        mut defs,
        calls,
        uses,
        ..
    } = scan;
    // DS-1941: a name means the nearest `let` of it whose scope holds the position.
    let resolve = |defs: &[LetDef], name: &str, at: usize| {
        defs.iter()
            .enumerate()
            .filter(|(_, def)| def.name == name && def.range.1 <= at && at < def.scope_end)
            .max_by_key(|(_, def)| def.range.1)
            .map(|(index, _)| index)
    };
    // DS-1944: a closure that is used other than by a call, or whose name another
    // pattern or an assignment binds, is not tracked.
    for (at, name) in &uses {
        if let Some(index) = resolve(&defs, name, *at) {
            defs[index].closure = None;
        }
    }
    for index in 0..defs.len() {
        let name = defs[index].name.clone();
        // An item of the same name may be the one a call means in its block.
        if counts.get(&name) > defs.iter().filter(|def| def.name == name).count()
            || counts.items.contains(&name)
        {
            defs[index].closure = None;
        }
    }
    let mut per_def: BTreeMap<usize, (Vec<&NameCall>, bool)> = BTreeMap::new();
    for call in &calls {
        if let Some(index) = resolve(&defs, &call.name, call.range.0) {
            let entry = per_def.entry(index).or_default();
            if call.counted && defs[index].counted {
                entry.0.push(call);
            } else {
                entry.1 = true;
            }
        }
    }
    // Every edit, as (range, replacement, the def it belongs to).
    let mut edits: Vec<((usize, usize), String, usize)> = Vec::new();
    for (index, (counted_calls, other_calls)) in &per_def {
        let Some(info) = &defs[*index].closure else {
            continue;
        };
        if counted_calls.is_empty()
            || counted_calls.iter().any(|call| {
                call.args.len() != info.params.len()
                    || (!call.discarded && (!info.returns.is_empty() || info.tail_join))
            })
        {
            continue;
        }
        for call in counted_calls {
            let replacement = inlined_call(text, info, call, fresh);
            edits.push((call.range, replacement, *index));
        }
        if !other_calls {
            edits.push((defs[*index].range, String::new(), *index));
        }
    }
    // The outermost edits are made in this round; a def whose edits are not all made
    // is left to a later round whole, so that its `let` is never removed early.
    edits.sort_by(|a, b| a.0 .0.cmp(&b.0 .0).then(b.0 .1.cmp(&a.0 .1)));
    let mut kept = vec![true; edits.len()];
    loop {
        let mut end = 0;
        let mut chosen = vec![false; edits.len()];
        for (position, edit) in edits.iter().enumerate() {
            if kept[position] && edit.0 .0 >= end {
                chosen[position] = true;
                end = edit.0 .1;
            }
        }
        let mut dropped = false;
        for position in 0..edits.len() {
            if kept[position] && !chosen[position] {
                let def = edits[position].2;
                for (other, edit) in edits.iter().enumerate() {
                    if edit.2 == def && kept[other] {
                        kept[other] = false;
                        dropped = true;
                    }
                }
            }
        }
        if !dropped {
            break;
        }
    }
    let mut result = text.to_owned();
    let mut any = false;
    for (position, edit) in edits.iter().enumerate().rev() {
        if kept[position] {
            result.replace_range(edit.0 .0..edit.0 .1, &edit.1);
            any = true;
        }
    }
    (any && syn::parse_str::<syn::ItemFn>(&result).is_ok()).then_some(result)
}

/// The block that runs the body of `info` for `call`.
fn inlined_call(text: &str, info: &ClosureInfo, call: &NameCall, fresh: &mut usize) -> String {
    while text.contains(&format!("_vtc{fresh}")) || text.contains(&format!("'vtc{fresh}")) {
        *fresh += 1;
    }
    let id = *fresh;
    *fresh += 1;
    let mut names: BTreeMap<&str, String> = info
        .bound
        .iter()
        .map(|name| (name.as_str(), format!("{name}_vtc{id}")))
        .collect();
    let mut bound_params = Vec::new();
    for ((pattern, plain), (argument, simple)) in info.params.iter().zip(&call.args) {
        let argument_text = &text[argument.0..argument.1];
        // A literal or a name replaces a parameter that the closure binds nowhere else:
        // every other name the closure binds is renamed, so the name keeps meaning the
        // call site's binding.
        match (plain, simple) {
            (Some(name), true) if info.once.contains(name) => {
                names.insert(name.as_str(), argument_text.to_owned());
            }
            _ => bound_params.push((*pattern, argument_text)),
        }
    }
    let label = format!("'vtc{id}");
    let rewrite = |range: (usize, usize)| {
        // A shorthand comes first, so that it wins over the rename of the same name.
        let mut pieces: Vec<((usize, usize), String)> = info
            .shorthands
            .iter()
            .filter(|(site, _)| site.0 >= range.0 && site.1 <= range.1)
            .map(|(site, name)| (*site, format!("{name}: {}", names[name.as_str()])))
            .chain(
                info.renames
                    .iter()
                    .filter(|(site, _)| site.0 >= range.0 && site.1 <= range.1)
                    .map(|(site, name)| (*site, names[name.as_str()].clone())),
            )
            .chain(
                info.returns
                    .iter()
                    .filter(|site| site.0 >= range.0 && site.1 <= range.1)
                    .map(|site| (*site, format!("break {label}"))),
            )
            .collect();
        pieces.sort_by_key(|(site, _)| std::cmp::Reverse(site.0));
        pieces.dedup_by_key(|(site, _)| site.0);
        let mut written = text[range.0..range.1].to_owned();
        for (site, replacement) in pieces {
            written.replace_range(site.0 - range.0..site.1 - range.0, &replacement);
        }
        written
    };
    let mut block = String::new();
    if !info.returns.is_empty() {
        block.push_str(&label);
        block.push_str(": ");
    }
    block.push_str("{ ");
    for (pattern, argument) in bound_params {
        block.push_str(&format!("let {} = {argument}; ", rewrite(pattern)));
    }
    block.push_str(&rewrite(info.body));
    block.push_str(" }");
    block
}

/// Byte offset of a `proc_macro2` position in `text`. The column counts
/// characters, so it goes through the crate's character-aware conversion:
/// adding it to the line start as bytes lands inside a multi-byte character
/// and the slice panics (#264). A position outside `text` clamps to its end.
pub(crate) fn span_offset(location: proc_macro2::LineColumn, text: &str) -> usize {
    crate::line_column_byte_offset(
        text,
        &crate::line_offsets(text),
        location.line,
        location.column,
    )
    .unwrap_or(text.len())
}

fn find_matching_close(text: &str, open_pos: usize, open: char, close: char) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    for (index, ch) in text[open_pos..].char_indices() {
        let absolute = open_pos + index;
        match ch {
            '"' => in_string = !in_string,
            c if !in_string && c == open => depth += 1,
            c if !in_string && c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(absolute);
                }
            }
            _ => {}
        }
    }
    None
}

/// Diagnostic code this capability's caller uses when it cannot run this
/// analysis at all (no `fn` signature/body recognizable in the construct
/// text) — DS-615's "adapterが不完全…を報告した場合" fail-closed escape.
pub fn unanalyzable_diagnostic(reason: &str) -> Diagnostic {
    Diagnostic::warning(
        "W-DA-102",
        format!("oracle_presence static analysis could not run: {reason}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locator(value: &str) -> Locator {
        Locator {
            adapter: vtest_model::AdapterId::new("rust-cargo"),
            value: value.to_owned(),
        }
    }

    // A test compares a verdict's variant with `assert_eq!` instead of
    // `assert!(matches!(verdict, DaVerdict::Fail(_)))`: whether a path pattern
    // is irrefutable is shown only by its type, so DA-001 reads that `matches!`
    // as not decided constant (DS-621, DS-1965) and the test's own
    // oracle_presence would be UNKNOWN (#339). The variant leaves the reason
    // out, so the test checks what the `matches!` checked.
    impl DaVerdict {
        fn outcome(&self) -> RuleOutcome {
            match self {
                Self::NoViolation => RuleOutcome::NoViolation,
                Self::Fail(_) => RuleOutcome::Fail,
                Self::Unknown(_) => RuleOutcome::Unknown,
            }
        }
    }

    /// The variant of another crate's verdict type, read the same way.
    trait Variant {
        fn variant(&self) -> &'static str;
    }

    impl Variant for StaticAnalysisVerdict {
        fn variant(&self) -> &'static str {
            match self {
                Self::Pass(_) => "PASS",
                Self::Fail(_) => "FAIL",
                Self::Unknown(_) => "UNKNOWN",
            }
        }
    }

    /// @vtest.id TEST-ORACLE-TARGET-SYMBOL-FINAL-SEGMENT
    /// @vtest.covers VO-ORACLE-TARGET-SYMBOL-FINAL-SEGMENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::target_symbol
    /// @vtest.intent target候補名としてitem-pathの末尾segmentを抽出することを確認する
    #[test]
    fn target_symbol_takes_the_final_segment() {
        assert_eq!(
            target_symbol(&locator("src/lib.rs::double")),
            "double".to_owned()
        );
    }

    /// @vtest.id TEST-ORACLE-COMPOSE-ALL-NO-VIOLATION-PASSES
    /// @vtest.covers VO-ORACLE-COMPOSE-PASS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::compose
    /// @vtest.intent DA-001/003/004/005/006の全ルールが違反なしのときcomposeがFAILでもUNKNOWNでもないことを確認する
    #[test]
    fn a_direct_assert_eq_call_passes_all_five_rules() {
        let text = "#[test]\nfn it_doubles() {\n    assert_eq!(double(2), 4);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        let composed = compose(&analysis);
        assert!(!composed.0, "{:?}", composed.2);
        assert!(!composed.1, "{:?}", composed.2);
    }

    /// @vtest.id TEST-ORACLE-DA-005-EMPTY-BODY-FAILS
    /// @vtest.covers VO-ORACLE-DA-005-EMPTY-TEST
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_005_empty_test
    /// @vtest.intent 文を含まない関数本体がDA-005のFAILになることを確認する
    #[test]
    fn an_empty_test_fails_da_005() {
        let text = "#[test]\nfn empty() {\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_005, DaVerdict::Fail(_)));
        let (is_fail, _, _) = compose(&analysis);
        assert!(is_fail);
    }

    /// @vtest.id TEST-ORACLE-DA-006-NO-ASSERT-FAILS
    /// @vtest.covers VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent 検証構文が1つも無い関数本体がDA-006のFAILになることを確認する
    #[test]
    fn no_verification_syntax_fails_da_006() {
        let text = "#[test]\nfn calls_but_checks_nothing() {\n    let _ = 1 + 1;\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_006.outcome(), RuleOutcome::Fail);
    }

    /// @vtest.id TEST-ORACLE-DA-006-DELEGATING-BODY-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-006-DELEGATION-NOT-FAIL
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent a Test body with no assert-equivalent construct that calls a function or a method gets DA-006 Unknown, neither Fail nor NoViolation, and the composed verdict is Unknown, while a body with a call only inside a string stays Fail (DS-1345)
    #[test]
    fn a_body_that_only_delegates_gets_da_006_unknown_not_fail() {
        for text in [
            "#[test]\nfn delegates() {\n    check_all(1, 2);\n}\n",
            "#[test]\nfn delegates() {\n    let run = |x: u8| helper.compare(x);\n    let _ = run;\n}\n",
        ] {
            let analysis = analyze(text, &[], &[]);
            assert!(matches!(analysis.da_006, DaVerdict::Unknown(_)), "{text}");
            let (is_fail, is_unknown, _) = compose(&analysis);
            assert!(!is_fail && is_unknown, "{text}");
        }

        let no_call = "#[test]\nfn mentions() {\n    let _ = \"check_all(1, 2)\";\n}\n";
        assert!(matches!(
            analyze(no_call, &[], &[]).da_006,
            DaVerdict::Fail(_)
        ));
    }

    /// @vtest.id TEST-ORACLE-TEXT-LITERAL-IS-NOT-VERIFICATION-SYNTAX
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED, VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::analyze
    /// @vtest.intent 文字列literal内のtarget呼出しとunwrap表記を実行call・検証構文として数えないことを確認する
    #[test]
    fn text_literal_target_and_unwrap_are_not_verification_syntax() {
        let text = "#[test]\nfn mentions_only() {\n    let _ = \"parse(input).unwrap()\";\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert_eq!(analysis.da_003, DaVerdict::NoViolation);
        assert_eq!(analysis.da_006.outcome(), RuleOutcome::Fail);
    }

    /// @vtest.id TEST-ORACLE-DA-006-SHOULD-PANIC-PASSES
    /// @vtest.covers VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent #[should_panic]属性が検証構文として認められDA-006が違反なしになることを確認する
    #[test]
    fn a_should_panic_test_with_no_target_call_passes_da_006_via_the_attribute() {
        let text = "#[should_panic]\n#[test]\nfn panics() {\n    panic!(\"boom\");\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_006, DaVerdict::NoViolation);
    }

    /// REQ-074「不成立が構造から証明できる場合のみFAIL」: no
    /// assert-equivalent construct exists anywhere in the function, so no
    /// path — tracked or not — could carry the call's result to one. This
    /// is the one case this capability can prove structurally.
    /// @vtest.id TEST-ORACLE-DA-003-NO-ASSERT-ANYWHERE-FAILS
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果を検証するassert相当が関数内に一つも無い場合にDA-003がFAILになることを確認する
    #[test]
    fn a_target_call_with_no_assert_construct_anywhere_fails_da_003() {
        let text = "#[test]\nfn calls_but_asserts_nothing() {\n    double(2);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Fail(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// DS-615「解析限界…を報告した場合はUNKNOWNとし、違反なしと推測しない」:
    /// an assert-equivalent construct exists in the function (so a real
    /// verification path is structurally possible), but this capability's
    /// bounded dataflow tracking (direct-in-assert or one `let` binding)
    /// cannot trace the call's result into it. That is an analysis limit,
    /// not proof of non-verification — `Unknown`, not `Fail`.
    /// @vtest.id TEST-ORACLE-DA-003-UNTRACEABLE-CALL-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assertが存在するがtarget呼出結果をそこへ追跡できない場合、DA-003がFAILではなくUNKNOWNになることを確認する
    #[test]
    fn a_target_call_untraceable_into_an_unrelated_assert_is_unknown_not_fail() {
        let text = "#[test]\nfn calls_but_ignores_result() {\n    double(2);\n    assert!(true == false || true);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// @vtest.id TEST-ORACLE-DA-003-SHOULD-PANIC-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent #[should_panic]があるときassertが無くてもDA-003が違反なしになることを確認する
    #[test]
    fn a_should_panic_target_call_passes_da_003_without_an_assert() {
        let text = "#[should_panic]\n#[test]\nfn panics_on_call() {\n    divide(1, 0);\n}\n";
        let analysis = analyze(text, &["divide".to_owned()], &[]);
        assert_eq!(analysis.da_003, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DA-003-LET-BOUND-CALL-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果がlet束縛経由でassertへ到達する場合にDA-003が違反なしになることを確認する
    #[test]
    fn a_let_bound_call_used_in_an_assert_passes_da_003() {
        let text = "#[test]\nfn binds_then_asserts() {\n    let result = double(2);\n    assert_eq!(result, 4);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
    }

    /// @vtest.id TEST-ORACLE-DA-003-DIRECT-EXPECT-ERR-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD,VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果へ直接expectを連鎖した場合に到達と判定することを確認する
    #[test]
    fn a_direct_expect_err_call_passes_da_003_and_da_006() {
        let text = "#[test]\nfn rejects() {\n    parse(input).expect_err(\"invalid\");\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert_eq!(analysis.da_003, DaVerdict::NoViolation);
        assert_eq!(analysis.da_006, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DA-003-PATH-QUALIFIED-EXPECT-ERR-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD,VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent パス付きtarget呼出しへexpect_errを連鎖した場合にDA-003が違反なしになることを確認する
    #[test]
    fn a_path_qualified_expect_err_call_passes_da_003_and_da_006() {
        let text =
            "#[test]\nfn rejects() {\n    Type::from_yaml(input).expect_err(\"invalid\");\n}\n";
        let analysis = analyze(text, &["from_yaml".to_owned()], &[]);
        assert_eq!(analysis.da_003, DaVerdict::NoViolation);
        assert_eq!(analysis.da_006, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DA-003-DIRECT-EXPECT-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD,VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果へ直接expectを連鎖した場合に到達と判定することを確認する
    #[test]
    fn a_direct_expect_call_passes_da_003_and_da_006() {
        let text = "#[test]\nfn accepts() {\n    parse(input).expect(\"valid\");\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert_eq!(analysis.da_003, DaVerdict::NoViolation);
        assert_eq!(analysis.da_006, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DA-003-LOOP-CALL-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent loop内のtarget呼出はbounded解析でUNKNOWNに退避することを確認する
    #[test]
    fn a_target_call_inside_a_loop_is_unknown() {
        let text = "#[test]\nfn repeated() {\n    for item in inputs {\n        parse(item);\n    }\n    assert!(true);\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert_eq!(analysis.da_003.outcome(), RuleOutcome::Unknown);
    }

    /// @vtest.id TEST-ORACLE-DA-003-LOOP-CALL-WITH-ASSERT-CHAIN
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent a loop call whose result goes through an assert-equivalent method or a let binding that is asserted, or a later directly verified call, makes DA-003 no violation regardless of call order
    #[test]
    fn a_verified_call_is_no_violation_even_when_another_call_is_in_a_loop() {
        for text in [
            "#[test]\nfn chained() {\n    for item in inputs {\n        parse(item).expect(\"valid\");\n    }\n}\n",
            "#[test]\nfn bound() {\n    for item in inputs {\n        let parsed = parse(item);\n        assert!(parsed.is_ok());\n    }\n}\n",
            "#[test]\nfn later() {\n    for item in inputs {\n        let _ = parse(item);\n    }\n    parse(\"x\").unwrap();\n}\n",
        ] {
            let analysis = analyze(text, &["parse".to_owned()], &[]);
            assert_eq!(analysis.da_003, DaVerdict::NoViolation, "{text}");
        }
    }

    /// @vtest.id TEST-ORACLE-DA-003-LOOP-KEYWORD-IN-STRING
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::loop_body_spans
    /// @vtest.intent only real for / while / loop bodies count as loops; the keyword inside a string literal or an identifier such as default_for does not
    #[test]
    fn a_loop_keyword_in_a_string_or_identifier_is_not_a_loop() {
        let text = "#[test]\nfn plain() {\n    let default_for = 1;\n    assert!(default_for == 1, \"for {}\", 1);\n    parse(input);\n}\n";
        assert!(loop_body_spans(text).is_empty());
        let looped =
            "#[test]\nfn looped() {\n    for item in inputs {\n        parse(item);\n    }\n}\n";
        assert_eq!(loop_body_spans(looped).len(), 1);
    }

    /// @vtest.id TEST-ORACLE-DA-003-DELEGATED-CALL-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target結果を別関数へ渡す形は委譲先を追跡せずUNKNOWNにすることを確認する
    #[test]
    fn a_target_call_passed_to_another_function_is_unknown() {
        let text = "#[test]\nfn delegated() {\n    check(parse(input));\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert_eq!(analysis.da_003.outcome(), RuleOutcome::Unknown);
    }

    /// DS-642: a body with no assert-equivalent construct delegates to every
    /// non-target call in it; the delegation target's termination is not
    /// confirmed (DS-640, DS-652), so DA-003 is UNKNOWN, not FAIL.
    /// @vtest.id TEST-ORACLE-DA-003-RESULT-FLOWING-TO-OTHER-CALL-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATION-NOT-FAIL
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert macroが無く、target以外の関数へ結果が流れる本体(#277の実物の形で.expect付きを含む)はDA-003がFAILではなくUNKNOWNになり、basisが委譲(DS-642、DS-652)を理由に挙げることを確認する
    #[test]
    fn a_result_flowing_into_a_non_target_call_without_assert_is_unknown() {
        for text in [
            "#[test]\nfn real() {\n    let data = build(&scan);\n    let mut json = to_json(&data).expect(\"serialize data\");\n    json.push('\\n');\n    golden(\"data.json\", &json);\n}\n",
            "#[test]\nfn flows() {\n    let data = build(&scan);\n    let json = to_json(&data);\n    golden(\"a\", &json);\n}\n",
            "#[test]\nfn method_flows() {\n    let data = build(&scan);\n    data.render();\n}\n",
        ] {
            let analysis = analyze(text, &["build".to_owned()], &[]);
            let DaVerdict::Unknown(basis) = &analysis.da_003 else {
                panic!("{text}: {:?}", analysis.da_003);
            };
            assert!(
                basis.contains("DS-642") && basis.contains("DS-652") && basis.contains("delegation"),
                "{basis}"
            );
        }
    }

    /// The target's own call is not a delegation target (a derivation: DS-642
    /// does not exclude it), so a discarded result stays FAIL.
    /// @vtest.id TEST-ORACLE-DA-003-DISCARDED-RESULT-STAYS-FAIL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出だけで結果を捨てる本体(double(2); 、let _ = double(2); 、let _ = Type::double(2); の3形)はDA-003がFAILのままであることを確認する
    #[test]
    fn a_discarded_target_result_with_no_other_call_stays_fail() {
        for text in [
            "#[test]\nfn bare() {\n    double(2);\n}\n",
            "#[test]\nfn discarded() {\n    let _ = double(2);\n}\n",
            "#[test]\nfn qualified() {\n    let _ = Type::double(2);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Fail(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// #279: a target called as a method (`c.double(2)`) is a `syn::ExprMethodCall`,
    /// which DA-003 did not read as a call, so a discarded result beside an assert
    /// on something else composed to PASS. The composed verdict must not be PASS.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-TARGET-DISCARDED-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target をメソッドとして呼び結果を捨て、target と無関係な assert_eq! が本体にある Test は、DA-001〜006 の合成結果が PASS にならない(UNKNOWN)ことを確認する
    #[test]
    fn a_method_target_whose_result_is_discarded_beside_an_unrelated_assert_is_not_pass() {
        for text in [
            "#[test]\nfn t() {\n    let c = Calc::default();\n    c.double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let c = Calc::default();\n    let _ = c.double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    Calc::default().double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let c = Calc::default();\n    c.double(2);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A method call whose result reaches an assert-equivalent construct is still
    /// no violation: the method-call reading only moves NoViolation toward
    /// FAIL/UNKNOWN, never the reverse, and does not turn verified calls UNKNOWN.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-TARGET-VERIFIED-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent メソッドとして呼んだ target の結果が let 束縛経由の assert、.unwrap()、.expect()、? の連鎖に到達し、関数呼出が混ざらない Test は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_method_target_whose_result_reaches_an_assert_is_no_violation() {
        for text in [
            "#[test]\nfn t() {\n    let r = c.double(2);\n    assert_eq!(r, 4);\n}\n",
            "#[test]\nfn t() {\n    c.double(2).unwrap();\n}\n",
            "#[test]\nfn t() {\n    Calc::default().double(2).expect(\"ok\");\n}\n",
            "#[test]\nfn t() -> Result<(), E> {\n    c.double(2)?;\n    Ok(())\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// B1: a verified method call is matched to the target by name only, so it
    /// cannot stand in for a function call of the target that stays unverified.
    /// The composed verdict must not be PASS for any of these.
    /// @vtest.id TEST-ORACLE-DA-003-VERIFIED-METHOD-CANNOT-CLEAR-UNVERIFIED-CALL
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 検証済みの根拠がメソッド呼出だけで、検証されていない関数呼出が別に残る本体(.unwrap() の連鎖、assert と併存、let 束縛経由)は、DA-001〜006 の合成結果が PASS にならないことを確認する
    #[test]
    fn a_verified_method_call_does_not_clear_an_unverified_function_call() {
        for text in [
            "#[test]\nfn t() {\n    double(2);\n    o.double(3).unwrap();\n}\n",
            "#[test]\nfn t() {\n    double(2);\n    o.double(3).unwrap();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    double(2);\n    let r = o.double(3);\n    assert_eq!(r, 6);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A method call is identified by name only (the receiver's type is not
    /// resolved), so a call to a same-named method of another type cannot be told
    /// from the target: the discard is neither proven a violation (FAIL) nor
    /// absent. A path call keeps its FAIL.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-CALL-IS-IDENTIFIED-BY-NAME-ONLY
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent メソッド呼出は名前だけで target と同定できないため、結果を捨てる形は FAIL でも違反なしでもなく UNKNOWN になり、関数呼出の形は FAIL のままであることを確認する
    #[test]
    fn a_method_call_matched_by_name_only_is_unknown_not_fail() {
        let method = "#[test]\nfn t() {\n    other.double(2);\n}\n";
        let analysis = analyze(method, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
        let path = "#[test]\nfn t() {\n    Other::double(2);\n}\n";
        let analysis = analyze(path, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Fail(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// #281: a let binding reaches an assert only when its name appears there as
    /// an identifier of its own. `r` inside `other` does not reach.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-NAME-INSIDE-ANOTHER-IDENTIFIER-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent let 束縛名が assert 内の別の識別子(other)の一部にすぎない Test は、結果が assert に到達したと読まれず、DA-001〜006 の合成結果が PASS にならないことを確認する
    #[test]
    fn a_binding_name_inside_another_identifier_does_not_reach_the_assert() {
        for text in [
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let c = Calc::default();\n    let t = (c.double(2), 1);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(x.r, 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(x.r(), 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(\"r\", \"r\");\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A binding that reaches an assert by its exact name stays no violation,
    /// through any expression shape the name can take there.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IDENTIFIER-IN-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛名が assert の引数に識別子としてそのまま現れる Test(比較、メソッド、フィールド、参照、メッセージ引数)は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_binding_named_exactly_in_the_assert_is_no_violation() {
        for body in [
            "assert_eq!(r, 4);",
            "assert_eq!(4, r);",
            "assert!(r > 0);",
            "assert!(r.is_ok());",
            "assert_eq!(r.len(), 1);",
            "assert_eq!(r.field, 1);",
            "assert_eq!(&r, &4);",
            "assert_eq!(*r, 4);",
            "assert_ne!(r, 5, \"msg\");",
            "assert!(r > 0, \"got {}\", other());",
            "assert_eq!((r, 1), (4, 1));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A binding made again under the same name cannot be followed to the assert
    /// by name alone: the name in the assert may belong to either binding.
    /// @vtest.id TEST-ORACLE-DA-003-SHADOWED-BINDING-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 同じ名前の束縛を作り直す(let、closure 引数、match 腕)Test は、assert 内の名前がどの束縛を指すか決められないため、DA-001〜006 の合成結果が PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_shadowed_binding_is_unknown_not_no_violation() {
        for text in [
            "#[test]\nfn t() {\n    let r = double(2);\n    let r = 1;\n    assert_eq!(r, 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert!(xs.iter().all(|r| *r > 0));\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    match o {\n        Some(r) => assert_eq!(r, 1),\n        None => {}\n    }\n}\n",
            "#[test]\nfn t(r: u8) {\n    let r = double(r);\n    assert_eq!(r, 1);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A name inside a nested macro's tokens is not parsed, so whether it is the
    /// binding cannot be told: UNKNOWN, not a reach.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IN-NESTED-MACRO-TOKENS-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛名が assert 内の入れ子のマクロのトークンの中にしか現れない Test は、識別子を確実に判定できないため PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_binding_only_inside_nested_macro_tokens_is_unknown() {
        for body in [
            "assert!(my_check!(r));",
            "assert!(matches!(r, Some(_)) && my_check!(r));",
            "assert!(vec![r; 2].len() == 2);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// The expression macros `matches!`, `format!`, `vec!` nested in an assert are
    /// read as expressions, so the binding named there reaches the assert; a
    /// binding that is not named in an expression position does not.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IN-EXPRESSION-MACRO-IS-READ
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert 内の matches!/format!/vec! の式位置に束縛名がある Test は違反なしのままであり、束縛名が現れない、または matches! のパターン側にしか現れない Test は PASS にならないことを確認する
    #[test]
    fn a_binding_in_an_expression_macro_inside_an_assert_is_read_as_an_expression() {
        for body in [
            "assert!(matches!(r, Some(_)));",
            "assert!(matches!(r.get(), Some(1)));",
            "assert!(matches!(r, Some(x) if x > 0));",
            "assert!(format!(\"{}\", r) == \"4\");",
            "assert_eq!(vec![r], vec![4]);",
            "assert_eq!(vec![r, 1], other());",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        for body in [
            "assert!(matches!(other(), Some(_)));",
            "assert!(matches!(other(), Some(r)));",
            "assert!(format!(\"{}\", other()) == \"4\");",
            "assert_eq!(vec![other()], vec![4]);",
            "assert!(matches!(other(), Some(_)), \"{}\", my_check!(r));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{body}: composed to PASS: {basis:?}");
        }
    }

    /// A binding that appears only in the message arguments of an assert, or as the
    /// left side of a named format argument, verifies nothing: it is printed on
    /// failure only.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IN-ASSERT-MESSAGE-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛名が assert のメッセージ引数、または名前付き書式引数の左辺にしか現れない Test は、結果が検証されたと読まれず、DA-001〜006 の合成結果が PASS にならないことを確認する
    #[test]
    fn a_binding_only_in_an_assert_message_does_not_reach_the_assert() {
        for body in [
            "assert!(other() > 0, \"got {}\", r);",
            "assert!(other() > 0, \"{r}\", r = other());",
            "assert_eq!(other(), 1, \"{r}\", r = 5);",
            "assert_eq!(other(), 1, \"{}\", r);",
            "assert!(format!(\"{r}\", r = other()) == \"4\");",
            "assert!(matches!(other(), Some(_)), \"{}\", r);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{body}: composed to PASS: {basis:?}");
        }
    }

    /// A binding assigned again is a new value under the same name, like a second
    /// `let`: the name in the assert cannot be told to mean the target's result.
    /// @vtest.id TEST-ORACLE-DA-003-REASSIGNED-BINDING-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛を同じ名前で再代入する Test(文での代入、複合代入、assert 内の代入)は、assert 内の名前が target の結果を指すと決められず、PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_reassigned_binding_is_unknown_not_no_violation() {
        for body in [
            "let mut r = double(2); r = other(); assert_eq!(r, 1);",
            "let mut r = double(2); r += 1; assert_eq!(r, 1);",
            "let mut r = double(2); assert!({ r = other(); r == 1 });",
            "let mut r = double(2); assert!({ r += 1; r == 1 });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A `let mut` binding changed through a place (a field, an index, a dereference),
    /// through `&mut`, or through a method receiver is not the target's result when the
    /// assert reads it.
    /// @vtest.id TEST-ORACLE-DA-003-OVERWRITE-THROUGH-PLACE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW, VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-BINDING-MUTATION-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent `let mut` の束縛を、assert が読む位置への代入、参照外しへの代入、`&mut`、メソッドの受け手、クロージャ・マクロの中の代入で変更する Test は、assert が読む値が target の結果と決められず、PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_let_mut_binding_changed_through_a_place_is_unknown() {
        for body in [
            "let mut r = double(2); r.0 = other(); assert_eq!(r.0, 1);",
            "let mut r = double(2); r[0] = other(); assert_eq!(r[0], 1);",
            "let mut r = double(2); *r = other(); assert_eq!(*r, 1);",
            "let mut r = double(2); (*r).x = other(); assert_eq!(r.x, 1);",
            "let mut r = double(2); *(&mut r) = other(); assert_eq!(r, 1);",
            "let mut r = double(2); let p = &mut r; *p = other(); assert_eq!(r, 1);",
            "let mut r = double(2); touch(&mut r.0); assert_eq!(r.0, 1);",
            "let mut r = double(2); (r.0, x) = (other(), 1); assert_eq!(r.0, 1);",
            "let mut r = double(2); r.clear(); assert_eq!(r, 1);",
            "let mut r = double(2); r.0.push(1); assert_eq!(r.0, 1);",
            "let mut r = double(2); r[0].push(1); assert_eq!(r[0], 1);",
            "let mut r = double(2); let f = || r.0 = other(); f(); assert_eq!(r.0, 1);",
            "let mut r = double(2); assert_eq!(touch(&mut r), 1);",
            "let mut r = double(2); assert!({ r.0 = other(); r.0 == 1 });",
            "let mut r = double(2); set!{ r.0 = other(); } assert_eq!(r.0, 1);",
            "let mut r = double(2); reset!(r); assert_eq!(r, 1);",
            "let mut r = double(2); let p = addr_of_mut!(r); assert_eq!(r, 1);",
            "let mut r = double(2); let p = &raw mut r; assert_eq!(r, 1);",
            "let mut r = double(2); let (ref mut a, _) = r; *a = other(); assert_eq!(r.0, 1);",
            "let mut r = double(2); let ref mut a = r; *a = other(); assert_eq!(r, 1);",
            "let mut r = double(2); match r { (ref mut a, _) => *a = other() } assert_eq!(r.0, 1);",
            "let mut r = double(2); if let Some(ref mut a) = r { *a = other(); } assert_eq!(r, 1);",
            "let mut r = double(2); let b = matches!(r.take(), Some(ref a)); assert_eq!(r, None);",
            "let mut r = double(2); let b = matches!(r, Some(ref mut a) if { *a = 1; true }); assert_eq!(r, Some(1));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A binding that cannot be changed through a place (not `let mut`), or a `let mut`
    /// that is only read, stays as it was.
    /// @vtest.id TEST-ORACLE-DA-003-UNCHANGED-BINDING-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent `let` の束縛と、変更のない `let mut` の束縛が assert に届く Test は、違反なしのままであることを確認する
    #[test]
    fn a_binding_that_is_only_read_stays_no_violation() {
        for body in [
            "let r = double(2); assert_eq!(r.0, 4);",
            "let r = double(2); let n = r.len(); assert_eq!(r, n);",
            "let r = double(2); assert_eq!(r.len(), 4);",
            "let mut r = double(2); assert_eq!(r, 4);",
            "let mut r = double(2); assert_eq!(r.0, 4);",
            "let mut r = double(2); let mut v = other(); v.0 = 1; assert_eq!(r, 4);",
            "let mut r = double(2); v[r] = 1; assert_eq!(r, 4);",
            "let mut r = double(2); let c = &r; assert_eq!(r, 4);",
            "let mut r = double(2); let s = format!(\"{}\", r); assert_eq!(r, 4);",
            "let mut r = double(2); assert!(matches!(r, Some(_)));",
            "let mut r = double(2); let b = matches!(r, Some(_)); assert_eq!(r, 4);",
            "let mut r = double(2); match r { (a, _) => drop(a) } assert_eq!(r, 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    fn da_003_of(body: &str) -> DaVerdict {
        let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
        analyze(&text, &["double".to_owned()], &[]).da_003
    }

    /// An assert-equivalent that syntax fixes never to run is not a destination of
    /// DA-003, as a failure divergence there is not reached (DS-1952, DS-1953,
    /// DS-1983): after a statement that leaves on every path, in a branch or an arm
    /// that a constant condition or an earlier irrefutable arm never takes, and in a
    /// loop that a constant never runs. With no other destination DA-003 is UNKNOWN,
    /// never FAIL (DS-615, DS-1680).
    /// @vtest.id TEST-ORACLE-DA-003-NEVER-RUN-ASSERT-IS-NOT-REACHED
    /// @vtest.covers VO-ORACLE-DIVERGENCE-AFTER-EXIT, VO-ORACLE-DIVERGENCE-UNDER-CONSTANT-FALSE, VO-ORACLE-DIVERGENCE-AFTER-IRREFUTABLE-ARM, VO-ORACLE-DA-003-COUNTED-POSITIONS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 無条件の return / continue / break・発散が確定した文の後、定数で偽になる条件・ガードの枝、反駁不能な腕より後の腕、定数で回らないループの中にだけある assert 相当(assert マクロ、束縛を通した assert、unwrap / expect_err / ?、失敗としての発散の決める位置)へ target の結果が届く Test は、DA-003 が違反なしにならず FAIL にもならず UNKNOWN になることを確認する
    #[test]
    fn an_assert_equivalent_that_never_runs_is_not_reached() {
        let never_run = [
            "return; assert_eq!(double(2), 4);",
            "let x = double(2); return; assert_eq!(x, 4);",
            "return; double(2).unwrap();",
            "if false { double(2).expect_err(\"e\"); }",
            "if true { } else { let v = double(2).unwrap(); let _ = v; }",
            "if !matches!(ok, _ if false) { return; } assert_eq!(double(2), 4);",
            "loop { continue; assert_eq!(double(2), 4); }",
            "for i in xs() { let _ = i; break; assert_eq!(double(2), 4); }",
            "while c() { return; assert_eq!(double(2), 4); }",
            "match k() { _ => {} _ => { assert_eq!(double(2), 4); } }",
            "match k() { _ if false => { assert_eq!(double(2), 4); } _ => {} }",
            "panic!(); if double(2) != 4 { panic!(); }",
            "let x = double(2); unreachable!(); if x != 4 { panic!(); }",
            "while false { assert_eq!(double(2), 4); }",
            "for _ in 0..0 { assert_eq!(double(2), 4); }",
            "if c() { return; } else { return; } assert_eq!(double(2), 4);",
            "'a: { break 'a; assert_eq!(double(2), 4); }",
            "let f = || { return; assert_eq!(double(2), 4); }; f();",
        ];
        for body in never_run {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
        let text = "#[test]\nfn t() -> Result<(), E> {\n    return Ok(()); double(2)?; Ok(())\n}\n";
        assert!(matches!(
            analyze(text, &["double".to_owned()], &[]).da_003,
            DaVerdict::Unknown(_)
        ));
    }

    /// The other side: an assert-equivalent that may run stays a destination, also
    /// under a condition decided at run time (DS-1935 reads the flow of the value),
    /// before an unconditional `return`, after a conditional `return`, after a
    /// labeled block left by `break`, after an exit or an arm under `cfg`, and beside
    /// one that never runs.
    /// @vtest.id TEST-ORACLE-DA-003-ASSERT-THAT-MAY-RUN-IS-REACHED
    /// @vtest.covers VO-ORACLE-DIVERGENCE-AFTER-EXIT, VO-ORACLE-DIVERGENCE-UNDER-CONSTANT-FALSE, VO-ORACLE-DIVERGENCE-AFTER-IRREFUTABLE-ARM, VO-ORACLE-DA-003-COUNTED-POSITIONS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 実行時の値で決まる条件の中、無条件の return より前、条件付きの return の後、break で抜けるラベル付きブロックの後、クロージャ自身の return の後に呼ぶ位置、実行されない照合と並ぶ実行されうる照合にある assert 相当へ target の結果が届く Test は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn an_assert_equivalent_that_may_run_stays_reached() {
        for body in [
            "if flag() { assert_eq!(double(2), 4); }",
            "if !flag() { double(2).unwrap(); }",
            "assert_eq!(double(2), 4); return;",
            "if c() { return; } assert_eq!(double(2), 4);",
            "match k() { _ if flag() => {} _ => { assert_eq!(double(2), 4); } }",
            "match k() { Foo(_) => {} _ => { assert_eq!(double(2), 4); } }",
            "'a: { break 'a; } assert_eq!(double(2), 4);",
            "let f = || { return; }; f(); assert_eq!(double(2), 4);",
            "loop { assert_eq!(double(2), 4); break; }",
            "if 1 + 1 == 3 { assert_eq!(double(2), 4); }",
            "assert_eq!(double(2), 4); return; assert_eq!(double(3), 6);",
            "let x = double(2); if c() { return; } if x != 4 { panic!(); }",
            "#[cfg(windows)] return; assert_eq!(double(2), 4);",
            "#[cfg(windows)] { return; } assert_eq!(double(2), 4);",
            "#[cfg(windows)] panic!(); double(2).unwrap();",
            "match k() { #[cfg(windows)] _ => {} _ => { assert_eq!(double(2), 4); } }",
        ] {
            assert_eq!(da_003_of(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// DS-1956: after a projection assignment whose path is fixed by syntax, the
    /// positions not assigned keep the target's result; DS-1966: an assignment in a
    /// branch, a loop or the right side of `&&` / `||` leaves the other positions
    /// derived on every path.
    /// @vtest.id TEST-ORACLE-DA-003-PROJECTION-ASSIGNMENT-KEEPS-OTHER-POSITIONS
    /// @vtest.covers VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT, VO-ORACLE-DA-003-CONDITIONAL-ASSIGNMENT-MERGE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の結果を束縛した `let mut` の束縛に、フィールド名・タプルの番号・整数リテラルの添字だけからなる経路で射影代入（条件付き・ループ内・`&&`/`||` の右辺を含む）をしたあと、代入していない位置を assert が読む Test と、複合代入した位置を読む Test は、DA-003 が違反なしになることを確認する
    #[test]
    fn a_projection_assignment_keeps_the_positions_it_does_not_assign() {
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.0 = 5; assert_eq!(r.1, 4);",
            "let mut v = double(2); v[0] = 7; assert_eq!(v[1], 4);",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.f.h, 4);",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.h, 4);",
            "let mut r = double(2); r[0][1] = 5; assert_eq!(r[0][0], 4);",
            "let mut r = double(2); r.r#f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f = 5; assert_eq!(Some(&r.g), Some(&4));",
            "let mut r = double(2); r.b &= x(); assert_eq!(r.b, true);",
            "let mut r = double(2); r.b |= false; assert_eq!(r.b, true);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.g, 4, \"{}\", r.f);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.g, 4); assert_eq!(r.f, 5);",
            "let mut r = double(2).clone(); r.f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.0 += 1; assert_eq!(r.0, 5);",
            "let mut r = double(2); r.f += x(); assert_eq!(r.f, 5);",
            "let mut r = double(2); if c() { r.f = 5; } assert_eq!(r.g, 4);",
            "let mut r = double(2); match k() { 1 => r.f = 5, _ => {} } assert_eq!(r.g, 4);",
            "let mut r = double(2); for x in xs() { r.f = x; } assert_eq!(r.g, 4);",
            "let mut r = double(2); while c() { r.f = 5; } assert_eq!(r.g, 4);",
            "let mut r = double(2); let ok = c() || { r.f = 5; true }; assert_eq!(r.g, 4);",
        ] {
            assert_eq!(da_003_of(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// DS-1956: an assigned position holds the assigned value, not the target's
    /// result; DS-1966: one assigned on some paths only is neither. Reading it in an
    /// assert is not PASS, also through a reference, a block, a conversion or a
    /// nested macro.
    /// @vtest.id TEST-ORACLE-DA-003-ASSIGNED-POSITION-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT, VO-ORACLE-DA-003-CONDITIONAL-ASSIGNMENT-MERGE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 射影代入した位置（条件付き・ループ内・`&&` の右辺の代入を含む）を assert が読む Test、平の代入のあとに複合代入した位置を読む Test、assert の中で参照・ブロック・変換・入れ子のマクロを通して読む Test は、DA-003 が違反なしにならず UNKNOWN になることを確認する
    #[test]
    fn a_read_of_an_assigned_position_is_not_pass() {
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(r.f, 5);",
            "let mut v = double(2); v[0] = 7; assert_eq!(v[0], 7);",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.f.g, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.f.0, 5);",
            "let mut r = double(2); r.f = 5; r.f += 1; assert_eq!(r.f, 6);",
            "let mut r = double(2); r.f = 5; r.f.g += 1; assert_eq!(r.f.g, 6);",
            "let mut r = double(2); if c() { r.f = 5; } assert_eq!(r.f, 5);",
            "let mut r = double(2); if c() { r.f = 5; } else { r.f = 6; } assert_eq!(r.f, 5);",
            "let mut r = double(2); match k() { 1 => r.f = 5, _ => {} } assert_eq!(r.f, 5);",
            "let mut r = double(2); let ok = c() && { r.f = 5; true }; assert_eq!(r.f, 5);",
            "let mut r = double(2); for x in xs() { r.f = x; } assert_eq!(r.f, 5);",
            "let mut r = double(2); while c() { r.f = 5; } assert_eq!(r.f, 5);",
            "let mut r = double(2); loop { r.f = 5; break; } assert_eq!(r.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!((&r).f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!({ r }.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!({ let a = r; a.f }, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!((r, 1).0.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(Some(r.f), Some(5));",
            "let mut r = double(2); r.f = 5; assert!(matches!(r.f, 5));",
            "let mut r = double(2); r.f = 5; assert_eq!(vec![r.f][0], 5);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1956: a projection whose path is not fixed by syntax makes the whole
    /// binding UNKNOWN; DS-1959: so do `&mut x.p`, a closure or `async` block that
    /// assigns it, a method receiver and `ref mut`, and an assignment in macro
    /// tokens or by dereference is not read as a projection assignment.
    /// @vtest.id TEST-ORACLE-DA-003-UNFIXED-PATH-OR-MUTATION-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT, VO-ORACLE-DA-003-BINDING-MUTATION-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::place_changes
    /// @vtest.intent 経路を構文から確定できない射影代入（変数・文字列・式の添字、参照外し、分割代入）と、`&mut x.p`・可変でキャプチャするクロージャと async ブロック・メソッドの受け手・`ref mut`・マクロの中の代入を経た束縛は、代入していない位置を読む Test でも DA-003 が UNKNOWN になることを確認する
    #[test]
    fn an_unfixed_path_or_a_mutation_keeps_the_whole_binding_unknown() {
        for body in [
            "let mut v = double(2); let i = 0; v[i] = 7; assert_eq!(v[1], 4);",
            "let mut r = double(2); r[\"k\"] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r[k()] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f[i] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); (*r).f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); *r.f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); (r.f, x) = (5, 1); assert_eq!(r.g, 4);",
            "let mut r = double(2); touch(&mut r.f); assert_eq!(r.g, 4);",
            "let mut r = double(2); let p = &mut r.f; *p = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); let f = || r.f = 5; f(); assert_eq!(r.g, 4);",
            "let mut r = double(2); let f = async { r.f = 5; }; assert_eq!(r.g, 4);",
            "let mut r = double(2); set!{ r.f = 5 }; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f.push(1); assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f = 5; let (ref mut a, _) = r; assert_eq!(r.g, 4);",
            "let mut r = double(2); r = other(); r.f = 5; assert_eq!(r.g, 4);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1939: a value whose positions differ is UNKNOWN as a method receiver
    /// (also an operator, a formatting trait or a pattern over it, whose positions
    /// are not fixed by syntax) and through an index that is not an integer literal;
    /// compared whole it reaches the assert (the false PASS DS-1939 and DS-1956
    /// disclose). A binding of a literal that holds the result in one element is not
    /// followed through an assignment, so a read of an element that is not the
    /// result does not pass.
    /// @vtest.id TEST-ORACLE-DA-003-MIXED-POSITIONS-ARE-READ-BY-POSITION
    /// @vtest.covers VO-ORACLE-DA-003-PER-POSITION-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::read_is_derived
    /// @vtest.intent 一部の位置を射影代入した値を、メソッドの受け手・演算子の被演算子・`matches!` と `format!` の引数にする Test、整数リテラルでない添字で読む Test は UNKNOWN になり、`assert_eq!` や `==` で値全体を比べる Test は開示された偽 PASS のとおり違反なしになることを確認する。target の値を一つの要素に持つ構築式の束縛で、target の値の要素に射影代入したあと別の要素を読む Test が違反なしにならないことも確認する
    #[test]
    fn a_value_with_an_assigned_position_is_read_by_position() {
        for body in [
            "let mut v = double(2); v[0] = 7; assert_eq!(v.len(), 1);",
            "let mut r = double(2); r.f = 5; assert!(r.is_ok());",
            "let mut r = double(2); r.f = 5; assert!((&r).is_ok());",
            "let mut r = double(2); r.f = 5; assert!((*r).is_ok());",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.f.len(), 2);",
            "let mut r = double(2); r.f = 5; assert!(matches!(r, R { f: 5, .. }));",
            "let mut r = double(2); r.f = 5; assert_eq!(format!(\"{}\", r), \"5\");",
            "let mut r = double(2); r.f = 5; assert!(!r);",
            "let mut r = double(2); r.f = 5; assert_eq!(r + 1, x());",
            "let mut r = double(2); r.f = 5; assert_eq!(Some(r), Some(x()));",
            "let mut v = double(2); v[0] = 7; let i = 1; assert_eq!(v[i], 4);",
            "let mut v = double(2); v[0] = 7; assert_eq!(v[..1], [7]);",
            "let mut t = (double(2), 1); t.0 = 5; assert_eq!(t.1, 1);",
            "let mut t = S { a: double(2), b: 1 }; t.a = 5; assert_eq!(t.b, 1);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(r, R { f: 5, g: 4 });",
            "let mut r = double(2); r.f = 5; r.g = 6; assert_eq!(r, R { f: 5, g: 6 });",
            "let mut r = double(2); if c() { r.f = 5; } assert_eq!(r, R { f: 5, g: 4 });",
            "let mut r = double(2); r.f = 5; assert!(r == R { f: 5, g: 4 });",
            "let mut r = double(2); r.f = 5; assert_ne!(&r, &R { f: 5, g: 3 });",
        ] {
            assert_eq!(da_003_of(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// DS-1959: a `let mut` binding that is a method receiver, also inside the
    /// arguments of an assert, is not shown to keep the target's result.
    /// @vtest.id TEST-ORACLE-DA-003-LET-MUT-RECEIVER-INSIDE-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-BINDING-MUTATION-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::place_changes
    /// @vtest.intent `let mut` の束縛（射影代入の有無を問わない）を assert の引数の中でメソッドの受け手にする Test は、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_let_mut_receiver_inside_an_assert_is_a_change() {
        for body in [
            "let mut r = double(2); assert!({ r.clear(); true }); assert_eq!(r, vec![]);",
            "let mut r = double(2); assert_eq!(r.len(), 4);",
            "let mut r = double(2); r.f = 5; assert!({ r.g.clear(); true }); assert_eq!(r.g, vec![]);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.g.len(), 4);",
            "let mut r = double(2); r.f = 5; assert!(r.g.is_ok());",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1939: a literal that holds the target's result in one element is not a
    /// value whose positions are all derived, also after a compound assignment only.
    /// @vtest.id TEST-ORACLE-DA-003-LITERAL-BINDING-WITH-COMPOUND-ASSIGNMENT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-PER-POSITION-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::binding_reach
    /// @vtest.intent target の値を一つの要素に持つタプル・構造体・配列の構築式の束縛に複合代入だけをしたあと、target の値でない要素を読む Test が違反なしにならないことを確認する
    #[test]
    fn a_literal_binding_with_a_compound_assignment_is_unknown() {
        for body in [
            "let mut t = (double(2), 1); t.1 += 1; assert_eq!(t.1, 2);",
            "let mut t = S { a: double(2), b: 1 }; t.b += 1; assert_eq!(t.b, 2);",
            "let mut t = [double(2), 1]; t[1] += 1; assert_eq!(t[1], 2);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1940: a branch whose arms differ in derivation is not derived. After a
    /// projection assignment, a read of the binding in an arm is not shown derived.
    /// @vtest.id TEST-ORACLE-DA-003-BRANCH-ARMS-AFTER-PROJECTION-ASSIGNMENT-ARE-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-BRANCH-ARM-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::span_binding_reach
    /// @vtest.intent 射影代入したあと、assert の中の `if`・`if let`・`match` の腕や、ラベル付きブロックの `break` の値と末尾で、代入した位置と代入していない位置を読み分ける Test が違反なしにならないことを確認する
    #[test]
    fn a_branch_whose_arms_mix_assigned_positions_is_unknown() {
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(if c() { r.f } else { r.g[0] }, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(if let Some(_) = o() { r.f } else { r.g }, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(match k() { 1 => r.f, _ => r.g }, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!('a: { if c() { break 'a r.f; } r.g[0] }, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!('a: { match k() { 0 => break 'a r.f, _ => {} } r.g }, 5);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1940: a branch whose arms other than failure divergences (DS-1951) differ in
    /// derivation at the position read is not shown derived, wherever it is.
    /// @vtest.id TEST-ORACLE-DA-003-BRANCH-WITH-DIFFERING-ARMS-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-BRANCH-ARM-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent `if`・`if let`・`match` とラベル付きブロックの値が、失敗として発散する腕を除いて、読む位置で target の結果から派生する腕と派生しない腕を持つとき(入れ子の分岐、let で束縛した分岐、`return` で抜ける腕、射影した位置、発散を決める位置を含む)、DA-003 が違反なしにならないことを確認する
    #[test]
    fn a_branch_whose_arms_differ_in_derivation_is_unknown() {
        for body in [
            "let x = double(2); assert_eq!(if c() { 5 } else { x }, 5);",
            "let x = double(2); assert_eq!('a: { if c() { break 'a 5; } x }, 4);",
            "assert_eq!('a: { if c() { break 'a 5; } double(2) }, 4);",
            "assert_eq!(if c() { double(2) } else { 5 }, 4);",
            "assert_eq!(match k() { 0 => double(2), _ => 5 }, 4);",
            "assert_eq!(if let Some(_) = o() { double(2) } else { 5 }, 4);",
            "assert_eq!(if a() { if b() { double(2) } else { 5 } } else { double(3) }, 4);",
            "let y = if c() { double(2) } else { 5 }; assert_eq!(y, 4);",
            "let y = match k() { 0 => 5, _ => double(2) }; assert_eq!(y + 1, 5);",
            "assert_eq!(if c() { double(2) } else { return; }, 4);",
            "assert_eq!(if c() { 5 } else { assert!(false); double(3) }, 5);",
            "assert_eq!((if c() { (double(2), 1) } else { (double(3), 2) }).1, 1);",
            "let x = double(2); assert_eq!((if c() { (double(3), 1) } else { (double(3), x) }).1, 1);",
            "assert!(if c() { double(2) == 4 } else { false });",
            "if (if c() { double(2) } else { 5 }) != 4 { panic!() }",
            "let x = double(2); assert_eq!(if a() { x } else { if c() { return; } panic!() }, 4);",
            "assert_eq!(({ if c() { (double(2), 1) } else { (double(3), 2) } }).1, 1);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1940: a branch whose arms other than failure divergences are all derived,
    /// through any call of a declared target or any binding of one, stays derived.
    /// @vtest.id TEST-ORACLE-DA-003-BRANCH-WITH-DERIVED-ARMS-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-BRANCH-ARM-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent 失敗として発散する腕を除くすべての腕が、読む位置で target の結果(別の呼出や別の束縛を含む)から派生する分岐とラベル付きブロックは、assert に届けば DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_branch_whose_arms_are_all_derived_stays_no_violation() {
        for body in [
            "assert_eq!(if c() { double(2) } else { double(3) }, 4);",
            "let x = double(2); assert_eq!(if c() { double(3) } else { x }, 4);",
            "let x = double(2); let y = double(3); assert_eq!(if c() { x } else { y }, 4);",
            "let x = double(2); assert_eq!(match k() { 0 => x, _ => unreachable!() }, 4);",
            "let y = if c() { double(2) } else { double(3) }; assert_eq!(y, 4);",
            "assert_eq!(if c() { double(2) } else { panic!(\"no\") }, 4);",
            "assert_eq!((if c() { (double(2), 1) } else { (double(3), 2) }).0, 4);",
            "assert_eq!('a: { if c() { break 'a double(2); } double(3) }, 4);",
            "assert_eq!(if a() { if b() { double(2) } else { double(1) } } else { double(3) }, 4);",
            "assert_eq!(({ if c() { (double(2), 1) } else { (double(3), 2) } }).0, 4);",
            "let x = double(2); assert!(x == 4 && (if c() { x } else { 5 }) == 5);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::NoViolation),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
        // Derivation: an arm derived from another declared target's call counts as
        // derived, as `JoinSources` reads every declared target.
        let text =
            "#[test]\nfn t() {\n    assert_eq!(if c() { double(2) } else { triple(1) }, 4);\n}\n";
        let analysis = analyze(text, &["double".to_owned(), "triple".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
    }

    /// DS-1948: a compound assignment whose value absorbs fixes the position to the
    /// literal, so it is not derived; DS-1956: a raw field name is the same position.
    /// @vtest.id TEST-ORACLE-DA-003-ABSORBING-OR-RAW-PROJECTION-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-LITERAL-ABSORBED-OPERAND-NOT-FLOWING, VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::place_changes
    /// @vtest.intent `r.b &= false`・`r.b |= true` の複合代入をした位置を読む Test と、生識別子 `r#f` で射影代入した位置を `f` で読む Test（その逆も）が違反なしにならないことを確認する
    #[test]
    fn an_absorbing_or_raw_projection_assignment_is_not_pass() {
        for body in [
            "let mut r = double(2); r.b &= false; assert_eq!(r.b, false);",
            "let mut r = double(2); r.b |= true; assert_eq!(r.b, true);",
            "let mut r = double(2); r.b |= (true); assert!(r.b);",
            "let mut r = double(2); r.r#f = 5; assert_eq!(r.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.r#f, 5);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// Narrowings below DS-1956: the assigned value is not followed, statement
    /// order is not read, an integer-literal index is not told apart from a field
    /// or a tuple number at the same step, and an assignment inside the assert is
    /// not read as a projection assignment. Each keeps the verdict UNKNOWN where
    /// DS-1956 alone might derive the read.
    /// @vtest.id TEST-ORACLE-DA-003-PROJECTION-ASSIGNMENT-NARROWINGS-ARE-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::read_is_derived
    /// @vtest.intent 代入した値を追わない・文の順序を読まない・同じ段の整数リテラル添字とフィールドを区別しない・assert の中の代入を射影代入として読まない、という解析の限界に当たる Test が、違反なしにならず UNKNOWN になることを確認する
    #[test]
    fn projection_assignment_narrowings_are_unknown() {
        for body in [
            "let mut r = double(2); r.f = double(3); assert_eq!(r.f, 6);",
            "let mut r = double(2); assert_eq!(r.f, 4); r.f = 5;",
            "let mut r = double(2); r.0 = 5; assert_eq!(r[0], 5);",
            "let mut r = double(2); r[0] = 5; assert_eq!(r.0, 5);",
            "let mut r = double(2); r[0] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); assert!({ r.f = 5; r.g == 4 });",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// An `assertion_macros` entry of the config has a grammar vtest does not know,
    /// so a binding named in it cannot be read as a checked expression; the same
    /// shape in a built-in assert is read.
    /// @vtest.id TEST-ORACLE-DA-003-CONFIG-ASSERT-MACRO-ARGUMENTS-ARE-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent config の assertion_macros に挙げたマクロの引数に束縛名がある Test は、引数の文法が分からないため UNKNOWN になり、組込の assert では同じ形が違反なしのままであることを確認する
    #[test]
    fn a_binding_in_a_config_assertion_macro_is_unknown() {
        let config = ["assert_matches!".to_owned()];
        let body = |name: &str| {
            format!("#[test]\nfn t() {{\n    let r = double(2);\n    {name}!(r, Some(_));\n}}\n")
        };
        let analysis = analyze(&body("assert_matches"), &["double".to_owned()], &config);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
        let analysis = analyze(&body("assert_eq"), &["double".to_owned()], &config);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
    }

    /// #282: a target named in the tokens of a macro that is not an assert is not
    /// read, so the call is neither proven nor refuted: UNKNOWN, never PASS.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-IN-NON-ASSERT-MACRO-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の名前が assert 以外のマクロ(println!、dbg!、vec!、独自マクロ)のトークンに現れ、target と無関係な assert_eq! が本体にある Test は、DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_target_named_in_a_non_assert_macro_is_unknown_not_pass() {
        for text in [
            "#[test]\nfn t() {\n    println!(\"{}\", c.double(2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", double(2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    dbg!(double(2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    dbg!(double(2));\n}\n",
            "#[test]\nfn t() {\n    let v = vec![double(2)];\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let v = vec![double(2)];\n    assert_eq!(v.len(), 1);\n}\n",
            "#[test]\nfn t() {\n    let s = format!(\"{}\", double(2));\n    assert_eq!(s, \"4\");\n}\n",
            "#[test]\nfn t() {\n    my_macro! { double(2) }\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let _ = {\n        eprintln!(\"{}\", Calc::double(2));\n    };\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", assert_eq!(double(2), 4));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(other(), 1);\n    println!(\"{}\", double(2));\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A call written inside a built-in assert's checked arguments is read from
    /// the arguments' expressions: it verifies the target, as a call outside a
    /// macro that reaches an assert does.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-CALL-IN-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert!、assert_eq!、assert_ne! の検証される引数の中で target を呼ぶ Test(入れ子の matches!、format!、vec! を含む)は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_target_called_in_a_checked_assert_argument_is_no_violation() {
        for text in [
            "#[test]\nfn t() {\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_ne!(Calc::double(2), 5);\n}\n",
            "#[test]\nfn t() {\n    assert!(double(2) == 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(4, c.double(2));\n}\n",
            "#[test]\nfn t() {\n    assert!(matches!(double(2), 4));\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(vec![double(2)], vec![4]);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(format!(\"{}\", double(2)), \"4\");\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(double(2), 4, \"message {}\", 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(r, 4);\n    assert_eq!(double(3), 6);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A target named in an assert's message arguments, in an assert the argument
    /// grammar of which is not known, or in tokens that do not parse as
    /// expressions is not shown verified.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-IN-UNCHECKED-ASSERT-TOKENS-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の名前が assert のメッセージ引数、config の assertion_macros のマクロ、式として読めない assert の引数にだけ現れる Test は、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_target_named_only_in_unchecked_assert_tokens_is_unknown() {
        let config = ["assert_matches!".to_owned()];
        for text in [
            "#[test]\nfn t() {\n    assert_eq!(other(), 1, \"{}\", double(2));\n}\n",
            "#[test]\nfn t() {\n    assert!(other(), \"{x}\", x = double(2));\n}\n",
            "#[test]\nfn t() {\n    assert_matches!(double(2), Some(_));\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(double(2) => 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(my_macro!(double(2)), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(other(), 1);\n    assert_eq!(foo(), 2, \"{}\", double(2));\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &config);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// Reading a call inside an assert never clears a call outside a macro that
    /// stays unverified: the verdict is the one without the assert's call, so no
    /// shape moves from UNKNOWN or FAIL to PASS.
    /// @vtest.id TEST-ORACLE-DA-003-ASSERT-CALL-DOES-NOT-CLEAR-UNVERIFIED-CALL
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent マクロの外に検証されない target 呼出が残る Test は、別の target 呼出が assert の検証される引数にあっても DA-003 が違反なしにならない(UNKNOWN)ことを確認する
    #[test]
    fn an_assert_call_does_not_clear_an_unverified_call_outside_a_macro() {
        for text in [
            "#[test]\nfn t() {\n    double(3);\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let _ = double(3);\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    c.double(3);\n    assert_eq!(c.double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let Some(x) = double(3) else { return };\n    assert_eq!(double(2), 4);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A method call verified inside an assert is matched by name only: it does
    /// not clear a function call of the target that stays unverified, nor a name
    /// that a non-assert macro may call.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-IN-ASSERT-CANNOT-CLEAR-UNREAD-CALL
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert の中のメソッド呼出だけが検証済みの根拠で、検証されない関数呼出や非 assert マクロ内の target の名前が別に残る Test は、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_method_call_verified_in_an_assert_does_not_clear_other_uses() {
        for text in [
            "#[test]\nfn t() {\n    double(2);\n    assert_eq!(o.double(3), 6);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", double(2));\n    assert_eq!(o.double(3), 6);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A name inside macro tokens that is not a use of the target is not read as
    /// one: a path prefix, a field, a method of another name, a macro of the same
    /// name and a field label do not make DA-003 UNKNOWN; the turbofish call does.
    /// @vtest.id TEST-ORACLE-DA-003-NON-CALL-NAME-IN-MACRO-IS-NOT-A-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::NameUse::tokens_use_name
    /// @vtest.intent マクロのトークンの中の target の名前が、パスの前置(double::VALUE)、フィールド、メソッド名、同名のマクロ、構造体のフィールドのラベルであるときは使用と読まず、turbofish 付きの呼出(double::<u8>(2))は使用と読むことを確認する
    #[test]
    fn a_name_in_macro_tokens_that_is_not_a_use_is_not_read_as_one() {
        for text in [
            "#[test]\nfn t() {\n    println!(\"{}\", double::VALUE);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", s.double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{:?}\", Cfg { double: 1 });\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    double!(1);\n    assert_eq!(other(), 1);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
        let turbofish = "#[test]\nfn t() {\n    println!(\"{}\", double::<u8>(2));\n    assert_eq!(other(), 1);\n}\n";
        let analysis = analyze(turbofish, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// #288: the target's name as a value (not the callee of a direct call) may be
    /// an indirect call, which is not read: UNKNOWN, never PASS.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-USED-AS-VALUE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の名前が直接呼出の callee 以外の値の位置(括弧つきの callee、let への代入、引数、配列、型付きの束縛、パス)に現れ、target と無関係な assert_eq! が本体にある Test は、DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_target_used_as_a_value_is_unknown_not_pass() {
        for text in [
            "#[test]\nfn t() {\n    (double)(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n    f(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f: fn(i32) -> i32 = double;\n    let _ = f(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let v = [double];\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let r: Vec<_> = xs.iter().map(Calc::double).collect();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = double as fn(i32) -> i32;\n    assert_eq!(f(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n    assert_eq!(f(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n}\n",
            "#[test]\nfn t() {\n    let fs = vec![double];\n    fs[0](2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", apply(double, 2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    run_with!(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(apply(double), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(vec![double][0](2), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(other(), 1, \"{}\", apply(double));\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// The local exception covers only a `let` and the uses after it in its own
    /// block: a use outside that block or before the `let`, and a binding in a
    /// closure, a match arm, a nested fn or an assignment, are no exception.
    /// @vtest.id TEST-ORACLE-DA-003-LOCAL-EXCEPTION-IS-SCOPED
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::NameUse::new
    /// @vtest.intent 同名のローカルの例外は、let とその同じブロックの中の後ろの使用だけに効き、ブロックの外の使用、let より前の使用、クロージャの引数、match のアーム、ネストした fn の引数、代入には効かず、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn the_local_name_exception_is_scoped_to_its_let() {
        let in_block = "#[test]\nfn t() {\n    for i in 0..2 {\n        let double = make(i);\n        assert_eq!(double[\"a\"], 1);\n    }\n    assert_eq!(other(), 1);\n}\n";
        let analysis = analyze(in_block, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
        for text in [
            "#[test]\nfn t() {\n    { let double = 1; }\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n    let double = 5;\n    f(2);\n    assert_eq!(other(), double);\n}\n",
            "#[test]\nfn t() {\n    let g = |double: i32| double;\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    match x { Some(double) => {}, _ => {} }\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    fn h(double: i32) -> i32 { double }\n    apply(double);\n    assert_eq!(h(1), 1);\n}\n",
            "#[test]\nfn t() {\n    double = 3;\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let double = 1;\n    apply(self::double);\n    assert_eq!(other(), double);\n}\n",
            "#[test]\nfn t() {\n    let double = 1;\n    println!(\"{}\", apply(double));\n    assert_eq!(other(), 1);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let expected_unknown = !text.contains("println!");
            assert_eq!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                expected_unknown,
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A constant target is not callable and a local that is named like the
    /// target is not the target: neither is an indirect call. A path to the target
    /// through a type still is.
    /// @vtest.id TEST-ORACLE-DA-003-CONSTANT-OR-LOCAL-NAME-IS-NOT-AN-INDIRECT-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED, VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::is_constant_name
    /// @vtest.intent 大文字の名前の target(定数)の値の使用と、同じブロックの前の let で束縛された同名のローカル変数の使用は、間接呼出と読まず DA-003 が違反なしのままで、型を経由する同名の経路は値の使用として UNKNOWN になることを確認する
    #[test]
    fn a_constant_or_local_name_is_not_an_indirect_call() {
        let constant = "#[test]\nfn t() {\n    let form = parse(RUST_FORM);\n    assert_eq!(form.kind, 1);\n}\n";
        let analysis = analyze(constant, &["RUST_FORM".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
        let local = "#[test]\nfn t() {\n    let bundle = &envelope;\n    let n = count(bundle);\n    assert_eq!(n, 1);\n}\n";
        let analysis = analyze(local, &["bundle".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
        let through_type = "#[test]\nfn t() {\n    let bundle = 1;\n    let f = Calc::bundle;\n    assert_eq!(other(), bundle);\n}\n";
        let analysis = analyze(through_type, &["bundle".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// A name that is not a value use of the target is not an indirect call: a
    /// `use`, a type, a path prefix, a field, a method name, a string, and a
    /// direct call that is verified stay no violation.
    /// @vtest.id TEST-ORACLE-DA-003-NON-VALUE-NAME-IS-NOT-AN-INDIRECT-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent use 文、型、パスの前置、フィールド名、メソッド名、文字列に現れる target の名前と、検証された直接呼出に併存する値の使用は、間接呼出と読まず DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_name_that_is_not_a_value_use_is_not_an_indirect_call() {
        for text in [
            "#[test]\nfn t() {\n    use helpers::double;\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x: double::Kind = other();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x: Vec<double> = other();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = double::VALUE;\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = s.double;\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = s.double_it();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = s.double(2);\n    assert_eq!(x, 4);\n}\n",
            "#[test]\nfn t() {\n    let x = \"double\";\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(r, 4);\n    let f = double;\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A call in a position of an assert's checked argument where its value is
    /// dropped (a statement of a block, a `let _`, a closure body, a loop, a branch
    /// condition) does not verify the target.
    /// @vtest.id TEST-ORACLE-DA-003-DISCARDED-CALL-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、ブロックの文、let _、使われない let の束縛、クロージャの本体、ループ、if と match の条件と被検査式、代入の右辺にある target の呼出は、結果が比較される値へ流れないので検証済みと数えず、DA-003 が UNKNOWN で合成結果が PASS にならないことを確認する
    #[test]
    fn a_call_whose_value_is_dropped_inside_an_assert_argument_is_unknown() {
        for text in [
            "assert!({ double(2); true });",
            "assert_eq!({ let _ = double(2); 1 }, 1);",
            "assert_ne!({ let _ = double(2); 1 }, 2);",
            "assert_eq!(1, { double(2); 1 });",
            "assert!((|| { double(2); true })());",
            "assert!((|| double(2) == 4)());",
            "assert_eq!(xs.iter().map(|x| double(*x)).sum::<i32>(), 4);",
            "assert!({ for i in 0..2 { double(i); } true });",
            "assert!({ while c { double(2); } true });",
            "assert!({ loop { double(2); break; } true });",
            "assert!({ let x = double(2); true });",
            "assert!({ let x = double(2); let _y = 1; true });",
            "assert_eq!({ let (a, b) = (double(2), 1); b }, 1);",
            "assert!({ let r = double(2); true && { r; false } });",
            "assert!({ let r = double(2); (|| r)() == 4 });",
            "assert!({ let r = double(2); let r = 4; r == 4 });",
            "assert!({ let mut r = 0; r = double(2); r == 4 });",
            "assert!({ let mut r = 0; r += double(2); r == 4 });",
            "assert!(if double(2) > 0 { true } else { false });",
            "assert!(match double(2) { 4 => true, _ => false });",
            "assert!(match c { _ => { double(2); true } });",
            "assert!({ match c { _ => { double(2); } } true });",
            "assert_eq!(unsafe { double(2); 1 }, 1);",
            "assert!({ format!(\"{}\", double(2)); true });",
            "assert!({ let _f = async { double(2) }; true });",
            "assert!({ let r = double(2); let s = r + 1; s == 5 });",
            "assert!({ s.double(2); true });",
            "assert!({ let _ = s.double(2); true });",
            "assert!({ let _ = Calc::double(2); true });",
            "assert!({ let _ = double::<u8>(2); true });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A call whose value can reach the compared value still verifies the target.
    /// @vtest.id TEST-ORACLE-DA-003-CALL-REACHING-THE-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の式そのもの、二項演算、メソッドチェーン、フィールド、参照、キャスト、? の被演算子、ブロックの末尾の式、末尾で使われる let の束縛、失敗として発散する腕を除くすべての腕が派生である if と match の各分岐の末尾の式にある target の呼出は、検証済みと数えられ、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_call_whose_value_reaches_the_compared_value_inside_an_assert_stays_no_violation() {
        for text in [
            "assert_eq!(double(2), 4);",
            "assert!(double(2) > 0);",
            "assert_eq!(1, double(2));",
            "assert_eq!(double(2) + 1, 5);",
            "assert_eq!(-double(2), -4);",
            "assert!(!(double(2) == 5));",
            "assert_eq!(double(2).abs(), 4);",
            "assert_eq!(double(2).0, 4);",
            "assert_eq!(&double(2), &4);",
            "assert_eq!(double(2) as u8, 4);",
            "assert_eq!(double(2)?, 4);",
            "assert_eq!(Calc::double(2), 4);",
            "assert_eq!(double::<u8>(2), 4);",
            "assert_eq!(s.double(2), 4);",
            "assert_eq!((double(2), 1), (4, 1));",
            "assert_eq!(vec![double(2)], vec![4]);",
            "assert_eq!(format!(\"{}\", double(2)), \"4\");",
            "assert!(matches!(double(2), 4));",
            "assert!({ double(2) == 4 });",
            "assert!(unsafe { double(2) } == 4);",
            "assert_eq!({ let _unused = 1; double(2) }, 4);",
            "assert_eq!({ let r = double(2); r }, 4);",
            "assert_eq!({ let r: i32 = double(2); r }, 4);",
            "assert_eq!({ let r = double(2); r + 1 }, 5);",
            "assert_eq!({ let r = double(2); r.value }, 4);",
            "assert_eq!({ let r = double(2); &r }, &4);",
            "assert!(if c { double(2) == 4 } else { double(3) == 6 });",
            "assert_eq!(if c { double(2) } else { panic!() }, 4);",
            "assert!(match c { true => double(2) == 4, false => unreachable!() });",
            "assert_eq!(match c { 0 => double(1) * 4, _ => double(2) }, 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// Only the listed positions carry a call's value to the compared value: a
    /// position that is not listed (a literal's element that a
    /// projection leaves out or that an unresolved index may leave out, a `match` scrutinee, a `let` that is not
    /// used by the tail) does not.
    /// @vtest.id TEST-ORACLE-DA-003-UNLISTED-POSITION-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、流れる位置として列挙されていない位置(射影で残らない集約リテラルの要素、match の被検査式、ガード、if let、let-else、範囲、ラベル付きブロックの break、可変の束縛、末尾で使われない束縛、matches! のパターン側)にある target の呼出は検証済みと数えず、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_call_in_a_position_that_is_not_listed_as_flowing_is_unknown() {
        for text in [
            "assert_eq!((double(2), 1).1, 1);",
            "assert_eq!((1, double(2)).0, 1);",
            "assert_eq!(Pair { a: double(2), b: 1 }.b, 1);",
            "assert_eq!([double(2), 1][1], 1);",
            "assert_eq!([double(2), 1][i], 1);",
            "assert_eq!({ let r = double(2); (r, 1).1 }, 1);",
            "assert!(match double(2) { _ => true });",
            "assert!(match c { _ if double(2) == 4 => true, _ => false });",
            "assert!(if let Some(x) = double(2) { x == 4 } else { false });",
            "assert!({ let Some(x) = double(2) else { return; }; x == 4 });",
            "assert_eq!('a: { break 'a double(2); }, 4);",
            "assert_eq!({ { double(2) }; 1 }, 1);",
            "assert_eq!({ let mut r = double(2); r.clear(); r }, 0);",
            "assert_eq!({ let r = double(2); { let r = 1; r } }, 1);",
            "assert_eq!({ let r = double(2); format!(\"{r}\") }, \"4\");",
            "assert!(matches!(x, Some(_) if double(2) == 4));",
            "assert!(xs.iter().any(|x| double(*x) == 4));",
            "assert_eq!(v, 0..double(2));",
            "assert!({ println!(\"{}\", double(2)); true });",
            "assert!(other(), \"{}\", double(2));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
        }
    }

    /// The projected element of a literal and the listed positions of a `let` name
    /// still verify the target.
    /// @vtest.id TEST-ORACLE-DA-003-LISTED-POSITION-IN-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、射影された集約リテラルの要素、射影されない集約リテラル、末尾の式で format! の引数や参照として使われる束縛、定数ブロックの演算子の被演算子にある target の呼出は検証済みと数えられ、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_call_in_a_listed_position_in_an_assert_stays_no_violation() {
        for text in [
            "assert_eq!((double(2), 1).0, 4);",
            "assert_eq!(Pair { a: double(2), b: 1 }.a, 4);",
            "assert_eq!([double(2), 1][0], 4);",
            "assert_eq!(xs[double(2)], 4);",
            "assert_eq!(&function[rust.double(2)], \"x\");",
            "assert_eq!((double(2), 1), (4, 1));",
            "assert_eq!(double(2)[0], 4);",
            "assert_eq!(double(2) && ok, true);",
            "assert_eq!({ let r = double(2); format!(\"{}\", r) }, \"4\");",
            "assert_eq!({ { double(2) } }, 4);",
            "assert_eq!(const { 1 } + double(2), 5);",
            "assert!(if double(2) > 0 { double(2) == 4 } else { double(3) == 6 });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-1948: an operand of `||` / `|` / `&&` / `&` that a literal absorbs is not a
    /// position the value flows from; beside any other operand it flows.
    /// @vtest.id TEST-ORACLE-DA-003-LITERAL-ABSORBED-OPERAND-DOES-NOT-FLOW
    /// @vtest.covers VO-ORACLE-DA-003-LITERAL-ABSORBED-OPERAND-NOT-FLOWING
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent X || true、true || X、X && false、false && X と | / & の同じ形、その否定、括弧や入れ子でリテラルに値が確定する兄弟の横にある target の呼出と束縛は DA-003 で検証済みと数えず UNKNOWN になり、X || false、X || ok、ok && X、(X || true) && Y の Y、整数の X | 0 のように吸収されない位置では違反なしのままであることを確認する
    #[test]
    fn an_operand_that_a_literal_absorbs_does_not_flow_and_any_other_operand_does() {
        for body in [
            "assert!(double(2) == 4 || true);",
            "assert!(true || double(2) == 4);",
            "assert!(double(2) == 4 && false);",
            "assert!(false && double(2) == 4);",
            "assert!((double(2) == 4) | true);",
            "assert!(true | (double(2) == 4));",
            "assert!((double(2) == 4) & false);",
            "assert!(false & (double(2) == 4));",
            "assert!(!(double(2) == 4 && false));",
            "assert!(double(2) == 4 || (true));",
            "assert!(double(2) == 4 || !false);",
            "assert!(double(2) == 4 || (true || ok));",
            "assert!((double(2) == 4 || true) && ok);",
            "assert!(ok && (double(2) == 4 || true));",
            "assert!((double(2) == 4 && false) || ok);",
            "assert_eq!(false && double(2) == 4, false);",
            "assert!(s.double(2) || true);",
            "let r = double(2);\n    assert!(r == 4 && false);",
            "let r = double(2);\n    assert!(r == 4 || true);",
            "let r = double(2) == 4 && false;\n    assert!(r);",
            "let r = (double(2) == 4) | true;\n    assert!(r);",
            "assert!({ let r = double(2); r == 4 || true });",
            "assert!(double(2) == 4 || 1 == 1);",
            "assert!(double(2) == 4 || (1 < 2));",
            "assert!(double(2) == 4 || \"a\" == \"a\");",
            "assert!(double(2) == 4 || true == true);",
            "assert!(double(2) == 4 || (true ^ false));",
            "assert!(double(2) == 4 && 1 > 2);",
            "assert!(double(2) == 4 || 1 + 1 == 2);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        for body in [
            "assert!(double(2) == 4 || false);",
            "assert!(false || double(2) == 4);",
            "assert!(double(2) == 4 && true);",
            "assert!((double(2) == 4) | false);",
            "assert!((double(2) == 4) & true);",
            "assert!(double(2) == 4 || ok);",
            "assert!(ok || double(2) == 4);",
            "assert!(ok && double(2) == 4);",
            "assert!((double(2) == 4) & ok);",
            "assert!(double(2) == 4 || (false && ok));",
            "assert!((ok || true) && double(2) == 4);",
            "assert!((double(3) == 6 || true) && double(2) == 4);",
            "assert_eq!(double(2) | 0, 4);",
            "assert_eq!(double(2) & 0, 0);",
            "assert_eq!(0 | double(2), 4);",
            "assert!(s.double(2) || false);",
            "let r = double(2);\n    assert!(r == 4 || ok);",
            "let r = double(2) == 4 || false;\n    assert!(r);",
            "assert!({ let r = double(2); r == 4 || false });",
            "assert!(double(2) == 4 || 1 == 2);",
            "assert!(double(2) == 4 && 1 < 2);",
            "assert!(double(2) == 4 || (true ^ true));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(!is_fail && !is_unknown, "{body}: {basis:?}");
        }
    }

    /// DS-1949: a logical expression whose value literals fix is a constant
    /// expression for DA-001; one that an operand can change is not.
    /// @vtest.id TEST-ORACLE-DA-001-LITERAL-FIXED-LOGIC-IS-CONSTANT
    /// @vtest.covers VO-ORACLE-DA-001-LITERAL-FIXED-LOGIC-IS-CONSTANT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::classify_constancy
    /// @vtest.intent X || true、true || X、X && false、false && X と | / & の同じ形、その否定、括弧や入れ子でリテラルに値が確定する論理式だけを引数に持つ assert 相当は DA-001 の定数アサーションになって合成結果が FAIL になり、X || false、X || ok、(X || true) && ok、(X || true) == true、整数の X | 0 や X & 0、メッセージ引数に識別子がある assert は定数アサーションにならず、matches! 以外のマクロを含む引数は定数アサーションにならずに FAIL を出さず、X || m!() は DA-001 と DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_logical_expression_that_literals_fix_is_a_constant_assertion() {
        for body in [
            "assert!(double(2) == 4 || true);",
            "assert!(true || double(2) == 4);",
            "assert!(double(2) == 4 && false);",
            "assert!(false && double(2) == 4);",
            "assert!((double(2) == 4) | true);",
            "assert!(true | (double(2) == 4));",
            "assert!((double(2) == 4) & false);",
            "assert!(false & (double(2) == 4));",
            "assert!(!(double(2) == 4 || true));",
            "assert!(!(false & (double(2) == 4)));",
            "assert!(!!(double(2) == 4 || true));",
            "assert!(double(2) == 4 || ((true)));",
            "assert!(double(2) == 4 || !false);",
            "assert!((double(2) == 4 && false) || true);",
            "assert!(ok || double(2) == 4 || true);",
            "assert!((double(2) == 4 || true) && (double(3) == 6 || true));",
            "assert_eq!(double(2) == 4 || true, true);",
            "assert_ne!(double(2) == 4 && false, true);",
            "assert!(double(2) == 4 || true, \"message\");",
            "assert!(double(2).is_some() || true);",
            "assert!(true && true);",
            "assert!(!false);",
            "double(2);\n    assert!(true == false || true);",
            "assert!(double(2) == 4 || true);\n    assert!(ok || true);",
            "assert!(double(2) == 4 || 1 == 1);",
            "assert!(double(2) == 4 || (1 < 2));",
            "assert!(double(2) == 4 || \"a\" == \"a\");",
            "assert!(double(2) == 4 || true == true);",
            "assert!(double(2) == 4 || (true ^ false));",
            "assert!(double(2) == 4 && 'a' > 'b');",
            "assert!((double(2) == 4 || true) == true);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(compose(&analysis).0, "{body}");
        }
        for body in [
            "assert!(double(2) == 4 || false);",
            "assert!(double(2) == 4 || ok);",
            "assert!((double(2) == 4 || true) && ok);",
            "assert!(double(2) == 4 || always_true());",
            "assert_eq!(double(2) | 0, 4);",
            "assert_eq!(double(2) & 0, 0);",
            "assert_eq!(double(2) == 4 || true, ok);",
            "assert!(double(2) == 4 || true, \"{}\", x);",
            "assert!(double(2) == 4 || true);\n    assert_eq!(double(2), 4);",
            "assert!(double(2) == 4 || 1 == 2);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_001
            );
        }
        // An operand made only of literals whose value is not evaluated may absorb
        // the rest: DA-001's escape (DS-621).
        // Float literals are not compared, as `f32` and `f64` give different answers.
        for body in [
            "assert!(double(2) == 4 || 1 + 1 == 2);",
            "assert!(double(2) == 4 || 0.1f32 == 0.100000001f32);",
            "assert!(double(2) == 4 && 0.1f32 != 0.100000001f32);",
            "assert!(double(2) == 4 || 0.5 < 1.5);",
            "assert!(double(2) == 4 || if true { true } else { false });",
            "assert!(double(2) == 4 || { true });",
            "assert!(double(2) == 4 || matches!(1, 1));",
            "assert!(double(2) == 4 && (1 as u8 == 2));",
            "assert!(double(2) == 4 || m!());",
            "assert!(m!() || double(2) == 4);",
            "assert!(double(2) == 4 || !m!());",
            "assert!(double(2) == 4 && env!(\"X\") == \"1\");",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Unknown(_))
                    && matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?} / {:?}",
                analysis.da_001,
                analysis.da_003
            );
        }
        // A macro other than `matches!` may expand to any code (DS-629), so an
        // argument holding one is not a constant expression.
        for body in [
            "macro_rules! double_m { ($x:expr) => { double($x) } }\n    assert_eq!(double_m!(2), 4);",
            "assert_eq!(compute!(2, 3), 5);",
            "assert!(check_double!());",
            "assert!(four_m!() == 4);",
            "assert_eq!(env!(\"X\"), \"1\");",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                !matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(!compose(&analysis).0, "{body}");
        }
    }

    /// DS-1965: a `matches!` whose value the pattern and the guard fix is a constant
    /// expression for DA-001; one that the value can change is not.
    /// @vtest.id TEST-ORACLE-DA-001-IRREFUTABLE-MATCHES-IS-CONSTANT
    /// @vtest.covers VO-ORACLE-DA-001-IRREFUTABLE-MATCHES-IS-CONSTANT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::matches_value
    /// @vtest.intent 反駁不能なパターンでガードが無いか定数で真の matches!、ガードが定数で偽の matches!、それらの否定と、それを X || や && false の吸収の形で含む引数だけを持つ assert 相当は DA-001 の定数アサーションになって合成結果が FAIL になり、Some(4) や 4 などの反駁可能なパターン、Some(_)、None、反駁可能な部分を持つ構造体やタプルのパターン、定数でないガード、反駁可能なパターンと定数で真のガードの組は定数アサーションにならず違反なしのままで、X || matches!(x, _) は X || true と同じく DA-003 で UNKNOWN になることを確認する
    #[test]
    fn a_matches_that_its_pattern_and_guard_fix_is_a_constant_assertion() {
        for body in [
            "assert!(matches!(double(2), _));",
            "assert!(matches!(double(2), _,));",
            "assert!(matches!(double(2), x @ _));",
            "assert!(matches!(double(2), (_, _)));",
            "assert!(matches!(double(2), &_));",
            "assert!(matches!(double(2), 4 | _));",
            "assert!(matches!(double(2), Some(_) | _));",
            "assert!(matches!(double(2), [..]));",
            "assert!(matches!(double(2), _ if true));",
            "assert!(matches!(double(2), x @ _ if 1 == 1));",
            "assert!(matches!(double(2), _ if double(2) == 4 || true));",
            "assert!(matches!(double(2), _ if false));",
            "assert!(matches!(double(2), Some(4) if false));",
            "assert!(matches!(double(2), x if x == 4 && false));",
            "assert!(!matches!(double(2), _));",
            "assert!(!matches!(double(2), Some(4) if false));",
            "assert!(!!matches!(double(2), _));",
            "assert!((matches!(double(2), _)));",
            "assert!(double(2) == 4 || matches!(double(2), _));",
            "assert!(matches!(double(2), _) || double(2) == 4);",
            "assert!(double(2) == 4 && matches!(double(2), _ if false));",
            "assert!(matches!(double(2), _) && matches!(double(3), _));",
            "assert_eq!(matches!(double(2), _), true);",
            "match ok { _ if matches!(ok, _) => {}, _ => panic!() }",
            "assert!(matches!(double(2), _), \"message\");",
            "assert!(matches!(double(2), x @ _), \"{}\", 1);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(compose(&analysis).0, "{body}");
        }
        for body in [
            "assert!(matches!(double(2), Some(4)));",
            "assert!(matches!(double(2), Some(_)));",
            "assert!(matches!(double(2), Ok(_)));",
            "assert!(matches!(double(2), 4));",
            "assert!(matches!(double(2), 4 | 5));",
            "assert!(matches!(double(2), 0..=9));",
            "assert!(matches!(double(2), None));",
            "assert!(matches!(double(2), Option::None));",
            "assert!(matches!(double(2), S { x: 4, .. }));",
            "assert!(matches!(double(2), (S { .. }, 4)));",
            "assert!(matches!(double(2), _ if ok));",
            "assert!(matches!(double(2), x if x == 4));",
            "assert!(matches!(double(2), Some(4) if true));",
            "assert!(matches!(double(2), Some(4) if ok));",
            "assert!(!matches!(double(2), Some(4)));",
            "assert!(double(2) == 4 || matches!(double(2), Some(_)));",
            "assert!(double(2) == 4 && matches!(double(2), _ if ok));",
            "assert!(matches!(double(2), _) && ok);",
            "assert!(matches!(double(2), S { .. }) && ok);",
            "assert_eq!(matches!(double(2), Some(4)), true);",
            "assert!(matches!(double(2), _));\n    assert!(matches!(double(2), Some(4)));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(!compose(&analysis).0, "{body}");
        }
        // DS-1951 and DS-1953 read the same classifier: an `assert!` it fixes to
        // `false` is a failure divergence that a result in its deciding position
        // verifies (DA-003), and a branch under a `matches!` it fixes is read by
        // value.
        for body in [
            "if double(2) != 4 { assert!(!matches!(ok, _)); }",
            "if double(2) != 4 { assert!(matches!(ok, _ if false)); }",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        for body in [
            "if matches!(ok, _ if false) { panic!(); }",
            "if matches!(ok, _ if true) { panic!(); }",
            "match double(2) { x if matches!(ok, _) => {}, _ => panic!() }",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{body}: {:?}",
                analysis.da_001
            );
        }
        for (body, no_violation) in [
            (
                "if matches!(ok, _ if false) { return; }\n    if double(2) != 4 { panic!(); }",
                true,
            ),
            (
                "if matches!(ok, _ if true) { return; }\n    if double(2) != 4 { panic!(); }",
                false,
            ),
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert_eq!(
                matches!(analysis.da_001, DaVerdict::NoViolation),
                no_violation,
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(!matches!(analysis.da_001, DaVerdict::Fail(_)), "{body}");
        }
        // The same absorption as `X || true` (DS-1948): the value does not flow.
        for body in [
            "assert!(double(2) == 4 || matches!(ok, _));",
            "assert!(double(2) == 4 || matches!(ok, (a, b)));",
            "assert!(double(2) == 4 || matches!(ok, &x));",
            "assert!(double(2) == 4 || matches!(ok, x @ y));",
            "assert!(double(2) == 4 || true);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-621, DS-1952, DS-1965: a `matches!` whose pattern only its type shows to be
    /// irrefutable is an expression whose constancy is not decided, so DA-001 escapes
    /// to `Unknown`; a non-constant assert-equivalent beside it keeps `NoViolation`.
    /// @vtest.id TEST-ORACLE-DA-001-TYPE-DECIDED-MATCHES-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-001-UNDETERMINED-CONSTANCY-UNKNOWN, VO-ORACLE-DA-001-IRREFUTABLE-MATCHES-IS-CONSTANT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_argument_constancy
    /// @vtest.intent 構文からは反駁不能とも反駁可能とも決まらないパターン（構造体 S { .. }、列挙子 Enum::V(_)、パス、単独の識別子、長さを決めるスライス）の matches! だけを assert 相当に持つ Test は、DA-001 が FAIL にも違反なしにもならず UNKNOWN になり、その Test に非定数の assert_eq!(double(2), 4) を足すと違反なしになることを確認する
    #[test]
    fn a_matches_whose_pattern_only_its_type_decides_is_unknown_for_da_001() {
        let undecided = [
            "assert!(matches!(double(2), S { .. }));",
            "assert!(matches!(double(2), Enum::V(_)));",
            "assert!(matches!(double(2), DaVerdict::Fail(_)));",
            "assert!(matches!(double(2), Shape::Circle(..)));",
            "assert!(matches!(double(2), Shape::Circle { .. }));",
            "assert!(matches!(double(2), Shape::Circle(..) if true));",
            "assert!(matches!(double(2), E::A));",
            "assert!(matches!(double(2), Kind));",
            "assert!(matches!(double(2), four));",
            "assert!(matches!(double(2), ref four));",
            "assert!(matches!(double(2), mut four));",
            "assert!(matches!(double(2), four if true));",
            "assert!(matches!(double(2), (Kind, _)));",
            "assert!(matches!(double(2), [_, _]));",
            "assert!(matches!(double(2), Some(_) | S { .. }));",
            "assert!(matches!(double(2), Some(_) | Kind));",
            "assert!(!matches!(double(2), four));",
            "assert!(!matches!(double(2), S { .. }));",
            "assert!(double(2) == 4 || matches!(double(2), S { .. }));",
            "assert_eq!(matches!(double(2), S { .. }), true);",
            "if !matches!(double(2), S { .. }) { panic!(); }",
            "assert!(matches!(double(2), _));\n    assert!(matches!(double(2), S { .. }));",
        ];
        for body in undecided {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_001
            );
        }
        for body in undecided {
            let text =
                format!("#[test]\nfn t() {{\n    {body}\n    assert_eq!(double(2), 4);\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_001
            );
        }
    }

    /// DS-1952, DS-1965: an `or` pattern below the top with an irrefutable
    /// alternative is irrefutable whatever the other alternative is, so its
    /// `matches!` is `true`.
    /// @vtest.id TEST-ORACLE-DA-001-NESTED-IRREFUTABLE-OR-IS-CONSTANT
    /// @vtest.covers VO-ORACLE-DA-001-IRREFUTABLE-MATCHES-IS-CONSTANT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::nested_pattern_class
    /// @vtest.intent パターンの内側にあり、反駁不能な選択肢を含む or（(S { .. } | _, _)、x @ (E::B(_) | _)、&(S { .. } | _)）の matches! だけを持つ assert は DA-001 が FAIL になり、X || の形で含む assert は X || true と同じく DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_nested_or_pattern_with_an_irrefutable_alternative_is_constant() {
        for pattern in [
            "(mk_s(double(2)), 0), (S { .. } | _, _)",
            "mk_e(double(2)), x @ (E::B(_) | _)",
            "&mk_s(double(2)), &(S { .. } | _)",
        ] {
            let text = format!("#[test]\nfn t() {{\n    assert!(matches!({pattern}));\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{text}: {:?}",
                analysis.da_001
            );
            let text = format!(
                "#[test]\nfn t() {{\n    assert!(double(2) == 4 || matches!({pattern}));\n}}\n"
            );
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A `let` outside the assert reaches it only where the name flows to the
    /// compared value.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-USE-DROPPED-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::binding_reach
    /// @vtest.intent let で束縛された target の結果の名前が、assert の引数の中でブロックの文、let _ =、クロージャの本体、match の被検査式にだけ現れるとき、比較される値へ届いたとは数えず DA-003 が UNKNOWN になり、比較される値として使われる同じ束縛は違反なしのままであることを確認する
    #[test]
    fn a_binding_used_where_its_value_is_dropped_in_an_assert_is_unknown() {
        for text in [
            "let r = double(2);\n    assert!({ let _ = r; true });",
            "let r = double(2);\n    assert!({ r; true });",
            "let r = double(2);\n    assert!((|| { r; true })());",
            "let r = double(2);\n    assert!(match r { _ => true });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
        }
        for text in [
            "let r = double(2);\n    assert_eq!(r, 4);",
            "let r = double(2);\n    assert!({ r == 4 });",
            "let r = double(2);\n    assert_eq!({ let t = r; t }, 4);",
            "let range = double(2);\n    assert_eq!(&function[range], \"x\");",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A `let` outside the assert is derived from the target's result only where the
    /// target's value can flow to the initializer's value; a value dropped inside the
    /// initializer (a block statement, an operand a literal absorbs, a `match` scrutinee, a
    /// projection that leaves it out, a closure body) leaves the binding underived.
    /// @vtest.id TEST-ORACLE-DA-003-LET-INITIALIZER-DROPPING-CALL-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::find_let_bindings_containing_call
    /// @vtest.intent let の初期化式の中で target の値が捨てられる位置(ブロックの文、リテラルに吸収される || の被演算子、match の被検査式、射影で残らない要素、入れ子の射影、vec! の添字で残らない要素、クロージャの本体)にあるとき、その束縛は target の結果から派生した束縛と数えず、束縛が assert に届いても DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_let_initializer_that_drops_the_call_is_unknown_not_pass() {
        for text in [
            "let r = { double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = (double(2), 1).1;\n    assert_eq!(r, 1);",
            "let r = double(2) == 4 || true;\n    assert!(r);",
            "let r = match double(2) { _ => 1 };\n    assert_eq!(r, 1);",
            "let f = |x| { double(x); 1 };\n    assert_eq!(f(2), 1);",
            // DS-1944: a closure bound with `mut` is not tracked (DS-1941 tracks the
            // same closure bound without it).
            "let mut f = |x| double(x);\n    assert_eq!(f(2), 4);",
            "let r = ((double(2), 1), 2).0.1;\n    assert_eq!(r, 1);",
            "let r = vec![double(2), 1][1];\n    assert_eq!(r, 1);",
            "let r = [double(2), 1][1];\n    assert_eq!(r, 1);",
            "let r = Pair { a: double(2), b: 1 }.b;\n    assert_eq!(r, 1);",
            "let r = { let _ = double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = { let x = double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = if c { { double(2); 1 } } else { 0 };\n    assert_eq!(r, 1);",
            "let r = s.double(2).is_some() || true;\n    assert!(r);",
            "let r = { s.double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = xs.iter().map(|x| double(*x)).count();\n    assert_eq!(r, 1);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A closure body and a nested projection inside an assert's checked argument
    /// do not carry the call's value (DS-629; a projection selects one element).
    /// @vtest.id TEST-ORACLE-DA-003-CLOSURE-AND-NESTED-PROJECTION-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、インラインのクロージャの本体、入れ子の射影で残らない要素、vec! の添字で残らない要素にある target の呼出は検証済みと数えず、DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_closure_or_nested_projection_inside_an_assert_argument_is_unknown() {
        for text in [
            "assert_eq!((|x| double(x))(2), 4);",
            "assert_eq!(((double(2), 1), 2).0.1, 1);",
            "assert_eq!(vec![double(2), 1][1], 1);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
        }
    }

    /// A `let` initializer in which the target's value flows to the initializer's
    /// value still derives the binding from the result.
    /// @vtest.id TEST-ORACLE-DA-003-LET-INITIALIZER-FLOWING-CALL-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::find_let_bindings_containing_call
    /// @vtest.intent let の初期化式の中で target の値が初期化式の値へ流れる位置(式そのもの、二項演算、メソッドチェーン、? と unwrap、失敗として発散する腕を除くすべての腕が派生である if と match の分岐の末尾、ブロックの末尾、射影された要素、構築子の引数)にあるとき、束縛は結果から派生したものと数えられ、assert に届けば DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_let_initializer_that_carries_the_call_stays_no_violation() {
        for text in [
            "let r = double(2) + 1;\n    assert_eq!(r, 5);",
            "let r = c.double(2).unwrap();\n    assert_eq!(r, 4);",
            "let r = if c { double(2) } else { double(3) };\n    assert_eq!(r, 4);",
            "let r = match c { 0 => unreachable!(), _ => double(2) };\n    assert_eq!(r, 4);",
            "let r = { let q = double(2); q };\n    assert_eq!(r, 4);",
            "let r = { double(1); double(2) };\n    assert_eq!(r, 4);",
            "let r = (double(2), 1).0;\n    assert_eq!(r, 4);",
            "let r = ((double(2), 1), 2).0.0;\n    assert_eq!(r, 4);",
            "let r = ((1, double(2)), 2).0.1;\n    assert_eq!(r, 4);",
            "let r = Pair { a: double(2), b: 1 }.a;\n    assert_eq!(r, 4);",
            "let r = [double(2), 1][0];\n    assert_eq!(r, 4);",
            "let r = Some(double(2));\n    assert_eq!(r, Some(4));",
            "let r = s.double(2)?;\n    assert_eq!(r, 4);",
            "let r = &double(2);\n    assert_eq!(r, &4);",
            "let r = double(2) as u8;\n    assert_eq!(r, 4);",
            "let r = Calc::double(2);\n    assert_eq!(r, 4);",
            "let mut r = double(2);\n    assert_eq!(r, 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-1960: a call that is not a value conversion takes the result handed to
    /// its argument as a delegation target (DS-641), whose termination this
    /// capability does not confirm, so DA-003 is UNKNOWN (DS-652) when no result
    /// reaches an assert-equivalent directly.
    /// @vtest.id TEST-ORACLE-DA-003-RESULT-INTO-DELEGATION-TARGET-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS, VO-ORACLE-DA-003-DELEGATION-NOT-FAIL
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent target の結果が、assert の検査される引数またはそこへ届く let 束縛の中で、値の変換でない関数呼出(利用者の関数、クロージャ、タプル構造体、T::new、PathBuf::from)の引数かメソッド呼出の引数へ流れ、assert へ直接届く結果が 1 つもない Test は、DA-003 が委譲(DS-1960)を理由に UNKNOWN になり、合成結果が PASS にならないことを確認する
    #[test]
    fn a_result_flowing_into_a_delegation_target_is_unknown() {
        for body in [
            "assert_eq!(always_3(double(2)), 3);",
            "assert!(check(double(2)));",
            "assert!(std::mem::drop(double(2)) == ());",
            "assert_eq!(identity(double(2)), 4);",
            "assert!((|x: i32| x == 4)(double(2)));",
            "assert_eq!(Wrapper(double(2)), Wrapper(4));",
            "assert_eq!(T::new(double(2)), T::new(4));",
            "assert_eq!(PathBuf::from(double(2)), p);",
            "assert!(v.contains(&double(2)));",
            "assert_eq!(Some(always_3(double(2))), Some(3));",
            "assert_eq!(always_3(double(2)).abs(), 3);",
            "assert!(matches!(always_3(double(2)), 3));",
            "let r = double(2);\n    assert_eq!(always_3(r), 3);",
            "let r = double(2);\n    assert!(v.contains(&r));",
            "let r = always_3(double(2));\n    assert_eq!(r, 3);",
            "assert_eq!({ let r = double(2); check(r) }, true);",
            "let r = double(2);\n    assert!(check(r));\n    assert!(check(double(3)));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(!is_fail && is_unknown, "{body}: not UNKNOWN: {basis:?}");
            let DaVerdict::Unknown(reason) = &analysis.da_003 else {
                panic!("{body}: {:?}", analysis.da_003);
            };
            assert!(reason.contains("DS-1960"), "{body}: {reason}");
        }
        // A call in a method's turbofish is not carried to the method's value.
        for body in [
            "assert!({ x.f::<{ double(2) }>(); true });",
            "assert!(x.f::<{ double(2) }>() || true);",
            "assert!(f::<{ double(2) }>(1) == 1 || true);",
            "assert!({ f::<{ double(2) }>(); true });",
            "assert!({ (pick(double(2)))(); true });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        // The arguments of a declared target's call are not a value conversion
        // either (DS-1960), so the inner target's result is not carried there; it
        // is not a delegation target (DS-1960) either.
        let text = "#[test]\nfn t() {\n    assert_eq!(half(double(2)), 2);\n}\n";
        let analysis = analyze(text, &["double".to_owned(), "half".to_owned()], &[]);
        let DaVerdict::Unknown(reason) = &analysis.da_003 else {
            panic!("{:?}", analysis.da_003);
        };
        assert!(!reason.contains("DS-1960"), "{reason}");
    }

    /// A name handed to a delegation target, or to a declared target's call, is
    /// not shown to mean the binding when it is bound again or changed in place,
    /// so a verified method call beside it does not clear DA-003 (DS-1680).
    /// @vtest.id TEST-ORACLE-DA-003-DELEGATED-BINDING-REBOUND-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::binding_reach
    /// @vtest.intent 委譲先か宣言 target の引数に渡る束縛の名前が再束縛や可変変更を受ける Test は、別の呼出が照合されていても DA-003 が違反なしにならず、宣言 target の引数に入るだけの束縛は、その結果が直接照合されないことを示す理由で UNKNOWN になることを確認する
    #[test]
    fn a_delegated_binding_that_is_bound_again_stays_unknown() {
        for body in [
            "let r = c.m(2);\n    assert!(check(r));\n    let r = 0;\n    c.m(3).unwrap();",
            "let mut r = c.m(2);\n    r.push(1);\n    assert!(check(r));\n    c.m(3).unwrap();",
            "let r = c.m(2);\n    assert!(v.contains(&r));\n    for r in xs {}\n    c.m(3)?;",
            "let r = c.m(2);\n    assert!(check(r));\n    let s = c.m(3);\n    assert_eq!(s, 1);\n    let r = 1;",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["m".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(!is_fail && is_unknown, "{body}: not UNKNOWN: {basis:?}");
        }
        let text = "#[test]\nfn t() {\n    let yaml = to_yaml(&r);\n    assert_eq!(from_yaml(&yaml), r);\n}\n";
        let analysis = analyze(text, &["to_yaml".to_owned(), "from_yaml".to_owned()], &[]);
        let DaVerdict::Unknown(reason) = &analysis.da_003 else {
            panic!("{:?}", analysis.da_003);
        };
        assert!(
            reason.contains("only as an argument of another declared target"),
            "{reason}"
        );
    }

    /// The other side of DS-1960: a value conversion and a method's receiver
    /// carry the result, and a call that the result does not flow into, or that
    /// sits in a message argument (DS-1935), is not read as a delegation target.
    /// @vtest.id TEST-ORACLE-DA-003-VALUE-CONVERSION-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS, VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::is_value_conversion
    /// @vtest.intent target の結果が Some / Ok / Err、Box::new、String::from の引数、メソッドの受け手、検証されない位置の外の関数呼出を通って assert に届く Test、結果が流れ込まない関数呼出やメッセージ引数の関数呼出だけがある Test、assert へ直接届く結果が 1 つ以上あり別の結果が委譲先へ渡る Test は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_result_through_a_value_conversion_or_a_receiver_stays_no_violation() {
        for body in [
            "assert_eq!(Some(double(2)), Some(4));",
            "assert_eq!(Ok::<_, ()>(double(2)), Ok(4));",
            "assert_eq!(Err::<(), _>(double(2)), Err(4));",
            "assert_eq!(Option::Some(double(2)), Some(4));",
            "assert_eq!(Box::new(double(2)), Box::new(4));",
            "assert_eq!(std::boxed::Box::new(double(2)), b);",
            "assert_eq!(String::from(double(2)), s);",
            "assert_eq!(Some(Box::new(double(2))), b);",
            "assert!(double(2).contains(&x));",
            "assert_eq!(double(2).get(1), Some(&4));",
            "assert_eq!(double(2), expected());",
            "assert_eq!(double(2), 4, \"{}\", describe(x));",
            "assert_eq!(Some(double(2)).unwrap(), 4);",
            "let r = double(2);\n    assert!(r.contains(&x));",
            "let r = Some(double(2));\n    assert_eq!(r, Some(4));",
            "let r = Box::new(double(2));\n    assert_eq!(*r, 4);",
            "let r = double(2);\n    log(r);\n    assert_eq!(r, 4);",
            // The inner result is not carried into the target's own call (DS-1960),
            // but the outer result reaches the assert, which is enough (DS-1962).
            "assert_eq!(double(double(2)), 8);",
            // One result that reaches an assert-equivalent directly decides DA-003;
            // another result handed to a delegation target does not make it UNKNOWN
            // (derived from DS-623 and DS-1962).
            "let r = (double(2), check(double(3)));\n    assert_eq!(r.0, 4);",
            "let r = double(2);\n    assert_eq!(r, 4);\n    assert!(check(double(3)));",
            "assert!(check(double(3)));\n    let r = double(2);\n    assert_eq!(r, 4);",
            "assert_eq!(double(2), 4);\n    assert!(check(double(3)));",
            "assert!(check(double(3)));\n    assert_eq!(double(2), 4);",
            "assert!({ let r = double(2); r == 4 && check(r) });",
            "let r = double(2);\n    assert_eq!(r, 4);\n    assert!(check(r));",
            "let r = double(2);\n    assert!(r == 4 && check(r));",
            "assert_eq!(double(2).max(always_3(double(3))), 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-1935: a message argument (of an assert macro, and of DS-619's
    /// `.expect(..)` / `.expect_err(..)`) is not a position the value flows
    /// through, so a call there neither verifies the target nor is a delegation
    /// target (DS-641 names only calls outside the assert-equivalents).
    /// @vtest.id TEST-ORACLE-DA-003-MESSAGE-ARGUMENT-IS-NOT-A-FLOWING-POSITION
    /// @vtest.covers VO-ORACLE-DA-003-MESSAGE-ARGUMENT-NOT-FLOWING
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent target の結果が assert のメッセージ引数、または expect / expect_err の引数にだけ現れる Test は DA-003 が違反なしにならず、検査される引数で結果が届く Test はメッセージ引数の関数呼出があっても違反なしのままであることを確認する
    #[test]
    fn a_message_argument_is_not_a_flowing_position() {
        for body in [
            "assert!(other() > 0, \"{}\", double(2));",
            "assert!(true, \"{}\", double(2));",
            "let r = double(2);\n    assert!(other() > 0, \"{}\", r);",
            "assert_eq!(x.expect(&describe(double(2))), 4);",
            "let r = double(2);\n    assert_eq!(x.expect(&describe(r)), 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (_, _, basis) = compose(&analysis);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?} {basis:?}",
                analysis.da_003
            );
        }
        for body in [
            "assert_eq!(double(2), 4, \"{}\", describe(double(3)));",
            "let r = double(2);\n    assert_eq!(r, 4, \"{}\", describe(r));",
            "let r = double(2);\n    assert_eq!(r.checked().expect(&describe(r)), 4);",
            "assert_eq!(double(2).expect_err(&describe(x)), e);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// The per-target DA-003 of `audit_rules` excludes every declared target from
    /// the delegation targets, as `analyze` does.
    /// @vtest.id TEST-ORACLE-DA-003-PER-TARGET-AGREES-WITH-ANALYZE
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::analyze_da_003_for
    /// @vtest.intent 宣言 target が 2 つある Test で、一方の結果が他方の target の呼出の引数に渡る形を、target ごとの DA-003 が委譲と読まず、analyze の DA-003 と同じ判定になることを確認する
    #[test]
    fn the_per_target_da_003_does_not_read_another_target_as_a_delegation_target() {
        let targets = ["a".to_owned(), "b".to_owned()];
        let text = "#[test]\nfn t() {\n    assert_eq!(a(1), 2);\n    assert!(b(a(3)) == 4);\n}\n";
        assert_eq!(analyze(text, &targets, &[]).da_003, DaVerdict::NoViolation);
        for symbol in &targets {
            assert_eq!(
                analyze_da_003_for(text, symbol, &targets, &[]),
                DaVerdict::NoViolation,
                "{symbol}"
            );
        }
    }

    /// DS-1964: `.unwrap()` / `.expect(..)` / `?` on a value whose variant is
    /// `Some` / `Ok` by syntax, and `.unwrap_err()` / `.expect_err(..)` on one that
    /// is `Err`, cannot fail, so they are not assert-equivalents: a Test whose only
    /// verification is one of them does not get DA-006's no violation.
    /// @vtest.id TEST-ORACLE-DA-006-INFALLIBLE-UNWRAP-IS-NOT-VERIFICATION
    /// @vtest.covers VO-ORACLE-INFALLIBLE-UNWRAP-NOT-ASSERT-EQUIVALENT, VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::VerificationSyntaxVisitor
    /// @vtest.intent 受け手の列挙子が構文から Some / Ok と決まる値(構築式と、それを一度だけ束縛した変数)への unwrap / expect / ?、Err と決まる値への unwrap_err / expect_err だけを検証構文に持つ Test は、DA-006 が違反なしにならず合成結果が PASS にならないことを確認する
    #[test]
    fn an_unwrap_that_cannot_fail_is_not_verification_syntax() {
        for text in [
            "#[test]\nfn t() {\n    Some(double(2)).unwrap();\n}\n",
            "#[test]\nfn t() {\n    Some(double(2)).expect(\"x\");\n}\n",
            "#[test]\nfn t() {\n    (Ok::<_, ()>(double(2))).unwrap();\n}\n",
            "#[test]\nfn t() {\n    Err::<(), _>(double(2)).unwrap_err();\n}\n",
            "#[test]\nfn t() {\n    Err::<(), _>(double(2)).expect_err(\"x\");\n}\n",
            "#[test]\nfn t() {\n    let o = Some(double(2));\n    o.unwrap();\n}\n",
            "#[test]\nfn t() {\n    let o: Option<i32> = Some(double(2));\n    o.expect(\"x\");\n}\n",
            "#[test]\nfn t() -> Option<()> {\n    Some(double(2))?;\n    Some(())\n}\n",
            "#[test]\nfn t() -> Option<()> {\n    let o = Some(double(2));\n    o?;\n    Some(())\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                !matches!(analysis.da_006, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_006
            );
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
        }
    }

    /// The other side of DS-1964: a receiver whose variant is not fixed by syntax
    /// (a call's result, a branch, a `let mut` or a name bound again), or one whose
    /// unwrap can fail (`Ok(..).unwrap_err()`, `Err(..).unwrap()`), stays an
    /// assert-equivalent (DS-619).
    /// @vtest.id TEST-ORACLE-DA-006-FALLIBLE-UNWRAP-STAYS-VERIFICATION
    /// @vtest.covers VO-ORACLE-INFALLIBLE-UNWRAP-NOT-ASSERT-EQUIVALENT, VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::VerificationSyntaxVisitor
    /// @vtest.intent 受け手の列挙子が構文から決まらない値(呼出の結果、分岐、let mut、同じ名前の再束縛)、または失敗しうる組(Ok への unwrap_err、Err への unwrap)への unwrap / expect / ? は assert 相当のままで、DA-006 が違反なしのままであることを確認する
    #[test]
    fn an_unwrap_that_can_fail_stays_verification_syntax() {
        for body in [
            "double(2).unwrap();",
            "let o = double(2);\n    o.unwrap();",
            "let o = if c { Some(double(2)) } else { None };\n    o.unwrap();",
            "let mut o = Some(double(2));\n    o = None;\n    o.unwrap();",
            "let o = Some(double(2));\n    let o = other();\n    o.unwrap();",
            "Ok::<_, ()>(double(2)).unwrap_err();",
            "Err::<(), _>(double(2)).unwrap();",
            // The narrowed reach of DS-1964 disclosed in the module comment.
            "let mut o = Some(double(2));\n    o.unwrap();",
            "let (o, _) = (Some(double(2)), 1);\n    o.unwrap();",
            "let o = Some(double(2));\n    let o = o;\n    o.unwrap();",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_006, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_006
            );
        }
    }

    /// A SrcId target has no call symbol, so its call cannot be found: the
    /// static-analysis verdict is UNKNOWN, not the PASS the other rules compose to.
    /// @vtest.id TEST-ORACLE-DA-003-SRC-ID-TARGET-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::RustCargoStaticAnalysisAdapter
    /// @vtest.intent SrcId で宣言された target は呼出箇所を同定できないため、他の規則が違反なしでも静的解析の判定が PASS ではなく UNKNOWN になり、Locator の target だけの同じ本体は PASS のままであることを確認する
    #[test]
    fn a_src_id_target_makes_the_static_analysis_verdict_unknown_not_pass() {
        use vtest_model::{
            AdapterId, ContentHash, ExecutionDescriptor, ProjectPath, SourceLocation, SourceRange,
            SrcId, TestEntity, TestId,
        };

        let construct = "#[test]\nfn t() {\n    assert_eq!(double(2), 4);\n}\n";
        let hash = ContentHash::from_text(construct);
        let entity = |targets: Vec<TargetRef>| TestEntity {
            id: TestId::new("TEST-X"),
            covers: Vec::new(),
            targets,
            intent: "x".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: SourceLocation {
                adapter: AdapterId::new("rust-cargo"),
                path: ProjectPath("tests/x.rs".to_owned()),
                locator: "x".to_owned(),
                byte_range: SourceRange { start: 0, end: 1 },
            },
            content_hash: hash.clone(),
            execution: ExecutionDescriptor {
                adapter: AdapterId::new("rust-cargo"),
                project: None,
                suite: None,
                selector: "x".to_owned(),
            },
        };
        let verdict = |targets: Vec<TargetRef>| {
            let test = entity(targets);
            RustCargoStaticAnalysisAdapter::new().analyze(StaticAnalysisInput {
                test: &test,
                construct_text: construct,
                file_text: Some(construct),
                content_hash: &hash,
                extra_assertion_macros: &[],
            })
        };
        let locator_target = TargetRef::Locator(locator("src/lib.rs::double"));
        assert_eq!(verdict(vec![locator_target.clone()]).variant(), "PASS");
        let src_id_target = TargetRef::SrcId(SrcId::new("SRC-DOUBLE"));
        assert_eq!(verdict(vec![src_id_target.clone()]).variant(), "UNKNOWN");
        assert_eq!(
            verdict(vec![locator_target, src_id_target]).variant(),
            "UNKNOWN"
        );
    }

    /// @vtest.id TEST-ORACLE-DA-004-IDENTICAL-ARGUMENTS-FAILS
    /// @vtest.covers VO-ORACLE-DA-004-SELF-COMPARISON
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_004_self_comparison
    /// @vtest.intent assert_eq!の両引数がトークン列として同一のときDA-004がFAILになることを確認する
    #[test]
    fn identical_assert_eq_arguments_fail_da_004() {
        let text = "#[test]\nfn tautology() {\n    assert_eq!(compute(), compute());\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_004.outcome(), RuleOutcome::Fail);
    }

    /// @vtest.id TEST-ORACLE-DA-001-ALL-LITERAL-FAILS
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent assertの引数がすべてリテラルのときDA-001がFAILになることを確認する
    #[test]
    fn an_all_literal_assert_fails_da_001() {
        let text = "#[test]\nfn tautology() {\n    assert_eq!(1, 1);\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_001.outcome(), RuleOutcome::Fail);
    }

    /// @vtest.id TEST-ORACLE-LITERAL-FIXTURE-IS-NOT-CODE
    /// @vtest.covers VO-ORACLE-DA-004-SELF-COMPARISON
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::analyze
    /// @vtest.intent fixture source embedded in a string literal is not analyzed as executable Rust
    #[test]
    fn assert_macros_inside_fixture_strings_are_ignored() {
        let text = r##"#[test]
fn real_check() {
    let fixture = "assert_eq!(1, 1);";
    assert!(fixture.len() > 0);
}
"##;
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_001, DaVerdict::NoViolation);
        assert_eq!(analysis.da_004, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DA-001-CALL-RESULT-PASSES
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent assertが識別子（関数呼出結果）を参照するときDA-001が違反なしになることを確認する
    #[test]
    fn an_assert_referencing_a_call_result_does_not_fail_da_001() {
        let text = "#[test]\nfn real_check() {\n    assert_eq!(double(2), 4);\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_001, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DA-001-NON-ASCII-LINE
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::span_offset
    /// @vtest.intent an assert that follows non-ASCII text on its line, with LF or CRLF line ends, is read at its own byte range: a call-result assert is no violation and an all-literal assert still fails DA-001 (#264)
    #[test]
    fn an_assert_after_non_ascii_text_on_its_line_is_read_at_its_own_range() {
        for newline in ["\n", "\r\n"] {
            let real = format!(
                "#[test]{newline}fn real_check() {{{newline}    let label = \"日本語é\"; assert_eq!(double(2), 4, \"é\");{newline}}}{newline}"
            );
            let analysis = analyze(&real, &["double".to_owned()], &[]);
            assert_eq!(analysis.da_001, DaVerdict::NoViolation, "{real}");
            assert_eq!(analysis.da_003, DaVerdict::NoViolation, "{real}");

            let constant = format!(
                "#[test]{newline}fn constant() {{{newline}    let label = \"日本語é\"; assert_eq!(1, 1);{newline}}}{newline}"
            );
            let analysis = analyze(&constant, &[], &[]);
            assert_eq!(analysis.da_001.outcome(), RuleOutcome::Fail, "{constant}");
        }
    }

    /// @vtest.id TEST-ORACLE-DA-006-RESULT-RETURNING-SIGNATURE-PASSES
    /// @vtest.covers VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent Resultを返すTest関数シグネチャが検証構文として認められDA-006が違反なしになることを確認する
    #[test]
    fn a_result_returning_test_signature_counts_as_verification_syntax() {
        let text =
            "#[test]\nfn it_parses() -> Result<(), String> {\n    let _ = parse(\"x\")?;\n    Ok(())\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert_eq!(analysis.da_006, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-ASSERT-MACRO-CLOSED-LIST
    /// @vtest.covers VO-ORACLE-ASSERT-MACRO-CLOSED-LIST
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent unreachable!・todo!・unimplemented! を含む本体は DA-006 が違反なしになり、DS-617 の列挙と config の assertion_macros にないマクロ（assert_matches!、debug_assert!）だけの本体は違反なしにならないことを確認する
    #[test]
    fn only_the_closed_list_of_macros_is_verification_syntax() {
        for name in ["unreachable", "todo", "unimplemented"] {
            let text = format!(
                "#[test]\nfn t() {{\n    let x = compute();\n    if x != 4 {{\n        {name}!();\n    }}\n}}\n"
            );
            let analysis = analyze(&text, &[], &[]);
            assert_eq!(analysis.da_006, DaVerdict::NoViolation, "{text}");
        }
        let custom = "#[test]\nfn t() {\n    let x = 4;\n    assert_matches!(x, 4);\n}\n";
        assert_eq!(
            analyze(custom, &[], &[]).da_006.outcome(),
            RuleOutcome::Fail
        );
        assert_eq!(
            analyze(custom, &[], &["assert_matches!".to_owned()]).da_006,
            DaVerdict::NoViolation
        );
        let debug = "#[test]\nfn t() {\n    let x = 4;\n    debug_assert!(x == 4);\n}\n";
        assert_ne!(analyze(debug, &[], &[]).da_006, DaVerdict::NoViolation);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-ARGUMENT-METHOD-CALL-IS-A-CHANGE
    /// @vtest.covers VO-ORACLE-ASSERT-MACRO-CLOSED-LIST, VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent let mut の束縛を unreachable! などのメッセージ引数で読むだけなら assert_eq! への到達を保って DA-003 が違反なしになり、メソッドの受け手にすると UNKNOWN になることを確認する
    #[test]
    fn a_divergence_reading_a_mut_binding_keeps_it_but_a_method_call_does_not() {
        for name in ["unreachable", "todo", "unimplemented"] {
            let body = |argument: &str| {
                format!(
                    "#[test]\nfn t() {{\n    let mut r = double(2);\n    if c() {{\n        {name}!(\"{{}}\", {argument});\n    }}\n    assert_eq!(r, 4);\n}}\n"
                )
            };
            let read = body("r");
            assert_eq!(
                analyze(&read, &["double".to_owned()], &[]).da_003,
                DaVerdict::NoViolation,
                "{read}"
            );
            let changed = body("r.pow(2)");
            assert_eq!(
                analyze(&changed, &["double".to_owned()], &[])
                    .da_003
                    .outcome(),
                RuleOutcome::Unknown,
                "{changed}"
            );
        }
    }

    /// @vtest.id TEST-ORACLE-CHANGE-INSIDE-DIVERGENCE-ARGUMENTS-IS-A-CHANGE
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::place_changes
    /// @vtest.intent panic! / unreachable! / todo! / unimplemented! の引数の中で let mut の束縛を代入・複合代入・クロージャ内の代入・&mut・メソッドの受け手・読まないマクロで変えると DA-003 が UNKNOWN になり、読むだけなら違反なしのままであることを確認する
    #[test]
    fn a_change_inside_a_divergence_argument_keeps_da_003_unknown() {
        let wrap = |inner: &str| {
            format!(
                "#[test]\nfn t() {{\n    let mut r = double(2);\n{inner}    assert_eq!(r, 4);\n}}\n"
            )
        };
        for name in ["panic", "unreachable", "todo", "unimplemented"] {
            for inner in [
                format!("    loop {{\n        {name}!(\"{{:?}}\", {{ r = 4; break; }});\n    }}\n"),
                format!(
                    "    for _ in 0..1 {{\n        {name}!(\"{{:?}}\", {{ r = 4; continue; }});\n    }}\n"
                ),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", r += 0);\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", (|| r = 4)());\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", {{ let p = &mut r; *p = 4; }});\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{}}\", r.pow(2));\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", reset!(r));\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", r +);\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", format!(\"{{}}\", {{ r = 4; r }}));\n    }}\n"),
            ] {
                let text = wrap(&inner);
                assert_eq!(
                    analyze(&text, &["double".to_owned()], &[])
                        .da_003
                        .outcome(),
                    RuleOutcome::Unknown,
                    "{text}"
                );
            }
            let read = wrap(&format!(
                "    if c() {{\n        {name}!(\"{{}}\", r);\n    }}\n"
            ));
            assert_eq!(
                analyze(&read, &["double".to_owned()], &[]).da_003,
                DaVerdict::NoViolation,
                "{read}"
            );
        }
        // The plain name assigned inside an assert's argument is a change too.
        let in_assert = wrap("    assert!({ r = 4; true });\n");
        assert_eq!(
            analyze(&in_assert, &["double".to_owned()], &[])
                .da_003
                .outcome(),
            RuleOutcome::Unknown
        );
    }

    /// @vtest.id TEST-ORACLE-DA-003-FAIL-OF-ONE-TARGET-DECIDES
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 宣言 target が複数あり、1 つの結果が debug_assert にだけ流れて UNKNOWN、もう 1 つの結果がどこにも届かず FAIL のとき、DA-003 が FAIL になり、名前でしか照合できないメソッド呼出があるときは UNKNOWN のままであることを確認する
    #[test]
    fn a_fail_of_one_target_decides_over_another_targets_unknown() {
        let targets = ["double".to_owned(), "triple".to_owned()];
        let text = "#[test]\nfn t() {\n    let a = double(2);\n    let b = triple(2);\n    debug_assert_eq!(a, 4);\n}\n";
        assert_eq!(
            analyze(text, &targets, &[]).da_003.outcome(),
            RuleOutcome::Fail
        );
        let targets_with_check = ["double".to_owned(), "check".to_owned()];
        for (targets, body) in [
            (
                &targets_with_check,
                "    let a = double(2);\n    v.check(a);\n",
            ),
            (
                &targets_with_check,
                "    let a = double(2);\n    v.check(&a);\n",
            ),
            (
                &targets,
                "    let a = double(2);\n    let mut b = 0;\n    x.triple(&mut b, a);\n",
            ),
            (
                &targets,
                "    let a = double(2);\n    let b = x.triple(2);\n",
            ),
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            assert_eq!(
                analyze(&text, targets, &[]).da_003.outcome(),
                RuleOutcome::Unknown,
                "{text}"
            );
        }
        let one = "#[test]\nfn t() {\n    let a = double(2);\n    debug_assert_eq!(a, 4);\n}\n";
        assert_eq!(
            analyze(one, &targets, &[]).da_003.outcome(),
            RuleOutcome::Unknown
        );
    }

    /// @vtest.id TEST-ORACLE-DA-001-UNCONDITIONAL-DIVERGENCE-IS-CONSTANT
    /// @vtest.covers VO-ORACLE-DIVERGENCE-CONSTANCY-BY-DECIDING-POSITION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent 無条件に実行される unreachable!・todo!・unimplemented! は、メッセージ引数に識別子があっても DA-001 の定数アサーションとして FAIL になり、#[should_panic] があっても合成結果が PASS にならないことを確認する
    #[test]
    fn an_unconditional_divergence_is_a_constant_assertion_whatever_its_message() {
        for name in ["unreachable", "todo", "unimplemented"] {
            for body in [
                format!("    {name}!();\n"),
                format!("    let x = double(0);\n    {name}!(\"{{}}\", x);\n"),
                format!("    println!(\"{{:?}}\", 1);\n    {name}!(\"{{}}\", y)\n"),
            ] {
                let text = format!("#[should_panic]\n#[test]\nfn t() {{\n{body}}}\n");
                let analysis = analyze(&text, &["double".to_owned()], &[]);
                assert!(matches!(analysis.da_001, DaVerdict::Fail(_)), "{text}");
                assert!(compose(&analysis).0, "{text}");
            }
        }
    }

    /// The composed oracle_presence state of a Test whose body is `body`, with the
    /// targets `double`, `double_opt` and `kind`.
    fn divergence_state(head: &str, signature: &str, body: &str) -> &'static str {
        let text = format!("{head}#[test]\nfn t() {signature}{{\n{body}}}\n");
        let targets = [
            "double".to_owned(),
            "double_opt".to_owned(),
            "double_res".to_owned(),
            "kind".to_owned(),
        ];
        let (fail, unknown, _) = compose(&analyze(&text, &targets, &[]));
        if fail {
            "FAIL"
        } else if unknown {
            "UNKNOWN"
        } else {
            "PASS"
        }
    }

    fn assert_divergence_states(cases: &[(&str, &str, &str, &str)]) {
        for (expected, head, signature, body) in cases {
            assert_eq!(
                divergence_state(head, signature, body),
                *expected,
                "{head}fn t() {signature}{{\n{body}}}"
            );
        }
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-DECIDING-POSITION-REACH
    /// @vtest.covers VO-ORACLE-DIVERGENCE-DECIDING-POSITION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::target_call_result_is_verified
    /// @vtest.intent target の結果が、失敗として発散する構文の実行を決める位置（条件式・ガード・scrutinee のパターン検査・while の条件・for の反復対象）に流れれば PASS になり、決める位置に派生でない値しか無いか、結果がメッセージ引数にしか流れなければ PASS にならないことを確認する
    #[test]
    fn a_result_in_a_deciding_position_of_a_divergence_is_verified() {
        assert_divergence_states(&[
            ("PASS", "", "", "    if double(2) != 4 {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    if double(2) == 4 {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    let r = double(2);\n    if r != 4 {\n        panic!(\"bad: {}\", r);\n    }\n"),
            ("PASS", "", "", "    match double(2) {\n        4 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("PASS", "", "", "    let Some(_v) = double_opt(2) else {\n        panic!(\"bad\");\n    };\n"),
            ("PASS", "", "", "    if let Some(4) = double_opt(2) {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    while double(2) != 4 {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    for v in [double(2)] {\n        if v != 4 {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("PASS", "", "", "    let r = double_opt(2);\n    if !matches!(r, Some(4)) {\n        panic!(\"{:?}\", r);\n    }\n"),
            ("PASS", "", "", "    match (double(2), 1) {\n        (4, _) => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("PASS", "", "", "    match Some(double(2)) {\n        Some(4) => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("PASS", "", "", "    let r = double(2);\n    if r == 4 {\n    } else if r > 0 {\n        panic!(\"bad\");\n    } else {\n        unreachable!();\n    }\n"),
            ("UNKNOWN", "", "", "    let _r = double(2);\n    if !flag() {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    let k = 4;\n    if k != 4 {\n        panic!(\"{}\", r);\n    }\n"),
            ("UNKNOWN", "", "", "    if !is_four(double(2)) {\n        panic!(\"bad\");\n    }\n"),
            // DS-1938: the arm binds `x` to the whole result, so the guard is derived.
            ("PASS", "", "", "    match double(2) {\n        x if x == 4 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match (double(2), 1) {\n        (_, y) if y == 1 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match Some(double(2)) {\n        Some(_) => {}\n        None => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    let t = (double(2), 1);\n    match t {\n        (_, 1) => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    let _f = || {\n        if r != 4 {\n            panic!(\"bad\")\n        }\n    };\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    let _ = flag() && {\n        if r != 4 {\n            panic!(\"bad\");\n        }\n        true\n    };\n"),
        ]);
    }

    /// @vtest.id TEST-ORACLE-FAILURE-DIVERGENCE-CONSTRUCTS-COUNT
    /// @vtest.covers VO-ORACLE-FAILURE-DIVERGENCE-CONSTRUCTS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent panic!・unreachable!・todo!・unimplemented!・assert!(false)・Result を返す Test の return Err(..) を失敗としての発散に数えて PASS にし、発散しない println! や return Ok(..) だけの分岐は PASS にしないことを確認する
    #[test]
    fn the_failure_divergence_constructs_are_counted_and_others_are_not() {
        let mut cases = Vec::new();
        let bodies: Vec<String> = [
            "panic!(\"bad\")",
            "unreachable!()",
            "todo!()",
            "unimplemented!()",
            "assert!(false)",
        ]
        .iter()
        .map(|divergence| format!("    if double(2) != 4 {{\n        {divergence};\n    }}\n"))
        .collect();
        for body in &bodies {
            cases.push(("PASS", "", "", body.as_str()));
        }
        cases.push((
            "PASS",
            "",
            "-> Result<(), String> ",
            "    if double(2) != 4 {\n        return Err(\"bad\".to_string());\n    }\n    Ok(())\n",
        ));
        cases.push((
            "FAIL",
            "",
            "",
            "    if double(2) != 4 {\n        println!(\"bad\");\n    }\n",
        ));
        // `Ok(..)` is a call that is not a target, so DA-003 is not FAIL either.
        cases.push((
            "UNKNOWN",
            "",
            "-> Result<(), String> ",
            "    if double(2) != 4 {\n        return Ok(());\n    }\n    Ok(())\n",
        ));
        assert_divergence_states(&cases);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-AFTER-IRREFUTABLE-ARM-NOT-REACHED
    /// @vtest.covers VO-ORACLE-DIVERGENCE-AFTER-IRREFUTABLE-ARM
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent 反駁不能な腕（_・束縛・反駁不能な if let）の後の発散は到達を数えず PASS にならず、反駁可能な腕の後なら PASS になり、型によって反駁されうる構造体パターンや判定しないガードの後は UNKNOWN になることを確認する
    #[test]
    fn a_divergence_after_an_irrefutable_arm_is_not_reached() {
        assert_divergence_states(&[
            ("FAIL", "", "", "    match double(2) {\n        _ => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("FAIL", "", "", "    match double(2) {\n        x => {\n            let _ = x;\n        }\n        _ => panic!(\"bad\"),\n    }\n"),
            ("FAIL", "", "", "    if let x = double(2) {\n        let _ = x;\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    match (double(2), 1) {\n        (_, 1) => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("PASS", "", "", "    match double(2) {\n        4 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match kind(2) {\n        S { .. } => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match double(2) {\n        _ if 1 + 1 == 2 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
        ]);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-UNDER-CONSTANT-FALSE-NOT-REACHED
    /// @vtest.covers VO-ORACLE-DIVERGENCE-UNDER-CONSTANT-FALSE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent 定数 false のガード・条件・while の条件や空の範囲の for の下の発散は到達を数えず PASS にならず、ガードが定数でなければ到達を数えることを確認する
    #[test]
    fn a_divergence_under_a_constant_false_is_not_reached() {
        assert_divergence_states(&[
            ("FAIL", "", "", "    match double(2) {\n        _ if false => panic!(\"bad\"),\n        _ => {}\n    }\n"),
            ("FAIL", "", "", "    if false && double(2) != 4 {\n        panic!(\"bad\");\n    }\n"),
            ("FAIL", "", "", "    let r = double(2);\n    if false {\n        panic!(\"{}\", r);\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    while false {\n        if r != 4 {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    for _ in 0..0 {\n        if r != 4 {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("PASS", "", "", "    for _ in 0..1 {\n        if double(2) != 4 {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("PASS", "", "", "    let r = double(2);\n    match r {\n        _ if r != 4 => panic!(\"bad\"),\n        _ => {}\n    }\n"),
        ]);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-REACH-UNDECIDABLE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DIVERGENCE-REACH-UNDECIDABLE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent 決める位置が範囲パターンや Some / None / Ok / Err 以外の列挙子だけで検査されるか、前の腕が true と false・Some と None を両方検査するとき UNKNOWN になり、Some / None で検査される target の結果なら PASS になることを確認する
    #[test]
    fn a_divergence_whose_reach_is_not_decided_by_syntax_is_unknown() {
        assert_divergence_states(&[
            ("UNKNOWN", "", "", "    match double(2) {\n        0..=4 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match kind(2) {\n        Kind::A(_) => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match kind(2) {\n        Kind::A(4) => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    const FOUR: i32 = 4;\n    match double(2) {\n        FOUR => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("UNKNOWN", "", "", "    match double(2) == 4 {\n        true => {}\n        false => {}\n        _ => unreachable!(),\n    }\n"),
            ("UNKNOWN", "", "", "    match double_opt(2) {\n        Some(_) => {}\n        None => {}\n        _ => unreachable!(),\n    }\n"),
            ("PASS", "", "", "    match double_opt(2) {\n        Some(_) => {}\n        None => panic!(\"bad\"),\n    }\n"),
        ]);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-AFTER-EXIT-DECIDED-BY-EXIT
    /// @vtest.covers VO-ORACLE-DIVERGENCE-AFTER-EXIT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent 発散より前の return の実行を target の結果が決めれば PASS になり、無条件か定数条件の return の後の発散は PASS にならず、? の後、ループの中の return・break・continue、展開を読まないマクロ、ループ本体で代入される条件の後は UNKNOWN になることを確認する
    #[test]
    fn an_exit_before_a_divergence_decides_its_reach() {
        assert_divergence_states(&[
            ("PASS", "", "", "    if double(2) == 4 {\n        return;\n    }\n    panic!(\"bad\");\n"),
            ("UNKNOWN", "", "-> Result<(), ()> ", "    let v: i32 = \"4\".parse().map_err(|_| ())?;\n    if double(2) != v {\n        panic!(\"bad\");\n    }\n    Ok(())\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    return;\n    if r != 4 {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    if true {\n        return;\n    }\n    if r != 4 {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    loop {\n        if double(2) == 4 {\n            break;\n        }\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    for v in [1] {\n        if v + double(2) == 5 {\n            return;\n        }\n    }\n    panic!(\"bad\");\n"),
            ("UNKNOWN", "", "", "    'outer: for _ in 0..1 {\n        if double(2) == 4 {\n            continue 'outer;\n        }\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let mut i = 0;\n    while i != double(2) {\n        i += 1;\n        if i > 10 {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    dbg!(1);\n    if double(2) != 4 {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    let _ = (if r == 4 { return } else { 1 }, if r != 4 { panic!(\"bad\") } else { 2 });\n"),
        ]);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-AFTER-LEAVING-STATEMENT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DIVERGENCE-AFTER-EXIT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent 全分岐で関数を抜ける文（if / match の全腕の return、let の初期化式の if、入れ子のブロックの中の return、Result を返す Test の全分岐の return Ok）や ? の後の発散は PASS にならず、抜けない経路がある条件付き return の後なら PASS になることを確認する
    #[test]
    fn a_divergence_after_a_statement_that_always_leaves_is_unknown() {
        let after = "    #[allow(unreachable_code)]\n    if double(2) != 4 {\n        panic!(\"bad\");\n    }\n";
        let bodies = [
            "    if double(2) == 4 {\n        return;\n    } else {\n        return;\n    }\n",
            "    match double(2) {\n        4 => return,\n        _ => return,\n    };\n",
            "    let _x: i32 = if double(2) == 4 { return } else { return };\n",
            "    {\n        if double(2) == 4 {\n            return;\n        }\n        return;\n    }\n",
        ]
        .map(|before| format!("{before}{after}"));
        let mut cases: Vec<(&str, &str, &str, &str)> = bodies
            .iter()
            .map(|body| ("UNKNOWN", "", "", body.as_str()))
            .collect();
        cases.push((
            "UNKNOWN",
            "",
            "-> Result<(), String> ",
            "    if double(2) == 4 {\n        return Ok(());\n    } else {\n        return Ok(());\n    }\n    #[allow(unreachable_code)]\n    if double(2) != 4 {\n        return Err(\"bad\".to_string());\n    }\n    Ok(())\n",
        ));
        cases.push((
            "UNKNOWN",
            "",
            "-> Result<(), ()> ",
            "    let _v = double_opt(2).ok_or(())?;\n    if double(2) != 4 {\n        panic!(\"bad\");\n    }\n    Ok(())\n",
        ));
        cases.push((
            "PASS",
            "",
            "",
            "    if double(2) == 4 {\n        return;\n    }\n    panic!(\"bad\");\n",
        ));
        assert_divergence_states(&cases);
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-EXHAUSTING-PATTERNS-ARE-UNKNOWN
    /// @vtest.covers VO-ORACLE-DIVERGENCE-REACH-UNDECIDABLE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::collect_divergences
    /// @vtest.intent if let・let-else・matches! のパターンが、else 側やより後ろの腕の中のどの深さ（文・ブロック・括弧の中）でも経路上で反駁されたパターンと合わせて組の両方を検査するとき、反駁されたパターンの scrutinee が同じと確定できないとき、全経路で抜ける then や腕の後でそのパターンを反駁されたものとして合わせるとき、反駁されたパターンが自分のパターンを覆うとき、定数で真のガードの腕が組を検査するとき、Ok と Err を両方検査するとき、256 個以上のリテラルを検査するときは UNKNOWN になり、組を尽くさない else if の連鎖と 255 個のリテラルなら PASS のままであることを確認する
    #[test]
    fn patterns_that_may_match_every_value_leave_the_divergence_unknown() {
        assert_divergence_states(&[
            ("UNKNOWN", "", "", "    if let Some(_) | None = double_opt(2) {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let (true | false) = double(2) == 4 else {\n        panic!(\"bad\");\n    };\n"),
            ("UNKNOWN", "", "", "    if let (Some(_) | None, _) = (double_opt(2), 1) {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else if let None = o {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else if let None = (o) {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        if let None = o {\n        } else {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        let _k = 1;\n        if let None = o {\n        } else {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    let p = double_opt(3);\n    if let Some(4) = o {\n    } else if let Some(6) = p {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        if let None = o {\n        } else {\n            panic!(\"bad\");\n        };\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        if let None = o {\n        } else {\n            panic!(\"bad\");\n        }\n        let _k = 1;\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        {\n            if let None = o {\n            } else {\n                panic!(\"bad\");\n            }\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        (if let None = o {\n        } else {\n            panic!(\"bad\");\n        })\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    let Some(_) = o else {\n        let None = o else {\n            panic!(\"bad\");\n        };\n        return;\n    };\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    match o {\n        Some(_) => {}\n        _ => {\n            if let None = o {\n            } else {\n                panic!(\"bad\");\n            }\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n        return;\n    }\n    if let None = o {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    match o {\n        Some(_) => return,\n        _ => {}\n    }\n    if let None = o {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    if let Some(_) = o {\n    } else {\n        if let Some(5) = o {\n            panic!(\"bad\");\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    #[allow(unreachable_patterns)]\n    match double_opt(2) {\n        Some(_) => {}\n        Some(5) => panic!(\"bad\"),\n        _ => {}\n    }\n"),
            ("UNKNOWN", "", "", "    let o = double_opt(2);\n    let _k = if let Some(_) = o { return } else { 1 };\n    if let None = o {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    let o = double_opt(2);\n    if let Some(9) = o {\n        return;\n    }\n    if let Some(4) = o {\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    let o = double_opt(2);\n    let p = double(2);\n    if let Some(9) = o {\n        return;\n    }\n    match p {\n        4 => {}\n        _ => panic!(\"bad\"),\n    }\n"),
            ("PASS", "", "", "    match double_opt(2) {\n        Some(4) => {}\n        Some(_) => panic!(\"other\"),\n        None => panic!(\"none\"),\n    }\n"),
            ("PASS", "", "", "    let o = double_opt(2);\n    if let Some(4) = o {\n    } else if let None = o {\n        panic!(\"none\");\n    } else {\n        panic!(\"bad\");\n    }\n"),
            ("PASS", "", "", "    let o = double_opt(2);\n    if let Some(4) = o {\n    } else {\n        let _k = 1;\n        {\n            if let None = o {\n                panic!(\"none\");\n            } else {\n                panic!(\"bad\");\n            }\n        }\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double_opt(2);\n    if !matches!(r, Some(_) | None) {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    let r = double(2);\n    if !matches!(r, _) {\n        panic!(\"bad\");\n    }\n"),
            ("UNKNOWN", "", "", "    match double(2) == 4 {\n        true if true => {}\n        false => {}\n        _ => unreachable!(),\n    }\n"),
            ("PASS", "", "", "    match double(2) == 4 {\n        true if flag() => {}\n        false => {}\n        _ => unreachable!(),\n    }\n"),
            ("UNKNOWN", "", "", "    match double_res(2) {\n        Ok(_) => {}\n        Err(_) => {}\n        _ => unreachable!(),\n    }\n"),
        ]);
        let literal_arms = |count: usize| {
            let arms: String = (0..count)
                .map(|value| format!("        {value} => {{}}\n"))
                .collect();
            format!("    match double(2) {{\n{arms}        _ => panic!(\"bad\"),\n    }}\n")
        };
        assert_eq!(divergence_state("", "", &literal_arms(255)), "PASS");
        assert_eq!(divergence_state("", "", &literal_arms(256)), "UNKNOWN");
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-DA-003-REACH-BOTH-SIDES
    /// @vtest.covers VO-ORACLE-DIVERGENCE-DECIDING-POSITION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::target_call_result_is_verified
    /// @vtest.intent DA-001 の FAIL に頼らず、DA-003 自身が、派生した値が決める位置に流れる発散では違反なしになり、反駁不能な腕の後・定数 false の下・リテラルの要素を検査する腕の後・二つの要素を同時に検査する腕・全分岐で抜ける文の後・リテラルの範囲や配列でない for の本体の発散では違反なしにならないことを確認する
    #[test]
    fn da_003_reaches_only_a_divergence_a_derived_value_decides() {
        let da_003 = |body: &str| {
            let text = format!("#[test]\nfn t() {{\n    assert!(flag());\n{body}}}\n");
            let targets = ["double".to_owned(), "double_vec".to_owned()];
            analyze(&text, &targets, &[]).da_003
        };
        for body in [
            "    if double(2) != 4 {\n        panic!(\"bad\");\n    }\n",
            "    match double(2) {\n        4 => {}\n        _ => panic!(\"bad\"),\n    }\n",
            "    for v in [double(2)] {\n        if v != 4 {\n            panic!(\"bad\");\n        }\n    }\n",
        ] {
            assert_eq!(da_003(body), DaVerdict::NoViolation, "{body}");
        }
        for body in [
            "    match double(2) {\n        _ => {}\n        _ => panic!(\"bad\"),\n    }\n",
            "    let r = double(2);\n    if false {\n        panic!(\"{}\", r);\n    }\n",
            "    match (double(2) == 4, true) {\n        (true, _) => {}\n        (_, true) => {}\n        _ => panic!(\"bad\"),\n    }\n",
            "    match (double(2), 1) {\n        (4, _) | (_, 1) => {}\n        _ => panic!(\"bad\"),\n    }\n",
            "    let k = 1;\n    match (double(2), k) {\n        (4, 2) => panic!(\"bad\"),\n        _ => {}\n    }\n",
            "    if double(2) == 4 {\n        return;\n    } else {\n        return;\n    }\n    #[allow(unreachable_code)]\n    if double(2) != 4 {\n        panic!(\"bad\");\n    }\n",
            "    for v in double_vec(2).iter().skip(1) {\n        if *v != 4 {\n            panic!(\"bad\");\n        }\n    }\n",
        ] {
            assert_ne!(da_003(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// @vtest.id TEST-ORACLE-DA-001-DIVERGENCE-CONSTANCY-BY-DECIDER
    /// @vtest.covers VO-ORACLE-DIVERGENCE-CONSTANCY-BY-DECIDING-POSITION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent 失敗として発散する構文の DA-001 の定数性を決める位置で判定し、決める位置に定数でない値があれば違反なし、決める位置が無いか定数だけなら FAIL、決める位置を評価できないか #[should_panic] の下にあれば UNKNOWN になることを確認する
    #[test]
    fn da_001_decides_a_divergence_by_its_deciding_positions() {
        let da_001 = |body: &str| {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            analyze(&text, &["double".to_owned()], &[]).da_001
        };
        for body in [
            "    let x = double(2);\n    if x != 4 {\n        unreachable!();\n    }\n",
            "    match double(2) {\n        4 => {}\n        _ => unreachable!(),\n    }\n",
            "    if double(2) == 4 {\n        return;\n    }\n    unreachable!();\n",
            "    if double(2) != 4 {\n        std::process::exit(1);\n    }\n    unreachable!();\n",
            "    let x = double(2);\n    assert_eq!(x, 4);\n    unreachable!();\n",
        ] {
            assert!(matches!(da_001(body), DaVerdict::NoViolation), "{body}");
        }
        for body in [
            "    if true {\n        unreachable!(\"{}\", y);\n    }\n",
            "    match r {\n        _ => unreachable!(),\n    }\n",
            "    {\n        unreachable!()\n    }\n",
            "    let x: i32 = todo!();\n",
            "    if false {\n        panic!(\"{}\", y);\n    }\n",
        ] {
            assert!(matches!(da_001(body), DaVerdict::Fail(_)), "{body}");
        }
        // Under `#[should_panic]` a divergence on every path passes whatever the
        // target returns, so a non-constant deciding position does not decide it.
        for body in [
            "    if double(2) != 4 {\n        panic!(\"a\");\n    } else {\n        panic!(\"b\");\n    }\n",
            "    match double(2) {\n        4 => panic!(\"a\"),\n        _ => panic!(\"b\"),\n    }\n",
        ] {
            let text = format!("#[should_panic]\n#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(matches!(analysis.da_001, DaVerdict::Unknown(_)), "{body}");
            assert!(!compose(&analysis).0 && compose(&analysis).1, "{body}");
        }
        for body in [
            "    while c() {\n        break;\n    }\n    todo!();\n",
            "    my_macro!();\n    unimplemented!();\n",
            "    match double(2) {\n        0..=4 => {}\n        _ => unreachable!(),\n    }\n",
        ] {
            assert!(matches!(da_001(body), DaVerdict::Unknown(_)), "{body}");
        }
    }

    /// @vtest.id TEST-ORACLE-DA-001-DIVERGENCE-WITH-UNWRAP-IS-NOT-CONSTANT
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent assert マクロが unreachable! などや定数の assert! だけでも、識別子を受け手にする .unwrap()・.expect(..)・? があれば DA-001 が違反なしになり、失敗しえない Some(..).unwrap() は数えず assert!(true) の本体が FAIL のままであり、クロージャ・async ブロック・入れ子の fn の中にだけある .unwrap() は数えず DA-001 が UNKNOWN になることを確認する
    #[test]
    fn a_divergence_beside_an_unwrap_or_try_is_not_a_constant_assertion() {
        for body in [
            "    let v = parse(\"x\").unwrap();\n    unreachable!();\n",
            "    let v = double(2).checked_add(1).expect(\"p\");\n    match v {\n        5 => {}\n        _ => todo!(),\n    }\n",
            "    let v = parse(\"x\")?;\n    if v != 4 {\n        unimplemented!();\n    }\n",
            "    let v = parse(\"x\").unwrap();\n    assert!(true);\n    unreachable!();\n",
            "    let v = double(2).unwrap();\n    assert!(v == 4 || true);\n",
            "    let v = double(2)?;\n    assert!(v == 4 || true);\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(matches!(analysis.da_001, DaVerdict::NoViolation), "{text}");
        }
        // DS-1964: an `.unwrap()` that cannot fail is not an assert-equivalent.
        let constant = "#[test]\nfn t() {\n    let v = Some(1).unwrap();\n    assert!(true);\n}\n";
        assert!(matches!(
            analyze(constant, &[], &[]).da_001,
            DaVerdict::Fail(_)
        ));
        // One only in a closure, an `async` block or a nested item may not run, so it
        // is not counted, and DA-001 is neither FAIL nor no violation.
        for body in [
            "    let _f = || \"x\".parse::<i32>().unwrap();\n    assert!(true);\n",
            "    fn helper() -> i32 {\n        \"4\".parse().unwrap()\n    }\n    assert!(true);\n",
            "    assert!(true);\n    let _x = async { cfg().unwrap() };\n",
            "    let _f = |x: Option<i32>| x.unwrap();\n    unreachable!();\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &[], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_001
            );
            assert!(!compose(&analysis).0 && compose(&analysis).1, "{text}");
        }
    }

    /// @vtest.id TEST-ORACLE-DEBUG-ASSERT-ALONE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DEBUG-ASSERT-NOT-ASSERT-EQUIVALENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の結果が debug_assert* にだけ流れる Test は DA-003 と DA-006 が FAIL でも違反なしでもなく UNKNOWN になり、config の assertion_macros に debug_assert* を挙げても同じであることを確認する（#294）
    #[test]
    fn a_result_checked_only_by_debug_assert_is_unknown() {
        for config in [
            Vec::new(),
            vec!["debug_assert_eq".to_owned()],
            vec!["debug_assert_eq!".to_owned()],
        ] {
            for body in [
                "    debug_assert_eq!(double(2), 4);\n",
                "    let r = double(2);\n    debug_assert_eq!(r, 4);\n",
                "    let r = double(2);\n    debug_assert!(r == 4);\n",
                "    let r = double(2);\n    debug_assert_ne!(r.clone(), 5);\n",
            ] {
                let text = format!("#[test]\nfn t() {{\n{body}}}\n");
                let analysis = analyze(&text, &["double".to_owned()], &config);
                assert!(
                    matches!(analysis.da_003, DaVerdict::Unknown(_)),
                    "{text}: {:?}",
                    analysis.da_003
                );
                assert!(
                    matches!(analysis.da_006, DaVerdict::Unknown(_)),
                    "{text}: {:?}",
                    analysis.da_006
                );
                let (is_fail, is_unknown, _) = compose(&analysis);
                assert!(!is_fail && is_unknown, "{text}");
            }
        }
    }

    /// @vtest.id TEST-ORACLE-DEBUG-ASSERT-IS-NOT-AN-ASSERT-EQUIVALENT
    /// @vtest.covers VO-ORACLE-DEBUG-ASSERT-NOT-ASSERT-EQUIVALENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent debug_assert* は assert 相当として違反なしの根拠にならず（assert の引数の中の debug_assert* も同じ）、結果が流れない debug_assert* やメッセージ引数だけの debug_assert* では DA-003 が FAIL のままで、他の assert 相当に到達すれば違反なしになることを確認する
    #[test]
    fn a_debug_assert_never_counts_as_an_assert_equivalent() {
        let nested =
            "#[test]\nfn t() {\n    assert_eq!({ debug_assert_eq!(double(2), 4) }, ());\n}\n";
        assert_eq!(
            analyze(nested, &["double".to_owned()], &[])
                .da_003
                .outcome(),
            RuleOutcome::Unknown
        );
        for body in [
            "    let r = double(2);\n    debug_assert!(true);\n",
            "    let r = double(2);\n    debug_assert!(true, \"{}\", r);\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert_eq!(analysis.da_003.outcome(), RuleOutcome::Fail, "{text}");
        }
        let checked =
            "#[test]\nfn t() {\n    let mut r = double(2);\n    debug_assert!(r > 0);\n    assert_eq!(r, 4);\n}\n";
        let analysis = analyze(checked, &["double".to_owned()], &[]);
        assert_eq!(analysis.da_003, DaVerdict::NoViolation);
        assert_eq!(analysis.da_006, DaVerdict::NoViolation);
    }

    /// DA-003 of a Test whose declared targets are `double` and `maybe`.
    fn da_003_of_two(body: &str) -> DaVerdict {
        let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
        analyze(&text, &["double".to_owned(), "maybe".to_owned()], &[]).da_003
    }

    /// DS-1962 / DS-1944: a call and an assert-equivalent in a closure that is not
    /// tracked, an `async` block or a nested item are not counted. Such a call makes
    /// DA-003 UNKNOWN, never FAIL or no violation, unless a counted call's result
    /// reaches a counted assert-equivalent.
    /// @vtest.id TEST-ORACLE-DA-003-COUNTED-POSITIONS
    /// @vtest.covers VO-ORACLE-DA-003-COUNTED-POSITIONS, VO-ORACLE-DA-003-UNTRACKED-CLOSURE-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 呼ばれないクロージャ・関数へ渡すクロージャ・async ブロック・入れ子の関数の中の target 呼出と assert 相当（`?` を含む）では DA-003 が違反なしにも FAIL にもならず UNKNOWN になり、数える位置の呼出の結果が数える位置の assert に届けば、数える位置にない呼出があっても違反なしになることを確認する
    #[test]
    fn only_calls_and_asserts_at_counted_positions_decide_da_003() {
        for body in [
            "let _check = || assert_eq!(double(2), 4);",
            "let r = double(2); let _check = || assert_eq!(r, 4);",
            "let r: Option<()> = None::<i32>.map(|x| assert_eq!(double(x), 4)); let _ = r;",
            "let f = || -> Option<i32> { let v = maybe(2)?; Some(v) }; let _ = f();",
            "let _g = || double(2);",
            "let _fut = async { assert_eq!(double(2), 4) };",
            "fn inner() { assert_eq!(double(2), 4); } inner();",
            "[2].iter().for_each(|&x| assert_eq!(double(x), 4));",
        ] {
            assert!(
                matches!(da_003_of_two(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of_two(body)
            );
        }
        for body in [
            "let r = double(2); assert_eq!(r, 4); let _g = || double(3);",
            "assert_eq!(double(2), 4); let _fut = async { double(3) };",
        ] {
            assert_eq!(da_003_of_two(body), DaVerdict::NoViolation, "{body}");
        }
        assert!(matches!(
            da_003_of_two("let _ = double(2);"),
            DaVerdict::Fail(_)
        ));
    }

    /// DS-1938 / DS-1939: a name bound by a `match` arm, `if let`, `let ... else` or a
    /// destructuring `let` takes the derivation of the position it extracts. A
    /// derived position that reaches an assert-equivalent verifies the target; a
    /// name at another position, or one that an or-pattern binds from positions of
    /// different derivations, does not.
    /// @vtest.id TEST-ORACLE-DA-003-PATTERN-BINDING-TAKES-ITS-POSITION
    /// @vtest.covers VO-ORACLE-DA-003-PATTERN-BINDING, VO-ORACLE-DA-003-PER-POSITION-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent パターンで束縛した名前が target の結果の位置を取り出して assert に届けば違反なしになり、別の位置を取り出した名前や、or パターンで位置の導出が食い違う名前では違反なしにならないことを確認する
    #[test]
    fn a_pattern_binding_takes_the_derivation_of_its_position() {
        for body in [
            "if let Some(x) = maybe(2) { assert_eq!(x, 4); }",
            "let Some(x) = maybe(2) else { panic!() }; assert_eq!(x, 4);",
            "match maybe(2) { Some(x) => assert_eq!(x, 4), None => panic!() }",
            "let (a, _b) = (double(2), 3); assert_eq!(a, 4);",
            "match Some((3, double(2))) { Some((_, b)) => assert_eq!(b, 4), _ => {} }",
            "match Some(double(2)) { Some(v @ 0..=9) => assert_eq!(v, 4), _ => {} }",
            "match (double(2), double(3)) { (x, _) | (_, x) => assert!(x > 0) }",
            "let S { a, .. } = S { a: double(2), b: 0 }; assert_eq!(a, 4);",
            "let t = (double(2), 3); let (a, _) = t; assert_eq!(a, 4);",
            "let v = double(2); let Some(x) = &v.items[0] else { panic!() }; assert_eq!(x, &4);",
            "match E::A { x: double(2) } { E::A { x } => assert_eq!(x, 4), _ => {} }",
        ] {
            assert_eq!(da_003_of_two(body), DaVerdict::NoViolation, "{body}");
        }
        for body in [
            "let (_a, b) = (double(2), 3); assert_eq!(b, 3);",
            "match Some((double(2), 3)) { Some((_, b)) => assert_eq!(b, 3), _ => {} }",
            "match (double(2), 3) { (x, _) | (_, x) => assert_eq!(x, 4) }",
            "let S { b, .. } = S { a: double(2), b: 0 }; assert_eq!(b, 0);",
            "match Some(double(2)) { v @ Some(_) => assert_eq!(v, Some(4)), _ => {} }",
            // A constant or a unit variant in a pattern binds nothing.
            "match maybe(2) { None => {}, Some(_) => {} } assert_eq!(Option::<i32>::None, None);",
            "match double(2) { K => {}, _ => {} } assert_eq!(K, 4);",
            "if let K = double(2) {} assert_eq!(K, 4);",
            // A struct pattern of another variant never matches the literal.
            "match (E::A { x: double(2) }) { E::B { x } => assert_eq!(x, 4), _ => {} }",
        ] {
            assert_ne!(da_003_of_two(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// DS-1941 / DS-1942: a call `f(..)` of a closure bound by `let f = |..| body;`, at
    /// a counted position, is the execution of its body, `f` meaning the nearest `let`
    /// in scope. The value of a call is derived only when every exit of the body other
    /// than a failure divergence is: a value from a `return` or from a branch whose
    /// exits are not all derived is not.
    /// @vtest.id TEST-ORACLE-DA-003-TRACKED-CLOSURE-CALL-RUNS-ITS-BODY
    /// @vtest.covers VO-ORACLE-DA-003-CLOSURE-CALL-TRACKED, VO-ORACLE-DA-003-CLOSURE-VALUE-EXITS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent let で束縛したクロージャを数える位置で呼ぶと、その本体の実行として追跡されて target の結果が本体の assert や呼出の値を経て届けば違反なしになり、直近の let を指し、値を使う呼出では return や分岐の出口が派生でない本体なら違反なしにならないことを確認する
    #[test]
    fn a_tracked_closure_call_is_the_execution_of_its_body() {
        for body in [
            "let check = |v: i32| assert_eq!(v, 4); check(double(2));",
            "let f = |x| double(x); assert_eq!(f(2), 4);",
            "let check = |v: i32| assert_eq!(v, 4); check(double(2)); check(double(3));",
            "let inner = |v| assert_eq!(v, 4); let outer = |v| inner(v); outer(double(2));",
            "let outer = |v: i32| { let inner = |w| assert_eq!(w, 4); inner(v) }; outer(double(2));",
            "let check = |v: i32| { if v == 0 { return; } assert_eq!(v, 4); }; check(double(2));",
            "let check = |v: i32| if v != 4 { panic!() }; check(double(2));",
            "let check = |v: i32| assert_eq!(v, 4); { let check = |_v: i32| (); check(3); } check(double(2));",
            "let f = |v: i32| (v, 0); let (a, _) = f(double(2)); assert_eq!(a, 4);",
            "let f = |a: i32, b: i32| assert_eq!(b - a, 3); let b = 1; f(b, double(2));",
            "let r = double(2); let get = || r; assert_eq!(get(), 4);",
            "let f = |a: i32| S { a, b: 0 }; assert_eq!(f(double(2)).a, 4);",
            "let f = |v: i32| assert_eq!(v, 4, \"{:?}\", v); f(double(2));",
        ] {
            assert_eq!(da_003_of_two(body), DaVerdict::NoViolation, "{body}");
        }
        for body in [
            "let f = |v: i32| { if v > 0 { return v; } 0 }; assert_eq!(f(double(2)), 4);",
            "let f = |v: i32| if v > 0 { v } else { 0 }; assert_eq!(f(double(2)), 4);",
            "let f = |v: i32| (v, 0); let (_, b) = f(double(2)); assert_eq!(b, 0);",
            "let r = double(2); let get = || 4; assert_eq!(get(), 4); let _ = r;",
            // DS-1941: an item, or a binding made after the closure, is not what the
            // name means at the closure or at the call.
            "let f = |v: i32| assert_eq!(v, 4); { fn f(_v: i32) {} f(double(2)); }",
            "fn r() -> i32 { 4 } let get = || r(); let r = || double(2); assert_eq!(get(), 4);",
            "fn helper(v: i32) -> i32 { v } let g = |v: i32| helper(v); let helper = |v: i32| assert_eq!(v, 4); assert_eq!(g(double(2)), 4);",        ] {
            assert_ne!(da_003_of_two(body), DaVerdict::NoViolation, "{body}");
        }
        assert!(matches!(
            da_003_of_two("let f = |v: i32| v; f(double(2));"),
            DaVerdict::Fail(_)
        ));
    }

    /// DS-1944 / DS-1943 / DS-1959: a closure that is rebound, bound with `mut`,
    /// called on the spot, passed, never called or called only where a call is not
    /// counted is not tracked, and a target call or a derived value in its body makes
    /// DA-003 UNKNOWN, as does a value that flows into a nested item or an `async`
    /// block, or a `mut` binding a tracked-looking closure may change.
    /// @vtest.id TEST-ORACLE-DA-003-UNTRACKED-CLOSURE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNTRACKED-CLOSURE-UNKNOWN, VO-ORACLE-DA-003-NESTED-ITEM-ASYNC-NOT-FLOWING, VO-ORACLE-DA-003-BINDING-MUTATION-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 再束縛・mut 束縛・その場呼出・引数渡し・呼ばれない・数える位置にない位置からだけ呼ぶクロージャ、入れ子の項目や async ブロックへ流れる値、クロージャが変えうる mut 束縛では DA-003 が違反なしにも FAIL にもならず UNKNOWN になることを確認する
    #[test]
    fn an_untracked_closure_leaves_da_003_unknown() {
        for body in [
            "let check = |v: i32| assert_eq!(v, 4); let g = check; g(double(2));",
            "let mut check = |v: i32| assert_eq!(v, 4); check(double(2));",
            "(|v: i32| assert_eq!(v, 4))(double(2));",
            "let check = |v: i32| assert_eq!(v, 4); [double(2)].into_iter().for_each(check);",
            "let check = |v: i32| assert_eq!(double(v), 4);",
            "let check = |v: i32| assert_eq!(v, 4); let _later = || check(double(2));",
            "fn inner() -> i32 { double(2) } assert_eq!(inner(), 4);",
            "let f = |v: i32| { let _a = async move { assert_eq!(v, 4) }; }; f(double(2));",
            "let mut r = double(2); let f = || r.f = 5; f(); assert_eq!(r.g, 4);",
        ] {
            assert!(
                matches!(da_003_of_two(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of_two(body)
            );
        }
    }
}
