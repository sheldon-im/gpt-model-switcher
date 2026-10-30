# 기존 Mac 수정본을 GitHub 이력에 반영하기

대상은 `sheldon-im/gpt-model-switcher` private 저장소입니다. `gh`에 해당 저장소 접근 권한이 있는 계정으로 로그인하십시오. Windows가 올린 소스를 받았다는 사실과 기존 Mac 변경분이 병합됐다는 사실은 구분합니다.

1. 기존 Mac 수정 폴더를 보존하고, 추적되지 않은 소스 파일을 포함해 변경 목록과 별도 백업을 확보합니다. 현재 폴더가 이미 Git 저장소라면 로컬 변경을 의미 있는 구성요소별 커밋으로 보존합니다. 빌드 산출물과 인증/개인 설정은 반영 대상에 포함하지 않습니다.
2. 기존 수정 폴더를 덮어쓰지 않는 **새 폴더**에 clone합니다. 예를 들어 기존 폴더 옆에서 실행합니다.

```sh
gh repo clone sheldon-im/gpt-model-switcher gpt-model-switcher-sync
cd gpt-model-switcher-sync
```

3. 기존 ZIP의 `SOURCE_SNAPSHOT.json`에서 `sourceCommit`을 확인합니다. 그 커밋은 Windows 저장소 이력에 있습니다. 새 clone에서 이 커밋을 기준으로 Mac 통합 브랜치를 만들고, **기존 Mac 수정으로 생긴 차이만** 적용·검증·커밋합니다. 기존 폴더가 Git 이력이 있다면 패치/커밋을 사용하고, 없다면 해당 원본 커밋과 수정본을 비교하여 변경·추가·삭제 파일을 빠짐없이 식별합니다. 아래 `<sourceCommit>`은 실제 값으로 바꿉니다.

```sh
git switch -c macos-integration <sourceCommit>
```

4. Mac 변경 커밋을 만든 뒤 최신 Windows `main`을 병합합니다. 파일 전체를 최신 Windows 버전 위에 복사하면 포커스 보존, NVDA 음성 복구, 이미 적용된 프리셋 처리 등이 사라질 수 있으므로 세 방향 병합으로 충돌을 해결합니다.

```sh
git fetch origin
git merge origin/main
```

5. `MAC_CODEX_INSTRUCTIONS.md`의 빌드·AX/VoiceOver 검증을 수행하고, 관련 수정과 검증을 완결된 구성요소별 커밋으로 남깁니다. 검증 후 통합 브랜치를 올리고 브랜치명·커밋 ID·실제 Mac 검증 결과를 Windows 작업에 인계합니다.

```sh
git push -u origin macos-integration
```

기존 수정 폴더나 로컬 변경을 버리는 초기화·강제 덮어쓰기·force push는 필요하지 않습니다. 기준 커밋을 알 수 없으면 새 clone에 기존 폴더를 통째로 복사하지 말고, 기존 ZIP/로컬 Git 이력에서 기준을 먼저 확인하십시오.
