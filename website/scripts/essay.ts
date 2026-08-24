import { readFile } from 'node:fs/promises'

/**
 * Builds the essay pages from the repository's own documents, so the site and
 * `docs/why-tt*.md` cannot drift apart. Only the Markdown subset those
 * documents use is understood; anything else fails the build loudly instead of
 * being dropped silently from the page.
 */

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

type CodeHighlighter = {
  codeToHtml(code: string, options: { lang: string; theme: string; structure: 'inline' }): string
}

const sources = {
  en: '../../docs/why-tt.md',
  ko: '../../docs/why-tt.ko.md',
} as const

export const essayPaths = { en: '/why', ko: '/ko/why' } as const

/** Fence languages the documents may use, and the label shown above the block. */
const fences: Record<string, { lang: string; label: string }> = {
  ts: { lang: 'typescript', label: 'typescript' },
  tt: { lang: 'tt', label: 'example.tt' },
  ttx: { lang: 'ttx', label: 'example.ttx' },
  text: { lang: 'text', label: 'output' },
}

const inlinePattern = /`([^`]+)`|\*\*([^*]+)\*\*|\*([^*]+)\*|\[([^\]]+)\]\(([^)]+)\)/g

function parseInline(text: string, where: string): EssaySpan[] {
  const spans: EssaySpan[] = []
  let plain = ''
  let end = 0
  const push = (span: EssaySpan) => {
    if (plain) spans.push({ kind: 'text', text: plain })
    plain = ''
    spans.push(span)
  }
  for (const match of text.matchAll(inlinePattern)) {
    plain += text.slice(end, match.index)
    end = match.index + match[0].length
    if (match[1] !== undefined) push({ kind: 'code', text: match[1] })
    else if (match[2] !== undefined) push({ kind: 'strong', text: match[2] })
    else if (match[3] !== undefined) push({ kind: 'em', text: match[3] })
    else if (match[4] !== undefined && match[5] !== undefined) {
      push({ kind: 'link', text: match[4], href: match[5] })
    }
  }
  plain += text.slice(end)
  if (plain) spans.push({ kind: 'text', text: plain })
  for (const span of spans) {
    if (span.kind === 'text' && /[`*]/.test(span.text)) {
      throw new Error(`${where}: unbalanced Markdown markup in ${JSON.stringify(span.text)}`)
    }
  }
  return spans
}

function plainText(spans: EssaySpan[]) {
  return spans.map((span) => span.text).join('')
}

/** A paragraph of nothing but links is the repository's bilingual navigation
 * line; the site has its own language toggle, so it is not part of the page. */
function isNavigation(spans: EssaySpan[]) {
  return spans.some((span) => span.kind === 'link')
    && spans.every((span) => span.kind === 'link' || span.text.trim() === '')
}

function parse(source: string, file: string, highlighter: CodeHighlighter): Essay {
  const lines = source.split('\n')
  const first = lines[0] ?? ''
  if (!first.startsWith('# ')) throw new Error(`${file}:1: the document must start with a level-1 title`)
  const title = first.slice(2).trim()

  const blocks: EssayBlock[] = []
  let summary: string | undefined
  let index = 1

  while (index < lines.length) {
    const line = lines[index] ?? ''
    const where = `${file}:${index + 1}`

    if (line.trim() === '') {
      index += 1
    } else if (line.startsWith('## ')) {
      blocks.push({ kind: 'heading', text: line.slice(3).trim() })
      index += 1
    } else if (line.startsWith('```')) {
      const fence = fences[line.slice(3).trim()]
      if (!fence) throw new Error(`${where}: unsupported code fence ${JSON.stringify(line)}`)
      const body: string[] = []
      index += 1
      while (index < lines.length && !(lines[index] ?? '').startsWith('```')) {
        body.push(lines[index] ?? '')
        index += 1
      }
      if (index >= lines.length) throw new Error(`${where}: unterminated code fence`)
      index += 1
      blocks.push({
        kind: 'code',
        label: fence.label,
        html: highlighter.codeToHtml(body.join('\n'), {
          lang: fence.lang,
          theme: 'github-dark-default',
          structure: 'inline',
        }),
      })
    } else if (/^(#|>|-\s|\*\s|\d+\.\s|\|)/.test(line)) {
      throw new Error(`${where}: unsupported Markdown block ${JSON.stringify(line)}`)
    } else {
      const paragraph: string[] = []
      while (index < lines.length && (lines[index] ?? '').trim() !== '') {
        const next = lines[index] ?? ''
        if (next.startsWith('#') || next.startsWith('```')) break
        paragraph.push(next.trim())
        index += 1
      }
      const spans = parseInline(paragraph.join(' '), where)
      if (isNavigation(spans)) continue
      if (summary === undefined) summary = plainText(spans)
      else blocks.push({ kind: 'paragraph', spans })
    }
  }

  if (summary === undefined) throw new Error(`${file}: the document has no lead paragraph`)
  return { title, summary, blocks }
}

export async function buildEssays(highlighter: CodeHighlighter): Promise<Record<'en' | 'ko', Essay>> {
  const entries = await Promise.all(
    Object.entries(sources).map(async ([language, relative]) => {
      const url = new URL(relative, import.meta.url)
      const source = await readFile(url, 'utf8')
      return [language, parse(source, relative.replace('../../', ''), highlighter)] as const
    }),
  )
  return Object.fromEntries(entries) as Record<'en' | 'ko', Essay>
}
