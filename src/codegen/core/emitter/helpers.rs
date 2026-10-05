//! Formatting, naming, pattern, and variant-emission helpers.

use super::*;
use crate::ast::Comment;

/// Appends `value` to `out` in a position that can only regroup it across
/// a comma — an initializer, an assignment right-hand side, a `return`
/// operand, or a single call argument — wrapping it in parentheses only
/// when it has a top-level comma to protect.
///
/// The parentheses codegen writes around a lowered value are grouping, not
/// syntax: everything else in the expression binds tighter than the
/// position it lands in, so the pair is noise the reader has to see past.
/// A value whose text is not resolved yet (it carries layout breaks, so it
/// is a lowering rather than one expression) keeps its parentheses.
pub(super) fn push_grouped<'a>(out: &mut Rope<'a>, value: Rope<'a>, kind: SourceKind) {
    if needs_grouping(&value, kind) {
        out.push_lit("(");
        out.append(value);
        out.push_lit(")");
    } else {
        out.append(value);
    }
}

/// Appends `value` as the receiver of a postfix step (`value.map(f)`).
/// Member access binds tighter than every operator, so the parentheses are
/// needed unless the receiver is already one primary expression; a receiver
/// ending in an optional chain keeps them too, because the step would
/// otherwise join the chain and be skipped when it short-circuits, where
/// the step applies to the value the chain evaluates to.
pub(super) fn push_receiver<'a>(out: &mut Rope<'a>, value: Rope<'a>, kind: SourceKind) {
    let closed = value
        .resolved_text()
        .is_some_and(|text| crate::lexer::is_member_receiver(&text, 0, text.len(), kind));
    push_parenthesized_unless(out, value, closed);
}

/// Appends `value` as the callee of a call step (`f(value)`). A call binds
/// tighter than every operator, so the parentheses are needed unless the
/// callee is already one primary expression. A callee ending in an optional
/// chain keeps its chain: `x |> o?.m` is the optional call `o?.m(x)`.
pub(super) fn push_callee<'a>(out: &mut Rope<'a>, value: Rope<'a>, kind: SourceKind) {
    let primary = value
        .resolved_text()
        .is_some_and(|text| crate::lexer::is_primary_expression(&text, 0, text.len(), kind));
    push_parenthesized_unless(out, value, primary);
}

fn push_parenthesized_unless<'a>(out: &mut Rope<'a>, value: Rope<'a>, bare: bool) {
    if bare {
        out.append(value);
    } else {
        out.push_lit("(");
        out.append(value);
        out.push_lit(")");
    }
}

/// Whether a value delivered to one of those positions has to keep the
/// parentheses codegen wraps it in. See [`push_grouped`].
pub(super) fn needs_grouping(value: &Rope<'_>, kind: SourceKind) -> bool {
    crate::work::tick("grouping checks");
    match value.resolved_text() {
        Some(text) => grouping_required(&text, kind),
        None => true,
    }
}

/// The same question about text codegen has not yet made a rope of.
pub(super) fn grouping_required(text: &str, kind: SourceKind) -> bool {
    crate::lexer::has_top_level_comma(text, 0, text.len(), kind)
}

/// Ends the line when `rope` finishes inside a `//` comment, so whatever
/// codegen appends next is not swallowed by it. `depth` is where the
/// continued line starts inside the enclosing lowering.
pub(super) fn guard_line_comment(
    mut rope: Rope<'_>,
    depth: u16,
    source_kind: SourceKind,
) -> Rope<'_> {
    if rope.last_line_has_line_comment(source_kind) {
        rope.push_break(depth);
        return Rope::scoped(rope);
    }
    rope
}

/// How a pattern's bindings are declared when they outlive the decision:
/// a let-else's `const`/`let`/`var`, exported when its source was.
#[derive(Clone, Copy)]
pub(super) struct Declaration {
    pub(super) mode: BindingMode,
    pub(super) exported: bool,
}

pub(super) fn binding_keyword(mode: BindingMode) -> &'static str {
    match mode {
        BindingMode::Const => "const",
        BindingMode::Let => "let",
        BindingMode::Var => "var",
    }
}

pub(super) fn temp_base(temp: TempId) -> String {
    match temp {
        TempId::Statement(sequence) => format!("$tt_t{sequence}"),
        TempId::Result(sequence) => format!("$tt_r{sequence}"),
        TempId::Decision { depth: 0 } => "$tt_m".to_owned(),
        TempId::Decision { depth } => format!("$tt_m_{depth}"),
        TempId::DecisionElement { index, depth: 0 } => format!("$tt_m{index}"),
        TempId::DecisionElement { index, depth } => format!("$tt_m{index}_{depth}"),
    }
}

