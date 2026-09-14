use swc_common::input::StringInput;
use swc_common::sync::Lrc;
use swc_common::{BytePos, FileName, SourceFile, SourceMap};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::{Parser, Syntax, TsSyntax};

use crate::SourceKind;

pub(crate) struct HostInput {
    file: Lrc<SourceFile>,
    origin: HostOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HostOrigin {
    start: u32,
    trimmed: u32,
}

impl HostInput {
    pub(crate) fn new(text: &str) -> Self {
        let source_map: Lrc<SourceMap> = Default::default();
        let file = source_map.new_source_file(Lrc::new(FileName::Anon), text.to_owned());
        let trimmed = u32::try_from(text.len() - file.src.len()).unwrap_or(0);
        let origin = HostOrigin {
            start: file.start_pos.0,
            trimmed,
        };
        HostInput { file, origin }
    }

    pub(crate) fn parser(&self, source_kind: SourceKind) -> Parser<Lexer<'_>> {
        let lexer = Lexer::new(
            syntax(source_kind),
            Default::default(),
            StringInput::from(&*self.file),
            None,
        );
        Parser::new_from(lexer)
    }

    pub(crate) fn origin(&self) -> HostOrigin {
        self.origin
    }

    pub(crate) fn byte(&self, position: BytePos) -> usize {
        self.origin.byte(position)
    }
}

impl HostOrigin {
    pub(crate) fn byte(self, position: BytePos) -> usize {
        usize::try_from(position.0.saturating_sub(self.start)).unwrap_or(0) + self.trimmed as usize
    }
}

pub(crate) fn syntax(source_kind: SourceKind) -> Syntax {
    Syntax::Typescript(TsSyntax {
        tsx: source_kind.is_tsx(),
        decorators: true,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_bytes_of_the_text_given_including_a_byte_order_mark() {
        let plain = HostInput::new("const a = 1;\n");
        let with_bom = HostInput::new("\u{feff}const a = 1;\n");
        let module = plain.parser(SourceKind::TypeScript).parse_module().unwrap();
        let bom_module = with_bom
            .parser(SourceKind::TypeScript)
            .parse_module()
            .unwrap();
        use swc_common::Spanned;
        assert_eq!(plain.byte(module.body[0].span().lo), 0);
        assert_eq!(with_bom.byte(bom_module.body[0].span().lo), 3);
        assert_eq!(with_bom.byte(bom_module.body[0].span().hi), 15);
    }
}
