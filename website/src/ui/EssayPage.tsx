import { essay, essayPath, essaySource, type EssayBlock, type EssaySpan } from '../essay'
import { type Language } from '../content'
import { SiteShell } from './SiteShell'

export function essayHead(language: Language) {
  const item = essay[language]
  const origin = 'https://load28.github.io/tt'
  const canonical = `${origin}${essayPath(language)}`
  const title = `${item.title} — tt`
  return {
    meta: [
      { title },
      { name: 'description', content: item.summary },
      { property: 'og:title', content: title },
      { property: 'og:description', content: item.summary },
      { property: 'og:type', content: 'article' },
      { property: 'og:url', content: canonical },
    ],
    links: [
      { rel: 'canonical', href: canonical },
      { rel: 'alternate', hreflang: 'en', href: `${origin}${essayPath('en')}` },
      { rel: 'alternate', hreflang: 'ko', href: `${origin}${essayPath('ko')}` },
      { rel: 'alternate', hreflang: 'x-default', href: `${origin}${essayPath('en')}` },
    ],
  }
}

export function EssayPage({ language }: { language: Language }) {
  const item = essay[language]

  return (
    <SiteShell language={language} page={{ kind: 'essay' }}>
      <article className="reference-article">
        <p className="eyebrow">{language === 'ko' ? '배경' : 'Background'}</p>
        <h1 className="reference-title essay-title">{item.title}</h1>
        <p className="reference-summary">{item.summary}</p>

        <div className="essay-body">
          {item.blocks.map((block, index) => <Block block={block} key={index} />)}
        </div>

        <a className="essay-source" href={essaySource(language)} target="_blank" rel="noreferrer">
          {language === 'ko' ? '이 글의 원문을 GitHub에서 보기 ↗' : 'Read this document on GitHub ↗'}
        </a>
      </article>
    </SiteShell>
  )
}

function Block({ block }: { block: EssayBlock }) {
  if (block.kind === 'heading') return <h2 className="essay-heading">{block.text}</h2>
  if (block.kind === 'paragraph') {
    return (
      <p className="essay-paragraph">
        {block.spans.map((span, index) => <Span span={span} key={index} />)}
      </p>
    )
  }
  return (
    <div className="code-block essay-code">
      <span className="code-block__label">{block.label}</span>
      <pre tabIndex={0}><code dangerouslySetInnerHTML={{ __html: block.html }} /></pre>
    </div>
  )
}

function Span({ span }: { span: EssaySpan }) {
  switch (span.kind) {
    case 'code': return <code className="essay-inline-code">{span.text}</code>
    case 'strong': return <strong>{span.text}</strong>
    case 'em': return <em>{span.text}</em>
    case 'link': return <a className="essay-link" href={span.href}>{span.text}</a>
    default: return <>{span.text}</>
  }
}
