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


//// [a.ts]
export const x = Math.random() ? 0 : 1;
