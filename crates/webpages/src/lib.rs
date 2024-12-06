use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use common::Browser;
use common::types::BrowserConfig;
use serde::Deserialize;
use std::{fs, process};

pub fn is_valid_page(input: &str) -> Result<bool, regex::Error> {
    // CREATING THE REGEXES
    // Domain regex:
    let domain_re =
        regex::Regex::new(r"^(https?:\/\/)?(([a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,})(\/\S+)?$")?;

    // IPv4 regex:
    let ipv4_address_re = regex::Regex::new(
        r"^(https?:\/\/)?(((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?))(\/\S+)?$",
    )?;

    // IPv6 regex:
    // TODO 2.

    // Localhost regex:
    let localhost_re = regex::Regex::new(
        r"^(https?:\/\/)?localhost(:(6553[0-5]|655[0-2][0-9]|65[0-4][0-9]{2}|6[0-4][0-9]{3}|[1-5][0-9]{4}|[1-9][0-9]{1,3}|[0-9]))?$",
    )?;

    // "about:" regex
    let about_re = regex::Regex::new(r"^about:[A-Za-z]+$")?;

    // SUCCESS
    return Ok(domain_re.is_match(input)
        || ipv4_address_re.is_match(input)
        || localhost_re.is_match(input)
        || about_re.is_match(input));
}

#[derive(Deserialize)]
struct Config {
    prefix: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some(String::default()),
        }
    }
}

// QoL methods
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or_default()
    }
}

struct InitData {
    config: Config,
    browser_config: BrowserConfig,
    default_browser: Box<dyn Browser>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = match fs::read_to_string(format!("{config_dir}/bookmarks.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "(Webpages) Failed while parsing config file. Falling back to default...\n  {e}"
            );
            Config::default()
        }),
        Err(e) => {
            eprintln!(
                "(Webpages) Failed while reading config file. Falling back to default...\n {e}"
            );
            Config::default()
        }
    };

    let browser_config = match fs::read_to_string(format!("{config_dir}/browser.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "(Webpages) Failed while parsing browser config file. Falling back to default...\n  {e}"
            );
            BrowserConfig::default()
        }),
        Err(e) => {
            eprintln!(
                "(Webpages) Failed while reading browser config file. Falling back to default...\n  {e}"
            );
            BrowserConfig::default()
        }
    };

    let default_browser = common::get_default_browser().unwrap_or_else(|e| {
        eprintln!("(Webpages) Failed while getting default browser in init. Closing...\n  {e}");
        process::exit(1);
    });

    InitData {
        config,
        browser_config,
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
        browser_config: _,
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
            eprintln!("(Webpages) Failed while checking if input is a valid page. Returning no matches...\n  {e}.");
            return RVec::new();
        }
    };
    if !is_input_valid_page {
        return RVec::new();
    }

    // MAIN
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    RVec::from(vec![Match {
        title: RString::from(stripped_input),
        description: RSome(RString::from(format!(
            "Open page in {}",
            default_browser.name()
        ))),
        use_pango: false,
        icon: RSome(RString::from(default_browser.icon())),
        id: RNone,
    }])
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData {
        config: _,
        browser_config,
        default_browser,
    } = data;

    default_browser
        .open(&selection.title, browser_config.command_prefix())
        .unwrap_or_else(|e| eprintln!("(Webpages) Failed while opening URL in browser: {e}."));

    HandleResult::Close
}
