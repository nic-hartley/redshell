//! Representations of the various bits of "physical" computers in-game, at a high enough level to be convenient while
//! still offering the space for exciting and interesting tools.

use std::sync::Arc;

use dashmap::{mapref::entry::Entry as DMEntry, DashMap};

use crate::tools::Tool;

/// Represents a file on an in-game machine
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub contents: String,
}

/// Represents a directory entry on an in-game machine
#[derive(Clone, Debug)]
pub enum Entry {
    File(File),
    Directory(Arc<DashMap<String, Entry>>),
}

impl Entry {
    /// Convert this to a `File`, or return `None`
    pub fn file(self) -> Option<File> {
        match self {
            Self::File(f) => Some(f),
            _ => None,
        }
    }

    /// Check whether this is a file
    pub fn is_file(&self) -> bool {
        match self {
            Self::File(_) => true,
            _ => false,
        }
    }

    /// Convert this to a [`Entry::Directory`]'s contents, or return `None`
    pub fn dir(self) -> Option<Arc<DashMap<String, Entry>>> {
        match self {
            Self::Directory(d) => Some(d),
            _ => None,
        }
    }

    /// Check whether this is a directory
    pub fn is_dir(&self) -> bool {
        match self {
            Self::Directory(_) => true,
            _ => false,
        }
    }
}

