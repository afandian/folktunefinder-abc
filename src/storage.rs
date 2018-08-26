//! Storage
//! Scan a directory tree of ABC files and store them in a single file.

extern crate glob;
extern crate time;

use std::collections::HashMap;

use std::io::Write;

use std::fs::File;
use std::io::Read;

use std::env;
use std::path::PathBuf;

use std::io::{BufReader, BufWriter};

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

pub fn tune_cache_path() -> Option<PathBuf> {
    let key = "BASE";
    match env::var(key) {
        Ok(base) => {
            let mut path = PathBuf::new();
            path.push(&base);
            path.push("tunecache");

            Some(path)
        }
        _ => None,
    }
}

pub fn load(filename: &PathBuf) -> HashMap<u32, String> {
    let mut result = HashMap::new();

    // Limit the tunes to this max id for debugging / profiling.
    let key = "DEBUG_MAX_ID";
    let debug_max_id = match env::var(key) {
        Ok(value) => {
            eprintln!("Using {} {}", key, value);
            Some(value.parse::<u32>().unwrap())
        }
        _ => None,
    };

    // It may not exist, in which case skip.
    if let Ok(f) = File::open(filename) {
        let mut reader = BufReader::new(f);

        let mut metadata_buf = vec![0u8; 8];

        loop {
            match reader.read_exact(&mut metadata_buf) {
                // End of file is ok here.
                Err(_) => break,
                _ => (),
            }

            let tune_id: u32 = (metadata_buf[0] as u32) | (metadata_buf[1] as u32) << 8
                | (metadata_buf[2] as u32) << 16
                | (metadata_buf[3] as u32) << 24;

            let length: usize = (metadata_buf[4] as usize) | (metadata_buf[5] as usize) << 8
                | (metadata_buf[6] as usize) << 16
                | (metadata_buf[7] as usize) << 24;

            // Allocate a new buf each time. This becomes backing for the String.
            let mut string_buf = Vec::with_capacity(length);
            string_buf.resize(length, 0x0);

            reader.read_exact(&mut string_buf);

            // End of file here is unexpected. Panic!
            let tune_string = String::from_utf8(string_buf).unwrap();

            // Normally we load all tunes.
            // If this config is set, ignore those above this value.
            match debug_max_id {
                Some(x) if tune_id <= x => {
                    result.insert(tune_id, tune_string);
                }
                None => {
                    result.insert(tune_id, tune_string);
                }
                _ => (),
            };
        }

        eprintln!("Loaded {} tunes", result.len());
    } else {
        eprintln!("No pre-existing tune cache file found, starting from scratch.");
    }

    result
}

pub fn save(tunes: &HashMap<u32, String>, filename: &PathBuf) {
    eprintln!("Saving {} tunes", tunes.len());
    let f = File::create(filename).expect("Can't open!");
    let mut writer = BufWriter::new(f);

    let mut metadata_buf = vec![0u8; 8];

    for (tune_id, value) in tunes {
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

pub fn scan(tunes: &mut HashMap<u32, String>, base: &String) {
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
                    if !tunes.contains_key(&tune_id) {
                        let mut content = String::new();

                        let mut f = File::open(filepath).expect("file not found");

                        f.read_to_string(&mut content).expect("Can't read file");

                        tunes.insert(tune_id, content);
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

pub fn max_id(tunes: &HashMap<u32, String>) -> u32 {
    let mut max = 0;

    for tune_id in tunes.keys() {
        max = u32::max(max, *tune_id);
    }

    max
}
