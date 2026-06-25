#[allow(clippy::all)]
#[allow(unused, dead_code)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/codegen.rs"));
}

mod rule;

use crate::generated::RulesConfigRules;
use crate::rule::{BUFFER_SIZE, Context, apply_rule, apply_rules};
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use std::collections::VecDeque;
use std::env::current_dir;
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;
use std::str::FromStr;
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
                    .contains(
                        &generated::RulesConfigExcludeDirsItem::from_str(
                            path.strip_prefix(directory).unwrap().to_str().unwrap(),
                        )
                        .unwrap(),
                    )
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
                let ctx = Context::new(&path, directory);
                if match *rules {
                    RulesConfigRules::Variant0(rule_combinator) => {
                        apply_rules(rule_combinator, &ctx).await
                    }
                    RulesConfigRules::Variant1(rule) => apply_rule(rule, &ctx).await,
                } {
                    return Some(path);
                }
                None
            }
            .await
        })
        .buffer_unordered(BUFFER_SIZE)
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

    let mut matched_files_relative = matched_files
        .iter()
        .map(|path| path.strip_prefix(&directory).unwrap().to_path_buf())
        .collect::<Vec<_>>();
    matched_files_relative.sort_by(|a, b| {
        let a_root = a.parent().is_none() || a.parent().unwrap().as_os_str().is_empty();
        let b_root = b.parent().is_none() || b.parent().unwrap().as_os_str().is_empty();

        if a_root && !b_root {
            std::cmp::Ordering::Greater
        } else if !a_root && b_root {
            std::cmp::Ordering::Less
        } else {
            a.cmp(b)
        }
    });

    matched_files_relative.into_iter().for_each(|path| {
        println!("{}", path.display());
    });
}
