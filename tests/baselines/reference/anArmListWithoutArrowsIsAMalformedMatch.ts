//// [anArmListWithoutArrowsIsAMalformedMatch.tt] ////
// A match body of patterns whose `=>` is not written yet is an unfinished
// arm list where TypeScript cannot read the text: a located
// `malformed-match`, not a failure of the output self-check.
class MyErr extends Error {}
declare const e: unknown;
export const a = match (e) { is MyErr };
export const b = match (e) { A, B };

