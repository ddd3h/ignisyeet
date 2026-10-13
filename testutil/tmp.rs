//! Test-only scratch directory that is removed on drop (also while unwinding from a panic).
//! Shared by all crates through `#[path]` so no extra dependency is needed.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A fresh, empty directory `<temp_dir>/ignisyeet_<tag>_<pid>_<n>`, unique per process and call.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ignisyeet_{tag}_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("cannot create the temporary test directory");
        TempDir(p)
    }
}

impl std::ops::Deref for TempDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn removed_on_drop_and_unique() {
        let (a, b) = (TempDir::new("guard"), TempDir::new("guard"));
        assert_ne!(&*a, &*b);
        std::fs::write(a.join("f"), "x").unwrap();
        let p = a.to_path_buf();
        drop(a);
        assert!(!p.exists() && b.exists());
    }

    #[test]
    fn removed_when_the_test_panics() {
        let p = std::sync::Mutex::new(PathBuf::new());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let d = TempDir::new("guard_panic");
            *p.lock().unwrap() = d.to_path_buf();
            std::fs::write(d.join("f"), "x").unwrap();
            panic!("boom");
        }));
        assert!(r.is_err());
        assert!(!p.lock().unwrap().exists());
    }
}
