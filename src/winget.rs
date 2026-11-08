//! WinGet process execution and conservative, locale-aware table parsing.
//! Independent of GPUI; tested with the i18n module by scripts/test-core.ps1.
use crate::i18n::{Language, Text};
use std::{
    collections::HashSet,
    io::Read,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub id: String,
    pub installed: String,
    pub available: String,
    pub source: String,
}

impl Package {
    pub fn can_uninstall(&self) -> bool {
        valid_id(&self.id)
    }
    pub fn can_upgrade(&self) -> bool {
        self.can_uninstall() && !self.available.is_empty() && valid_source(&self.source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListMode {
    Updates,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Upgrade,
    Uninstall,
}

impl Operation {
    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::Upgrade => language.t("업데이트", "Update"),
            Self::Uninstall => language.t("제거", "Uninstall"),
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Log(String),
    Scanned(Result<Vec<Package>, Text>),
    Started(usize),
    Finished(usize, Result<(), Text>),
    Done { cancelled: bool },
}

pub fn scan(mode: ListMode, tx: Sender<Event>) {
    thread::spawn(move || {
        let args = vec![
            match mode {
                ListMode::Updates => "upgrade",
                ListMode::All => "list",
            }
            .into(),
            "--accept-source-agreements".into(),
            "--disable-interactivity".into(),
        ];
        let result = run(&args, &tx, Duration::from_secs(180)).and_then(|(code, output)| {
            match code as u32 {
                0 => parse_table(&output),
                // APPINSTALLER_CLI_ERROR_NO_APPLICATIONS_FOUND / UPDATE_NOT_APPLICABLE
                0x8A150014 | 0x8A15002B => Ok(Vec::new()),
                _ => Err(Text::new(
                    format!(
                        "목록 조회 실패 (0x{:08X}). 실행 로그를 확인하세요.",
                        code as u32
                    ),
                    format!(
                        "Package query failed (0x{:08X}). Check the execution log.",
                        code as u32
                    ),
                )),
            }
        });
        let _ = tx.send(Event::Scanned(result));
    });
}

pub fn upgrade_args(package: &Package) -> Result<Vec<String>, Text> {
    if !package.can_upgrade() {
        return Err(Text::new(
            "잘리거나 유효하지 않은 패키지 ID / 원본입니다. 새로고침해 주세요.",
            "The package ID or source is truncated or invalid. Refresh the list.",
        ));
    }
    // No shell and no --all: each request targets exactly one ID and source.
    Ok(vec![
        "upgrade".into(),
        "--id".into(),
        package.id.clone(),
        "--exact".into(),
        "--source".into(),
        package.source.clone(),
        "--silent".into(),
        "--accept-source-agreements".into(),
        "--accept-package-agreements".into(),
        "--disable-interactivity".into(),
    ])
}

pub fn uninstall_args(package: &Package) -> Result<Vec<String>, Text> {
    if !package.can_uninstall() {
        return Err(Text::new(
            "잘린 패키지 ID는 제거할 수 없습니다.",
            "A truncated package ID cannot be uninstalled.",
        ));
    }
    let mut args = vec![
        "uninstall".into(),
        "--id".into(),
        package.id.clone(),
        "--exact".into(),
    ];
    // Local ARP entries often have no repository source. Never invent one.
    if valid_source(&package.source) {
        args.extend(["--source".into(), package.source.clone()]);
    }
    if !package.installed.is_empty()
        && !matches!(package.installed.as_str(), "Unknown" | "알 수 없음")
        && !package.installed.contains('…')
        && !package.installed.contains("...")
    {
        args.extend(["--version".into(), package.installed.clone()]);
    }
    args.extend([
        "--silent".into(),
        "--accept-source-agreements".into(),
        "--disable-interactivity".into(),
    ]);
    Ok(args)
}

pub fn execute(
    operation: Operation,
    packages: Vec<(usize, Package)>,
    tx: Sender<Event>,
    stop: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        for (index, package) in packages {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let _ = tx.send(Event::Started(index));
            let args = match operation {
                Operation::Upgrade => upgrade_args(&package),
                Operation::Uninstall => uninstall_args(&package),
            };
            let result = args.and_then(|args| {
                run(&args, &tx, Duration::from_secs(3600)).and_then(|(code, _)| {
                    if code == 0 {
                        Ok(())
                    } else {
                        Err(Text::new(
                            format!("종료 코드 0x{:08X} — 로그 확인", code as u32),
                            format!("Exit code 0x{:08X} — check logs", code as u32),
                        ))
                    }
                })
            });
            let _ = tx.send(Event::Finished(index, result));
        }
        let _ = tx.send(Event::Done {
            cancelled: stop.load(Ordering::Relaxed),
        });
    });
}

fn run(args: &[String], tx: &Sender<Event>, timeout: Duration) -> Result<(i32, String), Text> {
    let _ = tx.send(Event::Log(format!("> winget {}", args.join(" "))));
    let mut command = Command::new("winget.exe");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(|e| {
        Text::new(
            format!(
                "winget 실행 실패: {e}. Microsoft Store의 '앱 설치 관리자'와 PATH를 확인하세요."
            ),
            format!("Failed to start winget: {e}. Check Microsoft Store App Installer and PATH."),
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Text::new("stdout 연결 실패", "Could not connect stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Text::new("stderr 연결 실패", "Could not connect stderr"))?;
    let tx_out = tx.clone();
    let tx_err = tx.clone();
    let out = thread::spawn(move || read_stream(stdout, tx_out));
    let err = thread::spawn(move || read_stream(stderr, tx_err));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Text::new(
                    "winget 응답 시간이 초과되었습니다. 설치 프로세스가 남아 있는지 확인하세요.",
                    "WinGet timed out. Check whether a child installer is still running.",
                ));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Text::new(
                    format!("winget 상태 확인 실패: {e}"),
                    format!("Could not read WinGet process status: {e}"),
                ));
            }
        }
    };
    let output = out
        .join()
        .map_err(|_| Text::new("stdout 읽기 스레드 오류", "stdout reader thread failed"))??;
    let errors = err
        .join()
        .map_err(|_| Text::new("stderr 읽기 스레드 오류", "stderr reader thread failed"))??;
    Ok((status.code().unwrap_or(-1), format!("{output}\n{errors}")))
}

