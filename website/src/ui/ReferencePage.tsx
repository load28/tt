import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import {
  content,
  highlighted,
  highlightedSections,
  topicIds,
  topicPath,
  type Language,
  type TopicId,
} from '../content'
import { SiteShell } from './SiteShell'

export function pageHead(language: Language, topic: TopicId) {
  const item = content.topics[topic]
  const title = topic === 'overview'
    ? 'tt — TypeScript, with better control flow'
    : `${item.nav[language]} — tt`
  const description = item.summary[language]
  const origin = 'https://load28.github.io/tt'
  const canonical = `${origin}${topicPath(language, topic)}`
  return {
    meta: [
      { title },
      { name: 'description', content: description },
      { property: 'og:title', content: title },
      { property: 'og:description', content: description },
      { property: 'og:type', content: 'website' },
      { property: 'og:url', content: canonical },
    ],
    links: [
      { rel: 'canonical', href: canonical },
      { rel: 'alternate', hreflang: 'en', href: `${origin}${topicPath('en', topic)}` },
      { rel: 'alternate', hreflang: 'ko', href: `${origin}${topicPath('ko', topic)}` },
      { rel: 'alternate', hreflang: 'x-default', href: `${origin}${topicPath('en', topic)}` },
    ],
  }
}

export function ReferencePage({ language, topic }: { language: Language; topic: TopicId }) {
  const item = content.topics[topic]
  const group = content.groups.find(({ topics }) => topics.includes(topic))!
  const nextTopic = topicIds[topicIds.indexOf(topic) + 1]
  const codeLabel = topic === 'cli' || topic === 'install' || topic === 'release'
    ? 'shell'
    : topic === 'ttx'
      ? 'example.ttx'
      : 'example.tt'

  return (
    <SiteShell language={language} page={{ kind: 'topic', topic }}>
      <article className="reference-article">
        <p className="eyebrow">{group[language]}</p>
        <h1 className="reference-title">{item.title[language]}</h1>
        <p className="reference-summary">{item.summary[language]}</p>
        {topic === 'overview' && <InstallCommand language={language} />}

        <div className="code-block">
          <span className="code-block__label">{codeLabel}</span>
          <pre tabIndex={0} role="region" aria-label={language === 'ko' ? '코드 예제' : 'Code example'}>
            <code dangerouslySetInnerHTML={{ __html: highlighted[topic] }} />
          </pre>
        </div>

        <div className="detail-grid">
          <DetailList
            title={topic === 'install'
              ? (language === 'ko' ? '지원 범위' : 'Supported setup')
              : (language === 'ko' ? '기능' : 'Features')}
            items={item.works[language]}
          />
        </div>

        {'sections' in item && (
          <GuideSections
            sections={item.sections}
            highlightedCode={highlightedSections[topic]}
            language={language}
          />
        )}

        {nextTopic && (
          <Link className="next-topic" to={topicPath(language, nextTopic)}>
            <span className="next-topic__label">{language === 'ko' ? '다음' : 'Next'}</span>
            <span className="next-topic__name">{content.topics[nextTopic].nav[language]} →</span>
          </Link>
        )}
      </article>
    </SiteShell>
  )
}

function InstallCommand({ language }: { language: Language }) {
  const command = 'bunx @openload28/create-tt@next my-app'
  const [copied, setCopied] = useState(false)
  return (
    <div className="install-command">
      <code className="install-command__code">{command}</code>
      <button
        className="install-command__copy"
        type="button"
        onClick={async () => {
          await navigator.clipboard.writeText(command)
          setCopied(true)
        }}
      >
        {copied ? (language === 'ko' ? '복사됨' : 'Copied') : (language === 'ko' ? '복사' : 'Copy')}
      </button>
    </div>
  )
}

function GuideSections({ sections, highlightedCode, language }: {
  sections: Array<{
    title: Record<Language, string>
    body: Record<Language, string>
    code: string
    link?: { href: string; en: string; ko: string }
  }>
  highlightedCode: string[] | undefined
  language: Language
}) {
  return (
    <div className="guide-sections">
      {sections.map((section, index) => {
        const code = highlightedCode?.[index]
        if (code === undefined) throw new Error(`Missing highlighted code for ${section.title.en}`)
        return (
          <section className="guide-section" key={section.title.en}>
            <h2>{section.title[language]}</h2>
            <p>{section.body[language]}</p>
            {section.link && (
              <a className="guide-section__link" href={section.link.href} target="_blank" rel="noreferrer">
                {section.link[language]}
              </a>
            )}
            <pre><code dangerouslySetInnerHTML={{ __html: code }} /></pre>
          </section>
        )
      })}
    </div>
  )
}

function DetailList({ title, items }: { title: string; items: string[] }) {
  return (
    <section className="detail-list">
      <h2 className="detail-list__title">{title}</h2>
      <ul className="detail-list__items">
        {items.map((item) => <li className="detail-list__item" key={item}>{item}</li>)}
      </ul>
    </section>
  )
}
