//! DA-002 (DS-622): `rust-cargo`'s static target reach, the static side of
//! `target_binding` (DS-659, DS-666, DS-672).
//!
//! Responsibility: for each declared target of one Test (DS-630), read the Test
//! body and the functions of the Test's own file that it calls, and report `Pass`
//! when a call of the target is confirmed there, `Fail` when that range holds no
//! call of the target and no call left unfollowed (DS-622), and `Unknown` otherwise
//! (DS-612: the boundary does not prove reach, which never means "not called").
//!
//! Boundary: this module reads no file and never runs anything; the Test's file
//! comes in `StaticAnalysisInput::file_text`. It does not decide `target_binding`
//! (`vtest-verify` combines this verdict with runtime coverage, DS-668).
//!
//! What is counted (DS-1962): a call outside the closures, `async` blocks and nested
//! items of the body, and inside a macro only in the checked (non-message) arguments
//! of `assert!` / `assert_eq!` / `assert_ne!` and in the expression arguments of
//! `vec!` and `matches!` (a `matches!` guard is not counted). A closure DS-1941
//! tracks is read as its body (`inline_tracked_closures`), in the Test and in a
//! followed function alike, which DS-1941's "少なくとも" allows. A function of the
//! same file called at a counted position is followed with the same rule, at any
//! depth (DS-628); one called elsewhere is followed with nothing in it counted. A
//! call that syntax fixes never to run (`never_run_regions`) is not counted either;
//! when the target's only calls are there, or at other uncounted positions, the
//! verdict is `Unknown`, never `Pass` or `Fail` (DS-1944, DS-1962, DS-639). A
//! function with a `cfg` attribute inside its body counts nothing, as the
//! attributed code may not be compiled, and neither does a followed `async`
//! function, whose body runs only when its future is polled. A function with an
//! attribute outside `PLAIN_ATTRIBUTES`, or inside a module or `impl` with one,
//! counts nothing either, as an attribute macro may replace what it runs. A `const`
//! block and an array length count nothing: they are evaluated at compile time.
//! The right side of `false && ..` and `true || ..` never runs either, nor does
//! code after an `assert_eq!` / `assert_ne!` whose compared values DS-1949's
//! classifier fixes so that it always fails (a failure divergence of DS-1951, read
//! by `never_run_regions` like `assert!(false)`, so also after a block holding it).
//! The standard macros above are read only when nothing in the file may take their
//! name: a `macro_rules!` of that name (any `macro_rules!` in statement position), a
//! `use` of that name, a `#[macro_use]` crate or out-of-line module, a glob import
//! of a module outside the file, or an item whose attribute may add items (an
//! attribute macro or a derive outside the standard ones). Even then, a macro
//! called by a bare name may be a `macro_rules!` of another file (DS-659 bounds the
//! analysis to this file), so it is a call left unfollowed: its arguments are read
//! for `Pass`, and it rules out `Fail`. A `std::` / `core::` path names the
//! standard macro itself. Such an item with an expanding attribute also keeps the
//! names of its module (or of the body that holds it) from resolving through a
//! glob import, since its expansion may add an item of the same name.
//!
//! Remaining false `Pass` cases:
//! - DS-1962 counts a call in a branch or a loop body whatever decides it at run
//!   time, so a branch the run does not take, a loop that runs zero times, a call
//!   after an exit under a run-time condition (`if c() { return; }`) and a call
//!   after a call that never returns (`std::process::exit`, a `-> !` function, or
//!   any function, of the file or not, whose every path panics, such as
//!   `fn boom() { panic!() }`) count as calls. Only a measured count of zero shows
//!   that such a call did not run (DS-684).
//! - Literal patterns are not evaluated, so an arm or an `if let` that never
//!   matches (`match 1 { 0 => { double(2) } _ => {} }`,
//!   `if let Some(_) = None::<i32> { double(2) }`) counts its calls.
//! - `cfg!(..)` is a macro whose value is not read, so a branch under
//!   `if cfg!(any()) { .. }` counts its calls.
//! - A bare-named standard macro's arguments are read as the standard one reads
//!   them, and it is taken to add no item, so a `macro_rules!` of another file
//!   under that name that drops its arguments or adds an item is not seen, nor is
//!   one that a macro expansion defines.
//! - `#[test]` is read as the prelude's unless a `use` of that name is in the
//!   file; an attribute macro named `test` that a glob import brings is not seen.
//!   Likewise a derive named like a standard one is read as the standard derive, so
//!   one a `use` brings under that name (`use pm::Debug;` with `#[derive(Debug)]`),
//!   whose expansion may add items, is not seen.
//!
//! A name a format string captures counts as a local when the body binds it
//! anywhere. A static of the same name in scope there cannot be shadowed by a
//! binding (E0530), and one in a nested block is a nested item, left unfollowed.
//!
//! Which call is the target's (narrower than DS-622, each toward `Unknown`):
//! - Only a target that is a function of the Test's own file is identified: a free
//!   function, or an inherent method of a type without generic parameters. A target
//!   in another file or crate is not, so a call of it is a call left unfollowed
//!   (DS-659; DS-637 names this case). A trait method is not identified either.
//! - A call is the target's when its callee path resolves to that function inside
//!   the file: a bare name, `self::` / `super::` paths, paths through modules written
//!   inline in the file, and `Type::name` for an inherent method. Names are resolved
//!   through the items of each module, its `use` declarations that start with
//!   `self::` or `super::` (with `as` aliases), its glob imports from modules of the
//!   file (an item a glob does not make visible is not imported), and the `use`
//!   declarations at the top level of the function body. A name that something else
//!   may bind is not resolved: a local binding, an item or `use` in a nested block,
//!   a glob in the body, a `use` from outside the file, an item under a `cfg`
//!   attribute other than `cfg(test)`, a module with an item-position macro or two
//!   definitions, a bare name in a body (the Test's or a followed function's) that
//!   holds a macro in statement position other than the standard ones read above
//!   (its expansion may add an item of any name, DS-639), `crate::` paths, and
//!   `Self::` paths.
//! - A method call (`receiver.name(..)`) is never the target's: its receiver type is
//!   not resolved, so a method of another type or trait with the same name is not
//!   ruled out. It is a call left unfollowed.
//!
//! What rules out a call (`Fail`): every construct of the range is one that cannot
//! run code of the project. Anything else is a call left unfollowed: a call that is
//! not resolved to a function of the file, a method call, a macro other than the
//! ones read above, a path used as a value other than a local binding, a struct
//! literal, a nested item, a callee that is not a path, Test parameters (an attribute
//! supplies them), a name a format string captures that is not a local (its
//! `Display` may run code of the project), and an attribute other than `test`,
//! `should_panic`, `ignore`, `doc`, `cfg` and the lint and codegen attributes (an
//! attribute macro may rewrite the function). Values are then only literals and locals bound from them, so the
//! operators, comparisons, indexing, `for`, `?` and drops that remain run the
//! standard library's code only (DS-629 reads every other path as `Unknown`).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use syn::{parse::ParseStream, spanned::Spanned, visit::Visit};
use vtest_adapter_api::{ReachOutcome, StaticAnalysisInput, TargetReach};
use vtest_model::TargetRef;

