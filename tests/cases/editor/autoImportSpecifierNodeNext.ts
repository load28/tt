// @filename: tsconfig.json
{
  "compilerOptions": {
    "target": "es2022",
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}
// @filename: package.json
{ "name": "case", "private": true, "type": "module" }
// @filename: shapes.ts
export const shapeK = 1;
// @filename: main.ts
export const y = shapeK/*jsEnding*/;
