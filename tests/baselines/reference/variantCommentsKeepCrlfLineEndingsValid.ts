//// [variantCommentsKeepCrlfLineEndingsValid.tt] ////
variant Flag {
  /** On. */
  On, // yes
  Off,
}


//// [variantCommentsKeepCrlfLineEndingsValid.ts]
type Flag =
  /** On. */
  | { kind: "On" } // yes
  | { kind: "Off" };
const Flag = {
  /** On. */
  On: { kind: "On" } as const,
  Off: { kind: "Off" } as const,
};
