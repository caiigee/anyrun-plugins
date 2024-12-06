use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use common::types::BrowserConfig;
use common::types::Engine;
use serde::Deserialize;
use std::{fs, process};

#[derive(Deserialize, Debug)]
struct Config {
    prefix: Option<String>,
}

impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or_default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some(String::default()),
        }
    }
}

struct InitData {
    config: Config,
    browser_config: BrowserConfig,
    default_browser: Box<dyn common::Browser>,
    engines: Vec<Engine>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = match fs::read_to_string(format!("{config_dir}/websearch.ron")) {
        Ok(s) => ron::from_str(&s)
            .map_err(|e| {
                format!(
                "(Websearch) Failed while parsing config file. Falling back to default...:\n  {e}"
            )
            })
            .unwrap_or_default(),
        Err(e) => {
            eprintln!(
                "(Websearch) Failed while reading config file. Falling back to default...\n  {e}"
            );
            Config::default()
        }
    };

    let browser_config = match fs::read_to_string(format!("{config_dir}/browser.ron")) {
        Ok(s) => ron::from_str(&s)
            .map_err(|e| {
                format!(
                    "(Websearch) Failed while parsing browser config file. \
                Falling back to default...\n  {e}"
                )
            })
            .unwrap_or_default(),
        Err(e) => {
            eprintln!(
                "(Websearch) Failed while reading browser config file. Falling back to default...\n  {e}"
            );
            BrowserConfig::default()
        }
    };

    let default_browser = match common::get_default_browser() {
        Ok(browser) => browser,
        Err(e) => {
            eprintln!("(Websearch) Failed while getting the default browser. Closing...:\n  {e}");
            process::exit(1)
        }
    };

    let engines = match default_browser.search_engines(browser_config.profile_name()) {
        Ok(engines) => engines,
        Err(e) => {
            eprintln!("(Websearch) Failed while getting the search engines. Closing...:\n  {e}");
            process::exit(1)
        }
    };

    InitData {
        config,
        browser_config,
        default_browser,
        engines,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Websearch".into(),
        icon: "distributor-logo-netrunner".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData {
        config,
        browser_config: _,
        default_browser: _,
        engines,
    } = data;

    // VALIDATING PLUGIN
    // Early return for wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    };

    // // Early return for a valid page, which is something the plugin "webpages" should handle:
    // match is_valid_page(&input) {
    //     Ok(is_input_valid_page) => {
    //         if is_input_valid_page {
    //             return RVec::new();
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("(Websearch) Failed while checking if input is a valid page in websearch plugin: {e}. Returning no matches...");
    //         return RVec::new();
    //     }
    // };

    // MAIN
    // We can safely unwrap here because of the first early return.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Handling blank input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    let Some(engine) = engines.iter().find(|engine| {
        engine
            .defined_aliases()
            .iter()
            .any(|alias| stripped_input.starts_with(alias))
    }) else {
        eprintln!("(Websearch) Failed while finding the engine with a matching alias. Returning no matches...");
        return RVec::new();
    };

    // The Python coder in me took over...
    let mut twice_stripped_input = "";
    for alias in engine.defined_aliases() {
        if !stripped_input.starts_with(alias) {
            continue;
        }
        twice_stripped_input = stripped_input.strip_prefix(alias).unwrap().trim()
    }

    RVec::from(vec![Match {
        title: RString::from(twice_stripped_input),
        description: RSome(RString::from(format!("Search with {}", engine._name()))),
        use_pango: false,
        icon: RSome(RString::from(engine.icon_url())),
        id: RNone,
    }])

    // let (always_valid_engines, possibly_valid_engines): (Vec<&util::Engine>, Vec<&util::Engine>) =
    //     config
    //         .engines()
    //         .into_iter()
    //         .partition(|engine| engine.prefix.is_empty());

    // let valid_engines: Vec<&util::Engine> = possibly_valid_engines
    //     .into_iter()
    //     .filter(|engine| stripped_input.starts_with(&engine.prefix))
    //     .collect();

    // // Early return for when more than two engines with nonblank prefixes are valid:
    // if valid_engines.len() >= 2 {
    //     eprintln!("Two or more nonblank engine prefixes are valid!");
    //     return RVec::new();
    // }

    // // Returning matches with "always valid engines" if there are no valid engines with a nonblank prefix:
    // if valid_engines.is_empty() {
    //     return always_valid_engines
    //         .into_iter()
    //         .map(|engine| Match {
    //             title: RString::from(stripped_input),
    //             description: RSome(RString::from(format!("Search with {}", engine.name))),
    //             use_pango: false,
    //             icon: RSome(RString::from(engine.icon())),
    //             id: RNone,
    //         })
    //         .collect();
    // }

    // // At this point it is obvious there is only one valid engine because of the earlier checks.
    // let valid_engine = valid_engines[0];

    // let twice_stripped_input = stripped_input
    //     .strip_prefix(valid_engine.prefix.as_str())
    //     .unwrap()
    //     .trim();

    // RVec::from(vec![Match {
    //     title: RString::from(twice_stripped_input),
    //     description: RSome(RString::from(format!("Search with {}", valid_engine.name))),
    //     use_pango: false,
    //     icon: RSome(RString::from(valid_engine.icon())),
    //     id: RNone,
    // }])
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData {
        config: _,
        browser_config,
        default_browser,
        engines,
    } = data;

    // The .unwrap() here cannot possibly fail because, well, every selection must have some description.
    let selected_engine_name = selection.description.unwrap().replace("Search with ", "");
    // The .unwrap() here cannot fail beacuse the engine vector must hold the element of interest.
    let engine = engines
        .iter()
        .find(|engine| engine._name() == selected_engine_name)
        .unwrap();

    let params = engine._urls()[0]
        .params()
        .iter()
        .map(|param| format!("{}={}", param.name(), param.value()))
        .collect::<Vec<String>>()
        .join("&");
    let url = format!("{}?{}", engine._urls()[0].template(), params)
        .replace("{searchTerms}", &selection.title);

    default_browser
        .open(&url, browser_config.command_prefix())
        .unwrap_or_else(|e| eprintln!("(Websearch) Failed while opening URL in browser:\n  {e}"));

    HandleResult::Close
}
