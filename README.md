# WUp

Rust + GPUI로 만든 Windows용 WinGet 패키지 관리자입니다. 현재 수정본은 v0.1.1이며 창 제목은 `WUp | WinGet Package Manager`입니다.

![WUp package manager screenshot](docs/images/wup-screenshot.png)

- 실행 시 업데이트 가능한 프로그램을 자동 조회합니다.
- 프로그램 이름, 패키지 ID, 현재 버전, 새 버전, 원본, 처리 상태를 표시합니다.
- 행을 클릭해 선택하고 **업데이트 (N)**를 누르면 선택한 항목만 순서대로 업데이트합니다.
- **전체 선택 / 전체 해제**는 하나의 버튼으로 전환됩니다. 일부 항목만 선택했다면 전체 선택, 선택 가능한 모든 항목을 선택했다면 전체 해제로 동작합니다.
- 통합 선택 버튼과 **전체 설치 패키지 보기**를 같은 줄에 표시합니다.
- 새로고침의 회전 화살표를 비롯해 주요 버튼과 체크박스에 단색 벡터 아이콘을 사용합니다. 별도 아이콘 파일이나 폰트가 필요하지 않습니다.
- 오른쪽 위 **… → 테마**에서 Charcoal Green / Dracula / One Dark / Nord / Tokyo Night / GitHub Light를 선택할 수 있습니다. 목록, 로그, 스크롤바, 버튼, 아이콘, 제거 확인창에 즉시 적용합니다.
- **… → 언어**에서 **한국어 / English**를 선택할 수 있습니다. 화면 문구, 상태 메시지, 확인창, 앱이 만든 오류 / 로그를 즉시 전환합니다.
- **… → UI 배율**에서 앱 전체 크기를 80%~120% 범위로 조절하고 다음 실행에도 유지합니다.
- **전체 설치 패키지 보기** 토글로 업데이트 목록과 설치된 전체 목록을 전환합니다. 전환하면 목록을 새로 조회하고 선택을 초기화합니다.
- 목록 오른쪽에 항상 스크롤바를 표시합니다. 마우스 휠, 스크롤바 드래그, 트랙 클릭으로 이동할 수 있습니다.
- 실행 로그 영역에도 별도 스크롤바를 표시합니다. 패키지 목록과 로그는 각각 독립적으로 스크롤됩니다.
- **제거 (N)**를 누르면 대상 이름 / ID / 버전을 확인한 뒤 제거할 수 있습니다.
- 전체 목록에서는 새 버전이 있는 선택 항목만 업데이트합니다. 원본이 없는 로컬 프로그램도 제거 대상으로 선택할 수 있습니다.
- 한 항목이 실패해도 다음 선택 항목을 계속 처리합니다. 실패한 항목은 다시 선택해 재시도할 수 있습니다.
- **현재 항목 완료 후 중단**으로 대기 중인 작업을 중단할 수 있습니다.
- 실행 로그를 확인하고 클립보드로 복사할 수 있습니다.

## 준비

1. Windows 10/11 x64, GPU 드라이버가 설치된 데스크톱 환경.
2. Rust stable의 MSVC 도구 체인 (`rustup update stable`).
3. Visual Studio Build Tools의 **C++를 사용한 데스크톱 개발** 및 Windows SDK.
4. Microsoft Store의 **앱 설치 관리자**에 포함된 최신 WinGet.
5. 첫 빌드 시 crates.io 및 GPUI 빌드 의존성을 다운로드할 인터넷 연결.

PowerShell에서 다음 명령으로 확인합니다.

```powershell
rustc --version
cargo --version
winget --version
```

## 실행

ZIP에는 소스와 Windows Release 실행 파일 `wup.exe`가 포함되어 있습니다. 앱 이름 WUp과 로고 W는 볼드체로 표시합니다.

프로젝트 폴더에서:

```powershell
cargo run --release
```

첫 빌드는 GPUI 및 그래픽 관련 라이브러리 때문에 시간이 걸릴 수 있습니다.
실행 파일은 `target\release\wup.exe`입니다. Release 빌드는 별도 콘솔 창을 표시하지 않습니다.

설치된 프로그램을 건드리지 않고 화면만 확인하려면:

```powershell
cargo run --release -- --demo
```

데모에서는 45개 예시 패키지로 목록 전환, 스크롤, 선택, 제거 확인 화면을 확인할 수 있습니다.
실제 조회 / 업데이트 / 제거 실행은 비활성화됩니다.

