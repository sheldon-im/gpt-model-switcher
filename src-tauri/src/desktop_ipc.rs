//! Private desktop protocol, verified against ChatGPT desktop 26.1027.3384.0.
//! No inference, resume, turn, permission, or message APIs are exposed here.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

const MAX_FRAME: usize = 1024 * 1024;
const LIMIT: Duration = Duration::from_secs(4);
pub fn home() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("CODEX_HOME") {
        return Ok(p.into());
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .ok_or("Cannot locate desktop data folder")?;
    Ok(PathBuf::from(home).join(".codex"))
}
pub fn resolve_in(db: &rusqlite::Connection, title: &str) -> Result<String, String> {
    if title.trim().is_empty() || title.contains('|') {
        return Err("대화 제목을 확정할 수 없습니다. / Cannot identify conversation title.".into());
    }
    let mut q = db.prepare("SELECT id FROM threads WHERE archived = 0 AND coalesce(nullif(name, ''), title) = ?1 LIMIT 2").map_err(|_| "Unsupported desktop index schema")?;
    let ids = q
        .query_map([title], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if ids.len() != 1 {
        return Err("같은 제목의 대화가 여러 개이거나 대화를 찾지 못했습니다. 고유한 제목을 사용하십시오. / Conversation title is missing or ambiguous; use a unique title.".into());
    }
    let id = &ids[0];
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid conversation ID")?;
    Ok(id.clone())
}
pub fn resolve(title: &str) -> Result<String, String> {
    // Read only: never migrate, create, or write the app's private database.
    let db = rusqlite::Connection::open_with_flags(
        home()?.join("state_5.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| {
        "데스크톱 대화 색인을 읽지 못했습니다. / Desktop conversation index unavailable."
    })?;
    db.busy_timeout(Duration::from_millis(250))
        .map_err(|e| e.to_string())?;
    resolve_in(&db, title)
}
pub fn validate_catalog(p: &switcher_core::Preset) -> Result<(), String> {
    let file = home()?.join("models_cache.codex_work_desktop.json");
    if std::fs::metadata(&file)
        .map_err(|_| "Desktop model catalog unavailable")?
        .len()
        > 8 * 1024 * 1024
    {
        return Err("Unsupported model catalog size".into());
    }
    let v: Value = serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
        .map_err(|_| "Unsupported model catalog format")?;
    let fetched = chrono::DateTime::parse_from_rfc3339(
        v["fetched_at"]
            .as_str()
            .ok_or("Missing catalog timestamp")?,
    )
    .map_err(|_| "Invalid catalog timestamp")?
    .timestamp();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    if !(0..=86400).contains(&(now - fetched)) {
        return Err("모델 목록이 오래되었습니다. 앱에서 모델 목록을 갱신한 뒤 재시도하십시오. / Desktop model catalog is stale; refresh it in the app and retry.".into());
    }
    catalog_contains(&v, p)
}
fn catalog_contains(v: &Value, p: &switcher_core::Preset) -> Result<(), String> {
    let models = v["models"].as_array().ok_or("Unsupported model catalog")?;
    let choices: Vec<_> = models
        .iter()
        .filter(|m| m["slug"] == p.model && m["visibility"] == "list")
        .collect();
    if choices.len() != 1
        || !choices[0]["supported_reasoning_levels"]
            .as_array()
            .is_some_and(|levels| levels.iter().any(|l| l["effort"] == p.effort))
    {
        return Err("이 화면의 모델 목록에서 해당 모델·추론 조합을 확인하지 못했습니다. / Requested model and reasoning pair is not available in the desktop catalog.".into());
    }
    Ok(())
}
pub trait Transport: Read + Write {}
impl<T: Read + Write> Transport for T {}
pub struct Desktop {
    stream: Box<dyn Transport>,
    client: String,
    owner: Option<String>,
}
fn read_frame(stream: &mut impl Read, deadline: Instant) -> Result<Value, String> {
    fn exact(s: &mut impl Read, b: &mut [u8], deadline: Instant) -> Result<(), String> {
        let mut n = 0;
        while n < b.len() {
            if Instant::now() > deadline {
                return Err("Desktop IPC timeout".into());
            }
            match s.read(&mut b[n..]) {
                Ok(0) => return Err("Desktop IPC closed".into()),
                Ok(k) => n += k,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) || e.raw_os_error() == Some(232) =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
    let mut header = [0; 4];
    exact(stream, &mut header, deadline)?;
    let n = u32::from_le_bytes(header) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err("Unsupported desktop IPC frame size".into());
    }
    let mut body = vec![0; n];
    exact(stream, &mut body, deadline)?;
    serde_json::from_slice(&body).map_err(|_| "Malformed desktop IPC frame".into())
}
fn write_frame(stream: &mut impl Write, v: &Value) -> Result<(), String> {
    let b = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if b.len() > MAX_FRAME {
        return Err("IPC request too large".into());
    }
    stream
        .write_all(&(b.len() as u32).to_le_bytes())
        .and_then(|_| stream.write_all(&b))
        .map_err(|e| e.to_string())
}
impl Desktop {
    pub fn connect(pid: u32) -> Result<Self, String> {
        Self::initialize(connect(pid)?)
    }
    fn initialize(stream: Box<dyn Transport>) -> Result<Self, String> {
        let mut d = Self {
            stream,
            client: "initializing-client".into(),
            owner: None,
        };
        let v = d.call("initialize", json!({"clientType":"gpt-model-switcher"}), 0)?;
        d.client = v["result"]["clientId"]
            .as_str()
            .ok_or("Unsupported desktop IPC handshake")?
            .into();
        Ok(d)
    }
    fn call(&mut self, method: &str, params: Value, version: u32) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let mut request = json!({"type":"request","requestId":id,"sourceClientId":self.client,"version":version,"method":method,"params":params,"timeoutMs":3000});
        if let Some(owner) = &self.owner {
            request["targetClientId"] = json!(owner)
        }
        write_frame(&mut self.stream, &request)?;
        let end = Instant::now() + LIMIT;
        loop {
            let v = read_frame(&mut self.stream, end)?;
            // Discovery requests are not handled by this client. No broadcast content is retained.
            if v["type"] == "client-discovery-request" {
                let reply = json!({"type":"client-discovery-response","requestId":v["requestId"],"response":{"canHandle":false}});
                write_frame(&mut self.stream, &reply)?;
                continue;
            }
            if v["type"] != "response" || v["requestId"] != id {
                continue;
            }
            if v["method"] != method || v["resultType"] != "success" {
                return Err("앱이 설정 요청을 수락하지 않았습니다. / Desktop rejected or does not support the settings request.".into());
            }
            if let Some(owner) = &self.owner {
                if v["handledByClientId"] != *owner {
                    return Err("Desktop conversation owner changed".into());
                }
            }
            return Ok(v);
        }
    }
    pub fn read(&mut self, id: &str) -> Result<Value, String> {
        let v = self.call(
            "thread-follower-read-model-settings",
            json!({"conversationId":id}),
            1,
        )?;
        let owner = v["handledByClientId"]
            .as_str()
            .ok_or("Missing conversation owner")?
            .to_string();
        self.owner = Some(owner);
        let s = v["result"]["settings"].clone();
        if s["resumeState"] != "resumed"
            || s["threadSource"] != "user"
            || s["modelProvider"] != "openai"
            || s["mode"] != "default"
            || s["daybreakEnabled"] == true
            || !s["model"].is_string()
        {
            return Err(
                "현재 화면의 대화를 안전하게 변경할 수 없습니다. / Unsupported conversation state."
                    .into(),
            );
        }
        Ok(s)
    }
    pub fn update(
        &mut self,
        id: &str,
        before: &Value,
        model: &str,
        effort: Value,
    ) -> Result<bool, String> {
        let v=self.call("thread-follower-update-thread-settings",json!({"conversationId":id,"threadSettings":{"model":model,"effort":effort},"condition":{"ifModelEquals":before["model"],"ifEffortEquals":before["reasoningEffort"]}}),2)?;
        v["result"]["applied"]
            .as_bool()
            .ok_or("Missing settings acknowledgement".into())
    }
}
#[cfg(windows)]
fn connect(expected_pid: u32) -> Result<Box<dyn Transport>, String> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        System::Pipes::{
            GetNamedPipeServerProcessId, SetNamedPipeHandleState, PIPE_NOWAIT, PIPE_READMODE_BYTE,
        },
    };
    let file=std::fs::OpenOptions::new().read(true).write(true).open(r"\\.\pipe\codex-ipc").map_err(|_|"포커스 없는 전환 연결을 사용할 수 없습니다. / Focus-free desktop connection unavailable.")?;
    unsafe {
        let h = HANDLE(file.as_raw_handle());
        let mut pid = 0;
        GetNamedPipeServerProcessId(h, &mut pid).map_err(|e| e.to_string())?;
        if pid != expected_pid {
            return Err("Desktop IPC does not belong to foreground app".into());
        }
        let mode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
        SetNamedPipeHandleState(h, Some(&mode), None, None).map_err(|e| e.to_string())?;
    }
    // A byte pipe in PIPE_NOWAIT mode may return a successful zero-byte read
    // while its peer is still connected. Treat that as pending, bounded by our deadline.
    struct Pipe(std::fs::File);
    impl Read for Pipe {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            match self.0.read(buffer) {
                Ok(0) => Err(std::io::ErrorKind::WouldBlock.into()),
                other => other,
            }
        }
    }
    impl Write for Pipe {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.0.write(buffer)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    Ok(Box::new(Pipe(file)))
}
#[cfg(target_os = "macos")]
fn connect(_pid: u32) -> Result<Box<dyn Transport>, String> {
    use std::os::{
        fd::AsRawFd,
        unix::{fs::MetadataExt, net::UnixStream},
    };
    let path = home()?.join("ipc/ipc.sock");
    let meta = std::fs::symlink_metadata(&path)
        .map_err(|_| "Focus-free desktop connection unavailable")?;
    if meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || !std::os::unix::fs::FileTypeExt::is_socket(&meta.file_type())
    {
        return Err("Unsafe desktop socket permissions".into());
    }
    let stream = UnixStream::connect(path).map_err(|e| e.to_string())?;
    let mut uid = 0;
    let mut gid = 0;
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0
        || uid != unsafe { libc::geteuid() }
    {
        return Err("Desktop socket owner mismatch".into());
    }
    stream.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok(Box::new(stream))
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Simulator {
        input: std::io::Cursor<Vec<u8>>,
        pending: Vec<u8>,
        seen: std::rc::Rc<std::cell::RefCell<Vec<Value>>>,
        change_owner: bool,
    }
    impl Read for Simulator {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            let count = b.len().min(3);
            self.input.read(&mut b[..count])
        }
    }
    impl Write for Simulator {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.pending.extend_from_slice(b);
            if self.pending.len() >= 4 {
                let n = u32::from_le_bytes(self.pending[..4].try_into().unwrap()) as usize;
                if self.pending.len() >= 4 + n {
                    let request: Value = serde_json::from_slice(&self.pending[4..4 + n]).unwrap();
                    self.pending.drain(..4 + n);
                    self.seen.borrow_mut().push(request.clone());
                    let method = request["method"].as_str().unwrap();
                    let result = match method {
                        "initialize" => json!({"clientId":"switcher"}),
                        "thread-follower-read-model-settings" => {
                            json!({"settings":{"model":"gpt-6.1-sol","reasoningEffort":null,"resumeState":"resumed","threadSource":"user","modelProvider":"openai","mode":"default"}})
                        }
                        "thread-follower-update-thread-settings" => json!({"applied":true}),
                        _ => panic!("Unexpected capability: {method}"),
                    };
                    let owner = if self.change_owner
                        && method == "thread-follower-update-thread-settings"
                    {
                        "another-owner"
                    } else {
                        "owner"
                    };
                    let response = json!({"type":"response","requestId":request["requestId"],"method":method,"resultType":"success","handledByClientId":owner,"result":result});
                    write_frame(self.input.get_mut(), &response).unwrap();
                }
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn owner_is_pinned_and_default_effort_condition_is_preserved() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sim = Simulator {
            input: std::io::Cursor::new(Vec::new()),
            pending: Vec::new(),
            seen: seen.clone(),
            change_owner: false,
        };
        let mut d = Desktop::initialize(Box::new(sim)).unwrap();
        let raw = d.read("conversation").unwrap();
        assert!(d
            .update("conversation", &raw, "gpt-6.1-terra", json!("medium"))
            .unwrap());
        let requests = seen.borrow();
        let update = &requests[2];
        assert_eq!(update["targetClientId"], "owner");
        assert_eq!(update["version"], 2);
        assert!(update["params"]["condition"]["ifEffortEquals"].is_null());
        assert_eq!(
            update["params"]["threadSettings"]
                .as_object()
                .unwrap()
                .len(),
            2
        );
        assert!(update["params"].get("activeTurnId").is_none());
    }
    #[test]
    fn changed_owner_response_is_rejected() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sim = Simulator {
            input: std::io::Cursor::new(Vec::new()),
            pending: Vec::new(),
            seen,
            change_owner: true,
        };
        let mut d = Desktop::initialize(Box::new(sim)).unwrap();
        let raw = d.read("conversation").unwrap();
        assert!(d
            .update("conversation", &raw, "gpt-6.1-terra", json!("medium"))
            .is_err());
    }
    #[test]
    fn frame_round_trip() {
        let value = json!({"type":"response","result":{"model":"gpt-6.1-terra"}});
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &value).unwrap();
        assert_eq!(
            read_frame(&mut std::io::Cursor::new(bytes), Instant::now() + LIMIT).unwrap(),
            value
        )
    }
    #[test]
    fn frames_reject_oversize_and_truncation() {
        assert!(read_frame(
            &mut std::io::Cursor::new(u32::MAX.to_le_bytes()),
            Instant::now() + LIMIT
        )
        .is_err());
        assert!(read_frame(
            &mut std::io::Cursor::new([5, 0, 0, 0, 123]),
            Instant::now() + LIMIT
        )
        .is_err())
    }
    #[test]
    fn title_must_be_exact_unique_and_not_archived() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE threads(id TEXT,title TEXT,name TEXT,archived INTEGER);INSERT INTO threads VALUES('01a18b72-4927-71a1-9d48-6cc6cbef99f3','old','unique',0);").unwrap();
        assert!(resolve_in(&db, "unique").is_ok());
        assert!(resolve_in(&db, "uni").is_err());
        db.execute_batch("INSERT INTO threads SELECT * FROM threads;")
            .unwrap();
        assert!(resolve_in(&db, "unique").is_err());
        db.execute_batch("UPDATE threads SET archived=1;").unwrap();
        assert!(resolve_in(&db, "unique").is_err());
    }
    #[test]
    fn catalog_rejects_unknown_and_hidden_combinations() {
        let p = &switcher_core::Settings::default().presets[0];
        let mut v = json!({"models":[{"slug":p.model,"visibility":"list","supported_reasoning_levels":[{"effort":"medium"}]}]});
        assert!(catalog_contains(&v, p).is_ok());
        v["models"][0]["supported_reasoning_levels"][0]["effort"] = json!("low");
        assert!(catalog_contains(&v, p).is_err());
        v["models"][0]["visibility"] = json!("hide");
        assert!(catalog_contains(&v, p).is_err());
    }
}
