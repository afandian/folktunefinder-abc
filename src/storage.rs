//! Storage
//! Scan a directory tree of ABC files and store them in a single file.
//! We expect to find a directory of ABC files with numerical filenames (e.g. "123.abc"). They can
//! be found in any directory structure. Internal u32 tune IDs are derived from these numerical IDs.
//! Duplicate IDs are ignored.
//! Because it takes a long time to scan hundreds of thousands of files, they must be scanned into
//! a 'tunecache' file, which is the concatenation of all the ABC files.
//! CacheScanner iterates over this, returning entries.
//! ReadOnlyCache maintains a set of file offsets for retrieval of strings.
//! ReadWriteCache stores the strings in memory for quick (large) access.

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

/// Object for returning iterators that scan over the TuneDB.
pub struct CacheScanner {
    cache_path: PathBuf,
}

impl CacheScanner {
    pub fn new(cache_path: PathBuf) -> CacheScanner {
        CacheScanner { cache_path }
    }

    pub fn iter(&self) -> CacheIterator {
        CacheIterator::new(&self.cache_path)
    }

    // Linear scan to retrieve tune by its ID.
    // Not quick, opens a file handle, but OK for quick lookups.
    pub fn find_by_id(&self, tune_id: u32) -> Option<CacheEntry> {
        self.iter().find(|x| x.tune_id == tune_id)
    }
}

pub struct CacheIterator {
    reader: BufReader<std::fs::File>,

    // Header for each chunk is:
    // 4 bytes of tune ID.
    // 4 bytes of length.
    header_buf: Vec<u8>,
}

impl CacheIterator {
    fn new(cache_path: &PathBuf) -> CacheIterator {
        let f = File::open(&cache_path).unwrap();
        let reader = BufReader::new(f);
        CacheIterator {
            reader,
            header_buf: vec![0u8; 8],
        }
    }
}

#[derive(Debug)]
pub struct CacheEntry {
    // ID of the tune.
    pub tune_id: u32,

    // Offset within file.
    pub offset: u64,
    // Length in bytes of the tune in the file.
    pub length: usize,

    // Text content of the tune.
    pub content: String,
}

