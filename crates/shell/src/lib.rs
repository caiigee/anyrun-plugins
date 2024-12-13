use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{
    env,
    process::{self, Command},
};

#[derive(Deserialize, Debug)]
struct Config {
    prefix: Option<String>,
    icon: Option<String>,
}

// QoL methods so I don't have to chain methods that much:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("$")
    }
    fn icon(&self) -> &str {
        self.icon.as_deref().unwrap_or("utilities-terminal")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some(String::from("$")),
            icon: Some(String::from("utilities-terminal")),
        }
    }
}

struct InitData {
    config: Config,
    shell: String,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = common::config(&config_dir, "Shell");
    let shell = env::var("SHELL").unwrap_or_else(|e| {
        eprintln!("(Shell) Failed while getting the SHELL env variable. Closing...:\n  {e}");
        process::exit(1);
    });

    InitData { config, shell }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Shell".into(),
        icon: "utilities-terminal".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData { config, shell } = data;

    // VALIDATING PLUGIN
    // Early return for when the prefix doesn't match:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    // MAIN
    // We can safely unwrap() here because we know the prefix exists.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    RVec::from(vec![Match {
        title: stripped_input.trim().into(),
        description: RSome(RString::from(shell.as_str())),
        use_pango: false,
        icon: RSome(RString::from(config.icon())),
        id: RNone,
    }])
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData { config: _, shell } = data;

    // I mean I don't 100% know if this unwrap will succeed or not...
    // I am guessing that the SHELL env variable always returns a path.
    let shell = shell.rsplit("/").next().unwrap();

    match shell {
        "bash" => {
            if let Err(e) = Command::new(shell)
                .args(["-i", "-c", &selection.title])
                .spawn()
            {
                eprintln!("(Shell) Failed while spawning shell command. Closing...\n  {e}")
            }
        }
        _ => eprintln!("(Shell) Unsupported shell! Closing..."),
    }

    HandleResult::Close
}
