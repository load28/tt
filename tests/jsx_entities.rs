//! Exercise the parser dependency directly, without tt preflight or panic recovery.
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};
use swc_ecma_visit::{Visit, VisitWith};

fn parse(source: &str) -> Result<swc_ecma_ast::Module, swc_ecma_parser::error::Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(Lrc::new(FileName::Anon), source.to_owned());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module()?;
    match parser.take_errors().into_iter().next() {
        Some(error) => Err(error),
        None => Ok(module),
    }
}

#[derive(Default)]
struct JsxValues(Vec<String>);

impl Visit for JsxValues {
    fn visit_jsx_text(&mut self, text: &swc_ecma_ast::JSXText) {
        self.0.push(text.value.to_string_lossy().into_owned());
    }

    fn visit_jsx_attr(&mut self, attr: &swc_ecma_ast::JSXAttr) {
        if let Some(swc_ecma_ast::JSXAttrValue::Str(value)) = &attr.value {
            self.0.push(value.value.to_string_lossy().into_owned());
        }
    }
}

#[test]
fn unrecognized_numeric_entities_are_literal_jsx_text() {
    for entity in [
        "&#;",
        "&#x;",
        "&#1114112;",
        "&#x110000;",
        "&#4294967296;",
        "&#xFFFFFFFFF;",
    ] {
        for source in [
            format!("const x = <div>{entity}</div>;"),
            format!("const x = <div title=\"{entity}\" />;"),
        ] {
            let module = parse(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let mut values = JsxValues::default();
            module.visit_with(&mut values);
            assert_eq!(
                values.0,
                [entity],
                "unrecognized references must retain their value"
            );
            let emitted = ttc::compile(
                &source,
                &ttc::Options {
                    source_kind: ttc::SourceKind::Tsx,
                    ..Default::default()
                },
            )
            .expect(&source);
            assert_eq!(emitted, source);
        }
    }
}

#[test]
fn original_incomplete_jsx_reproducer_does_not_unwind() {
    let source = "<>&>&w=<>&>&w=2&(&#;;\\w\u{1}";
    assert!(parse(source).is_err());
}

#[test]
fn jsx_entities_and_literal_ampersands_preserve_valid_source() {
    for entity in [
        "&amp;",
        "&#65;",
        "&#x41;",
        "&#128512;",
        "&#x10FFFF;",
        "&#0;",
        "&#xD800;&#xDC00;",
        "&#word;",
        "&#xZZ;",
        "&#65",
        "hello 한글",
    ] {
        for source in [
            format!("const x = <div>{entity}</div>;"),
            format!("const x = <div title=\"{entity}\" />;"),
        ] {
            parse(&source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let emitted = ttc::compile(
                &source,
                &ttc::Options {
                    source_kind: ttc::SourceKind::Tsx,
                    ..Default::default()
                },
            )
            .expect(&source);
            assert_eq!(emitted, source);
        }
    }
}
