//// [aValBindingPassedThroughAWrappedCalleeIsReported.tt] ////
function bad(p: { a: number }) { p.a = 1; }
val const x = { a: 1 };
(bad)(x);
bad!(x);
bad?.(x);
((bad))!(x);
x |> (bad);
x |> bad!;

