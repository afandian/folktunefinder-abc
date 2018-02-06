//! Storage
//! Scan a directory tree of ABC files and store them in a single file.

extern crate glob;
extern crate time;

use std::collections::HashMap;

use std::io::{Write, LineWriter};

use std::collections::btree_map::BTreeMap;
use std::fs::File;
use std::io::Read;


use std::path::PathBuf;
use std::env;


/// Read a file from a path, return bytes.
fn read_file(filename: &PathBuf) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut f = File::open(filename).expect("file not found");
    f.read_to_end(&mut buf).expect("can't read");
    return buf;
}

fn write_file(filename: &PathBuf, buf: &Vec<u8>) {
    let mut f = File::create(filename).expect("file not found");
    f.write_all(&buf).expect("can't write");
}

/// Take a PathBuf that represents a filename and parse out the tune ID.
/// None if one couldn't be extracted.
fn tune_id_from_filename(filepath: &PathBuf) -> Option<u32> {
    // let path = PathBuf::from(filepath);
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


/// A cache of MIDI files represented as one glob.
#[derive(Debug)]
pub struct TuneCache {
    filename: PathBuf,

    /// Copy of the file content.
    // Format:
    // u32 tune id, little-endian
    // u32 data length, little-endian
    // data
    buffer: Vec<u8>,

    // tune id -> (offset, length)
    index: HashMap<u32, (usize, usize)>,
}

impl TuneCache {
    /// Construct a new TuneCache at the given filename.
    pub fn new(filename: PathBuf) -> TuneCache {
        let mut result = TuneCache {
            filename,
            buffer: Vec::new(),
            index: HashMap::new(),
        };

        result.load();

        result
    }

    /// Scan the buffer and generate indexes.
    fn create_index(&mut self) {
        let mut i: usize = 0;
        let max = self.buffer.len();
        while i < max {
            let tune_id: u32 = (self.buffer[i + 0] as u32) | (self.buffer[i + 1] as u32) << 8 |
                (self.buffer[i + 2] as u32) << 16 |
                (self.buffer[i + 3] as u32) << 24;

            let length: usize = (self.buffer[i + 4] as usize) | (self.buffer[i + 5] as usize) << 8 |
                (self.buffer[i + 6] as usize) << 16 |
                (self.buffer[i + 7] as usize) << 24;

            // Move i to the data portion. Use the start of data to index.
            i += 8;

            self.index.insert(tune_id, (i, length));
            i += length;
        }
    }

    /// Read a TuneCache into memory, if there is one.
    fn load(&mut self) {

        println!("Read cache...");

        // If there's no file, return.
        let mut file = match File::open(&self.filename) {
            Err(_) => return,
            Ok(f) => f,
        };

        // If it exists but we can't read it, that's a problem.
        file.read_to_end(self.buffer.as_mut()).expect(
            "Can't read tune cache file.",
        );

        self.create_index();
    }

    fn save(&self) {
        println!("Write tune cache to {:?}", &self.filename);

        write_file(&self.filename, &self.buffer);
    }

    fn has_tune(&mut self, tune_id: u32) -> bool {
        self.index.contains_key(&tune_id)
    }

    /// Insert the tune data into the cache, if we don't have it, else skip.
    fn ensure(&mut self, tune_id: u32, data: &Vec<u8>) {
        if self.has_tune(tune_id) {
            // println!("Cache skip {}", &tune_id);
        } else {
            // println!("Cache add {}", &tune_id);

            // Previous length of the buffer is the new starting index for this tune.
            let offset = self.buffer.len();
            let length = data.len();

            self.buffer.push((tune_id & 0x000000FF) as u8);
            self.buffer.push(((tune_id & 0x0000FF00) >> 8) as u8);
            self.buffer.push(((tune_id & 0x00FF0000) >> 16) as u8);
            self.buffer.push(((tune_id & 0xFF000000) >> 24) as u8);


            self.buffer.push((length & 0x000000FF) as u8);
            self.buffer.push(((length & 0x0000FF00) >> 8) as u8);
            self.buffer.push(((length & 0x00FF0000) >> 16) as u8);
            self.buffer.push(((length & 0xFF000000) >> 24) as u8);

            self.buffer.extend(data.iter());

            // Skip the 4 bytes for the tune id and length;
            self.index.insert(tune_id, (offset + 4, length));
        }
    }

    /// Get the data for a given tune.
    pub fn get_tune(&self, tune_id: &u32) -> Option<&[u8]> {
        match self.index.get(tune_id) {
            None => None,
            Some(&(start, length)) => Some(&self.buffer[start..start + length]),
        }
    }

    pub fn get_tune_string(&self, tune_id: &u32) -> Option<String> {
        if let Some(content) = self.get_tune(tune_id) {
            if let Ok(string) = String::from_utf8(content.to_vec()) {
                Some(string)
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct TuneStore {
    // base: String,
    glob_path: PathBuf,
    pub tune_cache: TuneCache,
}

impl TuneStore {
    pub fn new() -> TuneStore {
        let key = "BASE";
        match env::var(key) {
            Ok(base) => {
                let mut tune_cache_path = PathBuf::new();
                tune_cache_path.push(&base);
                tune_cache_path.push("tunecache");

                let mut glob_path = PathBuf::new();
                glob_path.push(&base);
                glob_path.push("**");
                glob_path.push("*");
                glob_path.set_extension("abc");

                let tune_cache = TuneCache::new(tune_cache_path);
                return TuneStore {
                    tune_cache,
                    glob_path,
                };
            }
            Err(e) => panic!("Couldn't get config value {}: {}", key, e),
        }
    }

    /// Scan all files in the tune store, write to a consolidated cache file.
    pub fn scan(&mut self) {
        let mut num_scanned = 0;
        let mut num_indexed = 0;

        // Iterate and load into cache.
        for entry in glob::glob(self.glob_path.to_str().expect("Can't create path"))
            .expect("Failed to read glob pattern")
        {
            match entry {
                Ok(filepath) => {
                    if let Some(tune_id) = tune_id_from_filename(&filepath) {

                        // Check our index, only read the file if we haven't got it yet.
                        if !self.tune_cache.has_tune(tune_id) {
                            let data = read_file(&filepath);
                            self.tune_cache.ensure(tune_id, &data);
                            num_indexed += 1;
                        }

                    } else {
                        println!("Failed to get tune id for path: {}", filepath.display());
                    }
                }
                Err(e) => println!("Error {:?}", e),
            }

            num_scanned += 1;

            if num_scanned % 10000 == 0 {
                println!("Scanned {} tunes, indexed {}", num_scanned, num_indexed);
            }
        }

        self.tune_cache.save();
    }
}
