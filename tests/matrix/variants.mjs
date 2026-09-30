export function variant(name, cases, { exported = false, generics = "" } = {}) {
  const lead = exported ? "export " : "";
  const tt = `${lead}variant ${name}${generics} { ${cases
    .map(([tag, fields]) =>
      fields === null ? tag : `${tag}(${fields.map(([field, type, optional]) => `${field}${optional ? "?" : ""}: ${type}`).join(", ")})`,
    )
    .join(", ")} }`;
  const self = `${name}${generics.replace(/\s*extends\s+[^,>]+/g, "")}`;
  const members = cases.map(([tag, fields]) =>
    fields === null
      ? `{ kind: "${tag}" }`
      : `{ kind: "${tag}"${fields.map(([field, type, optional]) => `; ${field}${optional ? "?" : ""}: ${type}`).join("")} }`,
  );
  const constructors = cases.map(([tag, fields]) => {
    if (fields === null) return `  ${tag}: { kind: "${tag}" } as ${self},`;
    const params = fields.map(([field, type, optional]) => `${field}${optional ? "?" : ""}: ${type}`).join(", ");
    const required = fields.filter(([, , optional]) => !optional).map(([field]) => field);
    const optional = fields.filter(([, , optional]) => optional).map(([field]) => field);
    const body = [`kind: "${tag}" as const`, ...required].join(", ");
    if (optional.length === 0) return `  ${tag}: ${generics}(${params}): ${self} => ({ ${body} }),`;
    const sets = optional.map((field) => `${field} !== undefined ? { ${field} } : {}`);
    return `  ${tag}: ${generics}(${params}): ${self} => ({ ${body}, ${sets.map((set) => `...(${set})`).join(", ")} }),`;
  });
  const object = (list) => `${lead}type ${name}${generics} = ${members.join(" | ")};\n${lead}const ${name} = {\n${list.join("\n")}\n};`;
  const ts = object(constructors);
  const edit = object(constructors.map((line, i) => (cases[i][1] === null ? `  ${cases[i][0]}: { kind: "${cases[i][0]}" } as const,` : line)));
  return { tt, ts, edit };
}
