use crate::state::Store;
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// On-disk home for the deployment records and their logs.
///
/// Both the listener and the root runner write here, so every read-modify-write
/// goes through an flock on a separate lock file, and the state file itself is
/// replaced atomically so a crash mid-write can never truncate it.
#[derive(Debug, Clone)]
pub struct StateDir {
    root: PathBuf,
}

impl StateDir {
    pub fn new(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("logs"))?;
        Ok(StateDir { root })
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    fn lock_path(&self) -> PathBuf {
        self.root.join("state.lock")
    }

    pub fn log_path(&self, id: u64) -> PathBuf {
        self.root.join("logs").join(format!("{id}.log"))
    }

    fn read_unlocked(&self) -> io::Result<Store> {
        match fs::read_to_string(self.state_path()) {
            Ok(raw) => serde_json::from_str(&raw).map_err(|e| io::Error::other(e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Store::new(new_epoch())),
            Err(e) => Err(e),
        }
    }

    fn write_unlocked(&self, store: &Store) -> io::Result<()> {
        let tmp = self.root.join("state.json.tmp");
        let body = serde_json::to_vec_pretty(store).map_err(|e| io::Error::other(e.to_string()))?;
        {
            let mut f = File::create(&tmp)?;
            f.write_all(&body)?;
            f.sync_all()?;
        }
        fs::rename(&tmp, self.state_path())
    }

    pub fn load(&self) -> io::Result<Store> {
        let lock = self.lock()?;
        let store = self.read_unlocked();
        let _ = FileExt::unlock(&lock);
        store
    }

    fn lock(&self) -> io::Result<File> {
        let f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(self.lock_path())?;
        f.lock_exclusive()?;
        Ok(f)
    }

    /// Read, mutate and write back under one lock.
    pub fn update<T>(&self, f: impl FnOnce(&mut Store) -> T) -> io::Result<T> {
        let lock = self.lock()?;
        let result = (|| {
            let mut store = self.read_unlocked()?;
            let out = f(&mut store);
            self.write_unlocked(&store)?;
            Ok(out)
        })();
        let _ = FileExt::unlock(&lock);
        result
    }

    pub fn append_log(&self, id: u64, bytes: &[u8]) -> io::Result<u64> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path(id))?;
        f.write_all(bytes)?;
        Ok(f.metadata()?.len())
    }

    /// Serve a log by byte offset so a poller can stream it without re-reading.
    pub fn read_log(&self, id: u64, offset: u64, max: usize) -> io::Result<(Vec<u8>, u64, bool)> {
        let mut f = File::open(self.log_path(id))?;
        let len = f.metadata()?.len();
        if offset >= len {
            return Ok((Vec::new(), len, true));
        }
        f.seek(SeekFrom::Start(offset))?;
        let want = std::cmp::min(max as u64, len - offset) as usize;
        let mut buf = vec![0u8; want];
        f.read_exact(&mut buf)?;
        let next = offset + want as u64;
        Ok((buf, next, next >= len))
    }

    pub fn log_size(&self, id: u64) -> u64 {
        fs::metadata(self.log_path(id))
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Logs are big and records are not, so logs are reaped much sooner. The
    /// stored error tail means a reaped deploy still explains itself.
    pub fn reap_logs(&self, keep_ids: &[u64]) -> io::Result<Vec<u64>> {
        let mut reaped = Vec::new();
        for entry in fs::read_dir(self.root.join("logs"))? {
            let path = entry?.path();
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<u64>().ok());
            if let Some(id) = id
                && !keep_ids.contains(&id)
            {
                fs::remove_file(&path)?;
                reaped.push(id);
            }
        }
        Ok(reaped)
    }
}

/// Ids restart from 1 if the state file is ever lost, so a random epoch lets a
/// caller notice it is polling a different store than the one it started with.
fn new_epoch() -> String {
    let mut buf = [0u8; 8];
    if let Ok(mut f) = File::open("/dev/urandom") {
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Admission, State};

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("hived-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn state_survives_a_round_trip() {
        let dir = StateDir::new(tmpdir("roundtrip")).unwrap();
        dir.update(|s| s.admit("aaa", 10)).unwrap();

        let store = dir.load().unwrap();
        assert_eq!(store.running().unwrap().rev, "aaa");
        assert_eq!(store.next_id, 2);
    }

    #[test]
    fn epoch_is_stable_across_writes() {
        let dir = StateDir::new(tmpdir("epoch")).unwrap();
        dir.update(|s| s.admit("aaa", 1)).unwrap();
        let first = dir.load().unwrap().epoch;
        dir.update(|s| s.admit("bbb", 2)).unwrap();
        assert_eq!(dir.load().unwrap().epoch, first);
        assert!(!first.is_empty());
    }

    #[test]
    fn updates_from_separate_handles_do_not_clobber() {
        let path = tmpdir("concurrent");
        let a = StateDir::new(&path).unwrap();
        let b = StateDir::new(&path).unwrap();

        assert_eq!(
            a.update(|s| s.admit("aaa", 1)).unwrap(),
            Admission::Started(1)
        );
        // b re-reads under the lock rather than writing a stale copy
        assert_eq!(
            b.update(|s| s.admit("bbb", 2)).unwrap(),
            Admission::Queued(2)
        );

        let store = a.load().unwrap();
        assert_eq!(store.deployments.len(), 2);
        assert_eq!(store.running().unwrap().rev, "aaa");
        assert_eq!(store.queued().unwrap().rev, "bbb");
    }

    #[test]
    fn logs_stream_by_offset() {
        let dir = StateDir::new(tmpdir("logs")).unwrap();
        dir.append_log(1, b"hello ").unwrap();
        dir.append_log(1, b"world").unwrap();

        let (chunk, next, eof) = dir.read_log(1, 0, 5).unwrap();
        assert_eq!(chunk, b"hello");
        assert_eq!(next, 5);
        assert!(!eof);

        let (chunk, next, eof) = dir.read_log(1, next, 4096).unwrap();
        assert_eq!(chunk, b" world");
        assert_eq!(next, 11);
        assert!(eof);

        // reading at the end is not an error, it is just empty
        let (chunk, _, eof) = dir.read_log(1, 11, 4096).unwrap();
        assert!(chunk.is_empty());
        assert!(eof);
    }

    #[test]
    fn reaping_keeps_the_listed_logs() {
        let dir = StateDir::new(tmpdir("reap")).unwrap();
        for id in 1..=4 {
            dir.append_log(id, b"x").unwrap();
        }
        let reaped = dir.reap_logs(&[3, 4]).unwrap();
        assert_eq!(reaped.len(), 2);
        assert!(dir.log_path(3).exists());
        assert!(!dir.log_path(1).exists());
    }

    #[test]
    fn a_missing_state_file_reads_as_empty() {
        let dir = StateDir::new(tmpdir("fresh")).unwrap();
        let store = dir.load().unwrap();
        assert!(store.deployments.is_empty());
        assert_eq!(store.next_id, 1);
    }

    #[test]
    fn finishing_persists() {
        let dir = StateDir::new(tmpdir("finish")).unwrap();
        dir.update(|s| s.admit("aaa", 1)).unwrap();
        dir.update(|s| s.finish(1, State::Succeeded, 5)).unwrap();
        assert_eq!(dir.load().unwrap().get(1).unwrap().state, State::Succeeded);
    }
}
