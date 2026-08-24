import { Link } from '@tanstack/react-router'
import { content, topicPath, type Language, type TopicId } from '../content'
import { essay, essayPath } from '../essay'

/** Which page the shell is wrapping: it decides the active navigation item and
 * where the language toggle points. */
export type ShellPage = { kind: 'topic'; topic: TopicId } | { kind: 'essay' }

function otherLanguagePath(page: ShellPage, language: Language) {
  return page.kind === 'essay' ? essayPath(language) : topicPath(language, page.topic)
}

export function SiteShell({ language, page, children }: {
  language: Language
  page: ShellPage
  children: React.ReactNode
}) {
  const otherLanguage: Language = language === 'ko' ? 'en' : 'ko'

  return (
    <div className="site-shell">
      <header className="topbar">
        <Link className="brand" to={topicPath(language, 'overview')} aria-label={language === 'ko' ? 'tt 홈' : 'tt home'}>
          <span className="brand__mark">tt</span>
          <span className="brand__tag">{language === 'ko' ? 'TypeScript와 TSX를 위한 작은 언어' : 'a small language for TypeScript and TSX'}</span>
        </Link>
        <div className="topbar__actions">
          <Link className="language-toggle" to={otherLanguagePath(page, otherLanguage)} aria-label={language === 'ko' ? '영어로 보기' : 'View in Korean'}>
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
              {(navGroup.topics as TopicId[]).map((id) => {
                const active = page.kind === 'topic' && id === page.topic
                return (
                  <Link
                    className={`reference-nav__item${active ? ' is-active' : ''}`}
                    to={topicPath(language, id)}
                    aria-current={active ? 'page' : undefined}
                    key={id}
                  >
                    {content.topics[id].nav[language]}
                  </Link>
                )
              })}
            </section>
          ))}
          <section className="reference-nav__group">
            <h2 className="reference-nav__heading">{language === 'ko' ? '배경' : 'Background'}</h2>
            <Link
              className={`reference-nav__item${page.kind === 'essay' ? ' is-active' : ''}`}
              to={essayPath(language)}
              aria-current={page.kind === 'essay' ? 'page' : undefined}
            >
              {essay[language].title}
            </Link>
          </section>
        </aside>

        <main className="reference-content" id="content">{children}</main>
      </div>
    </div>
  )
}
