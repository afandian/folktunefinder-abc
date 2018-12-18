//! Storage
//! Scan a directory tree of ABC files and store them in a single file.
//! We expect to find a directory of ABC files with numerical filenames (e.g. "123.abc"). They can
//! be found in any directory structure. Internal u32 tune IDs are derived from these numerical IDs.
//! Duplicate IDs are ignored.
//! Because it takes a long time to scan hundreds of thousands of files, they must be scanned into
//! a 'tunecache' file, which is the concatenation of all the ABC files. The AbcCache object wraps
//! this.
//! The ABCCache offers an additional caching layer which stores the ABC tunes in memory. For
//! server memory efficiency at runtime, this can be enabled or disabled. When disabled, every
//! request goes to disk.
//! These two levels of cache are built into the same object, rather than represented as two
//! distinct layers, because we either want the whole cache at once, or nothing. Loading in two
//! steps would require a linear file scan, followed by a potentially random file scan, which would
//! be very slow.
//!
//! Usage:

//! Scan a directory and save the tunecache file:
//! let x = ABCCache::new("/my/tunecache", CacheBehaviour.ReadWrite);
//! x.scan("/my/tunes");
//! x.flush();
//!
//! Load a tunecache for live use, no string caching:
//! let x = ABCCache::new("/my/tunecache", CacheBehaviour.ReadOnly);
//! let content = x.get(32);
//!
//! Load a tunecache for live use, all strings cached:
//! let x = ABCCache::new("/my/tunecache", CacheBehaviour.ReadWrite);
//! let content = x.get(32);

extern crate glob;
extern crate time;

use std::collections::HashMap;

use std::io::SeekFrom;
use std::io::Write;

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::sync::Arc;

use std::env;
use std::path::PathBuf;

use std::io::{BufReader, BufWriter};

#[derive(Clone)]
pub enum CacheBehaviour {
    // Read only, light memory footprint but potentially slower.
    ReadOnly,

    // Read and write, cache all strings in memory.
    ReadWrite,
}

// Cache of ABC tunes, indexed by u32 ID, returning a string. TODO ARC?
// For read-write mode, this loads all strings into memory.
// For read-only mode, it can store only the file offsets in memory.
pub struct ABCCache {
    // Path of the tune cache file.
    cache_path: PathBuf,

    // Map of Tune ID to ABC String.
    // This doesn't need to be populated necessarily,
    // but all lookups will look here first.
    string_cache: HashMap<u32, Arc<String>>,

    // Map of Tune ID to start offset, length.
    // This is always populated, and serves as the canonical index of tune IDs we know about.
    offset_cache: HashMap<u32, (usize, usize)>,

    behaviour: CacheBehaviour,

    // Open file handle which we keep for the lifetime of this object.
    reader: BufReader<std::fs::File>,
}

// Given a filename of a source ABC file, return the tune ID.
fn tune_id_from_filename(filepath: &PathBuf) -> Option<u32> {
    if let Some(file_name) = filepath.file_name() {
        if let Some(file_name) = file_name.to_str() {
            if let Some(first) = file_name.split(".").next() {
                if let Ok(val) = first.parse::<u32>() {
                    return Some(val);
                }
            }
        }
    }

    return None;
}

impl ABCCache {
    // Construct a new empty ABCCache.
    // If should_cache_strings is set to true, load the ABCs into memory on construct.
    // Otherwise every access means touching disk.
    pub fn new(cache_path: PathBuf, behaviour: CacheBehaviour) -> Result<ABCCache, String> {
        if let Ok(f) = File::open(&cache_path) {
            let mut reader = BufReader::new(f);
            Ok(ABCCache {
                cache_path,
                behaviour,
                reader,
                string_cache: HashMap::new(),
                offset_cache: HashMap::new(),
            })
        } else {
            Err("Failed to open file reader".to_string())
        }
    }

    // Load the cache file from disk (according to cache behaviour).
    pub fn load_cache(&mut self) {
        // Reset everything.
        self.string_cache = HashMap::new();
        self.offset_cache = HashMap::new();

        // Limit the tunes to this max id for debugging / profiling.
        let key = "DEBUG_MAX_ID";
        let debug_max_id = match env::var(key) {
            Ok(value) => {
                eprintln!("Using {} {}", key, value);
                Some(value.parse::<u32>().unwrap())
            }
            _ => None,
        };

        // If there's no cache file, skip loading it.
        if let Ok(f) = File::open(&self.cache_path) {
            let mut reader = BufReader::new(f);

            // Header for each chunk is:
            // 4 bytes of tune ID.
            // 4 bytes of length.
            let mut header_buf = vec![0u8; 8];

            loop {
                match reader.read_exact(&mut header_buf) {
                    // End of file is ok here.
                    Err(_) => break,
                    _ => (),
                }

                let tune_id: u32 = (header_buf[0] as u32)
                    | (header_buf[1] as u32) << 8
                    | (header_buf[2] as u32) << 16
                    | (header_buf[3] as u32) << 24;

                let length: usize = (header_buf[4] as usize)
                    | (header_buf[5] as usize) << 8
                    | (header_buf[6] as usize) << 16
                    | (header_buf[7] as usize) << 24;

                // Normally we load all tunes.
                // If this config is set, ignore those above this value.
                let keep = match debug_max_id {
                    // If there's a max tune ID and we're within it, keep.
                    Some(x) if tune_id <= x => true,
                    // If there's no max tune ID, keep.
                    None => true,
                    // Otherwise, skip.
                    _ => false,
                };

                if keep {
                    // Always read into the offset cache.

                    // Need to get the current offset.
                    let offset = reader.seek(SeekFrom::Current(0)).unwrap();
                    self.offset_cache.insert(tune_id, (offset as usize, length));

                    // And read the string based on configuration.
                    match self.behaviour {
                        CacheBehaviour::ReadWrite => {
                            // Allocate a new buf each time. This becomes backing for the String.
                            let mut string_buf = Vec::with_capacity(length);
                            string_buf.resize(length, 0x0);

                            reader.read_exact(&mut string_buf);

                            // End of file here is unexpected. Panic!
                            let tune_string = String::from_utf8(string_buf).unwrap();
                            self.string_cache.insert(tune_id, Arc::new(tune_string));
                        }
                        CacheBehaviour::ReadOnly => {
                            // Or just skip the content.
                            reader.seek(SeekFrom::Current(length as i64));
                        }
                    }
                } else {
                    // If we're ignoring this tune, skip it.
                    reader.seek(SeekFrom::Current(length as i64));
                }
            }

            eprintln!("Loaded {} tunes", self.offset_cache.len());
        } else {
            eprintln!("No pre-existing tune cache file found, starting from scratch.");
        }
    }

