//// [anAmbientModuleVariantDeclaresItsConstructorObject.tt] ////
declare namespace N { variant P { Q, R(v: number) } }
declare module "m" { export variant P3 { Q } }


//// [anAmbientModuleVariantDeclaresItsConstructorObject.ts]
declare namespace N { type P =
  | { kind: "Q" }
  | { kind: "R"; v: number };
const P: {
  readonly Q: { readonly kind: "Q" };
  readonly R: (v: number) => P;
}; }
declare module "m" { export type P3 =
  { kind: "Q" };
export const P3: {
  readonly Q: { readonly kind: "Q" };
}; }
