import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import {
  content,
  highlighted,
  topicIds,
  topicPath,
  type Language,
  type TopicId,
} from '../content'

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
  const otherLanguage: Language = language === 'ko' ? 'en' : 'ko'

  return (
    <div className="site-shell">
      <header className="topbar">
        <Link className="brand" to={topicPath(language, 'overview')} aria-label={language === 'ko' ? 'tt 홈' : 'tt home'}>
          <span className="brand__mark">tt</span>
          <span className="brand__tag">{language === 'ko' ? 'TypeScript와 TSX를 위한 작은 언어' : 'a small language for TypeScript and TSX'}</span>
        </Link>
        <div className="topbar__actions">
          <Link className="language-toggle" to={topicPath(otherLanguage, topic)} aria-label={language === 'ko' ? '영어로 보기' : 'View in Korean'}>
            {language === 'ko' ? 'EN' : '한국어'}
          </Link>
          <a className="github-link" href="https://github.com/load28/tt" target="_blank" rel="noreferrer">GitHub ↗</a>
        </div>
      </header>

      <aside className="development-notice" aria-label={language === 'ko' ? '개발 상태' : 'Development status'}>
        <strong className="development-notice__label">{language === 'ko' ? '개발 중' : 'Early development'}</strong>
        <span>{language === 'ko'
          ? '아직 프로덕션 사용을 권장하지 않습니다. 릴리스 사이에 API와 언어 동작이 바뀔 수 있습니다.'
          : 'Not yet recommended for production use. APIs and language behavior may change between releases.'}</span>
      </aside>

      <div className="reference-layout">
        <aside className="reference-nav" aria-label={language === 'ko' ? '언어 API 목록' : 'Language API list'}>
          {content.groups.map((navGroup) => (
            <section className="reference-nav__group" key={navGroup.id}>
              <h2 className="reference-nav__heading">{navGroup[language]}</h2>
              {(navGroup.topics as TopicId[]).map((id) => (
                <Link
                  className={`reference-nav__item${id === topic ? ' is-active' : ''}`}
                  to={topicPath(language, id)}
                  aria-current={id === topic ? 'page' : undefined}
                  key={id}
                >
                  {content.topics[id].nav[language]}
                </Link>
              ))}
            </section>
          ))}
        </aside>

        <main className="reference-content" id="content">
          <article className="reference-article">
            <p className="eyebrow">{group[language]}</p>
            <h1 className="reference-title">{item.title[language]}</h1>
            <p className="reference-summary">{item.summary[language]}</p>
            {topic === 'overview' && <InstallCommand language={language} />}

            <div className="code-block">
              <span className="code-block__label">{topic === 'cli' || topic === 'install' ? 'shell' : topic === 'ttx' ? 'example.ttx' : 'example.tt'}</span>
              <pre tabIndex={0} role="region" aria-label={language === 'ko' ? '코드 예제' : 'Code example'}>
                <code dangerouslySetInnerHTML={{ __html: highlighted[topic] }} />
              </pre>
            </div>

            <div className="detail-grid">
              <DetailList title={language === 'ko' ? '기능' : 'Features'} items={item.works[language]} />
            </div>

            {'sections' in item && <GuideSections sections={item.sections} language={language} />}

            {nextTopic && (
              <Link className="next-topic" to={topicPath(language, nextTopic)}>
                <span className="next-topic__label">{language === 'ko' ? '다음' : 'Next'}</span>
                <span className="next-topic__name">{content.topics[nextTopic].nav[language]} →</span>
              </Link>
            )}
          </article>
        </main>
      </div>
    </div>
  )
}

function InstallCommand({ language }: { language: Language }) {
  const command = 'bunx @load28/create-tt@dev my-app'
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

function GuideSections({ sections, language }: {
  sections: Array<{
    title: Record<Language, string>
    body: Record<Language, string>
    code: string
    link?: { href: string; en: string; ko: string }
  }>
  language: Language
}) {
  return (
    <div className="guide-sections">
      {sections.map((section) => (
        <section className="guide-section" key={section.title.en}>
          <h2>{section.title[language]}</h2>
          <p>{section.body[language]}</p>
          {section.link && (
            <a className="guide-section__link" href={section.link.href} target="_blank" rel="noreferrer">
              {section.link[language]}
            </a>
          )}
          <pre><code>{section.code}</code></pre>
        </section>
      ))}
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
