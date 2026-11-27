//! A separate macOS process releases the owned engine if its app dies before
//! async cleanup can run. It never scans or terminates another app's engine.
use anyhow::{ensure, Context, Result};
use std::{
    ffi::CString,
    io::Read,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncBufReadExt,
    process::{Child, ChildStdin, Command},
};

#[derive(Clone, Copy, PartialEq, Eq)]
struct ProcessIdentity {
    parent: u32,
    uid: u32,
    seconds: u64,
    micros: u64,
    zombie: bool,
}
fn identity(pid: u32) -> Result<ProcessIdentity> {
    ensure!(pid > 1 && pid <= i32::MAX as u32, "无效的引擎进程 ID");
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            &mut info as *mut _ as *mut _,
            size,
        )
    };
    ensure!(read == size && info.pbi_pid == pid, "无法核验引擎进程身份");
    Ok(ProcessIdentity {
        parent: info.pbi_ppid,
        uid: info.pbi_uid,
        seconds: info.pbi_start_tvsec,
        micros: info.pbi_start_tvusec,
        zombie: info.pbi_status == libc::SZOMB,
    })
}

fn owned_parents(owner: u32, guard_parent: u32, engine_parent: u32) -> bool {
    // These kernel snapshots are not atomic: either sibling can be reparented
    // to launchd first when the owner exits. UID and exact engine start time
    // are checked separately; never require matching parent snapshots.
    owner > 1 && [owner, 1].contains(&guard_parent) && [owner, 1].contains(&engine_parent)
}

// Qt's legacy System V keys for the bundled Deskflow 1.27 core. These files
// are created inside the per-launch TMPDIR, so no system-wide scan is needed.
const SHARED_MEMORY_FILE: &str =
    "qipc_sharedmemory_deskflowcorefbfa2630dd772f55959dda40b83431af95318db4";
const SEMAPHORE_FILE: &str = "qipc_systemsem_deskflowcorefbfa2630dd772f55959dda40b83431af95318db4";

fn qt_key(directory: &Path, name: &str, uid: u32) -> Option<libc::key_t> {
    let path = directory.join(name);
    let metadata = path.symlink_metadata().ok()?;
    if !metadata.is_file() || metadata.uid() != uid || metadata.nlink() != 1 {
        return None;
    }
    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let key = unsafe { libc::ftok(path.as_ptr(), b'Q'.into()) };
    (key > 0).then_some(key)
}

