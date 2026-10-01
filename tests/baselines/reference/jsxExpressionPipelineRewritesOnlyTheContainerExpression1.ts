//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression1.ttx] ////
declare const raw: string; declare const up: (x: string) => string;
const child = <p>{raw |> up}</p>;


//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression1.tsx]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const raw: string; declare const up: (x: string) => string;
const child = <p>{$tt_ap(raw, up)}</p>;
