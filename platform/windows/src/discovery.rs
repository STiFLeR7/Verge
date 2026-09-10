//! Per-user discovery stays in Verge's background poller; installed is never inferred to mean working.
use std::{path::PathBuf, time::SystemTime};
use verge_core::domain::*;
use windows::Win32::{
    Foundation::CloseHandle,
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    },
};

pub const PROVIDERS: &[(ToolId, &str, &[&str])] = &[
    (ToolId::ClaudeCode, "Claude", &["claude"]),
    (ToolId::Codex, "ChatGPT", &["codex", "chatgpt"]),
    (ToolId::Grok, "Grok", &["grok"]),
    (ToolId::KiloCode, "KiloCode", &["kilo", "kilocode"]),
    (ToolId::Hermes, "Hermes", &["hermes"]),
    (ToolId::Pi, "Pi", &["pi"]),
    (ToolId::OpenCode, "OpenCode", &["opencode"]),
    (ToolId::Cursor, "Cursor", &["cursor"]),
    (ToolId::Antigravity, "Antigravity", &["antigravity"]),
];

pub fn discover_tools() -> Vec<AmbientState> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend([
        home.join(".local/bin"),
        home.join("AppData/Roaming/npm"),
        home.join(".cargo/bin"),
        home.join("AppData/Local/Microsoft/WindowsApps"),
    ]);
    let processes = process_names();
    let packages = package_names();
    let mut found = Vec::new();
    for &(tool, label, names) in PROVIDERS {
        let executable = names.iter().find_map(|name| {
            dirs.iter().find_map(|dir| {
                ["exe", "cmd", "ps1"]
                    .into_iter()
                    .map(|ext| dir.join(format!("{name}.{ext}")))
                    .find(|p| executable_present(p))
            })
        });
        let desktop = names.iter().find_map(|name| {
            [
                home.join(format!("AppData/Local/Programs/{name}/{name}.exe")),
                home.join(format!("AppData/Local/{name}/{name}.exe")),
            ]
            .into_iter()
            .find(|p| executable_present(p))
        });
        let open = processes.iter().any(|name| {
            names
                .iter()
                .any(|candidate| name == &format!("{candidate}.exe"))
        });
        let extension = if tool == ToolId::KiloCode {
            std::fs::read_dir(home.join(".vscode/extensions"))
                .ok()
                .and_then(|entries| {
                    entries
                        .flatten()
                        .find(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with("kilocode.kilo-code-")
                        })
                        .map(|e| e.path())
                })
        } else {
            None
        };
        let package = packages.iter().find(|package| {
            names.iter().any(|name| {
                package.starts_with(&format!("{name}."))
                    || package.contains(&format!(".{name}_"))
                    || package.starts_with(&format!("{name}_"))
            })
        });
        if package.is_none()
            && executable.is_none()
            && desktop.is_none()
            && extension.is_none()
            && !open
        {
            continue;
        }
        let provenance = executable
            .or(desktop)
            .or(extension)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                package
                    .map(|p| format!("Windows package: {p}"))
                    .unwrap_or_else(|| "Running application".into())
            });
        let mut state = project_ambient_state(&UsageSnapshot {
            account: Account {
                tool,
                label: label.into(),
                provenance,
            },
            availability: Availability::Unsupported {
                reason: "Installed; live integration not available".into(),
            },
            windows: vec![],
        });
        state = state.with_sessions(Some(if open {
            vec![ActivitySession {
                intelligence: None,
                id: format!("{tool:?}-desktop"),
                name: "App open; activity unavailable".into(),
                state: ActivityState::Unknown,
                waiting_for: None,
                since: SystemTime::now(),
            }]
        } else {
            vec![]
        }));
        found.push(state);
    }
    found
}

fn process_names() -> Vec<String> {
    let mut names = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return names;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let n = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                names.push(String::from_utf16_lossy(&entry.szExeFile[..n]).to_lowercase());
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    names
}

