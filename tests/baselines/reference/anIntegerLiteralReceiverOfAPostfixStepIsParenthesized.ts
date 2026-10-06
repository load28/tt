//// [anIntegerLiteralReceiverOfAPostfixStepIsParenthesized.tt] ////
const fixed = 5 |> .toFixed(1);
const separated = 1_000 |> .toString();
const fraction = 2.5 |> .toFixed(2);
const hex = 0xff |> .toString(2);
console.log(fixed, separated, fraction, hex);


//// [anIntegerLiteralReceiverOfAPostfixStepIsParenthesized.ts]
const fixed = (5).toFixed(1);
const separated = (1_000).toString();
const fraction = 2.5.toFixed(2);
const hex = 0xff.toString(2);
console.log(fixed, separated, fraction, hex);
