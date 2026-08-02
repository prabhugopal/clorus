use std::env;
use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.0);
    }
}

pub fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn with_cwd<T, F>(dir: &Path, f: F) -> T
where
    F: FnOnce() -> T + UnwindSafe,
{
    let _guard = cwd_lock().lock().expect("cwd lock poisoned");
    let original = env::current_dir().expect("get cwd");
    let _cwd_guard = CwdGuard(original);
    env::set_current_dir(dir).expect("cd temp root");
    match catch_unwind(f) {
        Ok(value) => value,
        Err(payload) => resume_unwind(payload),
    }
}