// Only call after the owned process has stopped. ftok keys can collide, so
// kernel ownership, PID and attachment checks are required as well.
pub(crate) fn cleanup_engine_ipc(directory: &Path, pid: u32) {
    let uid = unsafe { libc::getuid() };
    let Ok(metadata) = directory.symlink_metadata() else {
        return;
    };
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o777 != 0o700 {
        return;
    }
    let pid = pid as libc::pid_t;
    if let Some(key) = qt_key(directory, SHARED_MEMORY_FILE, uid) {
        // Darwin can expose a zombie before exit has detached its System V
        // mappings. Give kernel teardown time to finish before giving up.
        let deadline = Instant::now() + Duration::from_millis(500);
        while !cleanup_shared_memory(key, pid, uid) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let semaphore = qt_key(directory, SEMAPHORE_FILE, uid);
    unsafe {
        if let Some(key) = semaphore {
            let id = libc::semget(key, 1, 0);
            let mut info: libc::semid_ds = std::mem::zeroed();
            if id >= 0
                && libc::semctl(id, 0, libc::IPC_STAT, &mut info as *mut libc::semid_ds) == 0
                && info.sem_perm.cuid == uid
                && info.sem_perm.uid == uid
                && info.sem_nsems == 1
                && libc::semctl(id, 0, libc::GETPID) == pid
            {
                libc::semctl(id, 0, libc::IPC_RMID, 0);
            }
        }
    }
}

// Return false only when our own segment is still attached and cleanup
// should be retried. Ownership is checked again on each attempt.
fn cleanup_shared_memory(key: libc::key_t, pid: libc::pid_t, uid: u32) -> bool {
    unsafe {
        let id = libc::shmget(key, 0, 0);
        let mut info: libc::shmid_ds = std::mem::zeroed();
        if id < 0
            || libc::shmctl(id, libc::IPC_STAT, &mut info) != 0
            || info.shm_perm.cuid != uid
            || info.shm_perm.uid != uid
            || info.shm_cpid != pid
            || info.shm_lpid != pid
            || info.shm_segsz != 1
        {
            return true;
        }
        if info.shm_nattch != 0 {
            return false;
        }
        libc::shmctl(id, libc::IPC_RMID, std::ptr::null_mut());
        true
    }
}

pub struct Guardian {
    child: Child,
    lease: Option<ChildStdin>,
}
impl Guardian {
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    pub async fn start(engine: &mut Child, executable: &Path, directory: &Path) -> Result<Self> {
        let pid = engine.id().context("引擎已结束")?;
        let fingerprint = identity(pid)?;
        ensure!(
            fingerprint.parent == std::process::id(),
            "引擎不属于当前应用"
        );
        let mut child = Command::new(executable)
            .args([
                "--engine-guardian",
                &std::process::id().to_string(),
                &pid.to_string(),
                &fingerprint.seconds.to_string(),
                &fingerprint.micros.to_string(),
            ])
            .arg(directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            // The guardian must outlive a crashed/killed app.
            .kill_on_drop(false)
            .spawn()
            .context("无法启动引擎退出保护")?;
        let output = child.stdout.take().context("退出保护确认通道不可用")?;
        let mut lines = tokio::io::BufReader::new(output).lines();
        let ready = tokio::time::timeout(Duration::from_secs(3), lines.next_line()).await;
        if !matches!(ready, Ok(Ok(Some(ref line))) if line == "ready") {
            let _ = child.kill().await;
            anyhow::bail!("引擎退出保护未就绪，禁止开始共享");
        }
        Ok(Self {
            lease: child.stdin.take(),
            child,
        })
    }

    // Only call once the engine has been reaped. EOF is also delivered by the
    // kernel if the app exits abruptly, so the guard does not depend on Tokio.
    pub async fn finish(mut self) {
        self.lease.take();
        if tokio::time::timeout(Duration::from_secs(2), self.child.wait())
            .await
            .is_err()
        {
            let _ = self.child.kill().await;
        }
    }
}

pub fn run_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--engine-guardian") {
        return None;
    }
    Some(match run(&args) {
        Ok(()) => 0,
        Err(_) => 2,
    })
}
fn run(args: &[String]) -> Result<()> {
    use std::io::Write;
    ensure!(args.len() == 7, "无效的退出保护参数");
    let parent: u32 = args[2].parse()?;
    let pid: u32 = args[3].parse()?;
    let initial = identity(pid)?;
    let actual_parent = unsafe { libc::getppid() } as u32;
    let owned = owned_parents(parent, actual_parent, initial.parent);
    ensure!(
        owned
            && initial.uid == unsafe { libc::getuid() }
            && initial.seconds == args[4].parse::<u64>()?
            && initial.micros == args[5].parse::<u64>()?,
        "退出保护引擎身份不匹配"
    );
    let mut output = std::io::stdout();
    // A dead owner may have already closed stdout. Its lease still needs to
    // be read so the engine is released rather than abandoned on a broken pipe.
    let _ = writeln!(output, "ready");
    let _ = output.flush();
    // No commands are accepted. The sole trigger is the owner's pipe closing.
    let mut byte = [0u8; 1];
    loop {
        match std::io::stdin().read(&mut byte) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    let engine_alive = || {
        identity(pid).is_ok_and(|current| {
            current.uid == initial.uid
                && current.seconds == initial.seconds
                && current.micros == initial.micros
                && !current.zombie
        })
    };
    if engine_alive() {
        // PID reuse cannot target an unrelated process: match kernel start time.
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine_alive() {
            ensure!(Instant::now() < deadline, "等待键鼠引擎退出超时");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    cleanup_engine_ipc(Path::new(&args[6]), pid);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reclaims_only_detached_qt_ipc_owned_by_the_engine() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let uid = unsafe { libc::getuid() };
        let pid = std::process::id();
        for name in [SHARED_MEMORY_FILE, SEMAPHORE_FILE] {
            std::fs::File::create(path.join(name)).unwrap();
        }
        unsafe {
            let key = qt_key(path, SHARED_MEMORY_FILE, uid).unwrap();
            let shm = libc::shmget(key, 1, libc::IPC_CREAT | libc::IPC_EXCL | 0o600);
            assert!(shm >= 0, "{}", std::io::Error::last_os_error());
            let sem_key = qt_key(path, SEMAPHORE_FILE, uid).unwrap();
            let sem = libc::semget(sem_key, 1, libc::IPC_CREAT | libc::IPC_EXCL | 0o600);
            assert!(sem >= 0, "{}", std::io::Error::last_os_error());
            let attached = libc::shmat(shm, std::ptr::null(), 0);
            assert_ne!(attached as isize, -1);
            assert_eq!(libc::semctl(sem, 0, libc::SETVAL, 1), 0);
            let mut op: libc::sembuf = std::mem::zeroed();
            op.sem_op = -1;
            assert_eq!(libc::semop(sem, &mut op, 1), 0);

            // A wrong creator must leave both resources alone.
            cleanup_engine_ipc(path, pid + 1);
            assert_eq!(libc::shmget(key, 0, 0), shm);
            assert_eq!(libc::semget(sem_key, 1, 0), sem);
            // A matching creator is insufficient while memory is attached.
            assert!(!cleanup_shared_memory(key, pid as libc::pid_t, uid));
            std::thread::scope(|scope| {
                let attached_address = attached as usize;
                let detach = scope.spawn(move || {
                    std::thread::sleep(Duration::from_millis(30));
                    assert_eq!(libc::shmdt(attached_address as *const libc::c_void), 0);
                });
                cleanup_engine_ipc(path, pid);
                detach.join().unwrap();
            });
            let remaining_shm = libc::shmget(key, 0, 0);
            let remaining_sem = libc::semget(sem_key, 1, 0);
            if remaining_shm >= 0 {
                libc::shmctl(shm, libc::IPC_RMID, std::ptr::null_mut());
            }
            if remaining_sem >= 0 {
                libc::semctl(sem, 0, libc::IPC_RMID, 0);
            }
            assert_eq!(remaining_shm, -1);
            assert_eq!(remaining_sem, -1);
        }
    }

    #[test]
    fn accepts_both_orders_of_owner_death_during_parent_snapshots() {
        for guard in [42, 1] {
            for engine in [42, 1] {
                assert!(owned_parents(42, guard, engine));
            }
        }
        assert!(!owned_parents(42, 43, 1));
        assert!(!owned_parents(42, 1, 43));
        assert!(!owned_parents(1, 1, 1));
        assert!(!owned_parents(0, 1, 1));
    }
}
