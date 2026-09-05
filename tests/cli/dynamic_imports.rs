use super::*;

#[test]
fn mixed_source_dynamic_imports_resolve_and_execute() {
    if !have("tsc") || !have("bun") || !have("node") {
        return;
    }
    let dir = tmpdir();
    let source = dir.join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("jsx.ts"), r#"
export function h(_tag: string, props: {n: number}): number { return props.n; }
declare global { namespace JSX { type Element = number; interface IntrinsicElements { value: {n: number} } } }
"#).unwrap();
    let kinds = [("ts", "ts"), ("tsx", "tsx"), ("tt", "ts"), ("ttx", "tsx")];
    for (index, (extension, _)) in kinds.iter().enumerate() {
        let value = if extension.starts_with("tt") {
            format!("match (true) {{ true => {}, _ => 0 }}", index + 1)
        } else {
            (index + 1).to_string()
        };
        let producer = if extension.ends_with('x') {
            format!("import {{h}} from './jsx.js';\nexport const value = <value n={{{value}}}/>;\n")
        } else {
            format!("export const value = {value};\n")
        };
        fs::write(
            source.join(format!("producer-{extension}.{extension}")),
            producer,
        )
        .unwrap();
        let imports = kinds
            .iter()
            .map(|(kind, emitted)| {
                let specifier = if kind.starts_with("tt") {
                    *kind
                } else if *emitted == "tsx" {
                    "jsx"
                } else {
                    "js"
                };
                format!("(await import('./producer-{kind}.{specifier}')).value")
            })
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            source.join(format!("consumer-{extension}.{extension}")),
            format!("export async function read() {{ return [{imports}]; }}\n"),
        )
        .unwrap();
    }
    let entry = kinds
        .iter()
        .enumerate()
        .map(|(index, (kind, emitted))| {
            let specifier = if kind.starts_with("tt") {
                *kind
            } else if *emitted == "tsx" {
                "jsx"
            } else {
                "js"
            };
            format!("import {{read as read{index}}} from './consumer-{kind}.{specifier}';\n")
        })
        .collect::<String>();
    fs::write(source.join("entry.ts"), format!("{entry}console.log(JSON.stringify(await Promise.all([read0(), read1(), read2(), read3()])));\n")).unwrap();
    for mode in ["js", "ts"] {
        let emitted = dir.join(mode);
        let output = ttc(&[
            "--no-banner",
            "--rewrite-imports",
            mode,
            "-o",
            emitted.to_str().unwrap(),
            source.to_str().unwrap(),
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = Command::new("tsc")
            .arg(emitted.join("entry.ts"))
            .args([
                "--noEmit",
                "--strict",
                "--target",
                "es2022",
                "--module",
                "esnext",
                "--moduleResolution",
                "bundler",
                "--jsx",
                "preserve",
                "--allowImportingTsExtensions",
                "--skipLibCheck",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::write(
            emitted.join("tsconfig.json"),
            r#"{"compilerOptions":{"jsx":"react","jsxFactory":"h"}}"#,
        )
        .unwrap();
        let bundle = dir.join(format!("{mode}.mjs"));
        let output = Command::new("bun")
            .args(["build"])
            .arg(emitted.join("entry.ts"))
            .args(["--target", "node", "--outfile"])
            .arg(&bundle)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = Command::new("node").arg(bundle).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "[[1,2,3,4],[1,2,3,4],[1,2,3,4],[1,2,3,4]]"
        );
    }
}