## 테마 선택

오른쪽 위 **…** 버튼을 누르면 테마 / 언어 / UI 배율을 모은 설정 메뉴가 뜹니다. 항목 위에 마우스를 올리거나 클릭하면 왼쪽에 하위 메뉴가 열립니다. **테마** 항목에서 원하는 테마를 선택합니다. 색상 미리보기와 현재 테마의 체크 표시를 보고 선택할 수 있습니다. 테마를 선택하거나 메뉴 밖을 클릭하면 닫히며, 목록과 로그 배치는 유지합니다.
다크 테마 5개와 밝은 GitHub Light를 제공합니다. Charcoal Green은 제공된 참고 화면의 차콜 회색 배경, 회색 선택 영역, 초록색 강조색을 적용한 테마입니다. 나머지는 각 편집기 테마의 색감을 앱 UI에 맞게 적용했습니다.
기본값은 **Tokyo Night**입니다.

선택한 테마는 `%LOCALAPPDATA%\Wup\theme.txt`에 저장하고 다음 실행 시 복원합니다.
`LOCALAPPDATA`가 없으면 `APPDATA`를 사용하며, 설정 파일이 없거나 손상된 경우 기본 테마로 시작합니다.
설정 저장에 실패해도 현재 화면에는 적용하고 실행 로그에 저장 실패를 표시합니다.

## 언어 선택

**… → 언어** 항목에서 하위 메뉴를 엽니다. **한국어** 또는 **English**를 선택하면 즉시 적용하고 메뉴를 닫습니다. 현재 언어에는 체크 표시가 붙으며, 메뉴 밖을 클릭해도 닫힙니다. 메뉴가 열려도 패키지 목록과 로그의 배치는 바뀌지 않습니다.
기본값은 한국어이며, 선택한 언어는 `%LOCALAPPDATA%\Wup\language.txt`에 저장해 다음 실행에 복원합니다.
테마와 언어 설정은 서로 독립적입니다. `LOCALAPPDATA`가 없으면 `APPDATA`를 사용합니다.

언어를 바꿔도 목록을 다시 조회하거나 선택 상태 / 진행 중인 작업을 초기화하지 않습니다.
기존 상태 메시지, 앱이 만든 오류, 앱 로그도 선택한 언어로 표시합니다. 패키지 이름, ID, 버전과 WinGet / 설치 프로그램의 원문 로그는 번역하지 않습니다.
로그 복사는 현재 선택한 언어로 표시한 앱 로그와 원문 로그를 시간순으로 복사합니다.

## UI 배율

**… → UI 배율 / UI scale** 하위 메뉴를 사용합니다. 80% / 90% / 100% / 110% / 120% 프리셋을 선택하거나 아래 조절기로 5% 단위로 변경합니다. 프리셋 선택 시 메뉴를 닫으며, − / + 조절 중에는 메뉴를 유지합니다.

- **− / +**: 5% 단위로 축소 / 확대, 최소 80%·최대 120%.
- 가운데 숫자: 현재 배율.
- **100%**: 새 기본 크기(초기 버전의 90%)로 초기화.

글자, 아이콘, 여백, 목록 행과 열, 로그, 스크롤바, 팝업 메뉴와 제거 확인창에 함께 적용합니다.
선택한 배율은 `%LOCALAPPDATA%\Wup\scale.txt`에 저장하며, `LOCALAPPDATA`가 없으면 `APPDATA`를 사용합니다.
기본값은 100%이며, 초기 버전의 90% 크기를 새 100% 기준으로 사용합니다. 기존 배율 설정은 새 기준의 가장 가까운 5% 단계로 환산합니다(기존 90% → 새 100%). 새 배율 설정은 `v2:100` 형식으로 저장해 중복 환산을 방지합니다. 확대 시 창이 좁으면 상단 버튼이 다음 줄로 배치될 수 있습니다.

## 빌드 및 테스트

```powershell
cargo fmt --check
cargo test
cargo build --release
```

또는 다음 스크립트로 검사 후 `dist\wup.exe`를 만듭니다.

```powershell
.\scripts\build.ps1
```

GPUI 다운로드 없이 WinGet 핵심 로직의 테스트만 실행하려면:

```powershell
.\scripts\test-core.ps1
```

## 사용 순서

