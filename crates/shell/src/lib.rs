// PIMP 1.
use abi_stable::std_types::{
    ROption::{self, RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{
    env, fs,
    process::{self, Command},
};

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
    shell: Option<String>,
    icon: Option<String>,
}

// QoL methods so I don't have to chain methods that much:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("$")
    }

    fn shell(&self) -> String {
        self.shell
            .clone()
            .unwrap_or(Config::default().shell.unwrap())
    }
}

impl Default for Config {
    fn default() -> Self {
        let shell = env::var("SHELL").unwrap_or_else(|e| {
            eprintln!("Failed while finding the SHELL env variable: {e}");
            process::exit(1);
        });
        Config {
            prefix: Some(String::from("$ ")),
            shell: Some(shell),
            icon: Some(String::from("utilities-terminal")),
        }
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{config_dir}/shell.ron")) {
        Ok(v) => ron::from_str(&v)
            .map_err(|e| {
                format!(
                    "(Shell) Failed while parsing config file. Falling back to default...\n  {e}"
                )
            })
            .unwrap_or_default(),
        Err(e) => {
            eprintln!(
                "(Shell) Failed while reading config file. Falling back to default...\n  {e}"
            );
            Config::default()
        }
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Shell".into(),
        icon: "utilities-terminal".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    // VALIDATING PLUGIN
    // Early return for when the prefix doesn't match:
    let plugin_prefix = config.prefix();
    if !input.starts_with(plugin_prefix) {
        return RVec::new();
    }

    // MAIN
    // We can safely unwrap() here because we know the prefix exists.
    let stripped_input = input.strip_prefix(plugin_prefix).unwrap().trim();

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    RVec::from(vec![Match {
        title: stripped_input.trim().into(),
        description: RSome(RString::from(config.shell())),
        use_pango: false,
        icon: ROption::from(config.icon.as_deref().map(RString::from)),
        id: RNone,
    }])
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let shell = selection.description.unwrap();
    let command = selection.title;

    match Command::new(shell.as_str())
        .args(["-ic", command.as_str()])
        .spawn()
    {
        Ok(_) => (),
        Err(e) => eprintln!("Failed while spawning shell command: {e}"),
    };

    HandleResult::Close
}
