//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression2.ttx] ////
declare const raw: string; declare const up: (x: string) => string;
const child = <P value={raw |> up} />;


//// [jsxExpressionPipelineRewritesOnlyTheContainerExpression2.tsx]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const raw: string; declare const up: (x: string) => string;
const child = <P value={$tt_ap(raw, up)} />;