pub(super) fn constructor_node(constructor: &Constructor) -> NodeId {
    match constructor {
        Constructor::Resolved { node, .. } | Constructor::Recovery { node, .. } => *node,
    }
}

pub(super) fn field_node(field: &FieldAccess) -> NodeId {
    match field {
        FieldAccess::Resolved { node, .. } | FieldAccess::Recovery { node, .. } => *node,
    }
}

pub(super) fn pattern_has_literal_test(plan: &PatternPlan) -> bool {
    match plan {
        PatternPlan::Test(Test::Literal { .. }) => true,
        PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => {
            parts.iter().any(pattern_has_literal_test)
        }
        PatternPlan::Any
        | PatternPlan::Bind(_)
        | PatternPlan::Test(Test::Variant { .. } | Test::InstanceOf { .. }) => false,
    }
}

pub(super) fn pattern_alternatives(plan: &PatternPlan) -> Vec<&PatternPlan> {
    match plan {
        PatternPlan::AnyOf(parts) => parts.iter().collect(),
        _ => vec![plan],
    }
}

pub(super) type BindingGroup<'a> = (Place, Vec<(&'a Bind, Option<&'a PatternPlan>)>);

pub(super) fn collect_binding_groups<'a>(
    plan: &'a PatternPlan,
    shared: Option<&'a PatternPlan>,
    groups: &mut Vec<BindingGroup<'a>>,
) {
    crate::stack::grow(|| collect_binding_groups_grown(plan, shared, groups));
}

fn collect_binding_groups_grown<'a>(
    plan: &'a PatternPlan,
    shared: Option<&'a PatternPlan>,
    groups: &mut Vec<BindingGroup<'a>>,
) {
    match plan {
        PatternPlan::Bind(binding) => {
            let mut receiver = binding.source.clone();
            receiver.fields.pop();
            if let Some((_, bindings)) = groups
                .iter_mut()
                .find(|(existing, _)| same_place(existing, &receiver))
            {
                bindings.push((binding, shared));
            } else {
                groups.push((receiver, vec![(binding, shared)]));
            }
        }
        PatternPlan::AllOf(parts) => {
            for part in parts
                .iter()
                .filter(|part| matches!(part, PatternPlan::Bind(_)))
            {
                collect_binding_groups(part, shared, groups);
            }
            for part in parts
                .iter()
                .filter(|part| !matches!(part, PatternPlan::Bind(_)))
            {
                collect_binding_groups(part, shared, groups);
            }
        }
        PatternPlan::AnyOf(parts) => {
            if let Some(first) = parts.first() {
                collect_binding_groups(first, shared.or(Some(plan)), groups);
            }
        }
        PatternPlan::Any | PatternPlan::Test(_) => {}
    }
}

pub(super) fn every_binding<'a>(plan: &'a PatternPlan, out: &mut Vec<&'a Bind>) {
    crate::stack::grow(|| every_binding_grown(plan, out));
}

fn every_binding_grown<'a>(plan: &'a PatternPlan, out: &mut Vec<&'a Bind>) {
    match plan {
        PatternPlan::Bind(binding) => out.push(binding),
        PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => {
            for part in parts {
                every_binding(part, out);
            }
        }
        PatternPlan::Any | PatternPlan::Test(_) => {}
    }
}

pub(super) fn same_place(left: &Place, right: &Place) -> bool {
    left.subject == right.subject
        && left.fields.len() == right.fields.len()
        && left
            .fields
            .iter()
            .zip(&right.fields)
            .all(|(left, right)| field_node(left) == field_node(right))
}

pub(super) struct BindingRecovery {
    available: HashSet<String>,
    emitted: HashSet<String>,
    discard_sequence: usize,
}

impl BindingRecovery {
    pub(super) fn new(emitter: &Emitter<'_>, plan: &PatternPlan) -> BindingRecovery {
        let selected = if let PatternPlan::AnyOf(parts) = plan {
            parts.first().unwrap_or(plan)
        } else {
            plan
        };
        let mut groups = Vec::new();
        collect_binding_groups(
            selected,
            matches!(plan, PatternPlan::AnyOf(_)).then_some(plan),
            &mut groups,
        );
        let available = groups
            .into_iter()
            .flat_map(|(_, bindings)| bindings)
            .map(|(binding, _)| emitter.source_node(binding.binding).0.to_owned())
            .collect();
        BindingRecovery {
            available,
            emitted: HashSet::new(),
            discard_sequence: 0,
        }
    }

