use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{self, Write},
};

use rusqlite::Connection;

use crate::{Bookmark, Bookmarks};

// PIMP 2, 3
fn is_firefox_running() -> Result<bool, Box<dyn Error>> {
    // Checking if Firefox is running:
    let mut is_firefox_running = false;
    for entry in fs::read_dir("/proc")
        .map_err(|e| format!("Failed while reading /proc directory:\n    {e}"))?
    {
        // Some entries inside /proc are not process directories, meaning that their name
        // isn't a PID. Those entries will cause .parse() to fail which will skip the iteration.
        let pid: u32 = match entry
            .map_err(|e| format!("Failed while unwrapping /proc DirEntry:\n    {e}"))?
            .file_name()
            .to_string_lossy()
            .parse()
        {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        let comm = match fs::read_to_string(format!("/proc/{pid}/comm")) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error while reading /proc/{pid}/comm file:\n  {e}");
                continue;
            }
        };
        
        // The .trim() is necessary for some reason...
        if comm.trim().contains("firefox") {
            is_firefox_running = true;
        }
    }
    Ok(is_firefox_running)
}

impl Bookmarks for common::Firefox {
    fn bookmarks(&self) -> Result<Vec<Bookmark>, Box<dyn Error>> {
        // Early return for when the firefox profile is already running:
        let home_dir = env::var("HOME")
            .map_err(|e| format!("Failed while getting HOME env variable:\n    {e}"))?;
        
        if is_firefox_running()? {
            let bookmarks_ron = fs::read_to_string(format!(
                "{home_dir}/.cache/anyrun-plugins/firefox-bookmarks.ron"
            ))
            .map_err(|e| format!("Failed while reading cached bookmarks file:\n    {e}"))?;
            
            let bookmarks = ron::from_str(&bookmarks_ron)
                .map_err(|e| format!("Failed while reading cached bookmarks file:\n    {e}"))?;

            return Ok(bookmarks);
        }

        // MAIN
        // Creating the connection:
        let profile_dir = common::Firefox::profile_dir2(&self)
            .map_err(|e| format!("Failed while getting Firefox profile directory:\n    {e}"))?;
        let conn = Connection::open(format!("{profile_dir}/places.sqlite"))
            .map_err(|e| format!("Failed while creating the DB connection:\n    {e}"))?;

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
            .map_err(|e| format!("Failed while preparing SQL query:\n    {e}"))?;

        // Getting the bookmark data into an iterator:
        let bookmarks = statement
            .query_map([], |row| {
                let title = row.get(0).unwrap_or_else(|e| {
                    eprintln!(
                        "Failed while getting the bookmark's title. \
                    Using empty string...\n    {e}"
                    );
                    String::new()
                });
                let url = row.get(1).unwrap_or_else(|e| {
                    eprintln!(
                        "Failed while getting the bookmark's url. \
                    Using empty string...\n    {e}"
                    );
                    String::new()
                });
                let keyword = row.get(2).unwrap_or_else(|e| {
                    eprintln!(
                        "Failed while getting the bookmark's keyword. \
                    Using empty string...\n    {e}"
                    );
                    String::new()
                });
                Ok(Bookmark {
                    title,
                    url,
                    keyword,
                })
            })
            .map_err(|e| format!("Failed while getting the bookmark iterator:\n    {e}"))?
            .filter_map(|r| match r {
                Ok(bookmark) => Some(bookmark),
                Err(e) => {
                    eprintln!(
                        "Failed while unwrapping bookmark. Skipping this bookmark...\n    {e}"
                    );
                    None
                }
            })
            .collect();

        // Caching the bookmarks:
        let bookmarks_ron =
            ron::ser::to_string_pretty(&bookmarks, ron::ser::PrettyConfig::default()).map_err(
                |e| format!("Failed to parse bookmarks to a RON formatted string:\n    {e}"),
            )?;
        let cache_dirpath = format!("{home_dir}/.cache/anyrun-plugins");
        match fs::create_dir(&cache_dirpath) {
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        };
        let mut file = File::create(format!("{cache_dirpath}/firefox-bookmarks.ron"))?;
        file.write_all(bookmarks_ron.as_bytes())?;

        // Success.
        Ok(bookmarks)
    }
}
