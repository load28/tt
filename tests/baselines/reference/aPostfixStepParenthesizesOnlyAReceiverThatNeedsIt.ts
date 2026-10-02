//// [aPostfixStepParenthesizesOnlyAReceiverThatNeedsIt.tt] ////
const a = s |> .trim();
const b = (x + y) |> .toFixed(2);
const c = await p |> .then(g);


//// [aPostfixStepParenthesizesOnlyAReceiverThatNeedsIt.ts]
const a = s.trim();
const b = (x + y).toFixed(2);
const c = (await p).then(g);
