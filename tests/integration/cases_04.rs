/* ------------------------------------------------------------------ */
/* result computation block                                            */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_using_disposes_when_try_propagates_err() {
    require_toolchain!();
    let lines = run_with_tsc_flags(
        r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const events: string[] = [];
const fail = (): Res<number, string> => Res.Err("boom");

const sync = () => {
  using resource = {
    [Symbol.dispose]() { events.push("sync-dispose"); },
  };
  const value = try fail();
  return Res.Ok(value);
};

const asyncRun = async () => {
  await using resource = {
    async [Symbol.asyncDispose]() { events.push("async-dispose"); },
  };
  const value = try fail();
  return Res.Ok(value);
};

console.log(JSON.stringify(sync()), events.join(","));
events.length = 0;
asyncRun().then((value) => console.log(JSON.stringify(value), events.join(",")));
"#,
        &["--lib", "es2022,dom,esnext.disposable"],
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Err","error":"boom"} sync-dispose"#,
            r#"{"kind":"Err","error":"boom"} async-dispose"#,
        ]
    );
}

#[test]
fn runtime_nested_results_preserve_constructor_and_generator_protocols() {
    require_toolchain!();
    for source in [
        "class C { constructor() { try fail(); } }\n",
        "function* values() { yield try fail(); }\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{source}\n{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
    }

    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const fail = (): Res<number, string> => Res.Err("boom");

class C {
  outcome;
  constructor() {
    this.outcome = result { return try fail(); };
  }
}

function* values() {
  yield result { return try fail(); };
  yield "after";
}

const instance = new C();
console.log(instance instanceof C, JSON.stringify(instance.outcome));
const iterator = values();
console.log(JSON.stringify(iterator.next()));
console.log(JSON.stringify(iterator.next()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"true {"kind":"Err","error":"boom"}"#,
            r#"{"value":{"kind":"Err","error":"boom"},"done":false}"#,
            r#"{"value":"after","done":false}"#,
            r#"{"kind":"Err","error":"boom"},"after""#,
        ]
    );
}

#[test]
fn runtime_result_block_replaces_nested_combinator_callbacks() {
    require_toolchain!();
    // The motivating shape: three dependent steps that all stay in scope,
    // written flat, against the real standard library.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type User = { id: number; companyId: number; name: string };
type Company = { id: number; name: string };

const getUser = (id: number): TResult<User, string> =>
  id === 1 ? Result.Ok({ id, companyId: 7, name: " Ada " }) : Result.Err("no user " + id);
const getCompany = (id: number): TResult<Company, string> =>
  Result.Ok({ id, name: "Acme" });
const getPermission = (u: User, c: Company): TResult<string, string> =>
  Result.Ok(u.name.trim() + "@" + c.name);

const view = (id: number) => result {
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  const normalized = user.name |> .trim() |> .toLowerCase();
  const permission = try getPermission(user, company);
  return { user, company, permission, normalized };
};

console.log(JSON.stringify(view(1)));
console.log(JSON.stringify(view(2)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":{"user":{"id":1,"companyId":7,"name":" Ada "},"company":{"id":7,"name":"Acme"},"permission":"Ada@Acme","normalized":"ada"}}"#,
            r#"{"kind":"Err","error":"no user 2"}"#,
        ]
    );
}
