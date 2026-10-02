//// [aCaseNamedLikeThePrototypeSetterIsAnOwnConstructorProperty2.tt] ////
declare variant V { __proto__(x: number), B }


//// [aCaseNamedLikeThePrototypeSetterIsAnOwnConstructorProperty2.ts]
declare type V =
  | { kind: "__proto__"; x: number }
  | { kind: "B" };
declare const V: {
  readonly __proto__: (x: number) => V;
  readonly B: { readonly kind: "B" };
};