fn read_stream(mut stream: impl Read, tx: Sender<Event>) -> Result<String, Text> {
    let mut output = String::new();
    let mut pending = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = stream.read(&mut buffer).map_err(|e| {
            Text::new(
                format!("출력 읽기 실패: {e}"),
                format!("Could not read process output: {e}"),
            )
        })?;
        if count == 0 {
            break;
        }
        for &byte in &buffer[..count] {
            if byte == b'\n' || byte == b'\r' {
                flush_line(&mut pending, &mut output, &tx);
            } else {
                pending.push(byte);
                if pending.len() >= 65536 {
                    flush_line(&mut pending, &mut output, &tx);
                }
            }
        }
    }
    flush_line(&mut pending, &mut output, &tx);
    Ok(output)
}

fn flush_line(pending: &mut Vec<u8>, output: &mut String, tx: &Sender<Event>) {
    if pending.is_empty() {
        return;
    }
    let line = clean_output(&String::from_utf8_lossy(pending));
    pending.clear();
    let trimmed = line.trim();
    if trimmed.is_empty() || matches!(trimmed, "-" | "\\" | "|" | "/") {
        return;
    }
    output.push_str(&line);
    output.push('\n');
    let _ = tx.send(Event::Log(line));
}

pub fn clean_output(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
        } else if ch == '\n' || ch == '\r' || (!ch.is_control() && ch != '\u{feff}') {
            result.push(ch);
        }
    }
    result
}

