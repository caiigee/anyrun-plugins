# PIMPs
1. Add multiple shells for the shell plugin, something similar to websearch, basically have a plugin prefix and then a prefix for each shell.
2. We can use `SiteSecurityServiceState.bin` as a way to determine whether some profile is running, because that file only exists then.
3. Maybe implement this function for non-NixOS systems using `ps` and `lsof`.
4. Implement a function which returns the desktop entry or something like that and then handle the icon and the opening with the DE.
5. add a browser.ron config which loads stuff like the default browser, profile name and command prefix.

# TODO
1. A lot of duplicate code man...
2. Include IPv6 addresses for is_valid_page() in browser/util.
3. Implement Frequent for Bib.
4. Find a way to determine the default terminal emulator.
5. Implement Desktop Actions for the applications plugin. This might require changing Anyruns source code. My main idea on how to implement this is using the HandleResult::Reset(bool).
6. Firefox bookmarks
7. refactor everything based on the rule of two for common.