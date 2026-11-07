//! `ruby`: Ruby sources (`.rb`, `.rake`).
//!
//! Functions are `def` methods (singleton methods as `self.name`, operator
//! and setter methods, and endless `def name = expr` methods included).
//! `class` and `module` (`A::B` names) qualify them. Minitest methods,
//! RSpec examples (`it "adds" do ... end`), test-unit, or any other style
//! are discovered the same way, because a Test is whatever follows its
//! `@vtest.` comment.

use crate::{
    header::{HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "ruby";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["#"],
    line_comment_exclusions: &[],
    block_comments: &[("=begin", "=end")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("\"", "\"", true),
        StringSyntax::multi("'", "'", true),
        StringSyntax::multi("`", "`", true),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &['?', '!'],
    newline_terminates: true,
    structure: Structure::KeywordEnd,
};

const VISIBILITY: &[&str] = &["private", "protected", "public", "module_function"];

#[derive(Clone, Copy, Debug, Default)]
pub struct Ruby;

impl LanguageProfile for Ruby {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["ruby"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rb", "rake"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["vendor", ".bundle", "tmp", "log", "coverage"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    /// `header` is the first line of a statement.
    fn classify(&self, header: &[HTok<'_>], _has_body: bool) -> HeaderKind {
        let mut header = header;
        while header.len() > 1
            && header[0].kind == TokKind::Word
            && VISIBILITY.contains(&header[0].text)
        {
            header = &header[1..];
        }
        let Some(first) = header.first() else {
            return HeaderKind::Other;
        };
        if first.is_word("def") {
            return def_name(&header[1..])
                .map_or(HeaderKind::Other, |name| HeaderKind::Function(vec![name]));
        }
        if first.is_word("class") || first.is_word("module") {
            let mut segments = Vec::new();
            let mut index = 1;
            while let Some(token) = header.get(index) {
                if token.kind != TokKind::Word {
                    break;
                }
                segments.push(token.text.to_owned());
                if header.get(index + 1).is_some_and(|t| t.is_punct(':'))
                    && header.get(index + 2).is_some_and(|t| t.is_punct(':'))
                {
                    index += 3;
                } else {
                    break;
                }
            }
            if !segments.is_empty() {
                return HeaderKind::Scope(segments);
            }
        }
        HeaderKind::Other
    }
}

/// The method name after `def`: `name`, `name?`, `self.name`, `name=`, or an
/// operator such as `==` or `[]`.
fn def_name(rest: &[HTok<'_>]) -> Option<String> {
    let first = rest.first()?;
    if first.kind == TokKind::Word {
        if rest.get(1).is_some_and(|t| t.is_punct('.')) {
            let name = rest.get(2).filter(|t| t.kind == TokKind::Word)?;
            return Some(format!("{}.{}", first.text, name.text));
        }
        let setter = rest.get(1).is_some_and(|t| t.is_punct('='))
            && rest.get(2).is_some_and(|t| t.is_open('('));
        return Some(if setter {
            format!("{}=", first.text)
        } else {
            first.text.to_owned()
        });
    }
    let operator = rest
        .iter()
        .take_while(|token| {
            matches!(
                token.kind,
                TokKind::Punct(_) | TokKind::Open('[') | TokKind::Close(']')
            )
        })
        .map(|token| token.text)
        .collect::<String>();
    (!operator.is_empty()).then_some(operator)
}

/// The `ruby` adapter.
pub fn adapter() -> LexicalAdapter<Ruby> {
    LexicalAdapter::new(Ruby)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"# frozen_string_literal: true

module Demo
  module Math
    class Calc < Base
      LABEL = "{ not
        code }"

      # @vtest.src-id SRC-RB-ADD
      def add(a, b)
        return 0 if a.nil?
        while a > 100 do a -= 1 end
        result = if b.zero? then a else a + b end
        result
      end

      def self.create = new

      def ==(other)
        other.is_a?(Calc) && other.range.end == range.end
      end

      def empty?
        [1, 2].each { |x| puts x unless x.zero? }
        {class: "x"}.empty?
      end

      private def secret
        :end
      end

=begin
def commented_out
=end
    end
  end
end

RSpec.describe Demo::Math::Calc do
  def helper(value)
    value
  end

  # @vtest.id TEST-RB-ADD
  # @vtest.covers VO-RB-ADD
  # @vtest.target lib/calc.rb::Demo::Math::Calc::add
  # @vtest.intent adding two numbers returns their sum
  it "adds two numbers" do
    expect(described_class.new.add(1, 2)).to eq(3)
  end
end
"#;

    /// @vtest.id TEST-ADAPTER-RUBY-DISCOVERS-METHODS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Ruby::classify
    /// @vtest.intent verifies Ruby methods (predicate, singleton, endless, operator, private) become module- and class-qualified Source Targets while modifier ifs, loop dos, keyword-named methods and symbols, and =begin blocks do not unbalance bodies, and an annotated RSpec example is a Test construct ending at its end
    #[test]
    fn discovers_ruby_methods_and_examples() {
        let outcome = discover_text(&Ruby, "lib/calc.rb", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "lib/calc.rb::Demo::Math::Calc::add",
                "lib/calc.rb::Demo::Math::Calc::self.create",
                "lib/calc.rb::Demo::Math::Calc::==",
                "lib/calc.rb::Demo::Math::Calc::empty?",
                "lib/calc.rb::Demo::Math::Calc::secret",
                "lib/calc.rb::helper",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-RB-ADD")
        );
        assert!(outcome.sources[0]
            .construct_text
            .ends_with("result\n      end"));
        assert_eq!(outcome.sources[1].construct_text, "def self.create = new");
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-RB-ADD");
        assert_eq!(test.location.locator, "it#1");
        assert!(test
            .construct_text
            .starts_with("it \"adds two numbers\" do"));
        assert!(test.construct_text.ends_with("eq(3)\n  end"));
    }
}