// this shouldn't be needed outside of tests so let's enforce that
#[cfg(test)]
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Entry::File(sf), Entry::File(of)) => sf == of,
            (Entry::Directory(sd), Entry::Directory(od)) => {
                for sitem in sd.iter() {
                    let oitem = match od.get(sitem.key()) {
                        Some(i) => i,
                        None => return false,
                    };
                    if sitem.value() != oitem.value() {
                        return false;
                    }
                }
                for oitem in od.iter() {
                    let sitem = match od.get(oitem.key()) {
                        Some(i) => i,
                        None => return false,
                    };
                    if sitem.value() != oitem.value() {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}
#[cfg(test)]
impl Eq for Entry {}

/// A single machine in-game, somewhere in the CyberZone. Possibly even the player's own.
#[derive(Default, Clone)]
pub struct Machine {
    /// The files on this machine
    pub root: Arc<DashMap<String, Entry>>,
    /// the tools available at the command line
    pub tools: DashMap<String, Arc<dyn Tool>>,
}

impl Machine {
    fn dir(&self, path: &str, make: bool) -> Result<Arc<DashMap<String, Entry>>, String> {
        if path.is_empty() {
            return Ok(self.root.clone());
        }
        if !path.starts_with('/') {
            return Err(format!("absolute path {} doesn't start with /", path));
        }
        let mut dir = self.root.clone();
        for comp in path.split('/') {
            if comp.is_empty() {
                // ignore consecutive slashes
                continue;
            }
            let into = match dir.entry(comp.to_owned()) {
                DMEntry::Occupied(p) => match p.get() {
                    Entry::File(_) => return Err(format!("{} is a file", comp)),
                    Entry::Directory(dm) => dm.clone(),
                },
                DMEntry::Vacant(p) => {
                    if make {
                        let dm = Arc::new(DashMap::new());
                        p.insert(Entry::Directory(dm.clone()));
                        dm
                    } else {
                        return Err(format!("Directory {} doesn't exist", comp));
                    }
                }
            };
            dir = into;
        }
        Ok(dir)
    }

    /// Create an empty directory on the filepath.
    ///
    /// If `parents` is true, will also create any parents. (`mkdir -p`)
    ///
    /// If the directory exists, this does nothing.
    pub fn mkdir(&self, path: &str, make_parents: bool) -> Result<(), String> {
        if !path.ends_with('/') {
            return Err(format!("directories must end with trailing slashes"));
        }

        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() {
            // root already exists
            return Ok(());
        }
        let (parent, file) = trimmed
            .rsplit_once('/')
            .ok_or(format!("absolute path {} doesn't start with /", path))?;
        let dir = self.dir(parent, make_parents)?;
        let res = match dir.entry(file.to_owned()) {
            DMEntry::Occupied(p) => {
                if p.get().is_file() {
                    Err(format!("{} is a file", trimmed))
                } else {
                    Ok(())
                }
            }
            DMEntry::Vacant(p) => {
                p.insert(Entry::Directory(Default::default()));
                Ok(())
            }
        };
        res
    }

    /// Write a file to the machine's disk at the absolute path.
    ///
    /// Will overwrite any files already there, but will not replace files with directories.
    ///
    /// Returns Ok(()) if everything worked, or Err(msg) if not.
    pub fn write(&self, path: &str, contents: String) -> Result<(), String> {
        // valid: /foo/bar
        //        /foo//bar
        // not:   foo/bar
        //        /foo/bar/
        let (parent, file) = path
            .rsplit_once('/')
            .ok_or(format!("absolute path {} doesn't start with /", path))?;
        if file.is_empty() {
            return Err(format!("filepaths cannot end with trailing slash"));
        }
        let dir = self.dir(parent, false)?;
        let new = Entry::File(File { contents });
        match dir.entry(file.to_owned()) {
            DMEntry::Occupied(mut p) => {
                if p.get().is_file() {
                    p.insert(new);
                } else {
                    return Err(format!("{} is a directory", path));
                }
            }
            DMEntry::Vacant(p) => {
                p.insert(new);
            }
        }

        Ok(())
    }

    /// Will get any kind of [`Entry`] from the machine's disk at the absolute path.
    ///
    /// Returns Ok(entry) if everything worked, or Err(msg) if not.
    pub fn entry(&self, path: &str) -> Result<Entry, String> {
        if path == "/" {
            // special-case for root: there isn't really an entry but we can fake one
            return Ok(Entry::Directory(self.root.clone()));
        }
        let (parent, file) = path
            .trim_end_matches('/')
            .rsplit_once('/')
            .unwrap_or(("", path));
        let dir = self.dir(parent, false)?;
        let entry = dir.get(file).ok_or(format!("no such entry: {}", path))?;
        Ok(entry.value().clone())
    }

    /// Read a file from the machine's disk at the absolute path.
    ///
    /// Can only read files; if you try to read a directory this fails.
    ///
    /// Returns Ok(file) if everything worked, or Err(msg) if not.
    ///
    /// See also [`Self::readdir`] and [`Self::entry`].
    pub fn read(&self, path: &str) -> Result<File, String> {
        let entry = self.entry(path)?;
        let file = entry.file().ok_or(format!("cannot read non-files"))?;
        Ok(file)
    }

    /// Iterate over the contents of a directory.
    ///
    /// Returns Ok(iter) if everything worked, or Err(msg) if not.
    ///
    /// See also [`Self::read`] and [`Self::entry`].
    pub fn readdir(&self, path: &str) -> Result<impl Iterator<Item = (String, Entry)>, String> {
        let entry = self.entry(path)?;
        let dir = entry
            .dir()
            .ok_or(format!("cannot readdir non-directory {}", path))?;
        Ok(dir.as_ref().clone().into_iter())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_machine_fs_empty() {
        let mach = Machine::default();
        assert!(mach.root.is_empty());
    }

    #[test]
    fn machine_writes_file_to_root() {
        let mach = Machine::default();
        mach.write("/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");

        let f = mach
            .read("/spooky")
            .expect("failed to read file just written");
        assert_eq!(f.contents, "ghost");
    }

    #[test]
    fn machine_overwrites_file_to_root() {
        let mach = Machine::default();
        mach.write("/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");
        mach.write("/spooky", "zombie".into())
            .expect("failed to overwrite file");

        let f = mach
            .read("/spooky")
            .expect("failed to read file just written");
        assert_eq!(f.contents, "zombie");
    }

    #[test]
    fn machine_file_wont_write_with_dir() {
        let mach = Machine::default();
        mach.write("/dir/file", "".into())
            .expect_err("successfully wrote to nonexistent subdirectory");
    }

    #[test]
    fn machine_writes_file_in_subdir() {
        let mach = Machine::default();
        mach.mkdir("/things/", true)
            .expect("failed to mkdir in empty filesystem");
        mach.write("/things/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");

        let f = mach
            .read("/things/spooky")
            .expect("failed to read file just written");
        assert_eq!(f.contents, "ghost");
    }

    #[test]
    fn machine_overwrites_file_in_subdir() {
        let mach = Machine::default();
        mach.mkdir("/things/", true)
            .expect("failed to mkdir in empty filesystem");
        mach.write("/things/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");
        mach.write("/things/spooky", "zombie".into())
            .expect("failed to overwrite file");

        mach.read("/things")
            .expect_err("/things should be a directory");
        mach.read("/spooky").expect_err("/spooky should not exist");
        let f = mach
            .read("/things/spooky")
            .expect("failed to read file just written");
        assert_eq!(f.contents, "zombie");
    }

    #[test]
    fn machine_wont_overwrite_file_with_dir() {
        let mach = Machine::default();
        mach.write("/things", "many".into())
            .expect("failed to write to empty filesystem");
        mach.write("/things/spooky", "ghost".into())
            .expect_err("didn't return error on attempted file overwrite with dir");

        let f = mach
            .read("/things")
            .expect("could not read file that should be there");
        assert_eq!(f.contents, "many");
    }

    #[test]
    fn machine_wont_overwrite_dir_with_file() {
        let mach = Machine::default();
        mach.mkdir("/things/", true)
            .expect("failed to mkdir in empty filesystem");
        mach.write("/things/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");
        mach.write("/things", "ghost".into())
            .expect_err("didn't return error on attempted dir overwrite with file");

        let f = mach
            .read("/things/spooky")
            .expect("could not read file that should be there");
        assert_eq!(f.contents, "ghost");
    }

    #[test]
    fn machine_entry_reads_file() {
        let mach = Machine::default();
        mach.write("/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");

        let e = mach
            .entry("/spooky")
            .expect("coud not read entry that should be there");
        assert_eq!(
            e,
            Entry::File(File {
                contents: "ghost".into()
            })
        );
    }

    #[test]
    fn machine_entry_reads_dir() {
        let mach = Machine::default();
        mach.mkdir("/things/", true)
            .expect("failed to mkdir in empty filesystem");
        mach.write("/things/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");

        let e = mach
            .entry("/things")
            .expect("coud not read entry that should be there");
        assert!(matches!(e, Entry::Directory(_)));
    }

    #[test]
    fn machine_readdir_reads_dir() {
        let mach = Machine::default();
        mach.mkdir("/things/", true)
            .expect("failed to mkdir in empty filesystem");
        mach.write("/things/spooky", "ghost".into())
            .expect("failed to write to empty filesystem");
        mach.write("/things/cute", "me".into())
            .expect("failed to write to empty filesystem");

        let es = mach
            .readdir("/things")
            .expect("could not read dir that should be there");
        let mut es: Vec<_> = es.collect();
        es.sort_by(|l, r| l.0.cmp(&r.0));
        assert_eq!(
            es[0],
            (
                "cute".to_owned(),
                Entry::File(File {
                    contents: "me".into()
                })
            )
        );
        assert_eq!(
            es[1],
            (
                "spooky".to_owned(),
                Entry::File(File {
                    contents: "ghost".into()
                })
            )
        );
    }
}
