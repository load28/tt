//// [unclosedJsxTagBeforeMatch.ttx] ////
// An opening tag left unclosed before a `{match ...}` child: the `{` in the
// tag can only open a spread attribute, so the TypeScript before the match
// does not parse, and that is what is reported, at the match.
export variant Status { Idle, Busy }
export function App(s: Status) {
  return (
    <div>
      <span
      {match (s) { Idle => "i", Busy => "b" }}
    </div>
  );
}

