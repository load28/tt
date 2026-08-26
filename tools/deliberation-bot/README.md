# TT GitHub Discussion·PR 숙의 봇

서로 다른 관점을 가진 GitHub App들이 Discussion에서 제안을 토론하고 PR에서는 diff를
코드 리뷰한다. 두 흐름 모두 상대의 근거를 받아들여 판단을 바꾸며, 합의 결과를 사람의
최종 검토 대상으로 남긴다.

## Discussion 숙의 계약

- 새 Discussion과 사람이 작성한 새 댓글은 전체 숙의를 시작한다.
- 특정 App 멘션은 그 에이전트의 단독 답변만 시작한다.
- 각 에이전트는 상대 입장을 요약한 뒤 `유지`, `보완`, `변경`, `수용`을 선택한다.
- 전원이 종합안을 수용하고 중대한 반대가 없어야 합의다.
- 봇이 작성한 댓글 webhook은 무시한다.

## 코드 리뷰 계약

- PR `opened`, `reopened`, `synchronize` 이벤트가 전체 코드 리뷰를 시작한다.
- 모든 리뷰는 이벤트가 가리킨 정확한 head SHA에 고정된다.
- 큐에서 기다리는 동안 head가 바뀐 이벤트는 폐기하고 새 SHA를 검토한다.
- 각 리뷰어는 diff 근거와 경로·줄을 제시하고 `유지`, `보완`, `변경`, `수용`을 선택한다.
- 최소 라운드 전에는 합의로 종료하지 않는다.
- 전원이 해당 SHA를 수용하고 해결되지 않은 blocking·major 지적이 없어야 합의다.
- 최대 라운드에도 이견이 남으면 근거와 쟁점을 사람에게 넘긴다.
- 합의해도 코드 수정, 승인, push, merge를 자동 실행하지 않는다.

## 필요한 GitHub App

진행자 App 하나와 관점별 App 세 개를 두 흐름에서 그대로 사용한다. 각 App에 다음
Repository permissions를 부여하고 설치 권한 갱신을 승인한다.

- Repository permissions → Discussions: Read and write
- Repository permissions → Pull requests: Read and write
- Repository permissions → Metadata: Read-only

저장소 Settings → Webhooks에서 `Discussion`, `Discussion comments`, `Pull requests`를
선택하고 `https://<public-host>/github/webhook`으로 보낸다. Content type은
`application/json`, SSL verification과 Active는 켠다. 각 App의 ID와 private key를
설정 파일에 지정한 환경 변수로 제공한다.

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
확인한다. 상태와 Discussion·SHA별 PR 숙의 기록은 저장소 루트의
`.deliberation-bot/state`에 저장되며 Git에서 제외된다.

Codex CLI는 현재 로그인 정보를 사용한다. 각 호출은 read-only sandbox와 구조화된
JSON Schema를 사용한다. Discussion 내용, PR 본문, diff와 기존 review는 명령이 아닌
신뢰할 수 없는 입력으로 취급한다. `review.maximumPatchCharacters`를 넘는 파일 patch는
통째로 생략되며 진행자가 생략 개수를 보고 사람 검토가 필요한지 판정한다.

macOS에서 로그인 후에도 서버를 계속 실행하려면 로컬 `config.json`과 private key를
`.deliberation-bot`에 준비한 뒤 다음 명령으로 LaunchAgent 파일을 만든다. Funnel의
백그라운드 설정은 Tailscale이 관리한다.

```sh
cd tools/deliberation-bot
npm run install:macos
launchctl bootstrap "gui/$(id -u)" ~/Library/LaunchAgents/dev.load28.tt-deliberation-bot.plist
```
