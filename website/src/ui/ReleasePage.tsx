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
    version: 'X.Y.0-beta',
    en: {
      name: 'Beta',
      body: 'Create release-X.Y from main. New features and large changes receive their widest testing here.',
    },
    ko: {
      name: 'Beta',
      body: 'main에서 release-X.Y를 만듭니다. 새 기능과 큰 변경은 이 단계에서 가장 넓게 검증합니다.',
    },
  },
  {
    version: 'X.Y.1-rc',
    en: {
      name: 'Release candidate',
      body: 'Sync main one last time, then limit the line to fixes needed for this release.',
    },
    ko: {
      name: '릴리스 후보',
      body: '마지막으로 main을 sync한 뒤, 이 릴리스에 필요한 수정으로 범위를 제한합니다.',
    },
  },
  {
    version: 'X.Y.2 → X.Y.3',
    en: {
      name: 'Stable and patches',
      body: 'Publish the stable release, then cherry-pick only prioritized fixes for later patches.',
    },
    ko: {
      name: 'Stable과 Patch',
      body: 'Stable을 게시한 뒤, 우선순위가 높은 수정만 cherry-pick하여 Patch로 냅니다.',
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
            <h2 id="release-stages-heading">{language === 'ko' ? '한 릴리스 라인이 지나가는 순서' : 'How a release line moves forward'}</h2>
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
            ? 'RC 전에는 main을 release-X.Y에 sync합니다. RC 뒤에는 main 전체를 가져오지 않고, 필요한 PR의 squash merge 커밋만 cherry-pick합니다.'
            : 'Before RC, sync main into release-X.Y. After RC, do not merge main wholesale; cherry-pick only the squash-merge commits required for that release.'}</p>
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
