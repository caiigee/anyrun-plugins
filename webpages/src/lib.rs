use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{fs, process};
use common::{is_valid_page, Browser};

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some("".to_string()),
        }
    }
}

// QoL methods
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("")
    }
}

struct InitData {
    config: Config,
    default_browser: Box<dyn Browser>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = match fs::read_to_string(format!("{config_dir}/bookmarks.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "Failed while parsing bookmarks config file: {e}. Falling back to default..."
            );
            Config::default()
        }),
        Err(e) => {
            eprintln!(
                "Failed while reading bookmarks config file: {e}. Falling back to default..."
            );
            Config::default()
        }
    };

    let default_browser = common::get_default_browser().unwrap_or_else(|e| {
        eprintln!("Failed while getting default browser in init: {e}. Crashing the program...");
        process::exit(1);
    });

    InitData {
        config,
        default_browser,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Webpages".into(),
        icon: RString::from("modem"),
    }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData {
        config,
        default_browser,
    } = data;

    // VALIDATING PLUGIN
    // Early return for when the prefix doesn't match:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    // Early return for when the input isn't a valid page:
    let is_input_valid_page = match is_valid_page(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed while checking if input is a valid page: {e}.");
            return RVec::new();
        }
    };
    if !is_input_valid_page {
        return RVec::new();
    }

    // MAIN
    let stripped_input = input.strip_prefix(config.prefix()).unwrap();

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    RVec::from(vec![Match {
        title: RString::from(stripped_input),
        description: RSome(RString::from("Open page in default browser.")),
        use_pango: false,
        icon: RSome(RString::from(default_browser.icon())),
        id: RNone,
    }])
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData {
        config: _,
        default_browser,
    } = data;
    
    default_browser
        .open(&selection.title)
        .unwrap_or_else(|e| eprintln!("Failed while opening URL in browser: {e}."));

    HandleResult::Close
}
