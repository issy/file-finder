#[allow(clippy::all)]
#[allow(unused, dead_code)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
}

mod rule;

use crate::rule::apply_rules;
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use std::collections::VecDeque;
use std::env::current_dir;
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;
use std::sync::Arc;

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

async fn find_files_in_directory_for_config(
    directory: &PathBuf,
    config: generated::RulesConfig,
) -> Vec<PathBuf> {
    let mut all_files: Vec<PathBuf> = Vec::new();
    let mut to_explore: VecDeque<PathBuf> = directory
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

    let rules = Arc::new(&config.rules);

    stream::iter(all_files)
        .map(|path| async {
            let rules = Arc::clone(&rules);
            async move {
                if apply_rules(&rules, &path, directory).await {
                    return Some(path);
                }
                None
            }.await
        })
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect()
}

#[tokio::main]
async fn main() {
    let args: Args = match serde_args::from_env() {
        Ok(args) => args,
        Err(error) => {
            println!("{error}");
            return;
        }
    };

    let file = File::open(args.config_file).unwrap();
    let config: generated::RulesConfig = serde_yaml_ng::from_reader(file).unwrap();

    let directory = args
        .directory
        .map(validate_directory)
        .map(Result::unwrap)
        .unwrap_or(current_dir().unwrap());

    let matched_files = find_files_in_directory_for_config(&directory, config).await;

    matched_files
        .into_iter()
        .for_each(|path| println!("{}", path.display()));
}
