use super::*;

#[test]
fn result_shape_probe_names_the_copied_return_expression() {
    let source = "variant R { Ok(value: number), Err(error: string) }\n\
             const value = result { const n = try read(); return R.Ok(n); };\n";
    let emit = compile_mapped(source, &Options::default()).expect("compile");
    assert_eq!(emit.result_return_temps.len(), 1, "{}", emit.code);
    let probe = emit.result_return_temps[0];
    assert_eq!(&emit.code[probe.out..probe.out_end], "R.Ok(n)");
    assert_eq!(&source[probe.src..probe.src_end], "R.Ok(n)");
}