use crate::oracle_presence::{
    fixed_value, in_regions, inline_tracked_closures, never_run_regions, pattern_ident_may_be_path,
    span_offset, Fixed,
};
use crate::RustLocator;

/// Attributes whose expansion is known not to change what the function runs.
const PLAIN_ATTRIBUTES: &[&str] = &[
    "test",
    "should_panic",
    "ignore",
    "doc",
    "cfg",
    "allow",
    "expect",
    "warn",
    "deny",
    "forbid",
    "inline",
    "must_use",
    "track_caller",
    "cold",
    "macro_use",
];

/// The derives of the standard library, which add trait impls only, no named item.
const STANDARD_DERIVES: &[&str] = &[
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];

/// Built-in item attributes that add no item.
const ITEM_ATTRIBUTES: &[&str] = &[
    "repr",
    "non_exhaustive",
    "macro_export",
    "path",
    "deprecated",
];

/// The most `use`, glob and module steps one name resolution takes. Beyond it the
/// name is not resolved, which also stops a cycle of glob imports.
const MAX_RESOLUTION_DEPTH: usize = 8;

/// How many parsed files one thread keeps (`FileModel::cached`).
const MODEL_CACHE_SIZE: usize = 256;

/// DA-002 for every declared target of `input.test`, in declaration order (DS-630).
pub(crate) fn target_reach(input: StaticAnalysisInput<'_>) -> Vec<TargetReach> {
    let model = input.file_text.and_then(FileModel::cached);
    let module = test_module(&input.test.location.locator);
    let test_path = input.test.location.path.as_str();
    input
        .test
        .targets
        .iter()
        .map(|target| {
            let locator = match target {
                TargetRef::Locator(locator) => RustLocator::parse(&locator.value),
                TargetRef::SrcId(_) => None,
            };
            let identified = locator.as_ref().and_then(|locator| {
                let model = model.as_ref()?;
                if locator.path != test_path {
                    return None;
                }
                model.target_fn(&locator.item_path)
            });
            let target_name = locator
                .as_ref()
                .and_then(|locator| locator.item_path.rsplit("::").next().map(str::to_owned));
            let (outcome, reason) = decide(
                input.construct_text,
                &module,
                model.as_deref(),
                identified,
                target_name.as_deref(),
            );
            TargetReach {
                target: target.clone(),
                outcome,
                reason,
            }
        })
        .collect()
}

/// The module of the file that holds the Test, from its item path.
fn test_module(locator: &str) -> Vec<String> {
    let mut segments: Vec<String> = locator.split("::").map(str::to_owned).collect();
    segments.pop();
    segments
}

/// DA-002 for one target: `target` is the target function's start in the file when
/// it is identified, `target_name` its final name.
fn decide(
    test_text: &str,
    module: &[String],
    model: Option<&FileModel>,
    target: Option<usize>,
    target_name: Option<&str>,
) -> (ReachOutcome, String) {
    let mut walk = Walk {
        model,
        target,
        target_name,
        found: Found::default(),
        visited: BTreeSet::new(),
        pending: VecDeque::new(),
    };
    let enclosed = model.is_some_and(|model| model.rewritten(module));
    walk.function(test_text, module, Position::Counted, None, enclosed);
    // The called functions are read nearest first, and only until a counted call of
    // the target is found, since nothing found later changes a `Pass`.
    while walk.found.reached.is_none() {
        let Some((start, position)) = walk.pending.pop_front() else {
            break;
        };
        walk.read(start, position);
    }
    let found = walk.found;
    let unidentified = if target.is_none() {
        "; the target is not identified as a function of the Test's own file, so no call \
         is confirmed as its call (DS-659, DS-637)"
    } else {
        ""
    };
    if let Some(place) = found.reached {
        return (
            ReachOutcome::Pass,
            format!(
                "a call of the target at a counted position of {place} is confirmed within the \
                 analysis boundary (DS-622, DS-1962, DS-666)"
            ),
        );
    }
    let reason = if found.never_run {
        format!(
            "the target's calls within the boundary are only in positions that syntax fixes \
             never to run: such a call does not prove reach statically, so it is no `Pass`, and \
             it is a call, so it is no `Fail` (DS-659, DS-622){unidentified}"
        )
    } else if found.not_counted {
        format!(
            "the target's calls within the boundary are only at positions DS-1962 does not \
             count (a closure DS-1941 does not track, an `async` block, a nested item, a macro \
             argument that is not counted, code under a `cfg` attribute or an attribute that may \
             rewrite it, or a `const` context evaluated at compile time) (DS-1944, DS-1962, \
             DS-639){unidentified}"
        )
    } else if let Some(what) = found.unfollowed {
        format!(
            "{what} is not followed, so an indirect call of the target is not ruled out \
             (DS-622, DS-659, DS-629){unidentified}"
        )
    } else {
        return (
            ReachOutcome::Fail,
            "no call of the target in the Test body and the same-file functions it calls, \
             and no call left unfollowed there (DS-622)"
                .to_owned(),
        );
    };
    (ReachOutcome::Unknown, reason)
}

/// Where a call stands for DS-1962.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Position {
    Counted,
    /// Syntax fixes it never to run.
    NeverRun,
    NotCounted,
}

#[derive(Debug, Default)]
struct Found {
    /// Where a counted call of the target is.
    reached: Option<String>,
    never_run: bool,
    not_counted: bool,
    /// The first construct that is not followed.
    unfollowed: Option<String>,
}

struct Walk<'m> {
    model: Option<&'m FileModel>,
    target: Option<usize>,
    target_name: Option<&'m str>,
    found: Found,
    /// The functions of the file already called, by start and position.
    visited: BTreeSet<(usize, Position)>,
    /// The called functions not read yet.
    pending: VecDeque<(usize, Position)>,
}

