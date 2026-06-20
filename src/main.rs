use serde::Deserialize;
use std::collections::VecDeque;
use std::env::current_dir;
use std::fs::read_to_string;
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

impl From<&Rule> for BaseRule {
    fn from(rule: &Rule) -> Self {
        BaseRule {
            dirpath: rule.dirpath.clone(),
            filename: rule.filename.clone(),
            content: rule.content.clone(),
        }
    }
}

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

fn apply_string_comparison_base_rule(rule: StringComparisonBaseRule, value: String) -> bool {
    match rule {
        StringComparisonBaseRule::Variant0 {
            startswith,
            contains,
            endswith,
        } => {
            startswith.map(|s| value.starts_with(&s)).unwrap_or(true)
                && contains.map(|c| value.contains(&c)).unwrap_or(true)
                && endswith.map(|e| value.ends_with(&e)).unwrap_or(true)
        }
        StringComparisonBaseRule::Variant1 { equals } => value.eq(&equals),
    }
}

fn apply_string_comparison_rule(rule: StringComparisonRule, value: String) -> bool {
    match rule {
        StringComparisonRule::Variant0 {
            startswith,
            contains,
            endswith,
            not,
        } => {
            let positive_section = apply_string_comparison_base_rule(
                StringComparisonBaseRule::Variant0 {
                    startswith,
                    contains,
                    endswith,
                },
                value.clone(),
            );
            let negative_section = not
                .map(|not_rule| apply_string_comparison_base_rule(not_rule, value.clone()).not())
                .unwrap_or(true);
            positive_section && negative_section
        }
        StringComparisonRule::Variant1 { equals, not } => {
            let positive_section = apply_string_comparison_base_rule(
                StringComparisonBaseRule::Variant1 { equals },
                value.clone(),
            );
            let negative_section = not
                .map(|not_rule| apply_string_comparison_base_rule(not_rule, value.clone()).not())
                .unwrap_or(true);
            positive_section && negative_section
        }
    }
}

fn apply_dirpath_rule(rule: StringComparisonRule, dirpath: String) -> bool {
    apply_string_comparison_rule(rule, dirpath)
}

fn apply_filename_rule(rule: StringComparisonRule, filename: String) -> bool {
    apply_string_comparison_rule(rule, filename)
}

fn apply_content_rule(rule: StringComparisonRule, path: &PathBuf) -> bool {
    let content = read_to_string(path).unwrap().to_string();
    apply_string_comparison_rule(rule, content)
}

fn apply_base_rule(rule: &BaseRule, path: &PathBuf, relative_to: &PathBuf) -> bool {
    let dirpath_result = rule
        .dirpath
        .as_ref()
        .map(|dirpath_rule| {
            apply_dirpath_rule(
                dirpath_rule.clone(),
                path.parent()
                    .map(|p| p.strip_prefix(relative_to).unwrap().to_str().unwrap().to_string())
                    .unwrap_or("".into()),
            )
        })
        .unwrap_or(true);

    let filename_result = rule
        .filename
        .as_ref()
        .map(|filename_rule| apply_filename_rule(filename_rule.clone(), path.file_name().unwrap().to_str().unwrap().to_string()))
        .unwrap_or(true);

    let content_result = rule
        .content
        .as_ref()
        .map(|content_rule| apply_content_rule(content_rule.clone(), path))
        .unwrap_or(true);

    dirpath_result && filename_result && content_result
}

fn apply_rule(rule: &Rule, path: &PathBuf, relative_to: &PathBuf) -> bool {
    let base_result = apply_base_rule(&BaseRule::from(rule), path, &relative_to);
    let not_result = rule.not.as_ref().map(|not_rule| apply_base_rule(&not_rule, path, &relative_to)).unwrap_or(true);

    base_result && not_result.not()
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

    let initial_directories: Vec<PathBuf> = directory
        .read_dir()
        .unwrap()
        .map(|e| {
            e.unwrap()
                .path()
                .strip_prefix(directory.clone())
                .unwrap()
                .to_path_buf()
        })
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

    let matched_files: Vec<&PathBuf> = all_files
        .iter()
        .filter(|path| {
            config
                .rules
                .iter()
                .map(|rule| apply_rule(rule, path, &directory))
                .all(|result| result)
        })
        .collect();

    matched_files
        .iter()
        .for_each(|path| println!("{}", path.display()));
}