    // Flush the string cache.
    pub fn flush(&mut self) {
        // Ignore in read-only mode.
        match self.behaviour {
            CacheBehaviour::ReadOnly => return,
            _ => (),
        };

        eprintln!("Saving {} tunes", self.string_cache.len());
        let f = File::create(&self.cache_path).expect("Can't open!");
        let mut writer = BufWriter::new(f);

        let mut metadata_buf = vec![0u8; 8];

        for (tune_id, value) in self.string_cache.iter() {
            let string_buf = value.as_bytes();
            let length = string_buf.len();

            metadata_buf[0] = (tune_id & 0x000000FF) as u8;
            metadata_buf[1] = ((tune_id & 0x0000FF00) >> 8) as u8;
            metadata_buf[2] = ((tune_id & 0x00FF0000) >> 16) as u8;
            metadata_buf[3] = ((tune_id & 0xFF000000) >> 24) as u8;

            metadata_buf[4] = (length & 0x000000FF) as u8;
            metadata_buf[5] = ((length & 0x0000FF00) >> 8) as u8;
            metadata_buf[6] = ((length & 0x00FF0000) >> 16) as u8;
            metadata_buf[7] = ((length & 0xFF000000) >> 24) as u8;

            writer.write_all(&metadata_buf).expect("Can't write");

            writer.write_all(&string_buf).expect("Can't write");
        }
    }

    // Recursively scan a directory of ABC files into String cache.
    pub fn scan_dir(&mut self, base: &String) {
        let mut glob_path = PathBuf::new();
        glob_path.push(base);
        glob_path.push("**");
        glob_path.push("*");
        glob_path.set_extension("abc");

        let mut num_scanned = 0;
        let mut num_indexed = 0;

        // Iterate and load into cache.
        for entry in glob::glob(glob_path.to_str().expect("Can't create path"))
            .expect("Failed to read glob pattern")
        {
            match entry {
                Ok(filepath) => {
                    if let Some(tune_id) = tune_id_from_filename(&filepath) {
                        // Check our index, only read the file if we haven't got it yet.
                        // The offset cache serves as the canonical index of tune IDs.
                        if !self.offset_cache.contains_key(&tune_id) {
                            let mut content = String::new();

                            let mut f = File::open(filepath).expect("file not found");

                            f.read_to_string(&mut content).expect("Can't read file");

                            self.string_cache.insert(tune_id, Arc::new(content));
                            num_indexed += 1;
                        }
                    } else {
                        eprintln!("Failed to get tune id for path: {}", filepath.display());
                    }
                }
                Err(e) => eprintln!("Error {:?}", e),
            }

            num_scanned += 1;

            if num_scanned % 10000 == 0 {
                eprintln!("Scanned {} tunes, indexed {}", num_scanned, num_indexed);
            }
        }
    }

    pub fn max_id(&self) -> u32 {
        let mut max = 0;

        for tune_id in self.offset_cache.keys() {
            max = u32::max(max, *tune_id);
        }

        max
    }

    // Because this involves a file seek, this needs to be mutable.
    pub fn get(&mut self, tune_id: u32) -> Option<Arc<String>> {
        // Return from string cache if it's there.
        match self.string_cache.get(&tune_id) {
            Some(val) => Some(Arc::clone(val)),

            // Otherwise retrieve from file, if it's there.
            _ => match self.offset_cache.get(&tune_id) {
                Some((offset, length)) => {
                    let mut string_buf = Vec::with_capacity(*length);

                    self.reader.seek(SeekFrom::Start(*offset as u64));
                    string_buf.resize(*length, 0x0);
                    self.reader.read_exact(&mut string_buf);

                    Some(Arc::new(String::from_utf8(string_buf).unwrap()))
                }

                // Or none.
                _ => None,
            },
        }
    }
}

// Cloning involves opening a new file handle.
impl Clone for ABCCache {
    fn clone(&self) -> ABCCache {
        let f = File::open(&self.cache_path).unwrap();
        let mut reader = BufReader::new(f);
        ABCCache {
            reader,
            cache_path: self.cache_path.clone(),
            behaviour: self.behaviour.clone(),
            string_cache: self.string_cache.clone(),
            offset_cache: self.offset_cache.clone(),
        }
    }
}