impl Walk<'_> {
    fn unfollow(&mut self, what: impl FnOnce() -> String) {
        if self.found.unfollowed.is_none() {
            self.found.unfollowed = Some(what());
        }
    }

    /// Reads one function: the Test (`name` is `None`) or a function of the file.
    /// `enclosed` tells that a module or `impl` around it has an attribute that may
    /// rewrite it.
    fn function(
        &mut self,
        text: &str,
        module: &[String],
        base: Position,
        name: Option<&str>,
        enclosed: bool,
    ) {
        let inlined = inline_tracked_closures(text);
        let text = inlined.as_deref().unwrap_or(text);
        let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
            self.unfollow(|| "a function that does not parse".to_owned());
            return;
        };
        let label = name.map_or_else(
            || "the Test".to_owned(),
            |name| format!("the function `{name}`"),
        );
        if name.is_none() && !function.sig.inputs.is_empty() {
            self.unfollow(|| "a parameter of the Test, which an attribute supplies,".to_owned());
        }
        // `test` is a macro of the prelude, which a `use` of that name can replace; a
        // file that does not parse may hold one.
        let test_replaced = self
            .model
            .is_none_or(|model| model.shadowed_macros.contains("test"));
        let attribute = function.attrs.iter().find(|attribute| {
            !is_plain(attribute) || (test_replaced && attribute.path().is_ident("test"))
        });
        if let Some(attribute) = attribute {
            let path = path_text(attribute.path());
            self.unfollow(|| {
                format!("the attribute `#[{path}]` on {label}, which may rewrite it,")
            });
        } else if enclosed {
            self.unfollow(|| {
                format!("an attribute on a module or `impl` around {label}, which may rewrite it,")
            });
        }
        // Code an attribute macro may rewrite is not what runs, so nothing in it counts.
        let rewritten = enclosed || attribute.is_some();
        let mut cfg = CfgFinder(false);
        cfg.visit_block(&function.block);
        // An `async` function's body runs only when its future is polled, which the
        // call site does not show.
        let deferred = name.is_some() && function.sig.asyncness.is_some();
        let base = if (cfg.0 || deferred || rewritten) && base == Position::Counted {
            Position::NotCounted
        } else {
            base
        };
        let never_run = never_run_regions(text);
        let scope = BodyScope::new(&function, self.model);
        let place = name.map_or_else(
            || "the Test body".to_owned(),
            |name| format!("the same-file function `{name}`"),
        );
        let mut visitor = FnVisitor {
            walk: self,
            text,
            module,
            scope: &scope,
            never_run: &never_run,
            position: base,
            place: &place,
        };
        visitor.visit_block(&function.block);
    }

    /// Follows a call of the file's function that starts at `start`: it is read later
    /// (`decide`), once per position.
    fn follow(&mut self, start: usize, position: Position) {
        if self.visited.insert((start, position)) {
            self.pending.push_back((start, position));
        }
    }

    fn read(&mut self, start: usize, position: Position) {
        let Some(model) = self.model else {
            return;
        };
        let Some(info) = model.fns.get(&start) else {
            return;
        };
        self.function(
            &model.text[info.start..info.end],
            &info.module,
            position,
            Some(&info.name),
            info.enclosed || model.rewritten(&info.module),
        );
    }
}

fn is_plain(attribute: &syn::Attribute) -> bool {
    let path = attribute.path();
    if path.is_ident("cfg_attr") {
        // Plain when every attribute it may apply is.
        return attribute
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|metas| metas.iter().skip(1).all(|meta| plain_path(meta.path())));
    }
    plain_path(path)
}

/// Whether an item's attributes may add items beside it, whose expansion is not
/// read (DS-639): an attribute macro, or a derive outside `STANDARD_DERIVES`.
fn may_add_items(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        let path = attribute.path();
        if path.is_ident("derive") {
            return attribute
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                )
                .map_or(true, |derives| {
                    derives
                        .iter()
                        .any(|derive| !STANDARD_DERIVES.iter().any(|name| derive.is_ident(name)))
                });
        }
        !is_plain(attribute) && !ITEM_ATTRIBUTES.iter().any(|name| path.is_ident(name))
    })
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

/// A name of `PLAIN_ATTRIBUTES`, or a tool attribute (`rustfmt::`, `clippy::`),
/// which cannot be a macro.
fn plain_path(path: &syn::Path) -> bool {
    PLAIN_ATTRIBUTES.iter().any(|plain| path.is_ident(plain))
        || (path.segments.len() > 1
            && path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "rustfmt" || segment.ident == "clippy"))
}

/// Whether a format string captures a name that is not a local of `scope` (`{name}`),
/// whose `Display` or `Debug` may run code of the project.
fn captures_non_local(format: &str, scope: &BodyScope) -> bool {
    let mut rest = format;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        if let Some(escaped) = rest.strip_prefix('{') {
            rest = escaped;
            continue;
        }
        let end = rest.find(['}', ':']).unwrap_or(rest.len());
        let name = rest[..end].trim();
        if name.starts_with(|c: char| c.is_alphabetic() || c == '_') && !scope.bound.contains(name)
        {
            return true;
        }
    }
    false
}

/// The macros `FnVisitor` reads by their standard meaning.
const READ_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "vec",
    "matches",
];

/// Finds, over the whole file, what may take over a standard macro name and which
/// modules an attribute may rewrite.
struct MacroScan<'m> {
    model: &'m FileModel,
    module: Vec<String>,
    shadowed: BTreeSet<String>,
    rewritten: BTreeSet<Vec<String>>,
}

impl MacroScan<'_> {
    /// Whether a glob import's path names a module written inline in the file, whose
    /// macro imports this scan sees itself.
    fn glob_in_file(&self, path: &[String]) -> bool {
        let mut module = self.module.clone();
        let mut rest = path;
        if rest.first().is_some_and(|first| first == "self") {
            rest = &rest[1..];
        } else {
            while rest.first().is_some_and(|first| first == "super") {
                if module.pop().is_none() {
                    return false;
                }
                rest = &rest[1..];
            }
            // A path without `self` or `super` starts at the current module when that
            // module has an item of its first name (a uniform path).
        }
        module.extend(rest.iter().cloned());
        self.model.modules.contains_key(&module)
    }
}

impl<'ast> Visit<'ast> for MacroScan<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        // An expansion that adds items may add a `macro_rules!` of any name.
        if may_add_items(item_attrs(node)) {
            self.shadowed.insert("*".to_owned());
        }
        syn::visit::visit_item(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.module.push(node.ident.to_string());
        if node.content.is_none() {
            // Its `macro_rules!` items are in scope after it, in this file.
            if node
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("macro_use"))
            {
                self.shadowed.insert("*".to_owned());
            }
        } else if node.attrs.iter().any(|attribute| !is_plain(attribute)) {
            self.rewritten.insert(self.module.clone());
        }
        syn::visit::visit_item_mod(self, node);
        self.module.pop();
    }

    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        if node
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("macro_use"))
        {
            self.shadowed.insert("*".to_owned());
        }
    }

    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        match &node.ident {
            Some(ident) => {
                self.shadowed.insert(ident.to_string());
            }
            None => syn::visit::visit_item_macro(self, node),
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        // A `macro_rules!` in statement position; its name is not parsed here.
        if node.path.is_ident("macro_rules") {
            self.shadowed.insert("*".to_owned());
        }
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let mut uses = Vec::new();
        let mut globs = Vec::new();
        flatten_use(&node.tree, &mut Vec::new(), &mut uses, &mut globs);
        for (name, _) in uses {
            if READ_MACROS.contains(&name.as_str()) || name == "test" {
                self.shadowed.insert(name);
            }
        }
        if globs
            .iter()
            .any(|glob| node.leading_colon.is_some() || !self.glob_in_file(glob))
        {
            self.shadowed.insert("*".to_owned());
        }
    }
}

fn path_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Whether a block holds a `cfg` or `cfg_attr` attribute anywhere.
struct CfgFinder(bool);

impl<'ast> Visit<'ast> for CfgFinder {
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if node.path().is_ident("cfg") || node.path().is_ident("cfg_attr") {
            self.0 = true;
        }
    }
}

