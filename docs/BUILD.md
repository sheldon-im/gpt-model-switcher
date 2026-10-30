# 소스에서 빌드하기

사용자를 위한 설치와 사용 방법은 [README](../README.md)를 참고하십시오. 이 문서는 기존 README의 개발 환경·빌드 안내를 옮긴 것입니다.

## 준비

공통으로 Node.js 22 이상, pnpm 10, Rust stable이 필요합니다. Windows는 Visual Studio C++ Build Tools와 WebView2, Mac은 Xcode Command Line Tools가 필요합니다.

```sh
pnpm install --frozen-lockfile
cargo test --workspace --locked
pnpm test
pnpm build
```

## Windows

PowerShell에서 실행합니다.

```powershell
pnpm tauri build --bundles nsis
```

설치본은 `target/release/bundle/nsis/`, 단독 실행 파일은 `target/release/gpt-model-switcher.exe`에 생성됩니다. 코드 서명은 하지 않습니다. 빌드와 실제 설치는 별도입니다.

ZIP을 배포할 때는 실행 파일과 함께 `nvda` 폴더의 DLL·라이선스·대응 소스를 보존하십시오. 공식 NVDA 2026.3 Controller Client의 출처·해시·LGPL 2.1 라이선스와 전체 대응 소스가 해당 폴더에 있습니다.

## Mac

[Mac 작업 지침](../MAC_CODEX_INSTRUCTIONS.md)을 먼저 읽으십시오. Apple Silicon Mac에서 다음을 실행합니다.

```sh
sh scripts/build-macos.sh
```

이 스크립트는 의존성 설치, Rust·화면·Swift 검사와 임시 서명 앱·DMG 빌드를 수행합니다. 앱은 `target/aarch64-apple-darwin/release/bundle/macos/`, DMG는 같은 `bundle/` 아래 `dmg/`에 생성됩니다.

유료 Apple 개발자 계정 없이 임시 서명으로 빌드합니다. Developer ID 정식 서명과 공증은 포함하지 않습니다. 설치된 앱의 교체와 릴리스 발행은 빌드 작업과 구분하십시오.

## 설정과 검증 기록

설정은 Tauri 사용자 설정 폴더의 `io.github.gpt-model-switcher.desktop/settings.json`에 저장합니다. Windows는 보통 `%APPDATA%`, macOS는 `~/Library/Application Support` 아래입니다. 변경 전에 `.bak`을 보존하며 손상되거나 미래 버전인 원본은 덮어쓰지 않습니다. 최근 결과는 메모리에만 유지합니다.

자동 검사, 실제 앱 검사, 스크린 리더 사용자 확인은 구분해서 기록하십시오. 기존 [공통 검증 기록](VERIFICATION.md)과 [Mac 0.1.4 기록](MAC_VERIFICATION.md)은 당시 버전의 기록입니다. Mac 0.1.5의 빌드·자동 검사와 사용자 VoiceOver 확인은 [0.1.5 릴리스](https://github.com/sheldon-im/gpt-model-switcher/releases/tag/v0.1.5)에 첨부한 `MAC_VERIFICATION-0.1.5.json`에 있습니다.
