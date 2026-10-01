//// [variantGenericsFlowIntoConstructors.tt] ////
variant Option<T> {
  Some(value: T),
  None,
}


//// [variantGenericsFlowIntoConstructors.ts]
type Option<T> =
  | { kind: "Some"; value: T }
  | { kind: "None" };
const Option = {
  Some: <T>(value: T): Option<T> => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
