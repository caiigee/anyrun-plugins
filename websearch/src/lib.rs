use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use serde::{Deserialize, Serialize};
use std::fs;
use common::is_valid_page;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Engine {
    name: String,
    url: String,
    prefix: String,
    icon: Option<String>,
}

impl Engine {
    fn icon(&self) -> &str {
        self.icon.as_deref().unwrap_or("system-search")
    }
}

impl Default for Engine {
    fn default() -> Self {
        Engine {
            name: String::from("Google"),
            url: String::from("https://google.com/search?q={}"),
            prefix: String::new(),
            icon: Some(String::from("google")),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Config {
    prefix: Option<String>,
    engines: Option<Vec<Engine>>,
}

impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("?")
    }
    fn engines(&self) -> &[Engine] {
        self.engines.as_deref().unwrap_or_default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some(String::from("?")),
            engines: Some(vec![Engine::default()]),
        }
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{config_dir}/websearch.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "Failed while parsing websearch config file: {e:?}. Falling back to default..."
            );
            Config::default()
        }),
        Err(e) => {
            eprintln!(
                "Failed while reading websearch config file: {e:?}. Falling back to default..."
            );
            Config::default()
        }
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
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    // VALIDATING PLUGIN
    // Early return for wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    };

    // Early return for a valid page, which is something the plugin "webpages" should handle:
    match is_valid_page(&input) {
        Ok(is_input_valid_page) => {
            if is_input_valid_page {
                return RVec::new();
            }
        }
        Err(e) => {
            eprintln!("Failed while checking if input is a valid page in websearch plugin: {e:?}. Returning no matches...");
            return RVec::new();
        }
    };

    // MAIN
    // We can safely unwrap here because of the first early return.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Early return for an empty stripped input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    let (always_valid_engines, possibly_valid_engines): (Vec<&Engine>, Vec<&Engine>) = config
        .engines()
        .into_iter()
        .partition(|engine| engine.prefix.is_empty());

    let valid_engines: Vec<&Engine> = possibly_valid_engines
        .into_iter()
        .filter(|engine| stripped_input.starts_with(&engine.prefix))
        .collect();

    // Early return for when more than two engines with nonblank prefixes are valid:
    if valid_engines.len() >= 2 {
        eprintln!("Two or more nonblank engine prefixes are valid!");
        return RVec::new();
    }

    // Returning matches with "always valid engines" if there are no valid engines with a nonblank prefix:
    if valid_engines.is_empty() {
        return always_valid_engines
            .into_iter()
            .map(|engine| Match {
                title: RString::from(stripped_input),
                description: RSome(RString::from(format!("Search with {}", engine.name))),
                use_pango: false,
                icon: RSome(RString::from(engine.icon())),
                id: RNone,
            })
            .collect();
    }

    // At this point it is obvious there is only one valid engine because of the earlier checks.
    let valid_engine = valid_engines[0];

    let twice_stripped_input = stripped_input
        .strip_prefix(valid_engine.prefix.as_str())
        .unwrap()
        .trim();

    RVec::from(vec![Match {
        title: RString::from(twice_stripped_input),
        description: RSome(RString::from(format!("Search with {}", valid_engine.name))),
        use_pango: false,
        icon: RSome(RString::from(valid_engine.icon())),
        id: RNone,
    }])
}

#[handler]
fn handler(selection: Match, config: &Config) -> HandleResult {
    let selected_engine_name = selection.description.unwrap().replace("Search with ", "");
    let engine = config
        .engines()
        .into_iter()
        .find(|engine| engine.name == selected_engine_name)
        // It is safe to unwrap because the selected engine must be in config.engines.
        .unwrap();

    let default_browser = match common::get_default_browser() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("(Websearch Plugin) Failed while getting the default browser: {e}. Closing Anyrun...");
            return HandleResult::Close;
        }
    };

    default_browser
        .open(engine.url.replace("{}", &selection.title).as_str())
        .unwrap_or_else(|e| {
            eprintln!("(Websearch Plugin) Failed while opening URL in browser: {e}")
        });

    HandleResult::Close
}
