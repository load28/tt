# TT GitHub Discussion 숙의 봇

서로 다른 관점을 가진 GitHub App들이 Discussion에서 공개적으로 논증하고, 상대의
근거를 받아들여 입장을 바꾸며, 합의되지 않은 판단은 사람에게 넘기는 로컬 서비스다.

## 토론 계약

- 새 Discussion과 사람이 작성한 새 댓글은 전체 숙의를 시작한다.
- 특정 App 멘션은 그 에이전트의 단독 답변만 시작한다.
- 각 에이전트는 상대 입장을 요약한 뒤 `유지`, `보완`, `변경`, `수용`을 선택한다.
- 최소 라운드 전에는 합의로 종료하지 않는다.
- 전원이 종합안을 수용하고 중대한 반대가 없어야 합의다.
- 최대 라운드에도 이견이 남으면 선택지와 쟁점을 사람에게 넘긴다.
- 봇이 작성한 댓글 웹훅은 무시한다. 다음 공개 라운드는 오케스트레이터가 직접 진행한다.

## 필요한 GitHub App

`config.example.json` 기준으로 진행자 하나와 관점별 App 세 개를 만든다. 모든 App을
대상 저장소에 설치하고 다음 권한을 부여한다.

- Repository permissions → Discussions: Read and write
- Repository permissions → Metadata: Read-only

네 App은 댓글 작성과 읽기 신원으로 사용한다. 저장소 Settings → Webhooks에서
`Discussion`과 `Discussion comments`만 선택하고
`https://<public-host>/github/webhook`으로 보낸다. Content type은 `application/json`,
SSL verification과 Active는 켠다. 각 App의 ID와 private key를 설정 파일에 지정한
환경 변수로 제공한다.

## 로컬 실행

```sh
cd tools/deliberation-bot
cp config.example.json config.json

export TT_DELIBERATION_CONFIG="$PWD/config.json"
export TT_DELIBERATION_WEBHOOK_SECRET='<shared webhook secret>'
export TT_MODERATOR_APP_ID='<app id>'
export TT_MODERATOR_PRIVATE_KEY="$(awk '{printf "%s\\n", $0}' moderator.pem)"
# 나머지 세 App의 ID와 private key도 config.example.json의 변수명으로 설정한다.

npm test
npm start
```

서비스는 기본적으로 `127.0.0.1:8787`에서 실행된다. GitHub가 접근할 수 있도록
Tailscale Funnel을 연결한다.

```sh
tailscale funnel --bg 8787
tailscale funnel status
```

출력된 HTTPS 주소 뒤에 `/github/webhook`을 붙여 저장소 Webhook URL로 지정한다.
Funnel은 기기 이름을 공개 인증서 투명성 원장에 기록하므로 운영용 기기 이름을 먼저
확인한다. 상태와 토론 기록은 저장소 루트의 `.deliberation-bot/state`에 저장되며
Git에서 제외된다.

Codex CLI는 현재 로그인 정보를 사용한다. 각 호출은 read-only sandbox와 구조화된
JSON Schema를 사용하며, Discussion 본문과 댓글은 명령이 아닌 신뢰할 수 없는 입력으로
취급한다.

macOS에서 로그인 후에도 서버를 계속 실행하려면 로컬 `config.json`과 private key를
`.deliberation-bot`에 준비한 뒤 다음 명령으로 LaunchAgent 파일을 만든다. Funnel의
백그라운드 설정은 Tailscale이 관리한다.

```sh
cd tools/deliberation-bot
npm run install:macos
launchctl bootstrap "gui/$(id -u)" ~/Library/LaunchAgents/dev.load28.tt-deliberation-bot.plist
```