// Read a buffered reader at the current offset.
// Return a CacheEntry with a newly allocated string.
// Reuse the header_buf.
fn read_cache_entry(
    reader: &mut BufReader<std::fs::File>,
    header_buf: &mut [u8],
) -> Option<CacheEntry> {
    let tune_id: u32 = (header_buf[0] as u32)
        | (header_buf[1] as u32) << 8
        | (header_buf[2] as u32) << 16
        | (header_buf[3] as u32) << 24;

    let length: usize = (header_buf[4] as usize)
        | (header_buf[5] as usize) << 8
        | (header_buf[6] as usize) << 16
        | (header_buf[7] as usize) << 24;

    // Allocate each time, as we pass it into the result.
    let mut content_buf = vec![0u8; length];

    // Need to get the current offset.
    let offset = reader.seek(SeekFrom::Current(0)).unwrap();

    // This fills the buffer, which has been resized to the the right length.
    match reader.read_exact(&mut content_buf) {
        Err(_) => {
            eprintln!("Error! Tried to read invalid file offset.");
            return None;
        }
        _ => {
            let content = match String::from_utf8(content_buf) {
                Ok(content) => content,
                _ => {
                    // E.g. encoding issues.
                    // Return empty string rather than None, or we'd stop iteration.
                    eprintln!("Failed to read string buffer.");
                    String::new()
                }
            };
            Some(CacheEntry {
                tune_id,
                offset,
                length,
                content,
            })
        }
    }
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

impl Iterator for CacheIterator {
    type Item = CacheEntry;

    fn next(&mut self) -> Option<CacheEntry> {
        match self.reader.read_exact(&mut self.header_buf) {
            // End of file is ok here.
            Err(_) => return None,
            _ => (),
        };

        read_cache_entry(&mut self.reader, &mut self.header_buf)
    }
}

// Read-only cache of ABC tunes, indexed by u32 ID, returning a string.
// Doesn't store all the tunes in RAM, instead stores only offset pointers.
// Every access involves a file seek. Holds a file handle open.
pub struct ReadOnlyCache {
    cache_path: PathBuf,

    // Map of Tune ID to start offset, length.
    // This is always populated, and serves as the canonical index of tune IDs we know about.
    offset_cache: HashMap<u32, (usize, usize)>,

    // Open file handle which we keep for the lifetime of this object.
    reader: BufReader<std::fs::File>,
}

impl ReadOnlyCache {
    pub fn new(cache_path: PathBuf) -> Result<ReadOnlyCache, String> {
        if let Ok(f) = File::open(&cache_path) {
            let mut reader = BufReader::new(f);

            // Start by loading.
            let mut cache = ReadOnlyCache {
                cache_path,
                reader,
                offset_cache: HashMap::new(),
            };
            cache.load_cache();

            Ok(cache)
        } else {
            Err("Failed to open file reader".to_string())
        }
    }

    // Load the cache file from disk.
    pub fn load_cache(&mut self) {
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

        let mut scanner = CacheScanner::new(self.cache_path.clone());

        for entry in scanner.iter() {
            if let Some(max_id) = debug_max_id {
                if entry.tune_id > max_id {
                    continue;
                }
            }

            // Only need the offset and length.
            self.offset_cache
                .insert(entry.tune_id, (entry.offset as usize, entry.length));
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
    pub fn get(&mut self, tune_id: u32) -> Option<String> {
        match self.offset_cache.get(&tune_id) {
            Some((offset, length)) => {
                let mut string_buf = Vec::with_capacity(*length);

                match self.reader.seek(SeekFrom::Start(*offset as u64)) {
                    Err(_) => {
                        eprintln!("Error! Tried to seek to invalid file offset.");
                        return None;
                    }
                    Ok(_) => (),
                };

                string_buf.resize(*length, 0x0);
                match self.reader.read_exact(&mut string_buf) {
                    Ok(_) => Some(String::from_utf8(string_buf).unwrap()),
                    Err(_) => None,
                }
            }

            // Or none.
            _ => None,
        }
    }
}

// Read-write cache of ABC tunes, indexed by u32 tune ID, returning a string.
// Stores all tunes in a big hash table.
pub struct ReadWriteCache {
    // Path of the tune cache file.
    cache_path: PathBuf,

    // Map of Tune ID to ABC String.
    // This doesn't need to be populated necessarily,
    // but all lookups will look here first.
    string_cache: HashMap<u32, String>,
}

impl ReadWriteCache {
    // Load the cache file from disk.
    pub fn load_cache(&mut self) {
        // Reset everything.
        self.string_cache = HashMap::new();

        // Limit the tunes to this max id for debugging / profiling.
        let key = "DEBUG_MAX_ID";
        let debug_max_id = match env::var(key) {
            Ok(value) => {
                eprintln!("Using {} {}", key, value);
                Some(value.parse::<u32>().unwrap())
            }
            _ => None,
        };

        let mut scanner = CacheScanner::new(self.cache_path.clone());

        for entry in scanner.iter() {
            if let Some(max_id) = debug_max_id {
                if entry.tune_id > max_id {
                    continue;
                }
            }

            self.string_cache.insert(entry.tune_id, entry.content);
        }
    }

    // Construct a new empty ReadWriteCache.
    pub fn new(cache_path: PathBuf) -> Result<ReadWriteCache, String> {
        if let Ok(f) = File::open(&cache_path) {
            let mut reader = BufReader::new(f);
            let mut cache = ReadWriteCache {
                cache_path,
                string_cache: HashMap::new(),
            };
            cache.load_cache();
            Ok(cache)
        } else {
            Err("Failed to open file reader".to_string())
        }
    }

    // Flush the string cache.
    pub fn flush(&mut self) {
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
                        if !self.string_cache.contains_key(&tune_id) {
                            let mut content = String::new();

                            let mut f = File::open(filepath).expect("file not found");

                            f.read_to_string(&mut content).expect("Can't read file");

                            self.string_cache.insert(tune_id, content);
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

        for tune_id in self.string_cache.keys() {
            max = u32::max(max, *tune_id);
        }

        max
    }

    pub fn get(&self, tune_id: u32) -> Option<String> {
        // Return from string cache if it's there.
        match self.string_cache.get(&tune_id) {
            // TODO this could be wasteful making a copy. Maybe revert back to ARC.
            Some(val) => Some(val.to_string()),

            // Or none.
            _ => None,
        }
    }
}

// Cloning involves opening a new file handle.
impl Clone for ReadOnlyCache {
    fn clone(&self) -> ReadOnlyCache {
        let f = File::open(&self.cache_path).unwrap();
        let reader = BufReader::new(f);
        ReadOnlyCache {
            reader,
            cache_path: self.cache_path.clone(),
            offset_cache: self.offset_cache.clone(),
        }
    }
}
