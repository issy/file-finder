mod rule;

use crate::rule::apply_rule;
use serde::Deserialize;
use std::collections::VecDeque;
use std::env::current_dir;
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

#[serde_args::generate(version)]
#[derive(Deserialize)]
#[serde(expecting = "")]
#[serde(rename_all = "kebab-case")]
struct Args {
    config_file: PathBuf,
    #[serde(alias = "d")]
    directory: Option<PathBuf>,
}

fn validate_directory(path: PathBuf) -> Result<PathBuf, String> {
    if !path.exists() {
        Err(format!("The path '{}' does not exist.", path.display()))
    } else if !path.is_dir() {
        Err(format!("The path '{}' is not a directory.", path.display()))
    } else {
        Ok(path)
    }
}

fn find_files_in_directory_for_config(directory: &PathBuf, config: RulesConfig) -> Vec<PathBuf> {
    let initial_directories: Vec<PathBuf> = directory
        .read_dir()
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|path| {
            path.is_file()
                || config
                    .exclude_dirs
                    .contains(&path.to_str().unwrap().to_string())
                    .not()
        })
        .collect();

    initial_directories
        .iter()
        .for_each(|path| println!("{}", path.display()));

    let mut all_files: Vec<PathBuf> = Vec::new();
    let mut to_explore: VecDeque<PathBuf> = VecDeque::from(initial_directories.clone());
    while !to_explore.is_empty() {
        let current_path = to_explore.pop_front().unwrap();
        if current_path.is_file() {
            all_files.push(current_path);
        } else {
            let read_dir = current_path.read_dir().unwrap();
            read_dir.for_each(|entry| {
                let path = entry.unwrap().path();
                to_explore.push_back(path);
            });
        }
    }

    all_files
        .into_iter()
        .filter(|path| {
            config
                .rules
                .iter()
                .map(|rule| apply_rule(rule, path, &directory))
                .all(|result| result)
        })
        .map(|path| path.strip_prefix(directory).unwrap().to_path_buf())
        .collect()
}

fn main() {
    let args: Args = match serde_args::from_env() {
        Ok(args) => args,
        Err(error) => {
            println!("{error}");
            return;
        }
    };

    let file = File::open(args.config_file).unwrap();
    let config: RulesConfig = serde_yaml_ng::from_reader(file).unwrap();

    let directory = args
        .directory
        .map(validate_directory)
        .map(Result::unwrap)
        .unwrap_or(current_dir().unwrap());

    let matched_files = find_files_in_directory_for_config(&directory, config);

    matched_files
        .into_iter()
        .for_each(|path| println!("{}", path.display()));
}
