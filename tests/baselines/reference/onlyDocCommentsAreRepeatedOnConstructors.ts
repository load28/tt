//// [onlyDocCommentsAreRepeatedOnConstructors.tt] ////
variant Mode {
  // internal note
  /** Read only. */
  Read, /* block */
  Write(/** Target path. */ path: string),
}


//// [onlyDocCommentsAreRepeatedOnConstructors.ts]
type Mode =
  // internal note
  /** Read only. */
  | { kind: "Read" } /* block */
  | {
      kind: "Write";
      /** Target path. */
      path: string;
    };
const Mode = {
  /** Read only. */
  Read: { kind: "Read" } as const,
  Write: (
    /** Target path. */
    path: string,
  ): Mode => ({ kind: "Write", path }),
};
