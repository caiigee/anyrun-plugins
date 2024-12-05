use std::fs::{self};
use std::{borrow::Cow, collections::HashMap, env, error::Error};
use freedesktop_desktop_entry::DesktopEntry;

pub fn scrape_desktop_entries<'a>() -> Result<Vec<DesktopEntry<'a>>, Box<dyn Error>> {
    // Getting the XDG_DATA_HOME or defaulting to "~/.local/share/applications" or if that fails user specific desktop entries won't be used:
    let user_apps_path = match env::var("XDG_DATA_HOME") {
        Ok(v) => format!("{v}/applications"),
        Err(e) => {
            eprintln!("Failed while getting XDG_DATA_HOME env variable: {e}. Using '~/.local/share/applications' as 'user_apps_path'...");
            env::var("HOME")
                .map(|v| format!("{v}/.local/share/applications"))
                .unwrap_or("".to_string())
        }
    };

    // Getting the system Desktop Entries. Our goal in this function is to get full paths of every ".desktop" file
    // and then use "DesktopEntry::from_path()" to parse the paths to Desktop Entries:
    let sys_desktop_entries: Vec<DesktopEntry> = match env::var("XDG_DATA_DIRS") {
        Ok(output) => output
            // XDG_DATA_DIRS outputs a long string with paths which are delimited by a ":",
            // so we have to split the string to get every dirpath individually.
            .split(":")
            // Reading the directories. If any error happens, it doesn't matter we will just skip 
            // the directory. We expect to get an iterator over ReadDir types from this function:
            .filter_map(|dirpath| {
                // Only "/applications" directories can have Desktop Entries, so we have to read those directories:
                fs::read_dir(format!("{dirpath}/applications"))
                    .inspect_err(|e| {
                        eprintln!("Failed while reading the directory {dirpath}: {e}. Skipping this directory...");
                    })
                    .ok()
            })
            // So, at this point we have an iterator of ReadDir types, but ReadDir types are actually iterators of
            // Result<DirEntry, Error> themselves! This means that if we want to have an iterator of DirEntry
            // types we have to use ".flat_map()". Also, like before, if anything goes wrong here we don't care,
            // we will just skip that DirEntry:
            .flat_map(|read_dir| {
                read_dir.filter_map(|r| {
                    r.inspect_err(|e| {
                        eprintln!("Failed to unwrap Result<DirEntry, Error>: {e}. Skipping this DirEntry...")
                    })
                    .ok()
                })
            })
            // Now that we finally have an iterator over DirEntry types we will first filter
            // out any filesystem entry which is not a ".desktop" file:
            .filter(|direntry| direntry.path().extension().is_some_and(|v| v == "desktop"))
            // At this point we only have DirEntries with the correct path, meaning we can safely parse
            // it to a Desktop Entry. However, if anything goes wrong it is not the end of the world, we will just:
            .filter_map(|direntry| {
                DesktopEntry::from_path(direntry.path(), None::<&[&str]>)
                    .inspect_err(|e| {
                        eprintln!("Failed while parsing Desktop Entry from {:?}: {e}. Skipping the file...", direntry.path())
                    })
                    .ok()
            })
            .collect(),
        Err(e) => {
            eprintln!("Failed while getting XDG_DATA_DIRS: {e}. Trying to find Desktop Entries in '/usr/share/applications' now...");

            // "::read_dir()" can fail and if it does I decided it would be best to return from the root function.
            // That is why I use ".map_err()" and the "?" operator. I guess I could have let any error here slide,
            // but then all Desktop Entries would have to be parsed from user specific ".desktop" files, which is
            // definitely not ideal, so the better solution is to return from the function IMO:
            fs::read_dir("/usr/share/applications")
                .map_err(|e| format!("Failed while reading the '/usr/share/applications': {e}"))?
                .filter_map(|r| {
                    r.inspect_err(|e| eprintln!("Failed to unwrap Result<DirEntry, Error>: {e}")).ok()
                })
                .filter(|direntry| direntry.path().extension().is_some_and(|v| v == "desktop"))
                .filter_map(|direntry| {
                    let path = &direntry.path();
                    DesktopEntry::from_path(direntry.path(), None::<&[&str]>)
                        .inspect_err(|e|
                            eprintln!("Failed while parsing Desktop Entry from {path:?}: {e}. Skipping the file...")
                        )
                        .ok()
                })
                .collect()
        }
    };

    // Getting the user's directory entries (filesystem entries) which can be the Desktop Entries (.desktop files) we need:
    let user_desktop_entries: Vec<DesktopEntry> = fs::read_dir(&user_apps_path)
        .map(|read_dir| {
            read_dir
                .filter_map(|r| {
                    r.inspect_err(|e| eprintln!("Failed to unwrap Result<DirEntry, Error>: {e}. Skipping this DirEntry..."))
                    .ok()
                })
                .filter(|direntry| {
                    direntry
                        .path()
                        .extension()
                        .is_some_and(|v| v == "desktop")
                })
                .filter_map(|direntry| {
                    DesktopEntry::from_path(direntry.path(), None::<&[&str]>)
                        .inspect_err(|e| {
                            eprintln!(
                                "Failed while parsing Desktop Entry from {:?}: {e}. Skipping the file...",
                                direntry.path()
                            )
                        })
                        .ok()
                })
                .collect()
        })
        .map_err(|e| format!("Failed to read directory: {e}"))
        .unwrap_or_default();
    
    // Merging system and user Desktop Entries:
    let mut entries = merge_desktop_entries(
        sys_desktop_entries,
        user_desktop_entries,
    );
    // Removing the ones with NoDisplay set to true.
    entries = entries.into_iter().filter(|entry| !entry.no_display()).collect();
    
    // SUCCESS
    Ok(entries)
}

// Merges two vectors of DesktopEntry, preferring entries from the second vector if there's a collision on appid:
fn merge_desktop_entries<'a>(
    vec1: Vec<DesktopEntry<'a>>,
    vec2: Vec<DesktopEntry<'a>>,
) -> Vec<DesktopEntry<'a>> {
    // Create a HashMap with the appid as the key and the DesktopEntry as the value.
    let mut merged_map: HashMap<Cow<'a, str>, DesktopEntry<'a>> = HashMap::new();

    // Insert all entries from vec1 into the map:
    for entry in vec1 {
        merged_map.insert(entry.appid.clone(), entry);
    }

    // Insert all entries from vec2 into the map, overwriting any existing entries with the same appid:
    for entry in vec2 {
        merged_map.insert(entry.appid.clone(), entry);
    }

    // Collect the map values into a vector.
    merged_map.into_values().collect()
}

// #[cfg(test)]
// mod tests {
//     use super::*;

// }
