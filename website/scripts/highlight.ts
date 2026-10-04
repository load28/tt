import { readFile, writeFile } from 'node:fs/promises'
import { createHighlighter } from 'shiki'
import content from '../src/content.json'
import { buildEssays, essayPaths } from './essay'
import { highlightSections, highlightTopics } from './content-highlighting'

const grammarUrl = new URL('../../editors/vscode/syntaxes/tt.tmLanguage.json', import.meta.url)
const ttxGrammarUrl = new URL('../../editors/vscode/syntaxes/ttx.tmLanguage.json', import.meta.url)
const grammar = JSON.parse(await readFile(grammarUrl, 'utf8'))
const ttxGrammar = JSON.parse(await readFile(ttxGrammarUrl, 'utf8'))
const highlighter = await createHighlighter({
  langs: [grammar, ttxGrammar, 'shellscript', 'typescript'],
  themes: ['github-dark-default'],
})

const highlighted = highlightTopics(content, highlighter)

await writeFile(new URL('../src/highlighted.json', import.meta.url), `${JSON.stringify(highlighted, null, 2)}\n`)

const highlightedSections = highlightSections(content, highlighter)

await writeFile(
  new URL('../src/highlighted-sections.json', import.meta.url),
  `${JSON.stringify(highlightedSections, null, 2)}\n`,
)

const essays = await buildEssays(highlighter)
await writeFile(new URL('../src/essay.json', import.meta.url), `${JSON.stringify(essays, null, 2)}\n`)

const origin = 'https://load28.github.io/tt'
const paths = [
  ...Object.keys(content.topics).flatMap((topic) => [
    topic === 'overview' ? '/' : `/${topic}`,
    topic === 'overview' ? '/ko' : `/ko/${topic}`,
  ]),
  ...Object.values(essayPaths),
]
const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${paths.map((path) => `  <url><loc>${origin}${path}</loc></url>`).join('\n')}
</urlset>
`
await writeFile(new URL('../public/sitemap.xml', import.meta.url), sitemap)
