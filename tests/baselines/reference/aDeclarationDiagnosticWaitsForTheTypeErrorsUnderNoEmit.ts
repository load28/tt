//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "declaration": true,
    "isolatedDeclarations": true
  }
}

//// [a.tt] ////
export const x = Math.random() ? 0 : 1;

//// [b.tt] ////
export const y: number = "s";


//// [a.ts]
export const x = Math.random() ? 0 : 1;
\ No newline at end of file

//// [b.ts]
export const y: number = "s";
