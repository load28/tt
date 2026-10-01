//// [tsconfig.json] ////
{
  // JSONC
  "contentMappers": [
    { "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] }
  ],
  "compilerOptions": {
    "target": "es2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "jsxFactory": "id1 id2"
  }
}

//// [a.tt] ////
export const x = 1;


//// [a.ts]
export const x = 1;
