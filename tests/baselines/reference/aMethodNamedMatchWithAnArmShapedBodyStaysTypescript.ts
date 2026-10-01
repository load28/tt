//// [aMethodNamedMatchWithAnArmShapedBodyStaysTypescript.tt] ////
class C {
  match(x: number) { x }
  other(y: boolean) { if (y) return 1; }
}


//// [aMethodNamedMatchWithAnArmShapedBodyStaysTypescript.ts]
class C {
  match(x: number) { x }
  other(y: boolean) { if (y) return 1; }
}
