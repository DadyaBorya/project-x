use std::{fs, io};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::models::config_file::ConfigFile;

pub fn is_dir_exists(path: &str) -> bool {
    fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
}

pub fn is_file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn get_all_config_files(path: &str) -> Result<Vec<ConfigFile>, io::Error> {
    let entries = get_sub_entries(path)?;

    let mut config_files = vec![];
    
    for entry in entries {
        let path = entry.join(Path::new("extension.json"));

        if !is_file_exists(path.to_str().unwrap()) {
            // println!("Can't find extension.json in folder {:?}", entry.file_name().unwrap());
            continue;
        }

        let content = read_file(path.to_str().unwrap());

        if content.is_err() {
            // println!("Can't read extension.json in {:?}", entry.file_name().unwrap());
            continue;
        }

        let content = content.unwrap();
        let config_file: Result<ConfigFile, serde_json::Error> = serde_json::from_str(&content);

        if config_file.is_err() {
            // println!("Can't deserialize extension.json in {:?}", entry.file_name().unwrap());
            continue;
        }

        let config_file = config_file.unwrap();
        config_files.push(config_file);
    }

    Ok(config_files)
}

pub fn read_file(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents)
}

pub fn read_file_to_u8_array(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

pub fn get_sub_entries(path: &str) -> Result<Vec<PathBuf>, io::Error> {
    let path = Path::new(path);
    let mut paths = vec![];

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            match entry {
                Ok(entry) => {
                    let entry_path = entry.path();
                    paths.push(entry_path);
                }
                Err(err) => {
                    println!("{:?}", err)
                }
            }
        }
    }

    Ok(paths)
}

pub fn count_dirs(path: &str) -> io::Result<usize> {
    let mut count = 0;

    let path = Path::new(path);

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                count += 1;
            }
        }
    }

    Ok(count)
}