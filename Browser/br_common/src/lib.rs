use abi_stable::std_types::{ROption::{RNone, RSome}, RString, RVec};
use anyrun_plugin::Match;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use regex::Regex;
use ron::ser;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, File},
    io::{self, Write},
    process::Command,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Bookmark {
    title: Option<String>,
    pub url: String,
    pub keyword: Option<String>,
    domain: String,
}

impl Bookmark {
    // QoL method so I don't have to chain methods every time I want to access the title.
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.domain)
    }
}

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

pub fn get_default_browser() -> Result<Box<dyn Browser>, Box<dyn Error>> {
    let output = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()?;
    let browser_de = String::from_utf8(output.stdout)?;

    match browser_de.trim() {
        "firefox.desktop" => Ok(Box::new(Firefox {
            icon: "firefox".to_string(),
        })),
        _ => Err("Unsupported browser!".into()),
    }
}

pub trait Browser: Send + Sync {
    fn open(&self, url: &str) -> Result<(), Box<dyn Error>>;
    fn bookmarks(&self, profile_name: &str) -> Result<Vec<Bookmark>, Box<dyn Error>>;
    fn icon(&self) -> &str;
}

struct Firefox {
    icon: String,
}
// Functions which are only applicable to firefox:
impl Firefox {
    // The reason this function is not in the Browser trait is because I don't know how other browsers handle their bookmarks so I don't know if the same "DatabaseBusy" problem will be an issue:
    fn cached_bookmarks() -> Result<Vec<Bookmark>, Box<dyn Error>> {
        let home_dir = env!("HOME");
        let bookmarks_ron = fs::read_to_string(format!(
            "{home_dir}/.cache/anyrun-plugins/firefox-bookmarks.ron"
        ))?;
        let bookmarks = ron::from_str(&bookmarks_ron)?;
        Ok(bookmarks)
    }
    fn is_profile_running(profile_dir: &str) -> Result<bool, Box<dyn Error>> {
        // Construct the profile path:
        let home_dir = env!("HOME");
        let profile_path = format!("{home_dir}/.mozilla/firefox/{profile_dir}");

        // Get all Firefox process IDs:
        let ps_output = Command::new("ps")
            .args(&["-C", "firefox", "-o", "pid="])
            .output()?;
        let pids: Vec<String> = String::from_utf8(ps_output.stdout)?
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .collect();

        // Check each Firefox process' open files:
        for pid in pids {
            let lsof_output = Command::new("lsof").args(&["-p", &pid, "-Fn"]).output()?;
            // Shadowing lsof_output
            let lsof_output = String::from_utf8(lsof_output.stdout)?;

            if lsof_output.contains(&profile_path) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl Browser for Firefox {
    fn open(&self, url: &str) -> Result<(), Box<dyn Error>> {
        Command::new("firefox")
            .args(["--new-window", url])
            .spawn()?;
        Ok(())
    }

    fn bookmarks(&self, profile_name: &str) -> Result<Vec<Bookmark>, Box<dyn Error>> {
        // PROFILE DIRECTORY
        let home_dir = env!("HOME");
        let firefox_path = format!("{home_dir}/.mozilla/firefox");

        // Getting the profile name:
        let Some(profile_dir) = fs::read_dir(&firefox_path)
            .map_err(|e| format!("Failed while reading firefox directory: {e:?}"))?
            .find(|r| match r {
                Ok(entry) => entry.file_name().to_string_lossy().contains(profile_name),
                Err(e) => {
                    eprintln!("Failed while iterating over firefox dir entries: {e:?}");
                    false
                }
            })
        else {
            return Err("Failed while getting the default folder name in '~/.mozilla/firefox'. Please make sure the profile that you use is called 'default'.".into());
        };
        // Shadowing profile directory to a String:
        let profile_dir = profile_dir
            .map_err(|e| format!("Failed while unwrapping profile_dir: {e:?}"))?
            .file_name()
            .into_string()
            .map_err(|_| "Failed while converting OsString to String")?;
        
        // PROFILE RUNNING CHECK
        // Early return for when the firefox profile is already running:
        if Firefox::is_profile_running(&profile_dir)? {
            return Firefox::cached_bookmarks();
        }

        // MAIN
        // Creating the connection:
        let conn = Connection::open(format!("{firefox_path}/{profile_dir}/places.sqlite"))
            .map_err(|e| format!("Failed while creating the DB connection: {e:?}"))?;

        // Creating the SQL query:
        let mut statement = conn
            .prepare(
                "SELECT
            mb.title,
            mp.url,
            mk.keyword
        FROM
            moz_places mp
            LEFT JOIN moz_bookmarks mb ON mp.id = mb.fk
            LEFT JOIN moz_keywords mk ON mp.id = mk.place_id
        WHERE
            mb.type = 1; -- Only select bookmarks (type 1)",
            )
            .map_err(|e| format!("Failed while preparing SQL query: {e:?}"))?;

        // Regex for extracting the complete domain out of a URL.
        let domain_re = Regex::new(r"^https?://([^/]+)")?;

        // Getting the bookmark data into an iterator:
        let bookmark_iter = statement
            .query_map([], |row| {
                Ok(Bookmark {
                    title: row.get(0)?,
                    url: row.get(1)?,
                    keyword: row.get(2)?,
                    // Extracting the domain from the URL will be handled later.
                    domain: String::new(),
                })
            })
            .map_err(|e| format!("Failed while getting the bookmark iterator: {e:?}"))?;

        // Converting the iterator to a vector. It has to be done like this because the elements in the iterator are of type Result<T, E>. Also here we extract the domains from the URLs:
        let mut bookmarks = Vec::new();
        for bookmark in bookmark_iter {
            let mut bookmark = bookmark?;
            bookmark.domain = extract_domain(&bookmark.url, &domain_re)?;
            bookmarks.push(bookmark);
        }

        // Caching the bookmarks:
        let bookmarks_ron = ser::to_string_pretty(&bookmarks, ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to parse bookmarks to a RON formatted string: {e:?}"))?;
        let cache_dirpath = format!("{home_dir}/.cache/anyrun-plugins");
        match fs::create_dir(&cache_dirpath) {
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        };
        let mut file = File::create(format!("{cache_dirpath}/firefox-bookmarks.ron"))?;
        file.write_all(bookmarks_ron.as_bytes())?;

        // SUCCESS
        Ok(bookmarks)
    }

    fn icon(&self) -> &str {
        &self.icon
    }
}

// Utility function for extracting the domain from a URL:
fn extract_domain(url: &str, domain_re: &Regex) -> Result<String, String> {
    // GETTING THE CAPTURE GROUP
    let Some(captures) = domain_re.captures(&url) else {
        return Err("Failed while getting domain regex captures!".to_string());
    };
    // Getting the capture group 1:
    let Some(domain) = captures.get(1) else {
        return Err("Failed while getting domain regex first capture group!".to_string());
    };

    // SUCCESS
    Ok(domain.as_str().to_string())
}

pub fn fuzzy_match_bookmarks(
    bookmarks: Vec<Bookmark>,
    stripped_input: &str,
    max_entries: usize,
) -> RVec<Match> {
    let matcher = SkimMatcherV2::default();
    // Shadowing "bookmarks"; performing fuzzy matching:
    let mut bookmarks: Vec<(i64, Bookmark)> = bookmarks
        .into_iter()
        .filter_map(|bookmark| {
            let score = matcher.fuzzy_match(bookmark.title(), stripped_input)?;
            Some((score, bookmark))
        })
        .collect();
    // Sorting bookmarks by score in descending order.
    bookmarks.sort_by(|a, b| b.0.cmp(&a.0));

    // SUCCESS
    RVec::from_iter(
        bookmarks
            .into_iter()
            .take(max_entries)
            .map(|(_, bookmark)| Match {
                title: RString::from(bookmark.title()),
                description: RSome(RString::from(bookmark.url)),
                use_pango: false,
                icon: RSome(RString::from("user-bookmarks-symbolic")),
                id: RNone,
            }),
    )
}