/// The names a function body binds, which a callee name may mean instead of an item
/// of the module.
#[derive(Default)]
struct BodyScope {
    /// Names bound by patterns, parameters included.
    bound: BTreeSet<String>,
    /// `use` declarations at the top level of the body: name and path.
    top_uses: Vec<(String, Vec<String>)>,
    /// Other items at the top level of the body, and every name an item-position
    /// macro or a glob there may bind (`*`).
    top_items: BTreeSet<String>,
    /// Items and `use` names in nested blocks (`*` for a glob or a macro).
    nested: BTreeSet<String>,
}

impl BodyScope {
    fn new(function: &syn::ItemFn, model: Option<&FileModel>) -> Self {
        // The standard macros read here expand to no item; any other macro in
        // statement position may (DS-639).
        let read = READ_MACROS
            .iter()
            .filter(|name| {
                model.is_some_and(|model| {
                    !model.shadowed_macros.contains("*") && !model.shadowed_macros.contains(**name)
                })
            })
            .map(|name| (*name).to_owned())
            .collect();
        let mut collector = ScopeCollector {
            scope: Self::default(),
            depth: 0,
            read,
        };
        collector.visit_signature(&function.sig);
        collector.visit_block(&function.block);
        // A type or const parameter shadows an item of the same name in the body.
        for param in &function.sig.generics.params {
            match param {
                syn::GenericParam::Type(param) => {
                    collector.scope.bound.insert(param.ident.to_string());
                }
                syn::GenericParam::Const(param) => {
                    collector.scope.bound.insert(param.ident.to_string());
                }
                syn::GenericParam::Lifetime(_) => {}
            }
        }
        collector.scope
    }

    /// Whether a name of the body may shadow `name` in the module.
    fn shadows(&self, name: &str) -> bool {
        self.bound.contains(name)
            || self.top_items.contains(name)
            || self.top_items.contains("*")
            || self.nested.contains(name)
            || self.nested.contains("*")
    }
}

struct ScopeCollector {
    scope: BodyScope,
    depth: usize,
    /// The names of `READ_MACROS` that nothing in the file may take.
    read: BTreeSet<String>,
}

impl ScopeCollector {
    /// A macro in statement position, other than a standard one read here, may
    /// expand to items of any name, as an item-position macro of a module does.
    fn record_macro(&mut self, node: &syn::Macro) {
        let standard = node.path.segments.len() == 1
            || node
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "std" || segment.ident == "core");
        let read = standard
            && node
                .path
                .segments
                .last()
                .is_some_and(|segment| self.read.contains(&segment.ident.to_string()));
        if !read {
            if self.depth == 0 {
                self.scope.top_items.insert("*".to_owned());
            } else {
                self.scope.nested.insert("*".to_owned());
            }
        }
    }

    fn record(&mut self, item: &syn::Item) {
        let top = self.depth == 0;
        if let syn::Item::Use(item_use) = item {
            let mut uses = Vec::new();
            let mut globs = Vec::new();
            flatten_use(&item_use.tree, &mut Vec::new(), &mut uses, &mut globs);
            if top && item_use.leading_colon.is_none() {
                self.scope.top_uses.extend(uses);
            } else {
                // A `use ::path` names another crate, so its names are not resolved.
                self.scope
                    .nested
                    .extend(uses.into_iter().map(|(name, _)| name));
            }
            if !globs.is_empty() {
                self.scope.nested.insert("*".to_owned());
            }
            return;
        }
        // An item whose attribute may add items binds names that are not known.
        let name = item_name(item)
            .filter(|_| !may_add_items(item_attrs(item)))
            .unwrap_or_else(|| "*".to_owned());
        if top {
            self.scope.top_items.insert(name);
        } else {
            self.scope.nested.insert(name);
        }
    }
}

impl<'ast> Visit<'ast> for ScopeCollector {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        for stmt in &node.stmts {
            match stmt {
                syn::Stmt::Item(item) => self.record(item),
                syn::Stmt::Macro(node) => self.record_macro(&node.mac),
                _ => {}
            }
        }
        self.depth += 1;
        syn::visit::visit_block(self, node);
        self.depth -= 1;
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        if !pattern_ident_may_be_path(node) {
            self.scope.bound.insert(node.ident.to_string());
        }
        syn::visit::visit_pat_ident(self, node);
    }

    // Names inside a nested item belong to that item.
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

/// The name an item binds, `None` for one that may bind any name.
fn item_name(item: &syn::Item) -> Option<String> {
    Some(
        match item {
            syn::Item::Fn(item) => &item.sig.ident,
            syn::Item::Const(item) => &item.ident,
            syn::Item::Static(item) => &item.ident,
            syn::Item::Struct(item) => &item.ident,
            syn::Item::Enum(item) => &item.ident,
            syn::Item::Union(item) => &item.ident,
            syn::Item::Trait(item) => &item.ident,
            syn::Item::Type(item) => &item.ident,
            syn::Item::Mod(item) => &item.ident,
            syn::Item::Macro(item) if item.ident.is_some() => return Some("!".to_owned()),
            syn::Item::Impl(_) => return Some("!".to_owned()),
            _ => return None,
        }
        .to_string(),
    )
}

/// The `use` tree flattened into (name, path) pairs and glob prefixes. A `_` import
/// binds no name; `self` in a group binds the group's last segment.
fn flatten_use(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    uses: &mut Vec<(String, Vec<String>)>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use(&path.tree, prefix, uses, globs);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut path = prefix.clone();
            if name.ident != "self" {
                path.push(name.ident.to_string());
            }
            if let Some(last) = path.last() {
                uses.push((last.clone(), path.clone()));
            }
        }
        syn::UseTree::Rename(rename) => {
            if rename.rename == "_" {
                return;
            }
            let mut path = prefix.clone();
            if rename.ident != "self" {
                path.push(rename.ident.to_string());
            }
            uses.push((rename.rename.to_string(), path));
        }
        syn::UseTree::Glob(_) => globs.push(prefix.clone()),
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                flatten_use(tree, prefix, uses, globs);
            }
        }
    }
}

/// Reads one function body: records the target's calls and what is not followed.
struct FnVisitor<'v, 'm> {
    walk: &'v mut Walk<'m>,
    text: &'v str,
    module: &'v [String],
    scope: &'v BodyScope,
    never_run: &'v [(usize, usize)],
    position: Position,
    place: &'v str,
}

impl FnVisitor<'_, '_> {
    fn with_position(&mut self, position: Position, visit: impl FnOnce(&mut Self)) {
        let outer = self.position;
        self.position = self.position.max(position);
        visit(self);
        self.position = outer;
    }

    fn position_at(&self, span: proc_macro2::Span) -> Position {
        if self.position == Position::Counted
            && in_regions(span_offset(span.start(), self.text), self.never_run)
        {
            Position::NeverRun
        } else {
            self.position
        }
    }

