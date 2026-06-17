use std::path::PathBuf;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(expecting = "")]
#[serde(rename_all = "kebab-case")]
struct Args {
    config_file: PathBuf,
    #[serde(alias = "d")]
    directory: Option<PathBuf>,
}

fn main() {
    let args: Args = match serde_args::from_env() {
        Ok(args) => args,
        Err(error) => {
            println!("{error}");
            return;
        }
    };
    // TODO: Parse config file
    // TODO: Find directory

}
