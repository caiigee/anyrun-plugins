use abi_stable::std_types::{
    ROption::{RNone, RSome},
    RString, RVec,
};
use anyrun_plugin::*;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use std::{error::Error, process};

mod firefox;

#[derive(Debug, Serialize, Deserialize)]
struct Bookmark {
    title: String,
    url: String,
    keyword: String,
}

// impl Bookmark {
//     pub fn new(title: &str, url: &str, keyword: &str) -> Self {
//         Bookmark {
//             title: title.to_string(),
//             url: url.to_string(),
//             keyword: keyword.to_string(),
//         }
//     }
// }

trait Bookmarks: common::Browser {
    // I suspect every single browser would need the profile name as a parameter...
    fn bookmarks(&self) -> Result<Vec<Bookmark>, Box<dyn Error>>;
}

#[derive(Debug, Deserialize)]
pub struct Config {
    prefix: Option<String>,
    // It has to be usize because the .take() method takes usize...
    max_entries: Option<usize>,
    bib: Option<common::Bib>,
}

// QoL methods so I don't have to chain methods:
impl Config {
    fn prefix(&self) -> &str {
        self.prefix.as_deref().unwrap_or("*")
    }
    fn max_entries(&self) -> usize {
        self.max_entries.unwrap_or(7)
    }
    fn bib(&self) -> &common::Bib {
        self.bib.as_ref().unwrap_or(&common::Bib::None)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: Some("*".to_string()),
            max_entries: Some(7),
            bib: Some(common::Bib::All),
        }
    }
}

// This exists so I don't have to call util::get_default_browser() in get_matches() AND in handle():
struct InitData {
    config: Config,
    common_config: common::CommonConfig,
    browser: Box<dyn Bookmarks>,
    bookmarks: Vec<Bookmark>,
}

#[init]
fn init(config_dir: RString) -> InitData {
    let config = common::config(&config_dir, "Bookmarks");
    let common_config = common::common_config(&config_dir, "Bookmarks");

    let browser_id = common::default_browser_id().unwrap_or_else(|e| {
        eprintln!("Failed while getting default browser in init for bookmarks. Closing...:\n  {e}");
        process::exit(1);
    });
    let browser = match browser_id.as_str() {
        "firefox" => Box::new(common::Firefox::new(common_config.browser_profile_name())),
        _ => {
            eprintln!("(Bookmarks) Unsupported default browser! Closing...");
            process::exit(1)
        }
    };
    let bookmarks = browser.bookmarks().unwrap_or_else(|e| {
        eprintln!("(Bookmarks) Failed while getting bookmarks. Closing...\n  {e}");
        process::exit(1);
    });

    InitData {
        config,
        common_config,
        browser,
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
        common_config: _,
        browser: _,
        bookmarks,
    } = data;

    // Early return if a keyword matches:
    if let Some(bookmark) = bookmarks
        .iter()
        .find(|bookmark| !bookmark.keyword.is_empty() && &input == &bookmark.keyword)
    {
        return RVec::from(vec![Match {
            title: RString::from(bookmark.title.as_str()),
            description: RSome(RString::from(bookmark.url.as_str())),
            use_pango: false,
            icon: RSome(RString::from("user-bookmarks-symbolic")),
            id: RNone,
        }]);
    }

    // Early return for the wrong prefix:
    if !input.starts_with(config.prefix()) {
        return RVec::new();
    }

    let stripped_input = input.strip_prefix(config.prefix()).unwrap().trim();

    // Handling blank input:
    if stripped_input.is_empty() {
        match config.bib() {
            common::Bib::All => {
                return RVec::from_iter(bookmarks.into_iter().take(config.max_entries()).map(
                    |bookmark| Match {
                        title: RString::from(bookmark.title.as_str()),
                        description: RSome(RString::from(bookmark.url.as_str())),
                        use_pango: false,
                        icon: RSome(RString::from("user-bookmarks-symbolic")),
                        id: RNone,
                    },
                ))
            }
            common::Bib::None => return RVec::new(),
            common::Bib::Currated(v) => {
                return RVec::from_iter(
                    bookmarks
                        .into_iter()
                        .filter(|bookmark| v.contains(&bookmark.title))
                        .take(config.max_entries())
                        .map(|bookmark| Match {
                            title: RString::from(bookmark.title.as_str()),
                            description: RSome(RString::from(bookmark.url.as_str())),
                            use_pango: false,
                            icon: RSome(RString::from("user-bookmarks-symbolic")),
                            id: RNone,
                        }),
                )
            }
        }
    }

    // Fuzzy matching
    let matcher = SkimMatcherV2::default();
    // Shadowing "bookmarks"; performing fuzzy matching:
    let mut bookmarks: Vec<(i64, &Bookmark)> = bookmarks
        .into_iter()
        .filter_map(|bookmark| {
            let score = matcher.fuzzy_match(&bookmark.title, stripped_input)?;
            Some((score, bookmark))
        })
        .collect();
    // Sorting bookmarks by score in descending order.
    bookmarks.sort_by(|a, b| b.0.cmp(&a.0));

    // SUCCESS
    RVec::from_iter(
        bookmarks
            .iter()
            .take(config.max_entries())
            .map(|(_, bookmark)| Match {
                title: RString::from(bookmark.title.as_str()),
                description: RSome(RString::from(bookmark.url.as_str())),
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
        common_config,
        browser,
        bookmarks: _,
    } = data;

    // Description MUST be Some, this is just how I have implemented it, that's why it is safe to .unwrap() here.
    let url = &selection.description.unwrap();
    browser
        .new_window(url, common_config.prefix_args())
        .unwrap_or_else(|e| eprintln!("(Webpages) Failed while opening URL! Closing...\n  {e}"));

    HandleResult::Close
}