    fn call(&mut self, path: &syn::Path, span: proc_macro2::Span) {
        let position = self.position_at(span);
        let resolved = self.resolve_callee(path);
        match resolved {
            Some(start) if Some(start) == self.walk.target => match position {
                Position::Counted => {
                    if self.walk.found.reached.is_none() {
                        self.walk.found.reached = Some(self.place.to_owned());
                    }
                }
                Position::NeverRun => self.walk.found.never_run = true,
                Position::NotCounted => self.walk.found.not_counted = true,
            },
            Some(start) => {
                let same_name = self.walk.target.is_none()
                    && self
                        .walk
                        .model
                        .and_then(|model| model.fns.get(&start))
                        .is_some_and(|info| Some(info.name.as_str()) == self.walk.target_name);
                if same_name {
                    self.unfollow_path(path);
                } else {
                    self.walk.follow(start, position);
                }
            }
            None => self.unfollow_path(path),
        }
    }

    fn unfollow_path(&mut self, path: &syn::Path) {
        let text = path_text(path);
        self.walk.unfollow(|| {
            format!("the call of `{text}`, which is not resolved to a function of the file,")
        });
    }

    /// The start of the file's function that `path` names as a callee here.
    fn resolve_callee(&self, path: &syn::Path) -> Option<usize> {
        let model = self.walk.model?;
        if path.leading_colon.is_some() {
            return None;
        }
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        let first = segments.first()?;
        if first != "self" && first != "super" {
            if self.scope.shadows(first) {
                return None;
            }
            let uses: Vec<&Vec<String>> = self
                .scope
                .top_uses
                .iter()
                .filter(|(name, _)| name == first)
                .map(|(_, path)| path)
                .collect();
            match uses.as_slice() {
                [] => {}
                [use_path] if segments.len() == 1 => {
                    return model
                        .use_path(self.module, use_path, Ns::Value, 0)
                        .found()
                        .and_then(Def::into_fn);
                }
                _ => return None,
            }
        }
        model.expr_path(self.module, &segments)
    }
}

impl<'ast> Visit<'ast> for FnVisitor<'_, '_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        match node.func.as_ref() {
            syn::Expr::Path(path) if path.qself.is_none() => {
                self.call(&path.path, node.span());
                for segment in &path.path.segments {
                    self.visit_path_arguments(&segment.arguments);
                }
            }
            other => {
                self.walk
                    .unfollow(|| "a call whose callee is not a path".to_owned());
                self.visit_expr(other);
            }
        }
        for argument in &node.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let method = node.method.to_string();
        self.walk.unfollow(|| {
            format!("the method call `.{method}(..)`, whose receiver type is not resolved,")
        });
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        let local = node.qself.is_none()
            && node
                .path
                .get_ident()
                .is_some_and(|ident| self.scope.bound.contains(&ident.to_string()));
        if !local {
            let text = path_text(&node.path);
            self.walk
                .unfollow(|| format!("the value `{text}`, which may be a function or hold one,"));
        }
        for segment in &node.path.segments {
            self.visit_path_arguments(&segment.arguments);
        }
    }

    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        self.walk
            .unfollow(|| "a struct literal, whose type may run code of the project,".to_owned());
        syn::visit::visit_expr_struct(self, node);
    }

    // Patterns match by structure, so they call no code; only the type of a typed
    // pattern (`let x: T`) is read, for the expressions in it.
    fn visit_pat(&mut self, node: &'ast syn::Pat) {
        if let syn::Pat::Type(typed) = node {
            self.visit_type(&typed.ty);
        }
    }

    // A type and a generic argument run nothing when the Test runs; an expression in
    // them (an array length, a const argument) is evaluated at compile time.
    fn visit_type(&mut self, node: &'ast syn::Type) {
        self.with_position(Position::NotCounted, |visitor| {
            syn::visit::visit_type(visitor, node);
        });
    }

    fn visit_generic_argument(&mut self, node: &'ast syn::GenericArgument) {
        self.with_position(Position::NotCounted, |visitor| {
            syn::visit::visit_generic_argument(visitor, node);
        });
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.with_position(Position::NotCounted, |visitor| {
            syn::visit::visit_expr_closure(visitor, node);
        });
    }

    // A `const` block and an array length are evaluated when the Test is compiled,
    // not when it runs.
    fn visit_expr_const(&mut self, node: &'ast syn::ExprConst) {
        self.with_position(Position::NotCounted, |visitor| {
            syn::visit::visit_expr_const(visitor, node);
        });
    }

    fn visit_expr_repeat(&mut self, node: &'ast syn::ExprRepeat) {
        self.visit_expr(&node.expr);
        self.with_position(Position::NotCounted, |visitor| {
            visitor.visit_expr(&node.len)
        });
    }

    fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
        self.with_position(Position::NotCounted, |visitor| {
            syn::visit::visit_expr_async(visitor, node);
        });
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Verbatim(_) = node {
            self.walk
                .unfollow(|| "an expression that does not parse".to_owned());
            return;
        }
        syn::visit::visit_expr(self, node);
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        if let syn::Item::Use(_) = node {
            return;
        }
        self.walk.unfollow(|| "a nested item".to_owned());
        // A target call in it is not counted (DS-1962).
        self.with_position(Position::NotCounted, |visitor| {
            syn::visit::visit_item(visitor, node);
        });
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let standard = node.path.segments.len() == 1
            || node
                .path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "std" || segment.ident == "core");
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        // A macro of the file or one it imports may take the standard name.
        // A file that does not parse may take any name.
        let shadowed = self.walk.model.is_none_or(|model| {
            model.shadowed_macros.contains("*") || model.shadowed_macros.contains(&name)
        });
        let read = standard
            && !shadowed
            && match name.as_str() {
                "assert" | "assert_eq" | "assert_ne" => {
                    let checked = if name == "assert" { 1 } else { 2 };
                    self.macro_arguments(node, checked)
                }
                "panic" | "unreachable" | "todo" | "unimplemented" => self.macro_arguments(node, 0),
                "vec" => self.vec_arguments(node),
                "matches" => self.matches_arguments(node),
                _ => false,
            };
        if !read {
            self.walk
                .unfollow(|| format!("the macro `{name}!`, whose expansion is not read,"));
        } else if node.path.segments.len() == 1 {
            // A `macro_rules!` of another file is outside the boundary (DS-659) and may
            // take the name, so its expansion is one left unfollowed (DS-639): it rules
            // out `Fail`, while the arguments read above may still give `Pass`.
            self.walk.unfollow(|| {
                format!("the macro `{name}!`, which a `macro_rules!` outside the file may define,")
            });
        }
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        // The right side of `false && ..` and `true || ..` never runs (DS-1949 fixes
        // the left side).
        let skipped = match node.op {
            syn::BinOp::And(_) => fixed_value(&node.left) == Fixed::Value(false),
            syn::BinOp::Or(_) => fixed_value(&node.left) == Fixed::Value(true),
            _ => false,
        };
        if skipped {
            self.visit_expr(&node.left);
            self.with_position(Position::NeverRun, |visitor| {
                visitor.visit_expr(&node.right)
            });
        } else {
            syn::visit::visit_expr_binary(self, node);
        }
    }
}

type Arguments = syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>;

