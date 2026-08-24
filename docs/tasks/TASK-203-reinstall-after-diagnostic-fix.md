# TASK-203: TASK-202 로컬 개발 환경 재설치

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 커밋

## 목적

TASK-202의 진단 발행 개선이 포함된 현재 작업 트리를 공식 `scripts/setup` 절차로
다시 빌드하고, 로컬 `ttc`와 VS Code 확장에 설치한다.

## 범위

- 포함: 저장된 typescript-go checkout 설정 재사용, release `ttc` 빌드,
  VS Code 확장 패키징·재설치, 설치 결과 확인
- 제외: git pull, typescript-go 소스 변경, tour 프로젝트 의존성 갱신, 원격 배포

## 의사결정

### 결정 1: 저장된 checkout 설정을 인자 없이 재사용한다

- **상황**: `.tt-dev/toolchain.json`은
  `/Users/seominyong/Downloads/source/typescript-go` checkout을 가리키며 공식
  setup 스크립트는 인자가 없을 때 이 설정을 재사용한다.
- **검토한 대안**: `--tsgo-root`를 다시 지정해도 같은 결과지만 저장된 설정의
  재사용 경로를 검증하지 못한다. npm 모드로 바꾸면 현재 로컬 개발 환경의
  toolchain 계약이 달라진다.
- **선택과 근거**: 가이드의 후속 실행 방식인 `./scripts/setup`을 사용한다.

## 작업 내역

- 2026-08-24: `.tt-dev/toolchain.json`의 checkout 경로와 `code` CLI 사용 가능
  여부를 확인했다.
- 2026-08-24: 가이드의 후속 실행 명령 `./scripts/setup`을 실행했다.
  typescript-go native compiler와 API client, release `ttc`, VS Code 확장을
  차례로 빌드하고 기존 `tt-lang.tt-language` 확장을 제거한 뒤 새 VSIX를
  설치했다.
- 2026-08-24: release `ttc`가 `0.3.0-dev.6`, 설치된 확장이
  `tt-lang.tt-language@0.1.0`인지 확인했다. `.tt-dev/tt-language.vsix`와
  로컬 npm launcher 설정도 현재 저장소를 가리킨다.

## 이슈 및 해결

### 이슈 1: 확장 목록 확인 중 VS Code 로그 디렉터리 권한 경고

- **증상**: `code --list-extensions --show-versions`가 sandbox 밖의 VS Code
  로그 디렉터리를 만들지 못해 `EPERM`을 출력했다.
- **원인**: 확인 명령은 제한된 파일시스템 권한으로 실행됐고 VS Code CLI가
  `~/Library/Application Support/Code/logs`에 로그 디렉터리를 만들려고 했다.
- **해결**: 명령은 계속 실행되어 설치된 `tt-lang.tt-language@0.1.0`을
  반환했다. 설치 자체는 권한이 승인된 `scripts/setup` 실행에서 성공했다.

## 검증

- [x] `./scripts/setup`
- [x] `target/release/ttc --version` — `0.3.0-dev.6`
- [x] VS Code 확장 설치 목록 — `tt-lang.tt-language@0.1.0`

## 결과

완료. 현재 작업 트리의 release `ttc`와 TASK-202가 포함된 VS Code 확장을 로컬
개발 환경에 다시 설치했다.
