//// [aDeclaredVariantKeepsItsModifierOnBothDeclarations.tt] ////
export declare variant P2 { Q }
declare variant P4<T> { W(value: T) }


//// [aDeclaredVariantKeepsItsModifierOnBothDeclarations.ts]
export declare type P2 =
  { kind: "Q" };
export declare const P2: {
  readonly Q: { readonly kind: "Q" };
};
declare type P4<T> =
  { kind: "W"; value: T };
declare const P4: {
  readonly W: <T>(value: T) => P4<T>;
};
