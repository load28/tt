//// [genericLookAlikeWithAdjacentOperatorsPassesThrough.tt] ////
declare const result: number;
function f() {
  result
  { let x: Foo<-1>; }
}


//// [genericLookAlikeWithAdjacentOperatorsPassesThrough.ts]
declare const result: number;
function f() {
  result
  { let x: Foo<-1>; }
}
