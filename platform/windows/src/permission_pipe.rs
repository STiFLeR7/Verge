//! Private, local, one-request-per-connection permission transport. No payload logging.
use crate::WindowsProcessProbe;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Read, Write},
    os::windows::io::{AsRawHandle, FromRawHandle},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use verge_core::{
    domain::{PermissionBook, PermissionDecision, PermissionRequest},
    ports::{PermissionService, ProcessProbe},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::*,
        Security::{Authorization::*, *},
        Storage::FileSystem::*,
        System::{Diagnostics::ToolHelp::*, Pipes::*, Threading::*},
    },
};

const PIPE: &str = r"\\.\pipe\Verge.Claude.Permissions.v1";
const LIMIT: usize = 32768;
type Validator = Arc<dyn Fn(&str, u32) -> bool + Send + Sync>;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn handle(f: &File) -> HANDLE {
    HANDLE(f.as_raw_handle())
}
// A zero-byte std::fs read is ambiguous on a nonblocking Windows pipe.
// Peek first so a connected but empty pipe waits, while disconnects still fail closed.
fn read_pipe(file: &mut File, buffer: &mut [u8]) -> io::Result<usize> {
    let mut available = 0;
    unsafe { PeekNamedPipe(handle(file), None, 0, None, Some(&mut available), None) }
        .map_err(io::Error::other)?;
    if available == 0 {
        return Err(io::Error::from_raw_os_error(ERROR_NO_DATA.0 as i32));
    }
    file.read(buffer)
}
fn process_image(pid: u32) -> Option<PathBuf> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = vec![0u16; 32768];
        let mut len = buffer.len() as u32;
        let result =
            QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        result.ok()?;
        Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..len as usize],
        )))
    }
}
fn parent(pid: u32) -> Option<u32> {
    unsafe {
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(h, &mut entry).is_ok();
        let mut result = None;
        while ok {
            if entry.th32ProcessID == pid {
                result = Some(entry.th32ParentProcessID);
                break;
            }
            ok = Process32NextW(h, &mut entry).is_ok();
        }
        let _ = CloseHandle(h);
        result
    }
}
// Correlates local sessions; not an isolation boundary against a malicious same-user process.
// Windows parent-PID spoofing and readable session metadata can satisfy ancestor correlation.
// See docs/design/CLAUDE_APPROVAL_SETUP.md for the same-user trust assumption.
pub fn claude_ancestor(mut pid: u32) -> Option<u32> {
    for _ in 0..16 {
        pid = parent(pid)?;
        if process_image(pid)?
            .file_name()?
            .to_string_lossy()
            .eq_ignore_ascii_case("claude.exe")
        {
            return Some(pid);
        }
    }
    None
}
fn same_executable(pid: u32, name: &str) -> bool {
    let Some(path) = process_image(pid) else {
        return false;
    };
    let Ok(mut expected) = std::env::current_exe() else {
        return false;
    };
    expected.set_file_name(name);
    path.canonicalize()
        .ok()
        .zip(expected.canonicalize().ok())
        .is_some_and(|(a, b)| a == b)
}
struct Identity {
    session: String,
    root: u32,
    root_start: SystemTime,
    client: u32,
    client_start: SystemTime,
}
struct Shared {
    book: PermissionBook,
    identities: BTreeMap<u64, Identity>,
}
pub struct WindowsPermissions {
    shared: Arc<Mutex<Shared>>,
    validate: Validator,
}
impl WindowsPermissions {
    pub fn start(
        validate: impl Fn(&str, u32) -> bool + Send + Sync + 'static,
    ) -> io::Result<Arc<Self>> {
        // Reserve every instance before serving. FIRST_PIPE_INSTANCE prevents pre-existing impostors.
        let mut pipes = Vec::new();
        for i in 0..8 {
            pipes.push(new_pipe(i == 0)?);
        }
        let service = Arc::new(Self {
            shared: Arc::new(Mutex::new(Shared {
                book: PermissionBook::default(),
                identities: BTreeMap::new(),
            })),
            validate: Arc::new(validate),
        });
        let worker = service.clone();
        std::thread::spawn(move || worker.serve(pipes));
        Ok(service)
    }
    fn live(&self, i: &Identity) -> bool {
        WindowsProcessProbe.started_at(i.root).ok().flatten() == Some(i.root_start)
            && WindowsProcessProbe.started_at(i.client).ok().flatten() == Some(i.client_start)
            && (self.validate)(&i.session, i.root)
    }
    fn serve(&self, pipes: Vec<File>) {
        struct Peer {
            file: File,
            connected: Option<Instant>,
            raw: Vec<u8>,
            id: Option<u64>,
            sent: bool,
            nonce: String,
        }
        let mut peers = pipes
            .into_iter()
            .map(|file| Peer {
                file,
                connected: None,
                raw: vec![],
                id: None,
                sent: false,
                nonce: String::new(),
            })
            .collect::<Vec<_>>();
        loop {
            for peer in &mut peers {
                let h = handle(&peer.file);
                if peer.connected.is_none() {
                    let result = unsafe { ConnectNamedPipe(h, None) };
                    if result.is_ok()
                        || result.err().is_some_and(|e| {
                            e.code() == windows::core::HRESULT::from_win32(ERROR_PIPE_CONNECTED.0)
                        })
                    {
                        peer.connected = Some(Instant::now());
                    } else {
                        continue;
                    }
                }
                let mut close = false;
                if peer.id.is_none() {
                    let mut chunk = [0u8; 4096];
                    match read_pipe(&mut peer.file, &mut chunk) {
                        Ok(0) => close = true,
                        Ok(n) => peer.raw.extend_from_slice(&chunk[..n]),
                        Err(e) => {
                            if e.raw_os_error() != Some(ERROR_NO_DATA.0 as i32) {
                                close = true;
                            }
                        }
                    }
                    if peer.raw.len() > LIMIT
                        || peer.connected.unwrap().elapsed() > Duration::from_secs(3)
                    {
                        close = true;
                    }
                    if !close && peer.raw.ends_with(b"\n") {
                        let request = serde_json::from_slice::<serde_json::Value>(&peer.raw);
                        if let Ok(v) = request {
                            let mut client = 0;
                            let verified = unsafe { GetNamedPipeClientProcessId(h, &mut client) }
                                .is_ok()
                                && same_executable(client, "verge-claude-hook.exe");
                            let root = claude_ancestor(client);
                            let session = v["session"].as_str().unwrap_or("");
                            let nonce = v["nonce"].as_str().unwrap_or("");
                            if verified
                                && nonce.len() == 64
                                && nonce.bytes().all(|c| c.is_ascii_hexdigit())
                                && root.is_some_and(|p| (self.validate)(session, p))
                            {
                                let root = root.unwrap();
                                if let (Ok(Some(root_start)), Ok(Some(client_start))) = (
                                    WindowsProcessProbe.started_at(root),
                                    WindowsProcessProbe.started_at(client),
                                ) {
                                    let mut shared = self.shared.lock().unwrap();
                                    peer.id = shared.book.register(
                                        session.into(),
                                        v["tool"].as_str().unwrap_or("Action").into(),
                                        v["detail"].as_str().unwrap_or("").into(),
                                        v["cwd"].as_str().unwrap_or("").into(),
                                        Instant::now(),
                                    );
                                    if let Some(id) = peer.id {
                                        shared.identities.insert(
                                            id,
                                            Identity {
                                                session: session.into(),
                                                root,
                                                root_start,
                                                client,
                                                client_start,
                                            },
                                        );
                                        peer.nonce = nonce.into();
                                    }
                                }
                            }
                        }
                        if peer.id.is_none() {
                            close = true;
                        }
                        peer.raw.clear();
                    }
                } else {
                    let id = peer.id.unwrap();
                    let shared = self.shared.lock().unwrap();
                    let live = shared.identities.get(&id).is_some_and(|i| self.live(i));
                    let mut available = 0;
                    let connected =
                        unsafe { PeekNamedPipe(h, None, 0, None, Some(&mut available), None) }
                            .is_ok();
                    if !live
                        || !connected
                        || available > 0
                        || peer.connected.unwrap().elapsed() > Duration::from_secs(125)
                    {
                        close = true;
                    } else if let Some(decision) = shared
                        .book
                        .result(id, Instant::now())
                        .filter(|_| !peer.sent)
                    {
                        let i = &shared.identities[&id];
                        let response=serde_json::json!({"session":i.session,"nonce":peer.nonce,"allow":decision==PermissionDecision::Approve}).to_string()+"\n";
                        // No retry: a connection has one response and one authority-bound decision.
                        close = peer.file.write_all(response.as_bytes()).is_err();
                        peer.sent = true;
                    }
                }
                if close {
                    if let Some(id) = peer.id.take() {
                        let mut shared = self.shared.lock().unwrap();
                        shared.book.remove(id);
                        shared.identities.remove(&id);
                    }
                    unsafe {
                        let _ = DisconnectNamedPipe(h);
                    }
                    peer.connected = None;
                    peer.sent = false;
                    peer.raw.clear();
                    peer.nonce.clear();
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}
impl PermissionService for WindowsPermissions {
    fn pending(&self) -> Option<PermissionRequest> {
        let s = self.shared.lock().unwrap();
        let request = s.book.pending(Instant::now())?;
        s.identities
            .get(&request.id)
            .filter(|i| self.live(i))
            .map(|_| request)
    }
    fn decide(&self, id: u64, session: &str, decision: PermissionDecision) -> bool {
        let mut s = self.shared.lock().unwrap();
        let live = s.identities.get(&id).is_some_and(|i| self.live(i));
        s.book.decide(id, session, decision, Instant::now(), live)
    }
}
fn new_pipe(first: bool) -> io::Result<File> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(io::Error::other)?;
        let mut bytes = 0;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut bytes);
        let mut buffer = vec![0usize; (bytes as usize).div_ceil(std::mem::size_of::<usize>())];
        let result = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            bytes,
            &mut bytes,
        );
        let _ = CloseHandle(token);
        result.map_err(io::Error::other)?;
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut sid = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut sid).map_err(io::Error::other)?;
        let sid_text = sid.to_string().map_err(io::Error::other)?;
        let _ = LocalFree(HLOCAL(sid.0.cast()));
        let sddl = wide(&format!("D:P(A;;GA;;;{sid_text})"));
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
        .map_err(io::Error::other)?;
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: BOOL(0),
        };
        let name = wide(PIPE);
        let flags = PIPE_ACCESS_DUPLEX
            | if first {
                FILE_FLAG_FIRST_PIPE_INSTANCE
            } else {
                FILE_FLAGS_AND_ATTRIBUTES(0)
            };
        let h = CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            flags,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            8,
            65536,
            65536,
            0,
            Some(&sa),
        );
        let _ = LocalFree(HLOCAL(descriptor.0));
        if h == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        Ok(File::from_raw_handle(h.0))
    }
}

