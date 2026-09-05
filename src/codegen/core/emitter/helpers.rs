//! Formatting, naming, pattern, and variant-emission helpers.

use super::*;

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
pub(super) fn push_grouped<'a>(out: &mut Rope<'a>, value: Rope<'a>) {
    if needs_grouping(&value) {
        out.push_lit("(");
        out.append(value);
        out.push_lit(")");
    } else {
        out.append(value);
    }
}

/// Appends `value` as the receiver of a postfix step (`value.map(f)`).
/// Member access binds tighter than every operator, so the parentheses are
/// needed unless the receiver is already one primary expression.
pub(super) fn push_receiver<'a>(out: &mut Rope<'a>, value: Rope<'a>) {
    let primary = value
        .resolved_text()
        .is_some_and(|text| crate::scanner::is_primary_expression(text.as_bytes(), 0, text.len()));
    if primary {
        out.append(value);
    } else {
        out.push_lit("(");
        out.append(value);
        out.push_lit(")");
    }
}

/// Whether a value delivered to one of those positions has to keep the
/// parentheses codegen wraps it in. See [`push_grouped`].
pub(super) fn needs_grouping(value: &Rope<'_>) -> bool {
    match value.resolved_text() {
        Some(text) => grouping_required(&text),
        None => true,
    }
}

/// The same question about text codegen has not yet made a rope of.
pub(super) fn grouping_required(text: &str) -> bool {
    crate::scanner::has_top_level_comma(text.as_bytes(), 0, text.len())
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

pub(super) fn binding_keyword(mode: BindingMode) -> &'static str {
    match mode {
        BindingMode::Const => "const",
        BindingMode::Let => "let",
        BindingMode::Var => "var",
    }
}

pub(super) fn temp_name(temp: TempId) -> String {
    match temp {
        TempId::Statement(sequence) => format!("$tt_t{sequence}"),
        TempId::Result(sequence) => format!("$tt_r{sequence}"),
        TempId::Decision => "$tt_m".to_owned(),
        TempId::DecisionElement(sequence) => format!("$tt_m{sequence}"),
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

pub(super) fn pattern_has_test(plan: &PatternPlan) -> bool {
    match plan {
        PatternPlan::Any | PatternPlan::Bind(_) => false,
        PatternPlan::Test(_) => true,
        PatternPlan::AllOf(parts) | PatternPlan::AnyOf(parts) => parts.iter().any(pattern_has_test),
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

pub(super) type BindingGroup<'a> = (Place, Vec<(&'a Bind, bool)>);

pub(super) fn collect_binding_groups<'a>(
    plan: &'a PatternPlan,
    mapped: bool,
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
                bindings.push((binding, mapped));
            } else {
                groups.push((receiver, vec![(binding, mapped)]));
            }
        }
        PatternPlan::AllOf(parts) => {
            for part in parts
                .iter()
                .filter(|part| matches!(part, PatternPlan::Bind(_)))
            {
                collect_binding_groups(part, mapped, groups);
            }
            for part in parts
                .iter()
                .filter(|part| !matches!(part, PatternPlan::Bind(_)))
            {
                collect_binding_groups(part, mapped, groups);
            }
        }
        PatternPlan::AnyOf(parts) => {
            if let Some(first) = parts.first() {
                collect_binding_groups(first, false, groups);
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
            !matches!(plan, PatternPlan::AnyOf(_)),
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
            let candidate = format!("$tt_discard{}", self.discard_sequence);
            self.discard_sequence += 1;
            if self.available.insert(candidate.clone()) {
                return Some(candidate);
            }
        }
    }
}

/// The union type and constructor object one tt `variant` becomes, laid out
/// from the line the declaration sits on.
pub(super) fn emit_adt<'a>(adt: &Adt) -> Rope<'a> {
    let export = if adt.exported { "export " } else { "" };
    let arms = adt
        .variants
        .iter()
        .map(|variant| match &variant.fields {
            Some(fields) if !fields.is_empty() => format!(
                "{{ kind: \"{}\"; {} }}",
                variant.name,
                fields
                    .iter()
                    .map(|field| format!(
                        "{}{}: {}",
                        field.name,
                        if field.optional { "?" } else { "" },
                        field.ty_text
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            _ => format!("{{ kind: \"{}\" }}", variant.name),
        })
        .collect::<Vec<_>>();
    let type_args = if adt.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generic_param_names(&adt.generics).join(", "))
    };
    let constructors = adt
        .variants
        .iter()
        .filter_map(|variant| {
            if !variant.emit_constructor {
                return None;
            }
            Some(match &variant.fields {
                None => format!(
                    "{}: {{ kind: \"{}\" }} as const,",
                    variant.name, variant.name
                ),
                Some(fields) => {
                    let params = fields
                        .iter()
                        .map(|field| {
                            format!(
                                "{}{}: {}",
                                field.name,
                                if field.optional { "?" } else { "" },
                                field.ty_text
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let object = std::iter::once(format!("kind: \"{}\"", variant.name))
                        .chain(fields.iter().map(|field| field.name.clone()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "{}: {}({params}): {}{type_args} => ({{ {object} }}),",
                        variant.name, adt.generics, adt.name
                    )
                }
            })
        })
        .collect::<Vec<_>>();
    let mut out = Rope::new();
    out.push_lit(format!("{export}type {}{} =", adt.name, adt.generics));
    for arm in arms {
        out.push_break(1);
        out.push_lit(format!("| {arm}"));
    }
    out.push_lit(";");
    out.push_break(0);
    out.push_lit(format!("{export}const {} = {{", adt.name));
    for constructor in constructors {
        out.push_break(1);
        out.push_lit(constructor);
    }
    out.push_break(0);
    out.push_lit("};");
    Rope::scoped(out)
}

pub(super) fn generic_param_names(generics: &str) -> Vec<String> {
    let inner = &generics[1..generics.len() - 1];
    let source = inner.as_bytes();
    let mut names = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        index = skip_ws_comments(source, index, source.len());
        if index >= source.len() || !is_ident_start(source[index]) {
            break;
        }
        let end = ident_end(source, index, source.len());
        let word = &inner[index..end];
        if word == "const" || word == "in" || word == "out" {
            index = end;
            continue;
        }
        names.push(word.to_owned());
        index = scan_type_end(source, end, source.len());
        if at(source, index, source.len()) == Some(b',') {
            index += 1;
        }
    }
    names
}
