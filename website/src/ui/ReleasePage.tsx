import { content, highlightedSections, type Language } from '../content'
import { SiteShell } from './SiteShell'

type Stage = {
  version: string
  en: { name: string; body: string }
  ko: { name: string; body: string }
}

const stages: Stage[] = [
  {
    version: 'Nightly',
    en: {
      name: 'Current main',
      body: 'The scheduled main CI publishes the latest development build to npm next automatically.',
    },
    ko: {
      name: '현재 main',
      body: '예약된 main CI가 최신 개발 빌드를 npm next로 자동 게시합니다.',
    },
  },
  {
    version: '1.0.0-dev.YYYYMMDD.N',
    en: {
      name: 'Immutable build',
      body: 'A successful scheduled or manually dispatched main CI run produces one versioned artifact set.',
    },
    ko: {
      name: '불변 빌드',
      body: '성공한 예약 또는 수동 main CI 실행이 하나의 버전 지정 산출물 세트를 만듭니다.',
    },
  },
  {
    version: 'npm next',
    en: {
      name: 'Nightly channel',
      body: 'The verified artifacts are published to npm next and attached to a GitHub pre-release.',
    },
    ko: {
      name: 'Nightly 채널',
      body: '검증된 산출물을 npm next에 게시하고 GitHub pre-release에 첨부합니다.',
    },
  },
]

export function ReleasePage({ language }: { language: Language }) {
  const release = content.topics.release
  const highlighted = highlightedSections.release
  if (!highlighted) throw new Error('Missing highlighted release guide sections')

  return (
    <SiteShell language={language} page={{ kind: 'topic', topic: 'release' }}>
      <article className="release-article">
        <p className="eyebrow">{language === 'ko' ? '프로젝트 운영' : 'Project operations'}</p>
        <h1 className="release-title">{language === 'ko' ? '릴리스 절차' : 'Release process'}</h1>
        <p className="release-summary">{release.summary[language]}</p>

        <section className="release-stages" aria-labelledby="release-stages-heading">
          <div className="release-section-heading">
            <p className="release-section-kicker">{language === 'ko' ? '단계' : 'Stages'}</p>
            <h2 id="release-stages-heading">{language === 'ko' ? 'Nightly가 게시되는 순서' : 'How a Nightly is published'}</h2>
          </div>
          <ol className="release-stage-list">
            {stages.map((stage) => (
              <li className="release-stage" key={stage.version}>
                <code className="release-stage__version">{stage.version}</code>
                <div>
                  <h3>{stage[language].name}</h3>
                  <p>{stage[language].body}</p>
                </div>
              </li>
            ))}
          </ol>
        </section>

        <aside className="release-rule">
          <p className="release-rule__label">{language === 'ko' ? '핵심 규칙' : 'Core rule'}</p>
          <p>{language === 'ko'
            ? '현재 공개 설치는 Nightly를 사용합니다. npm 설치 명령은 모두 next dist-tag를 사용합니다.'
            : 'Public installations currently use Nightlies. Every npm installation command uses the next dist-tag.'}</p>
        </aside>

        <section className="release-flow" aria-labelledby="release-flow-heading">
          <div className="release-section-heading">
            <p className="release-section-kicker">{language === 'ko' ? '운영 흐름' : 'Operating flow'}</p>
            <h2 id="release-flow-heading">{language === 'ko' ? '준비, 검증, 게시' : 'Prepare, verify, publish'}</h2>
          </div>
          <div className="release-flow__items">
            {release.sections.map((section, index) => (
              <section className="release-flow__item" key={section.title.en}>
                <div className="release-flow__number">0{index + 1}</div>
                <div>
                  <h3>{section.title[language]}</h3>
                  <p>{section.body[language]}</p>
                  <pre><code dangerouslySetInnerHTML={{ __html: highlighted[index] }} /></pre>
                </div>
              </section>
            ))}
          </div>
        </section>
      </article>
    </SiteShell>
  )
}
