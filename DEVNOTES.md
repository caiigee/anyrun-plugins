# PIMPs
2. We can use `SiteSecurityServiceState.bin` as a way to determine whether some profile is running, because that file only exists then.
3. Maybe implement this function for non-NixOS systems using `ps` and `lsof`.
4. Implement a function which returns the desktop entry or something like that and then handle the icon and the opening with the DE.
5. add a browser.ron config which loads stuff like the default browser, profile name and command prefix.
6. instead of using .cache to save bookmarks why not just get bookmarks from bookmarksbackup directory

# TODO
1. A lot of duplicate code man...
2. Include IPv6 addresses for is_valid_page() in browser/util.
3. Implement Frequent for Bib.
4. Find a way to determine the default terminal emulator.
5. Implement Desktop Actions for the applications plugin. This might require changing Anyruns source code. My main idea on how to implement this is using the HandleResult::Reset(bool).
6. Firefox bookmarks
7. refactor everything based on the rule of two for common.
8. change the default browser function to return a desktop entry. the desktop entry is going to be created from the path of the default browser DE. you get the path by looping through XDG_DATA_DIRS and checking if firefox.desktop exists somehwere in the applications directory.

# NOTES
1. The reason I created `browser_id` and `browser` is because I want to keep all the code for search engines or bookmarks inside the individual crates. Here's the thing... If the `Browser` trait in the `common` crate has the search_engines() and the bookmarks() functions it would mean that those functions have to be implemented in the `common` crate and all other crates will therefore have access to those functions which is just stupid. Having bookmarks related functions inside the bookmarks crate and likewise for search engines and whatever else comes along just makes the most amount of sense. Again, the problem with that is that a common `default_browser()` function is impossible because how would that function know what to return if the `Bookmarks` and `SearchEngines` traits are defined in their individual crates. The point of this whole thing is to just solve the problem of which crate gets access to which functions and where those functions, logic and types are defined.