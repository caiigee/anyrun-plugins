use ron::ser;
use rusqlite::Connection;
use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    process::Command,
};
use types::Bookmark;

pub mod types;

pub fn get_default_browser() -> Result<Box<dyn Browser>, Box<dyn Error>> {
    let output = Command::new("xdg-settings get default-web-browser")
        .output()
        .map_err(|e| format!("Failed while executing xdg-settings command:\n  {e}"))?;
    let browser_de = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed while converting xdg-settings stdout to a String:\n  {e}"))?;

    match browser_de.trim() {
        "firefox.desktop" => Ok(Box::new(Firefox {
            icon: "firefox".to_string(),
            name: "Firefox".to_string(),
        })),
        _ => Err("Unsupported browser!".into()),
    }
}

// Send + Sync is necessary for defining the "default_browser: Box<dyn Browser>"
// in some code, for example in the bookmarks plugin.
// PIMP 4
pub trait Browser: Send + Sync {
    fn open(&self, url: &str, prefix: &str) -> Result<(), Box<dyn Error>>;
    fn bookmarks(&self, profile_name: &str) -> Result<Vec<Bookmark>, Box<dyn Error>>;
    fn search_engines(&self, profile_name: &str) -> Result<Vec<types::Engine>, Box<dyn Error>>;
    fn icon(&self) -> &str;
    fn name(&self) -> &str;
}

struct Firefox {
    name: String,
    icon: String,
}