    pub(super) fn replacement(&mut self, emitter: &Emitter<'_>, binding: &Bind) -> Option<String> {
        let name = emitter.source_node(binding.binding).0;
        if self.emitted.insert(name.to_owned()) {
            return None;
        }
        loop {
            let candidate =
                emitter.generated_name(&format!("$tt_discard{}", self.discard_sequence));
            self.discard_sequence += 1;
            if self.available.insert(candidate.clone()) {
                return Some(candidate);
            }
        }
    }
}

/// The one property name an object literal's `name: value` does not define:
/// it sets the object's prototype instead (ECMA-262 B.3.1, `__proto__`
/// Property Names in Object Initializers). A computed key, or shorthand,
/// defines an own data property.
const PROTOTYPE_SETTER_NAME: &str = "__proto__";

/// The union type and constructor object one tt `variant` becomes, laid out
/// from the line the declaration sits on.
///
/// A field's type is the user's TypeScript: each place it is written, in
/// the union and in the constructor's parameters, copies it from `source`,
/// so every question about it reaches the checker and every answer maps
/// back to it.
pub(super) fn emit_adt<'a>(
    adt: &Adt,
    source: &'a str,
    span: impl Fn(NodeId) -> hir::Span,
    import: impl Fn(&Import, &mut Rope<'a>),
    ambient: bool,
    source_kind: crate::SourceKind,
) -> Rope<'a> {
    let declared = |out: &mut Rope<'a>, name: &str, node: NodeId| {
        let span = span(node);
        out.push_declared_name(name.to_owned(), span.start, span.end);
    };
    let export = match (adt.exported, adt.declared) {
        (true, true) => "export declare ",
        (true, false) => "export ",
        (false, true) => "declare ",
        (false, false) => "",
    };
    let ambient = ambient || adt.declared;
    let annotation = |field: &AdtField, out: &mut Rope<'a>| {
        out.push_lit(if field.optional { "?: " } else { ": " });
        let mut at = field.ty_span.start;
        for specifier in &field.imports {
            let written = span(specifier.specifier);
            out.push_src(&source[at..written.start], at);
            import(specifier, out);
            at = written.end;
        }
        out.push_src(&source[at..field.ty_span.end], at);
    };
    let field_list = |fields: &[AdtField], separator: &str, out: &mut Rope<'a>| {
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                out.push_lit(separator.to_owned());
            }
            declared(out, &field.name, field.node);
            annotation(field, out);
        }
    };
    let parameter = |field: &AdtField, out: &mut Rope<'a>| {
        let start = span(field.node).start;
        let end = field.ty_span.end;
        let mut declaration = Rope::new();
        declaration.push_lit(field.name.clone());
        annotation(field, &mut declaration);
        out.anchored(AnchorKind::Variant, start, end, end, declaration);
    };
    let parameter_list = |fields: &[AdtField], out: &mut Rope<'a>| {
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                out.push_lit(", ");
            }
            parameter(field, out);
        }
    };
    let generics = &source[adt.generics.start..adt.generics.end];
    let type_args = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generic_param_names(generics).join(", "))
    };
    let push_generics = |out: &mut Rope<'a>| out.push_src(generics, adt.generics.start);
    // In a `.tsx` file `<T>(` would open a JSX element; a trailing comma in
    // the list makes it a type parameter list.
    let push_arrow_generics = |out: &mut Rope<'a>| {
        if source_kind != crate::SourceKind::Tsx || generics.is_empty() {
            push_generics(out);
            return;
        }
        let inner = &generics[..generics.len() - 1];
        let tokens = crate::lexer::lex(inner, 0, inner.len());
        if tokens
            .last()
            .is_some_and(|token| &inner[token.span.start..token.span.end] == ",")
        {
            push_generics(out);
        } else {
            out.push_src(inner, adt.generics.start);
            out.push_lit(",>");
        }
    };
    let parameters = |fields: &[AdtField], out: &mut Rope<'a>| {
        let documented = fields
            .iter()
            .any(|field| field.comments.leading.iter().any(Comment::is_doc));
        if !documented {
            parameter_list(fields, out);
            return;
        }
        for field in fields {
            for comment in field
                .comments
                .leading
                .iter()
                .filter(|comment| comment.is_doc())
            {
                out.push_break(2);
                push_comment(out, comment, 2);
            }
            out.push_break(2);
            parameter(field, out);
            out.push_lit(",");
        }
        out.push_break(1);
    };
    let mut out = Rope::new();
    out.push_lit(format!("{export}type "));
    declared(&mut out, &adt.name, adt.node);
    push_generics(&mut out);
    out.push_lit(" =");
    let last = adt.variants.len().saturating_sub(1);
    let (member, depth) = if last == 0 { ("", 1) } else { ("| ", 2) };
    for (index, variant) in adt.variants.iter().enumerate() {
        for comment in &variant.comments.leading {
            out.push_break(1);
            push_comment(&mut out, comment, 1);
        }
        out.push_break(1);
        match &variant.fields {
            Some(fields)
                if fields.iter().any(|field| {
                    !field.comments.leading.is_empty() || !field.comments.trailing.is_empty()
                }) =>
            {
                out.push_lit(format!("{member}{{"));
                out.push_break(depth + 1);
                out.push_lit(format!("kind: \"{}\";", variant.name));
                for field in fields {
                    for comment in &field.comments.leading {
                        out.push_break(depth + 1);
                        push_comment(&mut out, comment, depth + 1);
                    }
                    out.push_break(depth + 1);
                    declared(&mut out, &field.name, field.node);
                    annotation(field, &mut out);
                    out.push_lit(";");
                    push_trailing_comments(&mut out, &field.comments.trailing, depth + 1);
                }
                out.push_break(depth);
                out.push_lit("}");
            }
            Some(fields) if !fields.is_empty() => {
                out.push_lit(format!("{member}{{ kind: \"{}\"; ", variant.name));
                field_list(fields, "; ", &mut out);
                out.push_lit(" }");
            }
            _ => out.push_lit(format!("{member}{{ kind: \"{}\" }}", variant.name)),
        }
        if index == last {
            out.push_lit(";");
        }
        push_trailing_comments(&mut out, &variant.comments.trailing, 1);
    }
    out.push_break(0);
    out.push_lit(format!("{export}const "));
    declared(&mut out, &adt.name, adt.node);
    out.push_lit(if ambient { ": {" } else { " = {" });
    for variant in adt
        .variants
        .iter()
        .filter(|variant| variant.emit_constructor)
    {
        for comment in variant
            .comments
            .leading
            .iter()
            .filter(|comment| comment.is_doc())
        {
            out.push_break(1);
            push_comment(&mut out, comment, 1);
        }
        out.push_break(1);
        if ambient {
            out.push_lit("readonly ");
            declared(&mut out, &variant.name, variant.node);
            match &variant.fields {
                None => out.push_lit(format!(": {{ readonly kind: \"{}\" }};", variant.name)),
                Some(fields) => {
                    out.push_lit(": ");
                    push_generics(&mut out);
                    out.push_lit("(");
                    parameters(fields, &mut out);
                    out.push_lit(format!(") => {}{type_args};", adt.name));
                }
            }
            continue;
        }
        if variant.name == PROTOTYPE_SETTER_NAME {
            declared(&mut out, &format!("[\"{}\"]", variant.name), variant.node);
        } else {
            declared(&mut out, &variant.name, variant.node);
        }
        match &variant.fields {
            None => out.push_lit(format!(": {{ kind: \"{}\" }} as const,", variant.name)),
            Some(fields) => {
                let object = std::iter::once(format!("kind: \"{}\"", variant.name))
                    .chain(fields.iter().map(|field| {
                        if field.optional {
                            format!(
                                "...({} === undefined ? {{}} : {{ {} }})",
                                field.name, field.name
                            )
                        } else {
                            field.name.clone()
                        }
                    }))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_lit(": ");
                push_arrow_generics(&mut out);
                out.push_lit("(");
                parameters(fields, &mut out);
                out.push_lit(format!("): {}{type_args} => ({{ {object} }}),", adt.name));
            }
        }
    }
    out.push_break(0);
    out.push_lit("};");
    Rope::scoped(out)
}

fn push_comment<'a>(out: &mut Rope<'a>, comment: &Comment, depth: u16) {
    let mut lines = comment.text.lines();
    if let Some(first) = lines.next() {
        out.push_lit(first.to_owned());
    }
    for line in lines {
        out.push_break(depth);
        let indent = line
            .bytes()
            .take(comment.column)
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count();
        out.push_lit(line[indent..].to_owned());
    }
}

fn push_trailing_comments<'a>(out: &mut Rope<'a>, comments: &[Comment], depth: u16) {
    for comment in comments {
        if comment.own_line {
            out.push_break(depth);
        } else {
            out.push_lit(" ");
        }
        push_comment(out, comment, depth);
    }
}

pub(super) fn generic_param_names(generics: &str) -> Vec<String> {
    crate::lexer::type_parameter_names(generics)
}