impl FnVisitor<'_, '_> {
    /// Reads a macro's comma-separated arguments: the first `checked` count, the rest
    /// are message arguments (DS-1935), which are not counted.
    fn macro_arguments(&mut self, node: &syn::Macro, checked: usize) -> bool {
        let Ok(arguments) = node.parse_body_with(Arguments::parse_terminated) else {
            return false;
        };
        for (index, argument) in arguments.iter().enumerate() {
            if index < checked {
                self.visit_expr(argument);
            } else {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(format),
                    ..
                }) = argument
                {
                    if captures_non_local(&format.value(), self.scope) {
                        self.walk.unfollow(|| {
                            "a name a format string captures, whose formatting may run code \
                             of the project,"
                                .to_owned()
                        });
                    }
                }
                self.with_position(Position::NotCounted, |visitor| visitor.visit_expr(argument));
            }
        }
        true
    }

    fn vec_arguments(&mut self, node: &syn::Macro) -> bool {
        if let Ok(arguments) = node.parse_body_with(Arguments::parse_terminated) {
            for argument in &arguments {
                self.visit_expr(argument);
            }
            return true;
        }
        let repeat = |input: ParseStream| -> syn::Result<(syn::Expr, syn::Expr)> {
            let element = input.parse()?;
            input.parse::<syn::Token![;]>()?;
            Ok((element, input.parse()?))
        };
        let Ok((element, count)) = node.parse_body_with(repeat) else {
            return false;
        };
        self.visit_expr(&element);
        self.visit_expr(&count);
        true
    }

    fn matches_arguments(&mut self, node: &syn::Macro) -> bool {
        let parser = |input: ParseStream| -> syn::Result<(syn::Expr, Option<syn::Expr>)> {
            let scrutinee = input.parse()?;
            input.parse::<syn::Token![,]>()?;
            syn::Pat::parse_multi_with_leading_vert(input)?;
            let guard = if input.peek(syn::Token![if]) {
                input.parse::<syn::Token![if]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
            Ok((scrutinee, guard))
        };
        let Ok((scrutinee, guard)) = node.parse_body_with(parser) else {
            return false;
        };
        self.visit_expr(&scrutinee);
        if let Some(guard) = guard {
            self.with_position(Position::NotCounted, |visitor| visitor.visit_expr(&guard));
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Name resolution inside one file
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ns {
    Value,
    Type,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Def {
    /// A function of the file, by its start.
    Fn(usize),
    /// A struct, enum or union of the file without generic parameters, by its start.
    Type(usize),
    /// A module written inline in the file.
    Module(Vec<String>),
    /// Anything else (a constant, a trait, a type alias, a generic type, a module in
    /// another file, ..).
    Other,
}

impl Def {
    fn into_fn(self) -> Option<usize> {
        match self {
            Self::Fn(start) => Some(start),
            Self::Type(_) | Self::Module(_) | Self::Other => None,
        }
    }
}

/// What a name means in one namespace of one module.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Binding {
    Absent,
    /// The definition, and whether the binding is visible from every module of the
    /// crate (`pub` or `pub(crate)`).
    Found(Def, bool),
    Unknown,
}

impl Binding {
    fn found(self) -> Option<Def> {
        match self {
            Self::Found(def, _) => Some(def),
            Self::Absent | Self::Unknown => None,
        }
    }
}

#[derive(Debug)]
struct ScopeItem {
    name: String,
    ns: Ns,
    def: Def,
    public: bool,
    /// Under a `cfg` attribute other than `cfg(test)`.
    conditional: bool,
}

#[derive(Debug)]
struct ScopeUse {
    name: String,
    path: Vec<String>,
    public: bool,
    conditional: bool,
}

#[derive(Debug, Default)]
struct ModuleScope {
    items: Vec<ScopeItem>,
    uses: Vec<ScopeUse>,
    globs: Vec<(Vec<String>, bool)>,
    /// An item-position macro, a second definition of the module, or an item that
    /// does not parse: the names it binds are not known.
    unknown: bool,
    /// An item with an attribute that may add items beside it (`may_add_items`):
    /// the names its expansion binds shadow glob imports.
    expanded: bool,
}

#[derive(Debug)]
struct FnInfo {
    start: usize,
    end: usize,
    module: Vec<String>,
    name: String,
    /// Its `impl` has an attribute that may rewrite it.
    enclosed: bool,
}

#[derive(Debug)]
struct ImplInfo {
    module: Vec<String>,
    self_path: Vec<String>,
    is_trait: bool,
    /// The impl or its self type has generic parameters or arguments.
    generic: bool,
    conditional: bool,
    fns: Vec<(String, usize)>,
}

/// The items of the Test's file, by module, for resolving callee names.
#[derive(Debug)]
struct FileModel {
    text: String,
    /// The byte offset of each line of `text`, computed once for the whole file.
    line_offsets: Vec<usize>,
    modules: BTreeMap<Vec<String>, ModuleScope>,
    fns: BTreeMap<usize, FnInfo>,
    impls: Vec<ImplInfo>,
    /// The names of the macros `FnVisitor` reads that a macro of the file or an
    /// import may take over (`*` for any name).
    shadowed_macros: BTreeSet<String>,
    /// The modules with an attribute that may rewrite them.
    rewritten_modules: BTreeSet<Vec<String>>,
}

impl FileModel {
    /// The model of `text`, parsed once per thread for the last files read: one
    /// verification asks for every Test of a file, and parsing a large file for each
    /// would make it several times slower. The key is the whole text, so a changed
    /// file is parsed again.
    fn cached(text: &str) -> Option<Rc<Self>> {
        thread_local! {
            static MODELS: RefCell<VecDeque<Rc<FileModel>>> = const { RefCell::new(VecDeque::new()) };
        }
        MODELS.with(|models| {
            let mut models = models.borrow_mut();
            if let Some(model) = models.iter().find(|model| model.text == text) {
                return Some(Rc::clone(model));
            }
            let model = Rc::new(Self::parse(text)?);
            if models.len() == MODEL_CACHE_SIZE {
                models.pop_front();
            }
            models.push_back(Rc::clone(&model));
            Some(model)
        })
    }

    fn parse(text: &str) -> Option<Self> {
        let file = syn::parse_file(text).ok()?;
        let mut model = Self {
            text: text.to_owned(),
            line_offsets: crate::line_offsets(text),
            modules: BTreeMap::new(),
            fns: BTreeMap::new(),
            impls: Vec::new(),
            shadowed_macros: BTreeSet::new(),
            rewritten_modules: BTreeSet::new(),
        };
        model.modules.insert(Vec::new(), ModuleScope::default());
        model.collect(&file.items, &[]);
        let mut scan = MacroScan {
            model: &model,
            module: Vec::new(),
            shadowed: BTreeSet::new(),
            rewritten: BTreeSet::new(),
        };
        scan.visit_file(&file);
        let (shadowed, rewritten) = (scan.shadowed, scan.rewritten);
        model.shadowed_macros = shadowed;
        model.rewritten_modules = rewritten;
        Some(model)
    }

    /// Whether `module` or a module around it has an attribute that may rewrite it.
    fn rewritten(&self, module: &[String]) -> bool {
        (1..=module.len()).any(|len| self.rewritten_modules.contains(&module[..len]))
    }

    fn range(&self, span: proc_macro2::Span) -> (usize, usize) {
        let offset = |location: proc_macro2::LineColumn| {
            crate::line_column_byte_offset(
                &self.text,
                &self.line_offsets,
                location.line,
                location.column,
            )
            .unwrap_or(self.text.len())
        };
        (offset(span.start()), offset(span.end()))
    }

    fn collect(&mut self, items: &[syn::Item], module: &[String]) {
        for item in items {
            let (attrs, public) = match item {
                syn::Item::Fn(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Const(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Static(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Struct(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Enum(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Union(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Trait(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Type(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Mod(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Use(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::ExternCrate(item) => (&item.attrs, is_public(&item.vis)),
                syn::Item::Impl(item) => (&item.attrs, false),
                syn::Item::Macro(item) => (&item.attrs, false),
                _ => {
                    self.scope(module).unknown = true;
                    continue;
                }
            };
            let conditional = is_conditional(attrs);
            if may_add_items(attrs) {
                self.scope(module).expanded = true;
            }
            let push = |model: &mut Self, name: String, ns: Ns, def: Def| {
                model.scope(module).items.push(ScopeItem {
                    name,
                    ns,
                    def,
                    public,
                    conditional,
                });
            };
            match item {
                syn::Item::Fn(item) => {
                    let (start, end) = self.range(item.span());
                    self.fns.insert(
                        start,
                        FnInfo {
                            start,
                            end,
                            module: module.to_vec(),
                            name: item.sig.ident.to_string(),
                            enclosed: false,
                        },
                    );
                    push(self, item.sig.ident.to_string(), Ns::Value, Def::Fn(start));
                }
                syn::Item::Const(item) => push(self, item.ident.to_string(), Ns::Value, Def::Other),
                syn::Item::Static(item) => {
                    push(self, item.ident.to_string(), Ns::Value, Def::Other);
                }
                syn::Item::Struct(item) => {
                    let def = if item.generics.params.is_empty() {
                        Def::Type(self.range(item.span()).0)
                    } else {
                        Def::Other
                    };
                    push(self, item.ident.to_string(), Ns::Type, def);
                    if !matches!(item.fields, syn::Fields::Named(_)) {
                        push(self, item.ident.to_string(), Ns::Value, Def::Other);
                    }
                }
                syn::Item::Enum(item) => {
                    let def = if item.generics.params.is_empty() {
                        Def::Type(self.range(item.span()).0)
                    } else {
                        Def::Other
                    };
                    push(self, item.ident.to_string(), Ns::Type, def);
                }
                syn::Item::Union(item) => {
                    let def = if item.generics.params.is_empty() {
                        Def::Type(self.range(item.span()).0)
                    } else {
                        Def::Other
                    };
                    push(self, item.ident.to_string(), Ns::Type, def);
                }
                syn::Item::Trait(item) => push(self, item.ident.to_string(), Ns::Type, Def::Other),
                syn::Item::Type(item) => push(self, item.ident.to_string(), Ns::Type, Def::Other),
                syn::Item::ExternCrate(item) => {
                    let name = item
                        .rename
                        .as_ref()
                        .map_or(&item.ident, |(_, rename)| rename);
                    push(self, name.to_string(), Ns::Type, Def::Other);
                }
                syn::Item::Mod(item) => match &item.content {
                    Some((_, nested)) => {
                        let mut child = module.to_vec();
                        child.push(item.ident.to_string());
                        push(
                            self,
                            item.ident.to_string(),
                            Ns::Type,
                            Def::Module(child.clone()),
                        );
                        if self.modules.contains_key(&child) {
                            self.scope(&child).unknown = true;
                        } else {
                            self.modules.insert(child.clone(), ModuleScope::default());
                        }
                        self.collect(nested, &child);
                    }
                    None => push(self, item.ident.to_string(), Ns::Type, Def::Other),
                },
                syn::Item::Use(item) => {
                    let mut uses = Vec::new();
                    let mut globs = Vec::new();
                    flatten_use(&item.tree, &mut Vec::new(), &mut uses, &mut globs);
                    let absolute = item.leading_colon.is_some();
                    let scope = self.scope(module);
                    for (name, mut path) in uses {
                        if absolute {
                            path.insert(0, "::".to_owned());
                        }
                        scope.uses.push(ScopeUse {
                            name,
                            path,
                            public,
                            conditional,
                        });
                    }
                    for mut path in globs {
                        if absolute {
                            path.insert(0, "::".to_owned());
                        }
                        if conditional {
                            scope.unknown = true;
                        }
                        scope.globs.push((path, public));
                    }
                }
                syn::Item::Impl(item) => self.collect_impl(item, module, conditional),
                syn::Item::Macro(item) => {
                    if item.ident.is_none() {
                        self.scope(module).unknown = true;
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_impl(&mut self, item: &syn::ItemImpl, module: &[String], conditional: bool) {
        let (self_path, generic_arguments) = match item.self_ty.as_ref() {
            syn::Type::Path(path) if path.qself.is_none() => (
                path.path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect(),
                path.path
                    .segments
                    .iter()
                    .any(|segment| !segment.arguments.is_empty()),
            ),
            _ => (Vec::new(), true),
        };
        let mut fns = Vec::new();
        for impl_item in &item.items {
            if let syn::ImplItem::Fn(function) = impl_item {
                let (start, end) = self.range(function.span());
                self.fns.insert(
                    start,
                    FnInfo {
                        start,
                        end,
                        module: module.to_vec(),
                        name: function.sig.ident.to_string(),
                        enclosed: item.attrs.iter().any(|attribute| !is_plain(attribute)),
                    },
                );
                fns.push((function.sig.ident.to_string(), start));
            }
        }
        self.impls.push(ImplInfo {
            module: module.to_vec(),
            self_path,
            is_trait: item.trait_.is_some(),
            generic: generic_arguments || !item.generics.params.is_empty(),
            conditional,
            fns,
        });
    }

    fn scope(&mut self, module: &[String]) -> &mut ModuleScope {
        self.modules.entry(module.to_vec()).or_default()
    }

    /// The start of the function a target's item path names: a free function of its
    /// module, or an inherent method of a type without generic parameters.
    fn target_fn(&self, item_path: &str) -> Option<usize> {
        let segments: Vec<String> = item_path.split("::").map(str::to_owned).collect();
        let (name, prefix) = segments.split_last()?;
        let mut candidates = Vec::new();
        if let Some(scope) = self.modules.get(prefix) {
            if scope.unknown {
                return None;
            }
            for item in &scope.items {
                if &item.name == name {
                    if let Def::Fn(start) = item.def {
                        candidates.push((start, item.conditional, false));
                    }
                }
            }
        }
        if let Some((type_name, module)) = prefix.split_last() {
            for info in &self.impls {
                if info.module == module && info.self_path.last() == Some(type_name) {
                    for (fn_name, start) in &info.fns {
                        if fn_name == name {
                            let excluded = info.is_trait || info.generic || info.conditional;
                            candidates.push((*start, excluded, true));
                        }
                    }
                }
            }
        }
        match candidates.as_slice() {
            [(start, false, _)] => Some(*start),
            _ => None,
        }
    }

    /// What `name` means in namespace `ns` of `module`.
    fn binding(&self, ns: Ns, module: &[String], name: &str, depth: usize) -> Binding {
        if depth > MAX_RESOLUTION_DEPTH {
            return Binding::Unknown;
        }
        let Some(scope) = self.modules.get(module) else {
            return Binding::Unknown;
        };
        if scope.unknown {
            return Binding::Unknown;
        }
        let mut explicit = Vec::new();
        for item in scope
            .items
            .iter()
            .filter(|item| item.name == name && item.ns == ns)
        {
            if item.conditional {
                return Binding::Unknown;
            }
            explicit.push(Binding::Found(item.def.clone(), item.public));
        }
        for import in scope.uses.iter().filter(|import| import.name == name) {
            if import.conditional {
                return Binding::Unknown;
            }
            match self.use_path(module, &import.path, ns, depth + 1) {
                Binding::Absent => {}
                Binding::Found(def, _) => explicit.push(Binding::Found(def, import.public)),
                Binding::Unknown => return Binding::Unknown,
            }
        }
        match explicit.len() {
            0 => {}
            1 => return explicit.remove(0),
            _ => return Binding::Unknown,
        }
        // An item an expansion adds would shadow what a glob brings (DS-639).
        if scope.expanded {
            return Binding::Unknown;
        }
        // Explicit items and imports shadow glob imports. Two globs that bring
        // different items under one name make its use a compile error, so a glob of
        // the file that brings the name decides it whatever the others bring.
        let mut found: Option<(Def, bool)> = None;
        let mut unknown = false;
        for (prefix, glob_public) in &scope.globs {
            let Some(source) = self.use_module(module, prefix, depth + 1) else {
                unknown = true;
                continue;
            };
            match self.binding(ns, &source, name, depth + 1) {
                Binding::Absent => {}
                Binding::Unknown => unknown = true,
                // A glob imports only what is visible to the importing module.
                Binding::Found(def, public) if public || module.starts_with(&source) => {
                    match &found {
                        Some((previous, _)) if *previous != def => return Binding::Unknown,
                        Some(_) => {}
                        None => found = Some((def, *glob_public)),
                    }
                }
                Binding::Found(..) => {}
            }
        }
        match found {
            Some((def, public)) => Binding::Found(def, public),
            None if unknown => Binding::Unknown,
            None => Binding::Absent,
        }
    }

    /// The module a `use` prefix names: it starts with `self` or `super` and goes
    /// through modules of the file.
    fn use_module(
        &self,
        module: &[String],
        prefix: &[String],
        depth: usize,
    ) -> Option<Vec<String>> {
        let (mut current, rest) = Self::relative_start(module, prefix)?;
        for segment in rest {
            match self.binding(Ns::Type, &current, segment, depth + 1) {
                Binding::Found(Def::Module(child), _) => current = child,
                _ => return None,
            }
        }
        Some(current)
    }

    /// What a `use` path names in namespace `ns`.
    fn use_path(&self, module: &[String], path: &[String], ns: Ns, depth: usize) -> Binding {
        let Some((last, prefix)) = path.split_last() else {
            return Binding::Unknown;
        };
        if prefix.is_empty() {
            // `use name;` names a crate (or, in the 2015 edition, a root item).
            return Binding::Unknown;
        }
        let Some(source) = self.use_module(module, prefix, depth) else {
            return Binding::Unknown;
        };
        self.binding(ns, &source, last, depth + 1)
    }

    /// The module a path starting with `self` / `super` begins in, and the rest.
    fn relative_start<'p>(
        module: &[String],
        path: &'p [String],
    ) -> Option<(Vec<String>, &'p [String])> {
        let mut current = module.to_vec();
        let mut rest = path;
        match rest.first().map(String::as_str) {
            Some("self") => rest = &rest[1..],
            Some("super") => {
                while rest.first().map(String::as_str) == Some("super") {
                    current.pop()?;
                    rest = &rest[1..];
                }
            }
            _ => return None,
        }
        Some((current, rest))
    }

    /// The start of the function an expression path names as a callee in `module`.
    fn expr_path(&self, module: &[String], segments: &[String]) -> Option<usize> {
        let (mut current, rest) = match segments.first().map(String::as_str) {
            Some("self" | "super") => Self::relative_start(module, segments)?,
            Some("crate" | "Self") | None => return None,
            Some(_) => (module.to_vec(), segments),
        };
        let (last, prefix) = rest.split_last()?;
        for (index, segment) in prefix.iter().enumerate() {
            match self.binding(Ns::Type, &current, segment, 0).found()? {
                Def::Module(child) => current = child,
                Def::Type(start) if index + 1 == prefix.len() => {
                    return self.inherent_fn(start, last);
                }
                Def::Type(_) | Def::Fn(_) | Def::Other => return None,
            }
        }
        self.binding(Ns::Value, &current, last, 0)
            .found()
            .and_then(Def::into_fn)
    }

    /// The inherent method `name` of the type that starts at `type_start`, when the
    /// file's impls hold exactly one and no impl whose type is not resolved may hold
    /// another.
    fn inherent_fn(&self, type_start: usize, name: &str) -> Option<usize> {
        let mut found = Vec::new();
        for info in self.impls.iter().filter(|info| !info.is_trait) {
            let starts: Vec<usize> = info
                .fns
                .iter()
                .filter(|(fn_name, _)| fn_name == name)
                .map(|(_, start)| *start)
                .collect();
            if starts.is_empty() {
                continue;
            }
            let resolved = match info.self_path.split_last() {
                Some((type_name, [])) => self.binding(Ns::Type, &info.module, type_name, 0),
                Some((type_name, prefix)) => match self.use_module(&info.module, prefix, 0) {
                    Some(source) => self.binding(Ns::Type, &source, type_name, 0),
                    None => Binding::Unknown,
                },
                None => Binding::Unknown,
            };
            match resolved {
                Binding::Found(Def::Type(start), _) if start == type_start => {
                    if info.generic || info.conditional {
                        return None;
                    }
                    found.extend(starts);
                }
                Binding::Found(..) | Binding::Absent => {}
                Binding::Unknown => return None,
            }
        }
        match found.as_slice() {
            [start] => Some(*start),
            _ => None,
        }
    }
}

fn is_public(visibility: &syn::Visibility) -> bool {
    match visibility {
        syn::Visibility::Public(_) => true,
        syn::Visibility::Restricted(restricted) => restricted.path.is_ident("crate"),
        syn::Visibility::Inherited => false,
    }
}

/// Under a `cfg` or `cfg_attr` attribute other than `cfg(test)`, which holds when a
/// Test runs.
fn is_conditional(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        attribute.path().is_ident("cfg_attr")
            || (attribute.path().is_ident("cfg")
                && !attribute
                    .parse_args::<syn::Ident>()
                    .is_ok_and(|ident| ident == "test"))
    })
}

#[cfg(test)]
mod tests;
