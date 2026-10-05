//! Fixed public-wire answers for service response projections.

use super::*;
use serde_json::json;

fn project(files: &[(&str, &str)]) -> Workspace {
    let dir = Workspace::in_repo("server-response-shapes");
    fs::write(
        dir.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"target":"esnext","module":"esnext","moduleResolution":"bundler","noEmit":true},"include":["*.tt"]}"#,
    )
    .unwrap();
    for (name, source) in files {
        fs::write(dir.join(name), source).unwrap();
    }
    dir
}

fn answers(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let input: String = requests
        .iter()
        .map(|request| format!("{request}\n"))
        .collect();
    let (lines, status) = server_lines(input.as_bytes());
    assert!(status.success(), "{lines:?}");
    assert_eq!(lines.len(), requests.len());
    lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn completion_and_resolve_preserve_optional_wire_fields() {
    if !common::toolchain() {
        eprintln!("SKIP server response shapes: no TypeScript installed");
        return;
    }
    let dir = project(&[
        (
            "clean.tt",
            r#"export const value = 1;
"#,
        ),
        (
            "empty.tt",
            r#"export const empty = {};
empty.x;
"#,
        ),
        (
            "members.tt",
            r#"interface Box {
  plain: string;
  /** @deprecated Use plain. */
  old: number;
}
declare const box: Box;
box.plain;
export {};
"#,
        ),
    ]);
    let requests = [
        json!({
            "id": 0,
            "method": "completion",
            "params": {
                "position": {
                    "line": 6,
                    "character": 4
                },
                "member": true,
                "triggerCharacter": ".",
                "path": dir.join("members.tt")
            }
        }),
        json!({
            "id": 1,
            "method": "completionResolve",
            "params": {
                "position": {
                    "line": 6,
                    "character": 4
                },
                "label": "plain",
                "path": dir.join("members.tt")
            }
        }),
        json!({
            "id": 2,
            "method": "completionResolve",
            "params": {
                "position": {
                    "line": 6,
                    "character": 4
                },
                "label": "missing",
                "path": dir.join("members.tt")
            }
        }),
        json!({
            "id": 3,
            "method": "completion",
            "params": {
                "position": {
                    "line": 1,
                    "character": 6
                },
                "member": true,
                "path": dir.join("empty.tt")
            }
        }),
        json!({"id": 4, "method": "completion", "params": {}}),
        json!({"id": 5, "method": "tsDiagnostics", "params": {"path": dir.join("clean.tt")}}),
    ];
    assert_eq!(
        answers(&requests),
        vec![
            json!({
                "id": 0,
                "result": {
                    "items": [
                        {
                            "detail": null,
                            "filterText": null,
                            "insertText": null,
                            "kind": 5,
                            "label": "plain",
                            "labelDetails": null,
                            "range": null,
                            "snippet": false,
                            "sortText": "11",
                            "source": null,
                            "tags": []
                        },
                        {
                            "detail": null,
                            "filterText": null,
                            "insertText": null,
                            "kind": 5,
                            "label": "old",
                            "labelDetails": null,
                            "range": null,
                            "snippet": false,
                            "sortText": "z11",
                            "source": null,
                            "tags": [
                                1
                            ]
                        }
                    ],
                    "member": true,
                    "probe": null
                }
            }),
            json!({"id": 1, "result": {"documentation": "", "signature": "(property) Box.plain: string"}}),
            json!({"id": 2, "result": null}),
            json!({"id": 3, "result": {"items": [], "member": true, "probe": 1}}),
            json!({"error": "the request needs a \"path\"", "id": 4}),
            json!({"id": 5, "result": {"diagnostics": [], "restates": [], "retains": []}}),
        ]
    );
}

