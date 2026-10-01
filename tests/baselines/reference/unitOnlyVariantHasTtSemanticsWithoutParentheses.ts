//// [unitOnlyVariantHasTtSemanticsWithoutParentheses.tt] ////
variant Status { Active, Inactive }


//// [unitOnlyVariantHasTtSemanticsWithoutParentheses.ts]
type Status =
  | { kind: "Active" }
  | { kind: "Inactive" };
const Status = {
  Active: { kind: "Active" } as const,
  Inactive: { kind: "Inactive" } as const,
};
