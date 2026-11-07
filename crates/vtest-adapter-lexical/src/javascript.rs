//! `javascript`: JavaScript and TypeScript sources (`.js`, `.jsx`, `.mjs`,
//! `.cjs`, `.ts`, `.tsx`, `.mts`, `.cts`).
//!
//! Functions are `function` declarations, class methods, and variables or class fields initialized with an arrow function or a
//! function expression (`const add = (a, b) => a + b`). `class` and
//! TypeScript `namespace`/`module` bodies qualify them. Decorators are part
//! of the construct. Jest, Vitest, Mocha, Jasmine, `node:test`, or Playwright
//! calls such as `test("adds", () => { ... })` and `it(...)` are Test
//! constructs when a `@vtest.` comment precedes them, whatever the framework.

use crate::{
    header::{first_paren, skip_angle_brackets, skip_attributes, AttributeForms, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "javascript";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("`", "`", true),
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: true,
    structure: Structure::Braces,
};

/// Words that may precede a declaration without changing what it declares.
const MODIFIERS: &[&str] = &[
    "export",
    "default",
    "declare",
    "async",
    "static",
    "public",
    "private",
    "protected",
    "readonly",
    "abstract",
    "override",
    "get",
    "set",
    "const",
    "let",
    "var",
];

const NOT_METHOD_NAMES: &[&str] = &[
    "if", "for", "while", "switch", "catch", "with", "return", "function", "super", "new",
    "typeof", "await", "yield",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct JavaScript;

impl LanguageProfile for JavaScript {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["javascript", "typescript"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["node_modules", "dist", "build", "coverage", ".next", "out"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        let start = skip_attributes(
            header,
            AttributeForms {
                at_sign: true,
                ..AttributeForms::default()
            },
        );
        let mut header = &header[start..];
        // `get`/`set`/`static`/`async` are also valid member names
        // (`get() { ... }`), so a modifier is skipped only when a word
        // follows it.
        while header.len() > 1
            && header[0].kind == TokKind::Word
            && MODIFIERS.contains(&header[0].text)
            && header[1].kind == TokKind::Word
        {
            header = &header[1..];
        }
        let Some(first) = header.first() else {
            return HeaderKind::Other;
        };
        match first.text {
            "function" if first.kind == TokKind::Word => {
                let mut index = 1;
                if header.get(index).is_some_and(|token| token.is_punct('*')) {
                    index += 1;
                }
                return match header.get(index) {
                    Some(name) if name.kind == TokKind::Word && has_body => {
                        HeaderKind::Function(vec![name.text.to_owned()])
                    }
                    _ => HeaderKind::Other,
                };
            }
            "class" | "namespace" | "module" if first.kind == TokKind::Word => {
                return match header.get(1) {
                    Some(name) if name.kind == TokKind::Word && has_body => {
                        let mut segments = vec![name.text.to_owned()];
                        let mut index = 2;
                        while header.get(index).is_some_and(|t| t.is_punct('.'))
                            && header
                                .get(index + 1)
                                .is_some_and(|t| t.kind == TokKind::Word)
                        {
                            segments.push(header[index + 1].text.to_owned());
                            index += 2;
                        }
                        HeaderKind::Scope(segments)
                    }
                    _ => HeaderKind::Other,
                };
            }
            _ => {}
        }
        if first.kind != TokKind::Word {
            return HeaderKind::Other;
        }
        // `name = (...) => ...`, `name = function (...) {...}`,
        // `name: Type = async x => ...`.
        if let Some(equals) = header.iter().position(|token| token.is_punct('=')) {
            let is_arrow_or_function = header[equals + 1..]
                .windows(2)
                .any(|pair| pair[0].is_punct('=') && pair[1].is_punct('>'))
                || header
                    .get(equals + 1..)
                    .and_then(|rest| rest.iter().find(|t| !t.is_word("async")))
                    .is_some_and(|token| token.is_word("function"));
            let simple_left_side = header[1..equals]
                .iter()
                .all(|token| !token.is_open('(') && !token.is_punct('.'));
            if is_arrow_or_function
                && simple_left_side
                && header.get(equals + 1).is_none_or(|t| !t.is_punct('='))
            {
                return HeaderKind::Function(vec![first.text.to_owned()]);
            }
            return HeaderKind::Other;
        }
        // Methods: `name(...) { ... }`, `name<T>(...): T { ... }`.
        if has_body && !NOT_METHOD_NAMES.contains(&first.text) {
            let after_generics = skip_angle_brackets(header, 1);
            let after_optional = if header.get(after_generics).is_some_and(|t| t.is_punct('?')) {
                after_generics + 1
            } else {
                after_generics
            };
            if first_paren(header, 1) == Some(after_optional) {
                return HeaderKind::Function(vec![first.text.to_owned()]);
            }
        }
        HeaderKind::Other
    }
}

/// The `javascript` adapter (JavaScript and TypeScript).
pub fn adapter() -> LexicalAdapter<JavaScript> {
    LexicalAdapter::new(JavaScript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"import { describe, expect, test } from "vitest";

// @vtest.src-id SRC-JS-ADD
export function add(a: number, b: number): number {
  return a + b;
}

export const twice = (a: number) => a * 2;
const label = `{ ${"not"} code`;
export default async function* stream() { yield 1 }

export class Calc<T> {
  private base = 0;
  @memoize()
  sum(values: number[]): number {
    if (values.length === 0) { return this.base; }
    return values.reduce((x, y) => x + y, this.base);
  }
  get size() { return 1; }
  onClick = (event: Event) => { this.base += 1 };
}

namespace Shapes.Circles {
  export function area(r: number) { return Math.PI * r * r }
}

const api = {
  ping() { return "pong" },
};

describe("Calc", () => {
  // @vtest.id TEST-JS-ADD
  // @vtest.covers VO-JS-ADD
  // @vtest.target src/calc.ts::add
  // @vtest.intent adding two numbers returns their sum
  test("adds", () => {
    expect(add(1, 2)).toBe(3)
  })
})
"#;

    /// @vtest.id TEST-ADAPTER-JAVASCRIPT-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-JAVASCRIPT-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/javascript.rs::JavaScript::classify
    /// @vtest.intent verifies JS/TS function declarations, arrow-function variables and fields, class methods, and namespace functions become scope-qualified Source Targets while plain fields and calls do not, and an annotated test call nested in describe is a Test construct
    #[test]
    fn discovers_functions_and_nested_test_calls() {
        let outcome = discover_text(&JavaScript, "src/calc.ts", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/calc.ts::add",
                "src/calc.ts::twice",
                "src/calc.ts::stream",
                "src/calc.ts::Calc::sum",
                "src/calc.ts::Calc::size",
                "src/calc.ts::Calc::onClick",
                "src/calc.ts::Shapes::Circles::area",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-JS-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-JS-ADD");
        assert_eq!(test.location.locator, "test#1");
        assert!(test.construct_text.starts_with("test(\"adds\", () => {"));
        assert!(test.construct_text.ends_with("toBe(3)\n  })"));
    }
}
