use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use common::{Bib, Bookmark, Browser};
use serde::Deserialize;
use std::{fs, process};

#[derive(Debug, Deserialize)]
pub struct Config {
    prefix: Option<String>,
    // It has to be usize because the .take() method takes usize...
    max_entries: Option<usize>,
    profile_name: Option<String>,
    bib: Option<Bib>,
}

// QoL methods so I don't have to chain methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("*")
    }
    fn profile_name(&self) -> &str {
        self.profile_name.as_deref().unwrap_or("default")
    }
    fn max_entries(&self) -> usize {
        self.max_entries.unwrap_or(7)
    }
    fn bib(&self) -> &Bib {
        self.bib.as_ref().unwrap_or(&Bib::None)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some("*".to_string()),
            max_entries: Some(7),
            profile_name: Some("default".to_string()),
            bib: Some(Bib::All),
        }
    }
}

// This exists so I don't have to call util::get_default_browser() in get_matches() AND in handle():
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
        eprintln!("Failed while getting default browser in init: {e}");
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
        name: RString::from("Bookmarks"),
        icon: RString::from("user-bookmarks"),
    }
}

#[get_matches]
fn get_matches(input: RString, data: &InitData) -> RVec<Match> {
    let InitData {
        config,
        default_browser,
    } = data;

    // VALIDATING PLUGIN
    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }
    
    // MAIN
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    let bookmarks: Vec<Bookmark> = match default_browser.bookmarks(config.profile_name()) {
        Ok(v) => v
            .into_iter()
            .filter(|bookmark| !bookmark.title().ends_with("- Web"))
            .collect(),
        Err(e) => {
            eprintln!(
                "Failed while getting bookmarks in bookmarks plugin: {e}. Returning no matches..."
            );
            return RVec::new();
        }
    };

    // Handling blank input:
    if stripped_input.is_empty() {
        match config.bib() {
            Bib::All => {
                return RVec::from_iter(bookmarks.into_iter().take(config.max_entries()).map(
                    |bookmark| Match {
                        title: RString::from(bookmark.title()),
                        description: RSome(RString::from(bookmark.url)),
                        use_pango: false,
                        icon: RSome(RString::from("user-bookmarks-symbolic")),
                        id: RNone,
                    },
                ))
            }
            Bib::None => return RVec::new(),
            Bib::Currated(v) => {
                return RVec::from_iter(
                    bookmarks
                        .into_iter()
                        .filter(|bookmark| v.contains(&bookmark.title().to_string()))
                        .take(config.max_entries())
                        .map(|bookmark| Match {
                            title: RString::from(bookmark.title()),
                            description: RSome(RString::from(bookmark.url)),
                            use_pango: false,
                            icon: RSome(RString::from("user-bookmarks-symbolic")),
                            id: RNone,
                        }),
                )
            }
        }
    }

    // Returning a specifc bookmark based on the inputed keyword. I do not know about other browsers but Firefox doesn't allow two bookmarks to have the same keyword, it will automatically remove a keyword from a bookmark if another bookmark uses it. Because of that I don't have to implement checks for overlapping keywords:
    if let Some(valid_bookmark) = bookmarks.iter().find(|bookmark| {
        bookmark
            .keyword
            .as_deref()
            .is_some_and(|keyword| keyword == stripped_input)
    }) {
        return RVec::from(vec![Match {
            title: RString::from(valid_bookmark.title()),
            description: RSome(RString::from(valid_bookmark.url.as_str())),
            use_pango: false,
            icon: RSome(RString::from("user-bookmarks-symbolic")),
            id: RNone,
        }]);
    }

    // Fuzzy matching
    common::fuzzy_match_bookmarks(bookmarks, stripped_input, config.max_entries())
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData {
        config: _,
        default_browser,
    } = data;

    default_browser
        // Description MUST be Some, this is just how I have implemented it, that's why it is safe to .unwrap() here.
        .open(&selection.description.unwrap())
        .unwrap_or_else(|e| eprintln!("Failed while opening URL in browser: {e}"));

    HandleResult::Close
}