// Functions which are only applicable to firefox:
impl Firefox {
    // The reason this function is not in the Browser trait is because I don't know how other browsers handle their bookmarks so I don't know if the same "DatabaseBusy" problem will be an issue:
    fn cached_bookmarks(home_dir: &str) -> Result<Vec<Bookmark>, Box<dyn Error>> {
        let bookmarks_ron = fs::read_to_string(format!(
            "{home_dir}/.cache/anyrun-plugins/firefox-bookmarks.ron"
        ))
        .map_err(|e| format!("Failed while reading cached bookmarks file:\n  {e}"))?;
        let bookmarks = ron::from_str(&bookmarks_ron)
            .map_err(|e| format!("Failed while reading cached bookmarks file:\n  {e}"))?;
        Ok(bookmarks)
    }
    // PIMP 2, 3
    fn is_firefox_running() -> Result<bool, Box<dyn Error>> {
        // Checking if Firefox is running:
        let mut is_firefox_running = false;
        for entry in fs::read_dir("/proc")
            .map_err(|e| format!("Failed while reading /proc directory:\n  {e}"))?
        {
            // Some entries inside /proc are not process directories, meaning that their name
            // isn't a PID. Those entries will cause .parse() to fail which will skip the iteration.
            let pid: u32 = match entry
                .map_err(|e| format!("Failed while unwrapping /proc DirEntry:\n  {e}"))?
                .file_name()
                .to_string_lossy()
                .parse()
            {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            let comm = fs::read_to_string(format!("/proc/{pid}/comm"))
                .map_err(|e| format!("Failed while reading /proc/{pid}/comm file:\n  {e}"))?;
            // The .trim() is necessary for some reason...
            if comm.trim() == "firefox" {
                is_firefox_running = true;
            }
        }
        Ok(is_firefox_running)
    }

    fn get_profile_dir(firefox_path: &str, profile_name: &str) -> Result<String, Box<dyn Error>> {
        // The profile directory may have a name like "<random_characters>.<name>"
        // or just "<name>". Because it cannot be inferred from the profile name alone,
        // a .find() method is required to find the target dir:
        let profile_dir = fs::read_dir(firefox_path)
            .map_err(|e| format!("Failed while reading firefox directory:\n  {e}"))?
            // At this point we have an Iterator over Result<DirEntry, Error> and
            // we want the DirEntry which corresponds to the profile directory:
            .find_map(|entry| match entry {
                Ok(dir_entry) => {
                    if dir_entry
                        .file_name()
                        .to_string_lossy()
                        .contains(profile_name)
                    {
                        Some(dir_entry)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    eprintln!("Failed while unwrapping entry of {firefox_path}:\n  {e}");
                    None
                }
            })
            // At this point we either have the DirEntry we need or we don't. We have to
            // return from the function with an Err if we didn't find the directory:
            .ok_or_else(|| {
                format!(
                    "Cannot find the profile directory, please make sure that \
                    the profile with name {profile_name} exists!"
                )
            })?
            // In this part of the code we are converting DirEntry to String:
            .file_name()
            .to_string_lossy()
            .into_owned();

        Ok(profile_dir)
    }
}

impl Browser for Firefox {
    fn open(&self, url: &str, prefix: &str) -> Result<(), Box<dyn Error>> {
        Command::new(format!("{prefix}firefox --new-window {url}")).spawn()?;
        Ok(())
    }

    fn bookmarks(&self, profile_name: &str) -> Result<Vec<Bookmark>, Box<dyn Error>> {
        // GETTING PROFILE PATH
        let home_dir = env::var("HOME").map_err(|e| {
            format!("HOME env variable not set! Bookmarks cannot be obtained:\n  {e}")
        })?;

        let firefox_path = format!("{home_dir}/.mozilla/firefox");

        // PROFILE RUNNING CHECK
        // Early return for when the firefox profile is already running:
        if Firefox::is_firefox_running()? {
            return Firefox::cached_bookmarks(&home_dir);
        }

        // MAIN
        // Creating the connection:
        let profile_dir = Firefox::get_profile_dir(&firefox_path, profile_name)
            .map_err(|e| format!("Failed while getting Firefox profile directory:\n  {e}"))?;
        let conn = Connection::open(format!("{firefox_path}/{profile_dir}/places.sqlite"))
            .map_err(|e| format!("Failed while creating the DB connection:\n  {e}"))?;

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
            .map_err(|e| format!("Failed while preparing SQL query:\n  {e}"))?;

        // Getting the bookmark data into an iterator:
        let bookmarks = statement
            .query_map([], |row| {
                Ok(Bookmark::new(row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("Failed while getting the bookmark iterator:\n  {e}"))?
            .filter_map(|r| match r {
                Ok(bookmark) => Some(bookmark),
                Err(e) => {
                    eprintln!("Error while unwrapping bookmark: {e}.");
                    None
                }
            })
            .collect();

        // Caching the bookmarks:
        let bookmarks_ron = ser::to_string_pretty(&bookmarks, ser::PrettyConfig::default())
            .map_err(|e| format!("Failed to parse bookmarks to a RON formatted string:\n  {e}"))?;
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

    fn search_engines(&self, profile_path: &str) -> Result<Vec<types::Engine>, Box<dyn Error>> {
        // PROCESSING THE mozlz4 FILE
        let mut file = File::open(format!("{profile_path}/search.json.mozlz4"))
            .map_err(|e| format!("Failed while opening mozlz4 file:\n  {e}"))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed while reading mozlz4 file:\n  {e}"))?;

        // Skipping the 8-byte header and decompressing.
        let decompressed = lz4_flex::decompress(&buffer[8..], 1024 * 1024)
            .map_err(|e| format!("Failed while decompressing mozlz4 file:\n  {e}"))?;

        let json_str = String::from_utf8(decompressed)
            .map_err(|e| format!("Failed while creating String out of mozlz4 bytes:\n  {e}"))?;
        let search_data: types::SearchData = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed while deserializing JSON to search_data:\n  {e}"))?;
        let default_engine_index = search_data
            .engines()
            .iter()
            .position(|engine| engine._name() == search_data.default_engine())
            .ok_or_else(|| "Could not find the default engine index from browser's search data!")?;

        // Cloning the Vec<Engine> and placing the default_engine at the start:
        let mut search_engines = search_data.engines().to_vec();
        search_engines.swap(0, default_engine_index);

        Ok(search_engines)
    }

    fn icon(&self) -> &str {
        &self.icon
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Utility function for extracting the domain from a URL:
// fn extract_domain(url: &str) -> Result<String, Box<dyn Error>> {
//     let domain_re = Regex::new(r"^https?://([^/]+)")
//         .map_err(|e| format!("Failed while creating domain regex:\n  {e}"))?;
//     // Getting the captures:
//     let captures = domain_re
//         .captures(&url)
//         .ok_or_else(|| "Failed while getting domain regex captures!")?;
//     // Getting the capture group 1:
//     let domain = captures
//         .get(1)
//         .ok_or_else(|| "Failed while getting domain regex first capture group!")?;

//     // SUCCESS
//     Ok(domain.as_str().to_string())
// }