#[test]
fn installed_discovery_never_claims_usage_work_or_permission() {
    for state in discover_tools() {
        assert!(state.usage_windows.is_empty());
        assert!(matches!(
            state.activity,
            ActivityState::Unknown | ActivityState::RecentlyIdle
        ));
        assert!(state
            .sessions
            .unwrap()
            .iter()
            .all(|s| s.waiting_for.is_none()));
    }
}

fn package_names() -> Vec<String> {
    use windows::Win32::System::Registry::*;
    let mut names = Vec::new();
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::w!("Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages"),0,KEY_READ,&mut key).is_ok() {
            for index in 0..4096 {
                let mut buffer=[0u16;512]; let mut len=buffer.len() as u32;
                if RegEnumKeyExW(key,index,windows::core::PWSTR(buffer.as_mut_ptr()),&mut len,None,windows::core::PWSTR::null(),None,None).is_err() { break; }
                names.push(String::from_utf16_lossy(&buffer[..len as usize]).to_lowercase());
            }
            let _=RegCloseKey(key);
        }
    }
    names
}

fn executable_present(path: &std::path::Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if path.extension().is_some_and(|ext| ext == "exe") {
        return true;
    }
    // npm uninstall can leave command shims behind. Do not register a missing package.
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let raw = raw.replace('\\', "/");
    if let Some((_, tail)) = raw.split_once("node_modules/") {
        let relative = tail
            .split(['\"', '\'', ' ', '\r', '\n'])
            .next()
            .unwrap_or("");
        if relative.is_empty() || relative.split('/').any(|part| part == "..") {
            return false;
        }
        return path
            .parent()
            .is_some_and(|dir| dir.join("node_modules").join(relative).is_file());
    }
    true
}

pub fn install_observers(states: &[AmbientState]) {
    let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) else {
        return;
    };
    for state in states {
        install_observer(state.account.tool, &home);
    }
}

// Only these metadata-only files are installed; existing user files are never overwritten.
fn install_observer(tool: ToolId, home: &std::path::Path) {
    let (dir, files): (PathBuf, &[(&str,&str)]) = match tool {
        ToolId::Pi => (home.join(".pi/agent/extensions/verge-observer"), &[("index.ts",include_str!("../../../scripts/integrations/pi.ts")),("verge-observer-lib.mjs",include_str!("../../../scripts/integrations/verge-observer-lib.mjs"))]),
        ToolId::KiloCode => (home.join(".config/kilo/plugins"), &[("verge-observer.ts",include_str!("../../../scripts/integrations/kilo.ts")),("verge-observer-lib.mjs",include_str!("../../../scripts/integrations/verge-observer-lib.mjs"))]),
        ToolId::Hermes => (home.join(".hermes/plugins/verge-observer"), &[("__init__.py",include_str!("../../../scripts/integrations/hermes.py")),("plugin.yaml","name: verge-observer\nversion: '1.0'\ndescription: Local Verge activity observer\n")]),
        _ => return,
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    use std::io::Write;
    for (name, content) in files {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(name))
        {
            if file.write_all(content.as_bytes()).is_err() {
                drop(file);
                let _ = std::fs::remove_file(dir.join(name));
            }
        }
    }
}

