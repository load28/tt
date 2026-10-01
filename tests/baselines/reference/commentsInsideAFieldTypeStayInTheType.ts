//// [commentsInsideAFieldTypeStayInTheType.tt] ////
variant Size { Px(value: /* css */ number | /* auto */ "auto") }


//// [commentsInsideAFieldTypeStayInTheType.ts]
type Size =
  { kind: "Px"; value: /* css */ number | /* auto */ "auto" };
const Size = {
  Px: (value: /* css */ number | /* auto */ "auto"): Size => ({ kind: "Px", value }),
};