1. 앱을 실행하고 자동 조회가 끝날 때까지 기다립니다.
2. 업데이트할 프로그램 행을 클릭합니다. 초기에는 아무 항목도 선택하지 않습니다.
3. **업데이트 (N)**를 누릅니다. 숫자는 선택 항목 중 업데이트 가능한 개수입니다.
4. 각 항목의 완료 / 실패 상태와 아래 로그를 확인합니다.
5. 설치된 전체 목록을 보려면 **전체 설치 패키지 보기**를 켭니다. 다시 누르면 업데이트 목록으로 돌아갑니다.
6. 프로그램을 제거하려면 항목을 선택하고 **제거 (N)**를 누릅니다. 확인 화면에서 대상을 검토한 뒤 **제거 실행**을 누릅니다.
7. 작업 종료 후 **새로고침**을 눌러 실제 설치 상태를 다시 확인합니다. 제거한 행은 새로고침 전까지 '제거 완료'로 표시됩니다.

업데이트 버튼을 누르면 선택한 패키지의 이용 약관을 수락하는 옵션으로 실행합니다.
관리자 권한이 필요한 설치 프로그램은 UAC 승인을 요청할 수 있습니다.
권한 문제로 실패하면 로그를 확인하고 WUp을 관리자 권한으로 다시 실행해 주세요.

## 구현

- `src/main.rs`: GPUI 화면, 선택 상태, 작업 큐 상태 표시, 로그, 중단 예약.
- `src/winget.rs`: WinGet 프로세스 실행, 출력 수집, 표 파싱, 업데이트 명령, 단위 테스트.
- `src/scrollbar.rs`: 스크롤바 표시 위치 / 크기 / 드래그 계산과 단위 테스트.
- `src/icons.rs`: GPUI 캔버스로 그리는 단색 벡터 아이콘.
- `src/theme.rs`: 테마 6개, 색상 팔레트, 설정 저장 / 복원, 텍스트 대비 및 설정 테스트.
- `src/i18n.rs`: 한국어 / 영어 선택, 언어 설정 저장 / 복원, 상태 / 오류 / 앱 로그의 다국어 텍스트.
- `src/ui_scale.rs`: UI 배율 범위 / 단계, 저장 / 복원 및 테스트.
- `src/pixel_snap.rs`: 행 높이와 구분 영역의 크기를 화면 픽셀 단위로 계산하고, 소수점 좌표 및 DPI별로 구분 영역이 유지되는지 검증합니다.
- GPUI 버전은 `0.2.2`로 고정했고 `Cargo.lock`을 포함합니다.

조회에는 다음 명령을 사용합니다. 이 명령은 프로그램을 업데이트하지 않습니다.

```powershell
winget upgrade --accept-source-agreements --disable-interactivity
```

전체 설치 목록은 다음 명령으로 조회합니다. 새 버전 / 원본 열이 생략되거나 개별 행의 값이 비어 있어도 처리합니다.

```powershell
winget list --accept-source-agreements --disable-interactivity
```

업데이트는 각 선택 항목마다 다음 명령을 별도 프로세스로 실행합니다.

```powershell
winget upgrade --id <정확한-ID> --exact --source <원본> --silent --accept-source-agreements --accept-package-agreements --disable-interactivity
```

제거는 확인 화면에 표시한 각 패키지에 대해 다음 명령을 실행합니다.
원본이 없으면 `--source`를 생략하고, 설치 버전이 알려진 경우 `--version`으로 버전도 지정합니다.

```powershell
winget uninstall --id <정확한-ID> --exact --version <설치-버전> --source <원본> --silent --accept-source-agreements --disable-interactivity
```

제거 시 `--all-versions`, `--purge`, `--force`는 사용하지 않습니다.

셸 문자열로 실행하지 않고 `Command::args`로 인자를 전달합니다.
목록 조회와 설치는 작업 스레드에서 실행하며, GPUI는 채널로 전달받은 결과만 화면에 반영합니다.

## 범위와 제한