/// Native helper verifies the server executable before accepting a one-shot answer.
pub fn request_permission(session: &str, tool: &str, detail: &str, cwd: &str) -> bool {
    let run = || -> io::Result<bool> {
        unsafe {
            let name = wide(PIPE);
            let h = CreateFileW(
                PCWSTR(name.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
            .map_err(io::Error::other)?;
            let mut file = File::from_raw_handle(h.0);
            let mut server = 0;
            GetNamedPipeServerProcessId(h, &mut server).map_err(io::Error::other)?;
            if !same_executable(server, "verge.exe") {
                return Ok(false);
            }
            let start = WindowsProcessProbe.started_at(server)?;
            if start.is_none() {
                return Ok(false);
            }
            SetNamedPipeHandleState(h, Some(&PIPE_NOWAIT), None, None).map_err(io::Error::other)?;
            let mut nonce = [0u8; 32];
            windows::Win32::Security::Cryptography::BCryptGenRandom(
                None,
                &mut nonce,
                windows::Win32::Security::Cryptography::BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            )
            .ok()
            .map_err(io::Error::other)?;
            let nonce = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
            let request=serde_json::json!({"session":session,"tool":tool,"detail":detail,"cwd":cwd,"nonce":nonce}).to_string()+"\n";
            if request.len() > LIMIT {
                return Ok(false);
            }
            file.write_all(request.as_bytes())?;
            let deadline = Instant::now() + Duration::from_secs(120);
            let mut raw = Vec::new();
            while Instant::now() < deadline {
                let mut buf = [0u8; 1024];
                match read_pipe(&mut file, &mut buf) {
                    Ok(0) => return Ok(false),
                    Ok(n) => raw.extend_from_slice(&buf[..n]),
                    Err(e) if e.raw_os_error() == Some(ERROR_NO_DATA.0 as i32) => {}
                    Err(_) => return Ok(false),
                }
                if raw.len() > 4096 {
                    return Ok(false);
                }
                if raw.ends_with(b"\n") {
                    let v: serde_json::Value =
                        serde_json::from_slice(&raw).map_err(io::Error::other)?;
                    let valid = Instant::now() < deadline
                        && v["session"].as_str() == Some(session)
                        && v["nonce"].as_str() == Some(&nonce)
                        && v["allow"].as_bool() == Some(true)
                        && WindowsProcessProbe.started_at(server)? == start;
                    let _ = file.write_all(b"ack");
                    return Ok(valid);
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Ok(false)
        }
    };
    run().unwrap_or(false)
}
