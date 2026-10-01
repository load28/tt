//// [aCaptureNeverCopiesASiblingTtValue.tt] ////
declare function g(x: unknown, y: unknown): void;
declare const a: boolean;
g(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });


//// [aCaptureNeverCopiesASiblingTtValue.ts]
declare function g(x: unknown, y: unknown): void;
declare const a: boolean;
{
  let $tt_subject;
  let $tt_subject_1;
  
  g(a && ($tt_subject = a, ($tt_subject === true) ? 1 : 0), ($tt_subject_1 = a, ($tt_subject_1 === true) ? 2 : 3));
}
