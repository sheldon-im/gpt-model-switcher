# Mac에서 통합 저장소로 이어서 작업하기

사용자가 지정한 Mac 0.1.4 원본 `9007544`까지의 변경은 Windows 수정과 합쳐 통합 0.1.5에 반영했습니다. 이 변경을 다시 패치하지 마십시오. 원본 이력은 통합 main의 부모와 `mac-handoff-0.1.4` 보존 브랜치에 남깁니다. Mac 공유 폴더 자체는 Windows 작업에서 수정하지 않습니다.

## 기존 Mac Git 폴더를 계속 사용할 때

1. 먼저 `git status --short`로 추가 변경이 없는지 확인합니다. `9007544` 이후의 새 변경이 있으면 백업하고 의미 있는 구성요소별 로컬 커밋으로 보존합니다.
2. `git remote -v`를 확인합니다. origin이 아직 없을 때만 아래 명령으로 추가합니다. 기존 origin이 있다면 다른 저장소를 가리키는지 확인하고 임의로 덮어쓰지 않습니다.

```sh
git remote add origin https://github.com/sheldon-im/gpt-model-switcher.git
```

3. `gh`에 해당 private 저장소 접근 권한이 있는 계정으로 로그인한 뒤 기존 main에서 받습니다. Mac 원본 이력도 연결되어 있으므로 원본 9007544에 추가 커밋이 없는 경우 fast-forward할 수 있습니다.

```sh
git fetch origin
git merge --ff-only origin/main
```

4. 새 Mac 커밋 때문에 fast-forward가 안 되면 새 작업 브랜치에서 정상 병합하고 충돌을 해결합니다. 로컬 변경을 버리거나 force push하지 않습니다. 단순하게 새 clone을 사용하려면 기존 폴더 옆의 새 경로에서 아래처럼 시작해도 됩니다.

```sh
gh repo clone sheldon-im/gpt-model-switcher gpt-model-switcher-integrated
```

## 통합본의 Mac 검증

`MAC_CODEX_INSTRUCTIONS.md`와 `docs/INTEGRATION.md`를 읽고 `sh scripts/build-macos.sh`를 실행합니다. Swift AX 처리는 앱 내부에 정적으로 연결하며 별도 접근성 sidecar를 설치하지 않습니다. 기존 0.1.4 바이너리/사용자 설정을 보존하고, 새 임시 서명 파일은 사용자 승인 없이 설치본에 덮어쓰지 않습니다.

원본 0.1.4의 Mac 검증 보고와 통합 0.1.5의 결과를 분리합니다. 권한, 조용한 시작, 두 프리셋, 동일 프리셋 재입력, 입력/VoiceOver 커서·점자와 최종 한 번 안내를 다시 확인합니다. 추가 수정은 작업 브랜치에서 관련 검증과 함께 커밋하고 private 저장소에 push한 뒤 브랜치/커밋 ID와 결과를 인계합니다. 공개 전환·공개 패키지 배포는 승인 범위가 아닙니다.
