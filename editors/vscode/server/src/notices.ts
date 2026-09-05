/* Standing notices — what the server has already told the user it cannot do.
 *
 * Each one reports something only the person at the keyboard can fix: no
 * compiler, a compiler that cannot be started, no TypeScript toolchain, a
 * typed pass that failed inside the compiler. Repeating one on every
 * keystroke is noise, so each is said once and then stands.
 *
 * A standing notice has to end when something that could have fixed it
 * changes, or the editor stays silent about a problem that is no longer
 * there — and stays silent about the next one too. That "until" is why
 * these live in one object: as separate flags, only the one whose reset
 * someone remembered to write was ever re-armed, and each notice added
 * afterwards opted out of recovery by default.
 */
export type NoticeId =
  | "compiler-unusable"
  | "typed-check-unavailable"
  | "typed-compiler-failure"
  | "sidecar-dir-unresolved"
  | "type-layer-unreachable";

export class NoticeLedger {
  private standing = new Set<NoticeId>();

  /** Whether to say this now: true the first time, and again after
   * [`reset`], so a caller says it exactly when it is news. */
  raise(id: NoticeId): boolean {
    if (this.standing.has(id)) return false;
    this.standing.add(id);
    return true;
  }

  /** Something that could have fixed any of them changed, so every notice
   * is worth saying again if it is still true. */
  reset(): void {
    this.standing.clear();
  }
}
