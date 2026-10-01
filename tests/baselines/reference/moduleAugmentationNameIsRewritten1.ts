//// [moduleAugmentationNameIsRewritten1.tt] ////
declare module "./token.tt" {
  interface Token { extra: number }
}
export {};


//// [moduleAugmentationNameIsRewritten1.ts]
declare module "./token.js" {
  interface Token { extra: number }
}
export {};
