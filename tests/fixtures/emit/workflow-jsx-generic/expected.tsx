export type Box<T> =
  | { kind: "Value"; value: T }
  | { kind: "Empty" };
export const Box = {
  Value: <T,>(value: T): Box<T> => ({ kind: "Value", value }),
  Empty: { kind: "Empty" } as const,
};
