use std::env;
use std::path::PathBuf;

pub fn get_home_dir() -> PathBuf {
    env::var("HOME")
        .ok()
        .or_else(|| env::var("USERPROFILE").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}