/// Reads only the observer's small, whitelisted metadata records. No approval capability.
pub fn observed_tools() -> Vec<AmbientState> {
    let Some(root) =
        std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Verge/signals"))
    else {
        return vec![];
    };
    let Ok(entries) = std::fs::read_dir(root) else {
        return vec![];
    };
    let mut groups: Vec<(ToolId, Vec<ActivitySession>)> = vec![];
    for entry in entries.flatten().take(4096) {
        if !entry.file_name().to_string_lossy().starts_with("agent-")
            || entry.path().extension().is_none_or(|e| e != "json")
        {
            continue;
        }
        if entry.metadata().map_or(true, |m| m.len() > 4096) {
            continue;
        }
        let Ok(raw) = std::fs::read(entry.path()) else {
            continue;
        };
        if let Some((tool, session)) = parse_observation(&raw, SystemTime::now()) {
            if let Some((_, sessions)) = groups.iter_mut().find(|(t, _)| *t == tool) {
                sessions.push(session);
            } else {
                groups.push((tool, vec![session]));
            }
        }
    }
    groups
        .into_iter()
        .map(|(tool, sessions)| {
            project_ambient_state(&UsageSnapshot {
                account: Account {
                    tool,
                    label: format!("{tool:?}"),
                    provenance: "Local observer".into(),
                },
                availability: Availability::NotMetered,
                windows: vec![],
            })
            .with_sessions(Some(sessions))
        })
        .collect()
}
fn parse_observation(raw: &[u8], now: SystemTime) -> Option<(ToolId, ActivitySession)> {
    use std::time::Duration;
    let v: serde_json::Value = serde_json::from_slice(raw).ok()?;
    let tool = match v["provider"].as_str()? {
        "kilo" => ToolId::KiloCode,
        "hermes" => ToolId::Hermes,
        "pi" => ToolId::Pi,
        _ => return None,
    };
    let id = v["session"].as_str()?;
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let at = SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(v["at"].as_u64()?))?;
    let age = now.duration_since(at).ok()?;
    if age > Duration::from_secs(600) {
        return None;
    }
    let mut state = match v["state"].as_str()? {
        "working" => ActivityState::Working,
        "completed" => ActivityState::Completed,
        "stopped" => ActivityState::Stopped,
        "waiting" => ActivityState::WaitingOnUser,
        "idle" => ActivityState::RecentlyIdle,
        _ => return None,
    };
    if age > Duration::from_secs(120)
        && matches!(state, ActivityState::Working | ActivityState::WaitingOnUser)
    {
        state = ActivityState::Unknown;
    }
    Some((
        tool,
        ActivitySession {
            intelligence: None,
            id: id.into(),
            name: "Agent session".into(),
            state,
            waiting_for: (state == ActivityState::WaitingOnUser)
                .then(|| "Respond in the agent app".into()),
            since: at,
        },
    ))
}

#[test]
fn observations_are_bounded_and_never_grant_permission() {
    let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
    let v =
        serde_json::json!({"provider":"pi","session":"a".repeat(64),"state":"waiting","at":999000});
    assert_eq!(
        parse_observation(v.to_string().as_bytes(), now)
            .unwrap()
            .1
            .state,
        ActivityState::WaitingOnUser
    );
    assert!(parse_observation(
        v.to_string().as_bytes(),
        now + std::time::Duration::from_secs(601)
    )
    .is_none());
    assert!(parse_observation(br#"{"provider":"arbitrary"}"#, now).is_none());
}

#[test]
fn installed_plugin_dependencies_resolve_and_preserve_existing_files() {
    let home = std::env::temp_dir().join(format!("verge-plugin-layout-{}", std::process::id()));
    for (tool, relative, entry) in [
        (
            ToolId::Pi,
            ".pi/agent/extensions/verge-observer",
            "index.ts",
        ),
        (
            ToolId::KiloCode,
            ".config/kilo/plugins",
            "verge-observer.ts",
        ),
    ] {
        install_observer(tool, &home);
        let dir = home.join(relative);
        let source = std::fs::read_to_string(dir.join(entry)).unwrap();
        assert!(source.contains("from './verge-observer-lib.mjs'"));
        assert_eq!(
            std::fs::read_to_string(dir.join("verge-observer-lib.mjs")).unwrap(),
            include_str!("../../../scripts/integrations/verge-observer-lib.mjs")
        );
        std::fs::write(dir.join(entry), "user customization").unwrap();
        install_observer(tool, &home);
        assert_eq!(
            std::fs::read_to_string(dir.join(entry)).unwrap(),
            "user customization"
        );
    }
    std::fs::remove_dir_all(home).unwrap();
}
