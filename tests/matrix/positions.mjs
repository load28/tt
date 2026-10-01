const noAsync = ["await", "yield"];

export const valuePositions = [
  {
    id: "declarationInitializer",
    title: "a const declaration's initializer",
    host: (c) => c.probe(`const value = ${c.v};\nreturn ${c.ret("value")};`),
  },
  {
    id: "laterDeclarator",
    title: "the later declarator of a declaration list",
    host: (c) => c.probe(`const before = note("before", 1), value = ${c.v};\nreturn ${c.ret("[before, value]")};`),
  },
  {
    id: "assignment",
    title: "the right side of an assignment to a property",
    host: (c) =>
      c.probe(`const target = { value: note("initial", 0) as unknown };\nnote("target", target).value = ${c.v};\nreturn ${c.ret("target.value")};`),
  },
  {
    id: "returnOperand",
    title: "a return operand",
    host: (c) => c.probe(`return ${c.ret(c.v)};`),
  },
  {
    id: "callArgument",
    title: "a call argument between two others",
    host: (c) => c.probe(`return ${c.ret(`pack(note("a", 1), ${c.v}, note("c", 3))`)};`),
  },
  {
    id: "methodArgument",
    title: "a method call argument after its receiver",
    host: (c) =>
      c.probe(
        `const receiver = { tag: "receiver", take(...items: unknown[]) { return [this.tag, ...items]; } };\nreturn ${c.ret(`note("receiver", receiver).take(note("a", 1), ${c.v})`)};`,
      ),
  },
  {
    id: "optionalCall",
    title: "the argument of an optional call that runs and one that is skipped",
    host: (c) =>
      c.probe(
        `const ran = callee(true)?.(${c.v});\nconst skipped = callee(false)?.(${c.again()});\nreturn ${c.ret("[ran, skipped]")};`,
      ),
  },
  {
    id: "arrayElement",
    title: "an array element",
    host: (c) => c.probe(`return ${c.ret(`[note("first", 0), ${c.v}, note("last", 2)]`)};`),
  },
  {
    id: "spreadElement",
    title: "a spread element's operand",
    host: (c) => c.probe(`return ${c.ret(`[note("first", 0), ...[${c.v}]]`)};`),
  },
  {
    id: "objectProperty",
    title: "an object literal property value",
    host: (c) => c.probe(`return ${c.ret(`{ a: note("a", 1), value: ${c.v}, c: note("c", 3) }`)};`),
  },
  {
    id: "templateLiteral",
    title: "a template literal interpolation",
    host: (c) => c.probe(`return ${c.ret(`\`<\${note("left", "L")}|\${${c.v}}|\${note("right", "R")}>\``)};`),
  },
  {
    id: "conditionalTest",
    operand: true,
    title: "the test of a conditional expression",
    host: (c) => c.probe(`return ${c.ret(`${c.v} ? note("then", "truthy") : note("else", "falsy")`)};`),
  },
  {
    id: "conditionalBranch",
    operand: true,
    title: "a branch of a conditional expression",
    host: (c) => c.probe(`return ${c.ret(`flip() ? ${c.v} : note("else", "skipped")`)};`),
  },
  {
    id: "logicalAnd",
    operand: true,
    title: "the right operand of &&",
    host: (c) => c.probe(`return ${c.ret(`flip() && ${c.v}`)};`),
  },
  {
    id: "logicalOr",
    operand: true,
    title: "the right operand of ||",
    host: (c) => c.probe(`return ${c.ret(`flip() || ${c.v}`)};`),
  },
  {
    id: "nullish",
    operand: true,
    title: "the right operand of ??",
    host: (c) => c.probe(`return ${c.ret(`(flip() ? null : note("left", "set")) ?? ${c.v}`)};`),
  },
  {
    id: "parameterDefault",
    title: "a parameter default",
    inner: noAsync,
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) => c.probe(`function inner(value: unknown = ${c.v}) {\nreturn value;\n}\nreturn ${c.ret("inner()")};`),
  },
  {
    id: "destructuringDefault",
    title: "a destructuring default",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) =>
      c.probe(`const { value = ${c.v} }: { value?: unknown } = note("source", {});\nreturn ${c.ret("value")};`),
  },
  {
    id: "classField",
    title: "a class field initializer",
    inner: noAsync,
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) => c.probe(`class Holder {\nfield = ${c.v};\n}\nreturn ${c.ret("new Holder().field")};`),
  },
  {
    id: "staticBlock",
    title: "a class static block",
    inner: noAsync,
    rejects: { try: "try-placement" },
    host: (c) =>
      c.probe(`class Holder {\nstatic field: unknown;\nstatic {\nHolder.field = ${c.v};\n}\n}\nreturn ${c.ret("Holder.field")};`),
  },
  {
    id: "constructorBody",
    title: "a constructor body",
    inner: noAsync,
    rejects: { try: "try-placement" },
    host: (c) =>
      c.probe(`class Holder {\nfield: unknown;\nconstructor() {\nthis.field = ${c.v};\n}\n}\nreturn ${c.ret("new Holder().field")};`),
  },
  {
    id: "getter",
    title: "a getter's return",
    inner: noAsync,
    target: "inner",
    host: (c) =>
      c.probe(`const holder = {\nget field() {\n${c.fn(`return ${c.ret(c.v)};`)}\n},\n};\nreturn holder.field;`, { target: false }),
  },
  {
    id: "method",
    title: "a method's return, reading `this`",
    inner: ["yield"],
    target: "inner",
    host: (c) =>
      c.probe(
        `class Holder {\ntag = "holder";\n${c.head}read(input: ${c.In}) {\n${c.fn(`note("this", this.tag);\nreturn ${c.ret(c.v)};`)}\n}\n}\nreturn new Holder().read(input);`,
        { target: false, head: false },
      ),
  },
  {
    id: "arrowBody",
    title: "a concise arrow body",
    inner: ["yield"],
    target: "inner",
    host: (c) => c.probe(`const inner = ${c.arrow(`input: ${c.In}`, c.v)};\nreturn inner(input);`, { target: false, head: false }),
  },
  {
    id: "whileTest",
    operand: true,
    title: "a while loop's test",
    rejects: { try: "try-placement" },
    host: (c) =>
      c.probe(`let rounds = 0;\nwhile (rounds < 1 && ${c.v}) {\nrounds++;\n}\nreturn ${c.ret("rounds")};`),
  },
  {
    id: "doWhileTest",
    operand: true,
    title: "a do-while loop's test",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) =>
      c.probe(`let rounds = 0;\ndo {\nrounds++;\n} while (rounds < 2 && ${c.v});\nreturn ${c.ret("rounds")};`),
  },
  {
    id: "forOfHead",
    title: "a for-of loop's iterated expression",
    host: (c) =>
      c.probe(`const collected: unknown[] = [];\nfor (const item of [${c.v}]) {\ncollected.push(item);\n}\nreturn ${c.ret("collected")};`),
  },
  {
    id: "forInitializer",
    title: "a C-style for loop's initializer",
    host: (c) =>
      c.probe(
        `const collected: unknown[] = [];\nfor (let item = ${c.v}, round = 0; round < 1; round++) {\ncollected.push(item);\n}\nreturn ${c.ret("collected")};`,
      ),
  },
  {
    id: "forTest",
    operand: true,
    title: "a C-style for loop's test",
    rejects: { try: "try-placement" },
    host: (c) =>
      c.probe(`let rounds = 0;\nfor (; rounds < 1 && ${c.v}; ) {\nrounds++;\n}\nreturn ${c.ret("rounds")};`),
  },
  {
    id: "forUpdate",
    title: "a C-style for loop's update",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) =>
      c.probe(
        `let last: unknown = null;\nfor (let round = 0; round < 1; round++, last = ${c.v}) {\nnote("body", round);\n}\nreturn ${c.ret("last")};`,
      ),
  },
  {
    id: "loopBody",
    title: "a loop body that runs twice",
    host: (c) =>
      c.probe(`const collected: unknown[] = [];\nfor (let round = 0; round < 2; round++) {\ncollected.push(${c.v});\n}\nreturn ${c.ret("collected")};`),
  },
  {
    id: "switchDiscriminant",
    title: "a switch discriminant",
    host: (c) =>
      c.probe(
        `let collected: unknown = "none";\nswitch (${c.v}) {\ncase note("case", 0 as unknown):\ncollected = "zero";\nbreak;\ndefault:\ncollected = "default";\n}\nreturn ${c.ret("collected")};`,
      ),
  },
  {
    id: "switchCaseTest",
    title: "a switch case test",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) =>
      c.probe(
        `let collected: unknown = "none";\nswitch (note("discriminant", 0) as unknown) {\ncase ${c.v}:\ncollected = "matched";\nbreak;\ndefault:\ncollected = "default";\n}\nreturn ${c.ret("collected")};`,
      ),
  },
  {
    id: "labeledBlock",
    title: "a labeled block left by break",
    host: (c) =>
      c.probe(`let collected: unknown = "none";\nfound: {\ncollected = ${c.v};\nif (flip()) break found;\ncollected = [collected];\n}\nreturn ${c.ret("collected")};`),
  },
  {
    id: "throwOperand",
    title: "a throw operand caught by the caller",
    host: (c) => c.probe(`throw ${c.v};`),
  },
  {
    id: "awaitOperand",
    title: "an await operand",
    inner: ["yield"],
    forces: "await",
    host: (c) => c.probe(`return ${c.ret(`await (${c.v})`)};`),
  },
  {
    id: "yieldOperand",
    title: "a yield operand",
    inner: ["await"],
    forces: "yield",
    rejects: { try: "try-placement" },
    host: (c) => c.probe(`const sent = yield ${c.v};\nreturn ${c.ret("sent")};`),
  },
  {
    id: "topLevel",
    title: "a module's top-level declaration",
    inner: ["yield"],
    topLevel: true,
    rejects: { try: "try-placement" },
  },
  {
    id: "matchArm",
    title: "a match arm's value",
    host: (c) =>
      c.probe(`return ${c.ret(c.side === "tt" ? `match (flip()) { true => ${c.v}, false => note("other arm", 0) }` : `(flip() ? ${c.v} : note("other arm", 0))`)};`),
  },
  {
    id: "ifLetBody",
    title: "an if-let body",
    host: (c) =>
      c.probe(
        c.side === "tt"
          ? `if let Ok(value: n) = read(note("head", 1)) {\nreturn ${c.ret(`[n, ${c.v}]`)};\n}\nreturn ${c.ret("null")};`
          : `const head = read(note("head", 1));\nif (head.kind === "Ok") {\nconst n = head.value;\nreturn ${c.ret(`[n, ${c.v}]`)};\n}\nreturn ${c.ret("null")};`,
      ),
  },
];

export const statementPositions = [
  {
    id: "functionBody",
    title: "a function body",
    host: (c) => c.probe(`${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};`),
  },
  {
    id: "nestedBlock",
    title: "a nested block",
    host: (c) => c.probe(`{\n${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};\n}`),
  },
  {
    id: "ifBranch",
    title: "a braced if branch",
    host: (c) =>
      c.probe(`if (note("enter", true)) {\n${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};\n}\nreturn ${c.ret('"skipped"')};`),
  },
  {
    id: "unbracedIf",
    title: "the unbraced body of an if",
    unbraced: true,
    host: (c) =>
      c.probe(`if (note("enter", true)) ${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.hoisted)};`),
  },
  {
    id: "forOfBody",
    title: "a for-of body left by continue",
    host: (c) =>
      c.probe(
        `const collected: unknown[] = [];\nfor (const round of [0, 1]) {\nnote("round", round);\n${c.s("continue;")}\ncollected.push(${c.result});\n}\nreturn ${c.ret("collected")};`,
      ),
  },
  {
    id: "whileBody",
    title: "a while body left by break",
    host: (c) =>
      c.probe(`let collected: unknown = "none";\nwhile (note("test", true)) {\n${c.s("break;")}\ncollected = ${c.result};\nbreak;\n}\nreturn ${c.ret("collected")};`),
  },
  {
    id: "labeledBlock",
    title: "a labeled block left by a labeled break",
    host: (c) =>
      c.probe(`let collected: unknown = "none";\nfound: {\n${c.s("break found;")}\ncollected = ${c.result};\n}\nreturn ${c.ret("collected")};`),
  },
  {
    id: "switchClause",
    title: "a switch clause",
    host: (c) =>
      c.probe(
        `switch (note("discriminant", 1)) {\ncase 1: {\n${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};\n}\ndefault:\nreturn ${c.ret('"default"')};\n}`,
      ),
  },
  {
    id: "tryBlock",
    title: "a try block with a catch",
    host: (c) =>
      c.probe(
        `try {\n${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};\n} catch (error) {\n${c.guard("error")}return ${c.ret('"caught"')};\n}`,
      ),
  },
  {
    id: "catchBlock",
    title: "a catch block",
    host: (c) =>
      c.probe(
        `try {\nthrow new Error(note("throw", "boom"));\n} catch (error) {\n${c.guard("error")}${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};\n}`,
      ),
  },
  {
    id: "arrowBlock",
    title: "a block-bodied arrow",
    inner: ["yield"],
    target: "inner",
    host: (c) =>
      c.probe(
        `const inner = ${c.head}(input: ${c.In}) => {\n${c.fn(`${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};`)}\n};\nreturn inner(input);`,
        { target: false, head: false },
      ),
  },
  {
    id: "method",
    title: "a method body",
    inner: ["yield"],
    target: "inner",
    host: (c) =>
      c.probe(
        `class Holder {\n${c.head}read(input: ${c.In}) {\n${c.fn(`${c.s(`return ${c.ret('"diverged"')};`)}\nreturn ${c.ret(c.result)};`)}\n}\n}\nreturn new Holder().read(input);`,
        { target: false, head: false },
      ),
  },
  {
    id: "staticBlock",
    title: "a class static block",
    inner: noAsync,
    rejects: { try: "try-placement" },
    host: (c) =>
      c.probe(
        `class Holder {\nstatic field: unknown = "none";\nstatic {\n${c.s('throw new Error("diverged");')}\nHolder.field = ${c.result};\n}\n}\nreturn ${c.ret("Holder.field")};`,
      ),
  },
  {
    id: "constructorBody",
    title: "a constructor body",
    inner: noAsync,
    rejects: { try: "try-placement" },
    host: (c) =>
      c.probe(
        `class Holder {\nfield: unknown = "none";\nconstructor(input: ${c.In}) {\n${c.s('throw new Error("diverged");')}\nthis.field = ${c.result};\n}\n}\nreturn ${c.ret("new Holder(input).field")};`,
      ),
  },
  {
    id: "matchBlockArm",
    title: "a match block arm",
    rejects: { letElse: "let-else-placement", yield: "match-control-crossing" },
    host: (c) =>
      c.probe(
        c.side === "tt"
          ? `const value = match (note<boolean>("arm", true)) {\ntrue => {\n${c.s('throw new Error("diverged");')}\nreturn ${c.result};\n},\nfalse => "other",\n};\nreturn ${c.ret("value")};`
          : `note<boolean>("arm", true);\nconst value = ${c.iife(`${c.s('throw new Error("diverged");')}\nreturn ${c.result};`)};\nreturn ${c.ret("value")};`,
      ),
  },
  {
    id: "resultBody",
    title: "a result block's body",
    retargets: true,
    rejects: { yield: "result-yield-crossing" },
    host: (c) =>
      c.probe(
        c.side === "tt"
          ? `const block = result {\nconst base = try read(note("base", 1));\n${c.s("return base;")}\nreturn [base, ${c.result}];\n};\nreturn ${c.ret("block")};`
          : `const block = ${c.iife(`try {\nconst base = unwrap(read(note("base", 1)));\n${c.s("return ok(base);")}\nreturn ok([base, ${c.result}]);\n} catch (error) {\nreturn caught(error);\n}`)};\nreturn ${c.ret("block")};`,
      ),
  },
  {
    id: "topLevel",
    title: "a module's top level",
    inner: ["yield"],
    topLevel: true,
    rejects: { try: "try-placement" },
  },
  ...[
    ["ValueExit", "whose exit returns a tt value", 'return match (note("exit", 3)) { 3 => "three", _ => "other" };', 'return note("exit", 3) === 3 ? "three" : "other";'],
    ["TemplateExit", "whose exit returns a template holding a tt value", 'return `<${match (note("exit", 3)) { 3 => "three", _ => "other" }}>`;', 'return `<${note("exit", 3) === 3 ? "three" : "other"}>`;'],
  ].map(([suffix, title, ttExit, tsExit]) => ({
    id: `matchBlockArm${suffix}`,
    title: `a match block arm ${title}`,
    exitAxis: true,
    rejects: { letElse: "let-else-placement", yield: "match-control-crossing" },
    host: (c) =>
      c.probe(
        c.side === "tt"
          ? `const value = match (note<boolean>("arm", true)) {\ntrue => {\n${c.s(ttExit)}\nreturn ${c.result};\n},\nfalse => "other",\n};\nreturn ${c.ret("value")};`
          : `note<boolean>("arm", true);\nconst value = ${c.iife(`${c.s(tsExit)}\nreturn ${c.result};`)};\nreturn ${c.ret("value")};`,
      ),
  })),
  ...[
    ["ValueExit", "whose exit returns a tt value", 'return try read(note("exit", -7));', 'return ok(unwrap(read(note("exit", -7))));'],
    ["TemplateExit", "whose exit returns a template holding a tt value", 'return `<${match (base) { 1 => "one", _ => "many" }}>`;', 'return ok(`<${base === 1 ? "one" : "many"}>`);'],
  ].map(([suffix, title, ttExit, tsExit]) => ({
    id: `resultBody${suffix}`,
    title: `a result block's body ${title}`,
    exitAxis: true,
    retargets: true,
    rejects: { yield: "result-yield-crossing" },
    host: (c) =>
      c.probe(
        c.side === "tt"
          ? `const block = result {\nconst base = try read(note("base", 1));\n${c.s(ttExit)}\nreturn [base, ${c.result}];\n};\nreturn ${c.ret("block")};`
          : `const block = ${c.iife(`try {\nconst base = unwrap(read(note("base", 1)));\n${c.s(tsExit)}\nreturn ok([base, ${c.result}]);\n} catch (error) {\nreturn caught(error);\n}`)};\nreturn ${c.ret("block")};`,
      ),
  })),
];

