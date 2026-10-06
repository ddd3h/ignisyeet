//! Output-directory safety: atomic file writes and the per-directory run lock.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Name of the advisory lock file in the output directory.
pub const LOCK_FILE: &str = ".ignisyeet.lock";

/// Temporary name next to `path` (same directory, so the rename stays on one file system).
fn temp_name(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.tmp", std::process::id()))
}

/// Writes `path` through a temporary file and a rename: a reader or a concurrent writer never sees a
/// half-written file. The temporary file is removed when `f` or the write fails.
pub fn write_with(path: &Path, f: impl FnOnce(&mut std::io::BufWriter<std::fs::File>) -> Result<()>) -> Result<()> {
    use std::io::Write;
    let tmp = temp_name(path);
    let res = (|| -> Result<()> {
        let mut w = std::io::BufWriter::new(std::fs::File::create(&tmp).with_context(|| format!("cannot write {}", path.display()))?);
        f(&mut w)?;
        w.flush()?;
        std::fs::rename(&tmp, path).with_context(|| format!("cannot write {}", path.display()))
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// Like [`write_with`] for a writer that wants a path: `f` writes the temporary file `tmp`, which is then
/// renamed to `path`.
pub fn write_via_path(path: &Path, f: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let tmp = temp_name(path);
    let res = f(&tmp).and_then(|()| std::fs::rename(&tmp, path).with_context(|| format!("cannot write {}", path.display())));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// Atomic `std::fs::write`.
pub fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    use std::io::Write;
    write_with(path, |w| Ok(w.write_all(contents.as_ref())?))
}

/// Directories this process holds the lock of.
static HELD: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only checks that the process exists.
        let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
        r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

/// Takes the advisory lock of the output directory `dir` (creating it) for the rest of the process; a second
/// call for the same directory is a no-op. Fails when a live process holds it; a lock of a dead process is
/// removed.
pub fn lock_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let key = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let mut held = HELD.lock().unwrap();
    if held.contains(&key) {
        return Ok(());
    }
    let lock = dir.join(LOCK_FILE);
    for _ in 0..3 {
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&lock) {
            Ok(mut f) => {
                use std::io::Write;
                writeln!(f, "{}", std::process::id())?;
                held.push(key);
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let pid = std::fs::read_to_string(&lock).ok().and_then(|t| t.trim().parse::<u32>().ok());
                match pid {
                    Some(p) if pid_alive(p) => {
                        bail!("another ignisyeet run is using {} (lock file {}, pid {p}); wait for it or use a different --out (if no other run is active, delete {})", dir.display(), lock.display(), lock.display())
                    }
                    Some(_) => {
                        let _ = std::fs::remove_file(&lock);
                    }
                    // The owner has created the file but not yet written its pid.
                    None => std::thread::sleep(std::time::Duration::from_millis(50)),
                }
            }
            Err(e) => return Err(e).with_context(|| format!("cannot create the lock file {}", lock.display())),
        }
    }
    bail!(
        "another ignisyeet run is using {} (lock file {}); wait for it or use a different --out (if no other run is active, delete {})",
        dir.display(),
        lock.display(),
        lock.display()
    )
}

/// Removes the lock files this process holds.
pub fn unlock_all() {
    for d in HELD.lock().unwrap().drain(..) {
        let _ = std::fs::remove_file(d.join(LOCK_FILE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ignisyeet-outfile-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn write_is_atomic_and_leaves_no_temp_file() {
        let d = tmpdir("atomic");
        let p = d.join("a.csv");
        write(&p, "one").unwrap();
        write(&p, "two").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "two");
        // A failing writer keeps the old file and removes the temporary one.
        let r = write_with(&p, |_| bail!("boom"));
        assert!(r.is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "two");
        let names: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["a.csv"]);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn live_lock_is_refused_and_stale_lock_is_cleaned() {
        let d = tmpdir("lock");
        // pid 1 is alive and not this process.
        std::fs::write(d.join(LOCK_FILE), "1\n").unwrap();
        let e = lock_dir(&d).unwrap_err().to_string();
        assert!(e.starts_with("another ignisyeet run is using "), "{e}");
        assert!(e.contains("pid 1)") && e.contains("use a different --out"), "{e}");
        // A dead pid: spawn a child and reap it.
        let mut c = std::process::Command::new("true").spawn().unwrap();
        let dead = c.id();
        c.wait().unwrap();
        std::fs::write(d.join(LOCK_FILE), format!("{dead}\n")).unwrap();
        lock_dir(&d).unwrap();
        let pid: u32 = std::fs::read_to_string(d.join(LOCK_FILE)).unwrap().trim().parse().unwrap();
        assert_eq!(pid, std::process::id());
        lock_dir(&d).unwrap(); // the same process again
        std::fs::remove_dir_all(&d).unwrap();
    }
}
