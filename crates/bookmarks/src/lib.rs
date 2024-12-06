use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use common::types::{Bib, Bookmark, BrowserConfig};
use common::Browser;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::Deserialize;
use std::{fs, process};

#[derive(Debug, Deserialize)]
pub struct Config {
    prefix: Option<String>,
    // It has to be usize because the .take() method takes usize...
    max_entries: Option<usize>,
    bib: Option<Bib>,
}

// QoL methods so I don't have to chain methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("*")
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
            bib: Some(Bib::All),
        }
    }
}

// This exists so I don't have to call util::get_default_browser() in get_matches() AND in handle():
struct InitData {
    config: Config,
    browser_config: BrowserConfig,
    default_browser: Box<dyn Browser>,
    bookmarks: Vec<Bookmark>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = match fs::read_to_string(format!("{config_dir}/bookmarks.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "(Bookmarks) Failed while parsing config file. Falling back to default...:\n  {e}"
            );
            Config::default()
        }),
        Err(e) => {
            eprintln!(
                "(Bookmarks) Failed while reading config file. Falling back to default...:\n  {e}"
            );
            Config::default()
        }
    };

    let browser_config = match fs::read_to_string(format!("{config_dir}/browser.ron")) {
        Ok(v) => ron::from_str(&v).unwrap_or_else(|e| {
            eprintln!(
                "(Bookmarks) Failed while parsing browser config file. Falling back to default...:\n  {e}"
            );
            BrowserConfig::default()
        }),
        Err(e) => {
            eprintln!(
                "(Bookmarks) Failed while reading browser config file. Falling back to default...:\n  {e}"
            );
            BrowserConfig::default()
        }
    };

    let default_browser = common::get_default_browser().unwrap_or_else(|e| {
        eprintln!("Failed while getting default browser in init for bookmarks. Closing...:\n  {e}");
        process::exit(1);
    });

    let bookmarks = default_browser
        .bookmarks(browser_config.profile_name())
        .unwrap_or_else(|e| {
            eprintln!("(Bookmarks) Failed while getting bookmarks. Closing...\n  {e}");
            process::exit(1);
        });

    InitData {
        config,
        browser_config,
        default_browser,
        bookmarks,
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
        browser_config: _,
        default_browser: _,
        bookmarks,
    } = data;

    // VALIDATING PLUGIN
    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    // MAIN
    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Handling blank input:
    if stripped_input.is_empty() {
        match config.bib() {
            Bib::All => {
                return RVec::from_iter(bookmarks.into_iter().take(config.max_entries()).map(
                    |bookmark| Match {
                        title: RString::from(bookmark.title()),
                        description: RSome(RString::from(bookmark.url())),
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
                            description: RSome(RString::from(bookmark.url())),
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
            .keyword()
            .as_deref()
            .is_some_and(|keyword| keyword == stripped_input)
    }) {
        return RVec::from(vec![Match {
            title: RString::from(valid_bookmark.title()),
            description: RSome(RString::from(valid_bookmark.url())),
            use_pango: false,
            icon: RSome(RString::from("user-bookmarks-symbolic")),
            id: RNone,
        }]);
    }

    // Fuzzy matching
    let matcher = SkimMatcherV2::default();
    // Shadowing "bookmarks"; performing fuzzy matching:
    let mut bookmarks: Vec<(i64, &Bookmark)> = bookmarks
        .into_iter()
        .filter_map(|bookmark| {
            let score = matcher.fuzzy_match(&bookmark.title(), stripped_input)?;
            Some((score, bookmark))
        })
        .collect();
    // Sorting bookmarks by score in descending order.
    bookmarks.sort_by(|a, b| b.0.cmp(&a.0));

    // SUCCESS
    RVec::from_iter(
        bookmarks
            .into_iter()
            .take(config.max_entries())
            .map(|(_, bookmark)| Match {
                title: RString::from(bookmark.title()),
                description: RSome(RString::from(bookmark.url())),
                use_pango: false,
                icon: RSome(RString::from("user-bookmarks-symbolic")),
                id: RNone,
            }),
    )
}

#[handler]
fn handler(selection: Match, data: &InitData) -> HandleResult {
    let InitData {
        config: _,
        browser_config,
        default_browser,
        bookmarks: _,
    } = data;

    default_browser
        // Description MUST be Some, this is just how I have implemented it, that's why it is safe to .unwrap() here.
        .open(&selection.description.unwrap(), browser_config.command_prefix())
        .unwrap_or_else(|e| eprintln!("Failed while opening URL in browser: {e}"));

    HandleResult::Close
}
