import type { Highlighter } from 'shiki'
import type sourceContent from '../src/content.json'

function topicLanguage(id: string) {
  return id === 'cli' || id === 'install' || id === 'release'
    ? 'shellscript'
    : id === 'ttx'
      ? 'ttx'
      : 'tt'
}

export function highlightTopics(content: typeof sourceContent, highlighter: Highlighter) {
  return Object.fromEntries(
    Object.entries(content.topics).map(([id, topic]) => [
      id,
      highlighter.codeToHtml(topic.code, {
        lang: topicLanguage(id),
        theme: 'github-dark-default',
        structure: 'inline',
      }),
    ]),
  )
}

export function highlightSections(content: typeof sourceContent, highlighter: Highlighter) {
  return Object.fromEntries(
    Object.entries(content.topics).flatMap(([id, topic]) => 'sections' in topic
      ? [[id, topic.sections.map((section) => highlighter.codeToHtml(section.code, {
          lang: topicLanguage(id),
          theme: 'github-dark-default',
          structure: 'inline',
        }))]]
      : []),
  )
}
