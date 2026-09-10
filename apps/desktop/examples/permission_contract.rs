//! Isolated Windows transport contract: synthetic Claude ancestor, real helper and broker.
//! Never connects to a real Claude session or executes an approved command.
#[cfg(windows)]
fn main() {
    use std::{
        fs,
        io::Write,
        path::PathBuf,
        process::{Command, Stdio},
        time::{Duration, Instant, SystemTime},
    };
    use verge_core::{
        domain::PermissionDecision,
        ports::{PermissionService, ProcessProbe},
    };
    use verge_platform_windows::{WindowsPermissions, WindowsProcessProbe};
    let args = std::env::args().collect::<Vec<_>>();
    let wait = |path: &std::path::Path| {
        let end = Instant::now() + Duration::from_secs(12);
        while !path.exists() {
            assert!(Instant::now() < end, "contract timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    if args.get(1).map(String::as_str) == Some("server") {
        let dir = PathBuf::from(&args[2]);
        let mode = &args[3];
        let service = WindowsPermissions::start(|s, p| {
            verge_tool_claude::permission_session_matches(s, p, &WindowsProcessProbe)
        })
        .unwrap();
        fs::write(dir.join("ready"), "").unwrap();
        let end = Instant::now() + Duration::from_secs(12);
        let request = loop {
            if let Some(r) = service.pending() {
                break r;
            }
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(!service.decide(request.id, "other-session", PermissionDecision::Approve));
        assert!(!service.decide(
            request.id + 1,
            &request.session_id,
            PermissionDecision::Approve
        ));
        if mode == "disconnect" || mode == "terminated" {
            fs::write(dir.join("seen"), "").unwrap();
            while service.pending().is_some() {
                assert!(Instant::now() < end);
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(!service.decide(request.id, &request.session_id, PermissionDecision::Approve));
        } else {
            // Leave the connected pipe empty for several polls before the explicit decision.
            std::thread::sleep(Duration::from_millis(160));
            let decision = match mode.as_str() {
                "approve" => PermissionDecision::Approve,
                "dismiss" => PermissionDecision::Dismiss,
                _ => PermissionDecision::Deny,
            };
            assert!(service.decide(request.id, &request.session_id, decision));
            assert!(!service.decide(request.id, &request.session_id, PermissionDecision::Approve));
            wait(&dir.join("done"));
        }
        fs::write(dir.join("server-ok"), "").unwrap();
        return;
    }
    if args.get(1).map(String::as_str) == Some("parent") {
        let dir = PathBuf::from(&args[2]);
        let mode = &args[3];
        let pid = std::process::id();
        let start = WindowsProcessProbe
            .started_at(pid)
            .unwrap()
            .unwrap()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            / 100
            + 116_444_736_000_000_000;
        let sessions = dir.join(".claude/sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join(format!("{pid}.json")),
            format!(r#"{{"sessionId":"synthetic-contract","pid":{pid},"procStart":"{start}"}}"#),
        )
        .unwrap();
        assert!(verge_tool_claude::permission_session_matches(
            "synthetic-contract",
            pid,
            &WindowsProcessProbe
        ));
        let mut child = Command::new(dir.join("verge-claude-hook.exe"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(br#"{"hook_event_name":"PermissionRequest","session_id":"synthetic-contract","tool_name":"Bash","cwd":"synthetic-project","tool_input":{"command":"echo synthetic"}}"#).unwrap();
        if mode == "disconnect" {
            wait(&dir.join("seen"));
            child.kill().unwrap();
            let _ = child.wait();
            return;
        }
        if mode == "terminated" {
            wait(&dir.join("seen"));
            return;
        }
        let output = child.wait_with_output().unwrap();
        let output = String::from_utf8(output.stdout).unwrap();
        assert!(
            output.contains(if mode == "approve" {
                r#""behavior":"allow""#
            } else {
                r#""behavior":"deny""#
            }),
            "unexpected helper decision: {output}"
        );
        fs::write(dir.join("done"), "").unwrap();
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let root = exe.parent().unwrap().parent().unwrap();
    let base = root.join(format!(
        "permission-contract-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    for mode in ["approve", "deny", "dismiss", "disconnect", "terminated"] {
        let dir = base.join(mode);
        fs::create_dir_all(&dir).unwrap();
        fs::copy(&exe, dir.join("verge.exe")).unwrap();
        fs::copy(&exe, dir.join("claude.exe")).unwrap();
        fs::copy(
            root.join("verge-claude-hook.exe"),
            dir.join("verge-claude-hook.exe"),
        )
        .unwrap();
        let mut server = Command::new(dir.join("verge.exe"))
            .args(["server", dir.to_str().unwrap(), mode])
            .env("USERPROFILE", &dir)
            .spawn()
            .unwrap();
        wait(&dir.join("ready"));
        let parent = Command::new(dir.join("claude.exe"))
            .args(["parent", dir.to_str().unwrap(), mode])
            .env("USERPROFILE", &dir)
            .status()
            .unwrap();
        assert!(parent.success());
        assert!(server.wait().unwrap().success());
        assert!(dir.join("server-ok").exists());
        println!(
            "PASS {mode}: real native pipe, session/process identity, explicit one-shot decision"
        );
    }
}
#[cfg(not(windows))]
fn main() {}