- 한국어 / 영어 WinGet의 텍스트 표 출력을 지원합니다. WinGet에 일반적인 `upgrade --output json` 기능이 없으므로 열의 표시 너비를 기준으로 파싱합니다.
- 알 수 없는 출력은 오류를 표시합니다. 잘린 ID는 목록에 표시하되 업데이트 / 제거를 비활성화합니다. 특수 이모지 / 결합 문자가 포함된 이름이나 향후 WinGet 출력 변경은 추가 대응이 필요할 수 있습니다.
- 업데이트 전용 목록은 기본 `winget upgrade`, 전체 목록은 `winget list` 결과를 표시합니다. Windows의 모든 시스템 구성 요소를 별도로 열거하지는 않습니다. 알 수 없는 버전과 고정된 패키지를 강제로 포함하는 옵션은 사용하지 않습니다.
- 프로그램 업데이트는 사용자가 버튼을 누를 때만 실행합니다. 패키지 설치 프로그램 자체가 처리하는 의존성은 WinGet의 기본 동작을 따릅니다.
- 자동 재부팅 허용 옵션과 강제 설치 옵션은 사용하지 않습니다. 재부팅 필요 메시지나 비정상 종료 코드는 로그에서 확인합니다.
- 중단은 현재 설치가 끝난 다음 적용됩니다. 작업 중 창 닫기는 보류하고 남은 업데이트 중단을 예약합니다.
- 조회 제한 시간은 3분, 항목별 업데이트 제한 시간은 1시간입니다. 시간 초과 시 WinGet 프로세스를 종료하지만 하위 설치 프로그램은 남아 있을 수 있습니다.
- 로그는 메모리에 최근 1,000줄을 보관하며 화면에는 최신 120줄부터 표시합니다.

## 이 환경에서 확인한 내용

전체 GPUI 앱의 Release 빌드와 핵심 로직 테스트 25개를 통과했습니다.
실제 GPUI 창에서 메인 / 테마 / 언어 / 배율 메뉴를 80% / 95% / 100% / 120%로 렌더링하는 자동 검증을 통과했습니다. 95%에서는 소수점 스크롤 위치도 적용합니다. v0.1.1에서 실제 레이아웃의 행 간격 510곳을 검사해 통과했습니다. 화면 캡처를 통한 시각 비교와 마우스로 휠 / 드래그 / 확인 버튼을 직접 조작하는 검증은 별도입니다.
실제 프로그램을 업데이트하거나 제거하는 검증은 수행하지 않았습니다.

메뉴 렌더링만 다시 확인하려면 다음 명령을 실행합니다. 데모 데이터를 사용하고, 검사 후 자동 종료합니다. WinGet 실행이나 테마 / 언어 / 배율 설정 저장은 하지 않습니다.

```powershell
.\target\release\wup.exe --smoke-test-menus
```

메뉴 종료 오류는 GPUI 0.2.2에서 지연 그리기 안의 중첩 지연 그리기를 허용하지 않아 발생했습니다. 하위 메뉴는 부모 팝업과 같은 그리기 단계에서 처리하도록 수정했습니다.

v0.1.1은 행 구분선을 별도로 그리지 않습니다. 행 내용의 배경과 부모 배경의 구분 영역을 레이아웃에서 분리하고, 행 높이를 화면 픽셀 단위로 맞춥니다. 구분 영역은 최소 2물리 픽셀 및 1논리 픽셀을 확보합니다. 이전의 1px 선 보정만으로 해결되지 않은 경우를 대응한 방식입니다. 자체 검증에서는 실제 GPUI 행 영역과 내용 영역의 높이를 비교해 구분 공간이 최소 1물리 픽셀 이상인지 검사합니다.

## 참고

- GPUI: https://docs.rs/gpui/0.2.2/gpui/
- WinGet upgrade: https://learn.microsoft.com/windows/package-manager/winget/upgrade
- WinGet list: https://learn.microsoft.com/windows/package-manager/winget/list
- WinGet uninstall: https://learn.microsoft.com/windows/package-manager/winget/uninstall

- Active package operations display an animated progress ring in the Status column; completion and failure labels remain visible.

## GitHub Actions: Windows EXE / MSI

Commit `.github/`, `.config/`, `installer/`, `scripts/`, `build.rs`, `Cargo.toml`, `Cargo.lock`, and `src/` to your GitHub repository. Every **branch push** starts the Windows build. A local commit alone cannot trigger GitHub; push it to the remote. One push containing several commits builds its latest commit. You can also use **Actions > Windows build > Run workflow**.

After a successful run, open **Actions > Windows build > the run > Artifacts** and download:

- `WUp-<version>-windows-x64.exe`: EXE setup wizard with the MSI embedded; double-click to install.
- `WUp-<version>-windows-x64.msi`: per-user installer with a Start menu shortcut, installed under `%LOCALAPPDATA%\Programs\WUp`. Newer MSI versions upgrade the existing install. Older versions are blocked.
- `SHA256SUMS.txt` and `build-info.json`: checksums, version, commit SHA and workflow run details.