#[test]
fn signature_and_diagnostics_preserve_wire_values_and_omissions() {
    if !common::toolchain() {
        eprintln!("SKIP server response shapes: no TypeScript installed");
        return;
    }
    let dir = project(&[
        (
            "call.tt",
            r#"/** Adds values.
 * @param first first value
 * @param second second value
 */
declare function combine(first: number, second: string): string;
combine(1, "x");
export {};
"#,
        ),
        (
            "clean.tt",
            r#"export const value = 1;
"#,
        ),
        (
            "diagnostics.tt",
            r#"/** @deprecated Use current. */
function old(): void {}
export function use(): void {
  const unused = 1;
  old();
}
export const wrong: number = "x";
interface Shape { required: number; }
export const missing: Shape = {};
"#,
        ),
        (
            "unicode.tt",
            r#"declare function emoji(first: "😀", second: string): string;
emoji("😀", "x");
export {};
"#,
        ),
    ]);
    let requests = [
        json!({
            "id": 0,
            "method": "signatureHelp",
            "params": {
                "position": {
                    "line": 1,
                    "character": 12
                },
                "path": dir.join("unicode.tt")
            }
        }),
        json!({
            "id": 1,
            "method": "signatureHelp",
            "params": {
                "position": {
                    "line": 6,
                    "character": 0
                },
                "path": dir.join("call.tt")
            }
        }),
        json!({"id": 2, "method": "tsDiagnostics", "params": {"path": dir.join("diagnostics.tt")}}),
        json!({"id": 3, "method": "tsDiagnostics", "params": {"path": dir.join("clean.tt")}}),
    ];
    assert_eq!(
        answers(&requests),
        vec![
            json!({
                "id": 0,
                "result": {
                    "activeParameter": 1,
                    "activeSignature": 0,
                    "signatures": [
                        {
                            "documentation": "",
                            "label": "emoji(first: \"😀\", second: string): string",
                            "parameters": [
                                {
                                    "documentation": "",
                                    "label": [
                                        6,
                                        17
                                    ]
                                },
                                {
                                    "documentation": "",
                                    "label": [
                                        19,
                                        33
                                    ]
                                }
                            ]
                        }
                    ]
                }
            }),
            json!({"id": 1, "result": null}),
            json!({
                "id": 2,
                "result": {
                    "diagnostics": [
                        {
                            "code": 2322,
                            "message": "Type 'string' is not assignable to type 'number'.",
                            "range": {
                                "end": {
                                    "character": 18,
                                    "line": 6
                                },
                                "start": {
                                    "character": 13,
                                    "line": 6
                                }
                            },
                            "severity": "error"
                        },
                        {
                            "code": 2741,
                            "message": "Property 'required' is missing in type '{}' but required in type 'Shape'.",
                            "range": {
                                "end": {
                                    "character": 20,
                                    "line": 8
                                },
                                "start": {
                                    "character": 13,
                                    "line": 8
                                }
                            },
                            "related": [
                                {
                                    "message": "'required' is declared here.",
                                    "range": {
                                        "end": {
                                            "character": 26,
                                            "line": 7
                                        },
                                        "start": {
                                            "character": 18,
                                            "line": 7
                                        }
                                    }
                                }
                            ],
                            "severity": "error"
                        },
                        {
                            "code": 6133,
                            "message": "'unused' is declared but its value is never read.",
                            "range": {
                                "end": {
                                    "character": 14,
                                    "line": 3
                                },
                                "start": {
                                    "character": 8,
                                    "line": 3
                                }
                            },
                            "severity": "hint",
                            "tags": [
                                "unnecessary"
                            ]
                        },
                        {
                            "code": 6387,
                            "message": "The signature '(): void' of 'old' is deprecated.",
                            "range": {
                                "end": {
                                    "character": 5,
                                    "line": 4
                                },
                                "start": {
                                    "character": 2,
                                    "line": 4
                                }
                            },
                            "related": [
                                {
                                    "message": "The declaration was marked as deprecated here.",
                                    "range": {
                                        "end": {
                                            "character": 29,
                                            "line": 0
                                        },
                                        "start": {
                                            "character": 4,
                                            "line": 0
                                        }
                                    }
                                }
                            ],
                            "severity": "hint",
                            "tags": [
                                "deprecated"
                            ]
                        }
                    ],
                    "restates": [], "retains": []
                }
            }),
            json!({"id": 3, "result": {"diagnostics": [], "restates": [], "retains": []}}),
        ]
    );
}