// WinGet aligns its text table in display cells, not UTF-8 bytes. Cover the
// Korean/English headers and common CJK package names without another runtime.
fn char_width(c: char) -> usize {
    match c as u32 {
        0x0300..=0x036f
        | 0x1ab0..=0x1aff
        | 0x1dc0..=0x1dff
        | 0x200b..=0x200f
        | 0x202a..=0x202e
        | 0x2060..=0x206f
        | 0xfe00..=0xfe0f
        | 0xfe20..=0xfe2f => 0,
        0x1100..=0x115f
        | 0x2329..=0x232a
        | 0x2e80..=0xa4cf
        | 0xac00..=0xd7a3
        | 0xf900..=0xfaff
        | 0xfe10..=0xfe19
        | 0xfe30..=0xfe6f
        | 0xff01..=0xff60
        | 0xffe0..=0xffe6
        | 0x1f300..=0x1faff
        | 0x20000..=0x3fffd => 2,
        _ => 1,
    }
}

fn width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

fn cell_slice(text: &str, start: usize, end: usize) -> String {
    let mut pos = 0;
    text.chars()
        .filter(|&c| {
            let previous = pos;
            pos += char_width(c);
            previous >= start && previous < end
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.contains('…')
        && !id.contains("...")
        && !id.chars().any(|c| c.is_control())
}

fn valid_source(source: &str) -> bool {
    valid_id(source) && !source.trim().is_empty()
}

struct Columns {
    starts: Vec<usize>,
    available: Option<usize>,
    source: Option<usize>,
}

fn columns(header: &str) -> Option<Columns> {
    let locales = [
        ["Name", "Id", "Version", "Available", "Source"],
        ["Name", "ID", "Version", "Available", "Source"],
        ["이름", "장치 ID", "버전", "사용 가능", "원본"],
        ["이름", "ID", "버전", "사용 가능", "원본"],
        ["이름", "Id", "버전", "사용 가능", "원본"],
    ];
    for labels in locales {
        let mut offset = 0;
        let mut starts = Vec::new();
        for label in &labels[..3] {
            let Some(index) = header[offset..].find(label) else {
                break;
            };
            let byte = offset + index;
            starts.push(width(&header[..byte]));
            offset = byte + label.len();
        }
        if starts.len() == 3 {
            let mut available = None;
            let mut source = None;
            if let Some(index) = header[offset..].find(labels[3]) {
                let byte = offset + index;
                available = Some(starts.len());
                starts.push(width(&header[..byte]));
                offset = byte + labels[3].len();
            }
            if let Some(index) = header[offset..].find(labels[4]) {
                source = Some(starts.len());
                starts.push(width(&header[..offset + index]));
            }
            return Some(Columns {
                starts,
                available,
                source,
            });
        }
    }
    None
}

pub fn parse_table(output: &str) -> Result<Vec<Package>, Text> {
    let clean = clean_output(output);
    let lines: Vec<_> = clean.lines().collect();
    let mut packages = Vec::new();
    let mut seen = HashSet::new();
    let mut found_table = false;
    for (index, line) in lines.iter().enumerate() {
        let separator = line.trim();
        if separator.len() < 5 || !separator.chars().all(|c| c == '-') || index == 0 {
            continue;
        }
        let Some(columns) = columns(lines[index - 1]) else {
            continue;
        };
        let starts = &columns.starts;
        found_table = true;
        for row in &lines[index + 1..] {
            if row.trim().is_empty() {
                break;
            }
            let fields: Vec<_> = starts
                .iter()
                .enumerate()
                .map(|(i, start)| {
                    cell_slice(
                        row,
                        *start,
                        starts.get(i + 1).copied().unwrap_or(usize::MAX),
                    )
                })
                .collect();
            if fields[0].is_empty() || fields[1].is_empty() || fields[2].is_empty() {
                break;
            }
            let source = columns
                .source
                .map(|i| fields[i].clone())
                .unwrap_or_default();
            let available = columns
                .available
                .map(|i| fields[i].clone())
                .unwrap_or_default();
            // Keep truncated IDs visible, but disable all actions on them.
            if seen.insert((fields[1].clone(), source.clone(), fields[2].clone())) {
                packages.push(Package {
                    name: fields[0].clone(),
                    id: fields[1].clone(),
                    installed: fields[2].clone(),
                    available,
                    source,
                });
            }
        }
    }
    if !found_table {
        return Err(Text::new(
            "업데이트 표를 찾지 못했습니다. 한국어/영어 winget 출력만 지원합니다. 실행 로그를 확인하세요.",
            "No package table was found. Only Korean/English WinGet output is supported. Check the execution log.",
        ));
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_errors_and_operation_names_support_both_languages() {
        assert_eq!(Operation::Upgrade.label(Language::Korean), "업데이트");
        assert_eq!(Operation::Uninstall.label(Language::English), "Uninstall");
        let error = parse_table("source is unavailable").unwrap_err();
        assert!(error.value(Language::English).contains("package table"));
        assert_ne!(
            error.value(Language::Korean),
            error.value(Language::English)
        );
        let package = Package {
            name: "App".into(),
            id: "Vendor.App…".into(),
            installed: "1".into(),
            available: "2".into(),
            source: "winget".into(),
        };
        assert!(
            uninstall_args(&package)
                .unwrap_err()
                .value(Language::English)
                .contains("truncated")
        );
    }

    fn row(parts: &[&str]) -> String {
        let widths = [32, 30, 18, 18];
        parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                if i == 4 {
                    part.to_string()
                } else {
                    format!("{part}{}", " ".repeat(widths[i] - width(part)))
                }
            })
            .collect()
    }

    fn table(headers: &[&str], rows: &[&[&str]]) -> String {
        format!(
            "{}\n{}\n{}\n\n2 upgrades available.\n",
            row(headers),
            "-".repeat(110),
            rows.iter().map(|r| row(r)).collect::<Vec<_>>().join("\n")
        )
    }

    #[test]
    fn parses_english_and_preserves_names_with_spaces() {
        let data = table(
            &["Name", "Id", "Version", "Available", "Source"],
            &[
                &[
                    "Microsoft Visual Studio Code",
                    "Microsoft.VisualStudioCode",
                    "1.0",
                    "2.0",
                    "winget",
                ],
                &["7-Zip", "7zip.7zip", "24.0", "25.0", "winget"],
            ],
        );
        let packages = parse_table(&data).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "Microsoft.VisualStudioCode");
        assert_eq!(packages[0].name, "Microsoft Visual Studio Code");
    }

    #[test]
    fn parses_korean_display_width_and_unknown_versions() {
        let data = table(
            &["이름", "장치 ID", "버전", "사용 가능", "원본"],
            &[&["한글 프로그램", "Vendor.App", "알 수 없음", "2.0", "winget"]],
        );
        let package = &parse_table(&data).unwrap()[0];
        assert_eq!(package.name, "한글 프로그램");
        assert_eq!(package.installed, "알 수 없음");
        assert_eq!(package.available, "2.0");
    }

    #[test]
    fn strips_terminal_sequences_and_bom() {
        assert_eq!(clean_output("\u{feff}\x1b[31mhello\x1b[0m"), "hello");
        assert_eq!(clean_output("\x1b]0;title\x07text"), "text");
    }

    #[test]
    fn rejects_unrecognized_output_and_disables_truncated_ids() {
        assert!(parse_table("source is unavailable").is_err());
        let data = table(
            &["Name", "Id", "Version", "Available", "Source"],
            &[&["Long name", "Vendor.Long…", "1", "2", "winget"]],
        );
        let package = parse_table(&data).unwrap().remove(0);
        assert!(!package.can_uninstall());
        assert!(upgrade_args(&package).is_err());
        assert!(uninstall_args(&package).is_err());
    }

    #[test]
    fn only_selected_exact_id_is_targeted_without_shell_or_all() {
        let package = Package {
            name: "Test".into(),
            id: "Vendor.App".into(),
            installed: "1".into(),
            available: "2".into(),
            source: "winget".into(),
        };
        let args = upgrade_args(&package).unwrap();
        assert!(args.windows(2).any(|a| a == ["--id", "Vendor.App"]));
        assert!(args.windows(2).any(|a| a == ["--source", "winget"]));
        assert!(args.contains(&"--exact".into()));
        assert!(!args.contains(&"--all".into()));
        assert!(!args.contains(&"--allow-reboot".into()));
    }

    #[test]
    fn duplicate_ids_in_different_sources_stay_separate() {
        let data = table(
            &["Name", "Id", "Version", "Available", "Source"],
            &[
                &["App", "Vendor.App", "1", "2", "winget"],
                &["App", "Vendor.App", "1", "2", "private"],
            ],
        );
        assert_eq!(parse_table(&data).unwrap().len(), 2);
    }

    #[test]
    fn installed_list_keeps_rows_without_updates_or_sources() {
        let data = table(
            &["Name", "Id", "Version", "Available", "Source"],
            &[
                &["Updatable", "Vendor.App", "1", "2", "winget"],
                &["Local application", "ARP\\Machine\\Local App", "1", "", ""],
                &["Current", "Vendor.Current", "2", "", "winget"],
            ],
        );
        let packages = parse_table(&data).unwrap();
        assert_eq!(packages.len(), 3);
        assert!(packages[0].can_upgrade());
        assert!(!packages[1].can_upgrade());
        assert!(packages[1].can_uninstall());
        assert!(!packages[2].can_upgrade());
    }

    #[test]
    fn installed_table_without_available_column() {
        let data = table(
            &["Name", "Id", "Version", "Source"],
            &[
                &["Current", "Vendor.App", "2", "winget"],
                &["Local", "ARP\\Machine\\Local App", "1", ""],
            ],
        );
        let packages = parse_table(&data).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].source, "winget");
        assert!(packages[0].available.is_empty());
        assert_eq!(packages[1].source, "");
        assert!(packages[1].can_uninstall());
    }

    #[test]
    fn installed_table_without_optional_columns() {
        let data = table(
            &["이름", "ID", "버전"],
            &[&["로컬 앱", "ARP\\Machine\\Local App", "1.0"]],
        );
        let packages = parse_table(&data).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].installed, "1.0");
        assert!(packages[0].source.is_empty());
        assert!(packages[0].available.is_empty());
    }

    #[test]
    fn uninstall_uses_exact_local_id_and_version_without_force_or_purge() {
        let package = Package {
            name: "Local App".into(),
            id: "ARP\\Machine\\Local App".into(),
            installed: "1.0".into(),
            available: String::new(),
            source: String::new(),
        };
        let args = uninstall_args(&package).unwrap();
        assert_eq!(args[0], "uninstall");
        assert!(
            args.windows(2)
                .any(|a| a == ["--id", "ARP\\Machine\\Local App"])
        );
        assert!(args.windows(2).any(|a| a == ["--version", "1.0"]));
        assert!(args.contains(&"--exact".into()));
        for forbidden in [
            "--all",
            "--all-versions",
            "--purge",
            "--force",
            "--source",
            "--accept-package-agreements",
        ] {
            assert!(!args.iter().any(|a| a == forbidden));
        }
    }

    #[test]
    fn uninstall_with_source_and_unknown_version() {
        let package = Package {
            name: "App".into(),
            id: "Vendor.App".into(),
            installed: "알 수 없음".into(),
            available: String::new(),
            source: "winget".into(),
        };
        let args = uninstall_args(&package).unwrap();
        assert!(args.windows(2).any(|a| a == ["--source", "winget"]));
        assert!(!args.iter().any(|a| a == "--version"));
        assert!(upgrade_args(&package).is_err());
    }
}
