import rawEssay from './essay.json'
import type { Language } from './content'

export type EssaySpan =
  | { kind: 'text'; text: string }
  | { kind: 'code'; text: string }
  | { kind: 'strong'; text: string }
  | { kind: 'em'; text: string }
  | { kind: 'link'; text: string; href: string }

export type EssayBlock =
  | { kind: 'heading'; text: string }
  | { kind: 'paragraph'; spans: EssaySpan[] }
  | { kind: 'code'; label: string; html: string }

export type Essay = { title: string; summary: string; blocks: EssayBlock[] }

/** Generated from `docs/why-tt.md` and `docs/why-tt.ko.md` by `bun run highlight`. */
export const essay = rawEssay as Record<Language, Essay>

export function essayPath(language: Language) {
  return language === 'ko' ? '/ko/why' : '/why'
}

export function essaySource(language: Language) {
  const file = language === 'ko' ? 'why-tt.ko.md' : 'why-tt.md'
  return `https://github.com/load28/tt/blob/main/docs/${file}`
}
