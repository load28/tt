export type Shape = { kind: "Circle"; radius: number } | { kind: "Rect"; width: number; height: number } | { kind: "Point" };
export function area(s: /*variantName*/Shape): number {
  switch (s.kind) {
    case /*tag*/"Circle": { const { /*binding*/radius } = s; return Math.PI * /*use*/radius ** 2; }
    case "Rect": { const { width, height } = s; return width * height; }
    case "Point": return 0;
  }
}
