export const verbs = {
  operand: ["hover", "definition", "references", "rename", "completions", "signatureHelp"],
  bind: ["hover", "references", "rename"],
  use: ["hover", "definition", "completions"],
  call: ["hover", "definition"],
  arg: ["signatureHelp"],
  member: ["hover", "definition", "completions"],
  field: ["hover", "definition"],
  attr: ["hover", "completions"],
};

export const constructs = ["pipeline"];

export const fileVerbs = ["semanticTokens", "diagnostics"];

export const positions = {
  tt: {
    value: [
      "declarationInitializer",
      "callArgument",
      "returnOperand",
      "objectProperty",
      "templateLiteral",
      "conditionalBranch",
      "logicalAnd",
      "nullish",
      "getter",
      "method",
      "arrowBody",
      "loopBody",
      "awaitOperand",
      "topLevel",
      "matchArm",
      "ifLetBody",
    ],
    statement: "all",
  },
  ttx: { value: "all", statement: "all" },
};
