mod hotkey_listener;
#[cfg(feature = "scripting")]
mod scripting;
mod utils;

use std::{error::Error, fmt::Display, io::Write};

use directories::ProjectDirs;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};

const GIT_REV: &str = env!("GIT_REV");
const BUILD_NAME: &str = env!("BUILD_NAME");

#[derive(Debug, Clone, PartialEq, Eq)]
enum ViractionError {
    Other(String),
}

impl Error for ViractionError {}

impl Display for ViractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViractionError::Other(s) => write!(f, "Other: {}", s),
        }
    }
}

// Used to register hotkeys with the OS.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Action {
    name: String,
    keys: Vec<String>,
}

impl Action {
    fn new(name: &String, keys: &[&String]) -> Self {
        Action {
            name: name.clone(),
            keys: keys.into_iter().map(|x| String::from(*x)).collect(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Config {
    run_at_startup: bool,
    actions: Vec<Action>,
}

impl Config {
    fn new() -> Self {
        Config::default()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("---Initializing---");
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Starting vaction: {} - {}", BUILD_NAME, GIT_REV);

    let dirs = ProjectDirs::from("com", "vpuppr", "vaction").unwrap();

    debug!("Config dir: {}", dirs.config_dir().display());

    let config_dir = dirs.config_dir();

    if !config_dir.exists() {
        info!("Creating config directory {}", config_dir.display());
        std::fs::create_dir_all(config_dir)?;
    }

    let mut config_path = config_dir.to_path_buf();
    config_path.push("config.toml");

    let config_path = config_path.as_path();
    if !config_path.exists() {
        info!("Creating initial config {}", config_path.display());

        let mut file = std::fs::File::create(config_path)?;

        let config = toml::to_string_pretty(&Config::new())?;

        file.write_all(config.as_bytes())?;
    }

    info!("Reading config from {}", config_path.display());

    let config = std::fs::read_to_string(config_path)?;

    debug!("{}", config);

    // TODO testing
    {
        let lua = scripting::lua()?;

        lua.load(include_str!("test.lua")).exec()?;
    }

    Ok(())
}