The workflow uses `base patch + github.run_number`, starting from the checked-in Cargo version `0.1.1`: run 1 produces `0.1.2`, run 2 produces `0.1.3`. The About dialog, Windows EXE version resources, MSI product version, and filenames all use the same generated version. The window title is `WUp | WinGet Package Manager`, without a version. A rerun of the same workflow run retains its version; the artifact name includes the attempt number. Manual runs also consume a build number. Failed runs can leave gaps in the sequence.

Version edits happen only in the CI checkout; the workflow does not commit version changes back to the repository. Keep the checked-in base patch stable while using this version line. Change the major/minor baseline when starting a new version line. MSI limits are checked before writing; if the generated patch would exceed 65535, bump the minor version and reset the base patch (the workflow counter continues). Keep the workflow file path stable to preserve its run counter.

The CI job checks formatting, tests version numbering, runs the core tests, builds Rust in Release mode with the locked dependency set, then builds MSI and a WiX Burn EXE installer with the pinned WiX 4.0.6 CLI and Bal extension. It needs no repository secrets. Packages are retained for 30 days as workflow artifacts. After a successful build, a separate job creates a version tag (for example `v0.1.2`) for the exact built commit and publishes a GitHub Release containing the EXE, MSI, SHA256SUMS.txt and build-info.json. Default-branch builds create normal releases; other branches create prereleases. Only the release job receives `contents: write`, using the built-in GitHub token; no extra secret is needed. The workflow listens to branch pushes only, so generated tags do not trigger another build. Uploads complete in a draft before publication; retries resume drafts and preserve already-published files. Repositories with rules that prevent the built-in token from creating tags/releases must permit this workflow.

### Local packaging

Install Rust stable with MSVC tools / Windows SDK, and the .NET 8 SDK, then run from the project root:

```powershell
dotnet tool restore
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build.ps1 -Installer
```

For a local portable application only, omit `-Installer` (no .NET SDK required); this creates `dist/wup.exe` and does not overwrite the versioned EXE installer. `scripts/set-build-version.ps1 -BuildNumber N` is intended for a fresh CI checkout and modifies both Cargo files. Local builds otherwise retain the version in Cargo.toml. Commit source and configuration only; do not commit `target/`, `dist/`, or transfer helper directories.

## About

Open **... > About** (Korean: app information) for the application version and a brief description. The dialog follows the selected theme, language and UI scale. The compact layout shows inline green contact links and an orange Buy me a coffee button. Close it with the top-right X or by clicking the backdrop.

- Developer email: [chanhwahong@gmail.com](mailto:chanhwahong@gmail.com)
- GitHub: https://github.com/chanhwahong/wup
- Buy me a coffee: https://ko-fi.com/chanhwahong

Links open the default browser or mail application only when clicked.

## Application icon

WUp uses a balloon emoji-style icon in its executable, window/taskbar, Start menu shortcut, installed-app entry, and header. The icon is embedded, so no external image files are required at runtime.

Balloon artwork: Twemoji by Twitter, Inc. and other contributors, licensed under CC BY 4.0. See THIRD-PARTY-NOTICES.txt and assets/LICENSE-TWEMOJI.txt. Commit assets/ and THIRD-PARTY-NOTICES.txt with the rest of the source for CI builds.

### EXE installer

Both versioned release downloads install WUp. The EXE shows a setup wizard and embeds the same MSI distributed alongside it, so it does not need to download the MSI. It uses the balloon icon, the same build version and a stable bundle UpgradeCode for future upgrades. When installed through the EXE, the internal MSI entry is hidden to avoid duplicate installed-app entries. Installing the MSI directly still creates its normal WUp entry.

The application itself is `target/release/wup.exe`; the release EXE is built separately from `installer/Bundle.wxs`. Do not upload the raw application executable as the versioned installer.

## Package filter

Type a package name or ID in the filter field on the same row as the selection and package actions. Filtering is immediate, case-insensitive, and applies to both available updates and the full installed-package list. Separate words with spaces to require all words across the name and ID. The right-hand count shows matching / total packages; X clears the filter.

Select all / deselect all and update / uninstall actions apply to visible packages. Changing the filter deselects packages that become hidden, preventing hidden items from being changed by mistake. Refreshing or switching list modes keeps the filter. The field supports Korean IME composition, selection, keyboard navigation and Ctrl+A/C/V/X. Editing is paused while a package operation or confirmation/about dialog is active.

The EXE setup welcome page displays **Windows packages Update and uninstall**.
