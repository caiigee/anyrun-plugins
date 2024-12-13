use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{
    error::Error,
    process::{self},
};

mod firefox;

#[derive(Debug)]
struct Engine {
    name: String,
    url: String,
    alias: String,
    icon: String,
}

impl Engine {
    pub fn new(name: &str, url: &str, alias: &str, icon: &str) -> Self {
        Engine {
            name: name.to_string(),
            url: url.to_string(),
            alias: alias.to_string(),
            icon: icon.to_string(),
        }
    }
}

trait SearchEngines: common::Browser {
    // I suspect every single browser would need the profile name as a parameter...
    fn search_engines(&self, profile_name: &str) -> Result<Vec<Engine>, Box<dyn Error>>;
}

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
    common_config: common::CommonConfig,
    browser: Box<dyn SearchEngines>,
    engines: Vec<Engine>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = common::config(&config_dir, "Websearch");
    let common_config = common::common_config(&config_dir, "Websearch");

    // NOTE 1
    let browser_id = common::default_browser_id().unwrap_or_else(|e| {
        eprintln!("(Websearch) Failed while getting the default browser. Closing...\n  {e}");
        process::exit(1)
    });
    let browser = match browser_id.as_str() {
        "firefox" => Box::new(common::Firefox::new(common_config.browser_profile_name())),
        _ => {
            eprintln!("(Websearch) Unsupported default browser! Closing...");
            process::exit(1)
        }
    };    

    let engines = browser
        .search_engines(common_config.browser_profile_name())
        .unwrap_or_else(|e| {
            eprintln!("(Websearch) Failed while getting engines! Closing...\n  {e}");
            process::exit(1)
        });

    InitData {
        config,
        common_config,
        browser,
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
        common_config: _,
        browser: _,
        engines,
    } = data;

    // Early return for wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    };

    // We can safely unwrap here because of the first early return.
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Handling blank input:
    if stripped_input.is_empty() {
        return RVec::new();
    }

    // Finding the appropriate engine:
    let valid_engines: Vec<&Engine> = engines
        .iter()
        .filter(|engine| stripped_input.starts_with(&engine.alias))
        .collect();

    // I am pretty sure this is necessary in the case that a user doesn't have any engine
    // with an emptry string alias. I mean think about it, if there is even one empty-string-alias
    // engine, the `valid_engines` vector will never ever be empty. On the other hand however, if
    // a user only has non-empty-string-alias engines, there is a chance that `valid_engines` will
    // be empty. For that reason, the below code is necessary to protect the .unwrap() after the .find_map()
    // method (see below). I think...
    if valid_engines.is_empty() {
        return RVec::new();
    }

    // Returning matches for all valid engines which an empty string alias:
    if valid_engines.iter().all(|engine| engine.alias.is_empty()) {
        let matches: Vec<Match> = valid_engines
            .iter()
            .map(|engine| {
                let stripped_input = stripped_input.strip_prefix(&engine.alias).unwrap().trim();
                let description = format!("Search with {}", &engine.name);
                Match {
                    title: RString::from(stripped_input),
                    description: RSome(RString::from(description)),
                    use_pango: false,
                    icon: RSome(RString::from(engine.icon.as_str())),
                    id: RNone,
                }
            })
            .collect();
        return RVec::from(matches);
    }

    // Returning the match for a specfic engine. If one engine has the alias "a" and another has the
    // alias "ab", the valid engines vector will include both the "a" and the "ab" engine and every engine
    // which has an empty string alias. The question now is, which engine should we pick and create a Match with?
    // Well, my reasoning is that if a user is specifying an engine to search with using an alias, it must be that
    // they only intend to search with that engine. That being said, out of all the valid engines only ONE has to
    // show up as a Match. I do not have any intention of making this more complicated than it is, because of that
    // the below code will return the Match for the first engine that has a non-empty-string alias in the iteration:
    let matches = valid_engines
        .iter()
        .find_map(|engine| {
            if engine.alias.is_empty() {
                return None;
            }
            let stripped_input = stripped_input.strip_prefix(&engine.alias).unwrap().trim();
            let description = format!("Search with {}", &engine.name);
            Some(vec![Match {
                title: RString::from(stripped_input),
                description: RSome(RString::from(description)),
                use_pango: false,
                icon: RSome(RString::from(engine.icon.as_str())),
                id: RNone,
            }])
        })
        // There ain't no way that the .find() method doesn't find a non-empty-string alias, so that's why .unwrap() is here.
        .unwrap();

    RVec::from(matches)

    // OLD CODE GARBAGE SHIT
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

    // Stripping the input again...
    // let stripped_input = stripped_input.strip_prefix(&engine.alias).unwrap();

    // RVec::from(vec![Match {
    //     title: RString::from(stripped_input),
    //     description: RSome(RString::from(format!("Search with {}", engine.name))),
    //     use_pango: false,
    //     icon: RSome(RString::from(engine.icon.as_str())),
    //     id: RNone,
    // }])

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
        common_config,
        browser,
        engines,
    } = data;

    // The .unwrap() here cannot possibly fail because, well, every selection must have some description.
    let selected_engine_name = selection.description.unwrap().replace("Search with ", "");
    // The .unwrap() here cannot fail beacuse the engine vector must hold the element of interest.
    let engine = engines
        .iter()
        .find(|engine| engine.name == selected_engine_name)
        .unwrap();

    let url = &engine.url.replace("{searchTerms}", &selection.title);
    browser
        .new_window(url, common_config.prefix_args())
        .unwrap_or_else(|e| {
            eprintln!("(Websearch) Failed while opening a new browser window. Closing...\n  {e}")
        });

    HandleResult::Close
}
