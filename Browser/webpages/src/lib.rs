use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use br_common::is_valid_page;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config { prefix: None }
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{config_dir}/webpages.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!("Failed while parsing webpages config file: {e}. Falling back to default...");
            Config::default()
        }),
        Err(e) => {
            eprintln!("Failed while reading webpages config file: {e}. Falling back to default...");
            Config::default()
        }
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Webpages".into(),
        icon: RString::from("modem"), // Icon from the icon theme
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    // VALIDATING PLUGIN
    // Early return for when the prefix doesn't match:
    if config
        .prefix
        .as_deref()
        .is_some_and(|v| !input.starts_with(&v))
    {
        return RVec::new();
    }

    // Early return for when the input isn't a valid page:
    let is_input_valid_page = match is_valid_page(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed while checking if input is a valid page: {e}");
            return RVec::new();
        }
    };
    if !is_input_valid_page {
        return RVec::new();
    }

    // MAIN
    let stripped_input = match config.prefix.as_deref() {
        Some(v) => input.strip_prefix(v).unwrap(),
        None => &input,
    };

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    RVec::from(vec![Match {
        title: RString::from(stripped_input),
        description: RSome(RString::from("Open page in Firefox")),
        use_pango: false,
        icon: RSome(RString::from("firefox")),
        id: RNone,
    }])
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let default_browser = match br_common::get_default_browser() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed while getting the default browser: {e}");
            return HandleResult::Close;
        }
    };

    default_browser
        .open(&selection.title)
        .unwrap_or_else(|e| eprintln!("Failed while opening URL in browser: {e}"));

    HandleResult::Close
}