export const jsxPositions = [
  {
    id: "jsxAttribute",
    title: "a JSX attribute value",
    host: (c) => c.probe(`return ${c.ret(`<div /*@attrHost*/data-value={${c.v}} />`)};`),
  },
  {
    id: "jsxChild",
    title: "a JSX child expression",
    host: (c) => c.probe(`return ${c.ret(`<p>{${c.v}}</p>`)};`),
  },
  {
    id: "jsxConditional",
    operand: true,
    title: "conditional rendering with &&",
    host: (c) => c.probe(`return ${c.ret(`<p>{flip() && ${c.v}}</p>`)};`),
  },
  {
    id: "jsxTernary",
    operand: true,
    title: "conditional rendering with a conditional expression",
    host: (c) => c.probe(`return ${c.ret(`<p>{flip() ? ${c.v} : <i>none</i>}</p>`)};`),
  },
  {
    id: "jsxFragment",
    title: "a fragment child between two others",
    host: (c) => c.probe(`return ${c.ret(`<>{note("before", "[")}{${c.v}}{note("after", "]")}</>`)};`),
  },
  {
    id: "jsxSpreadAttribute",
    title: "a JSX spread attribute's object",
    editorWithholds: ["semanticTokens"],
    host: (c) => c.probe(`return ${c.ret(`<div {...{ value: ${c.v} }} />`)};`),
  },
  {
    id: "jsxComponentProp",
    title: "a component's prop",
    host: (c) => c.probe(`return ${c.ret(`<Show /*@attrHost*/value={${c.v}} />`)};`),
  },
  {
    id: "jsxNested",
    title: "a nested element's child after a sibling",
    host: (c) => c.probe(`return ${c.ret(`<ul><li>{note("first", 1)}</li><li>{${c.v}}</li></ul>`)};`),
  },
];

export const jsxStatementPositions = [
  {
    id: "componentBody",
    title: "a function component's body",
    inner: ["yield"],
    target: "inner",
    host: (c) =>
      c.probe(
        `${c.head}function Card(props: { input: ${c.In} }) {\n${c.fn(`const input = props./*@memberProps*/input;\n${c.s(`return ${c.ret("<p>diverged</p>")};`)}\nreturn ${c.ret(`<p>{text(${c.result})}</p>`)};`)}\n}\nreturn Card({ input });`,
        { target: false, head: false },
      ),
  },
];
