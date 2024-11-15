# PIMPs
1. Add multiple shells for the shell plugin, something similar to websearch, basically have a plugin prefix and then a prefix for each shell.

# TODO
1. A lot of duplicate code man...
2. Include IPv6 addresses for is_valid_page() in browser/util.
3. Implement Frequent for Bib.
4. Find a way to determine the default terminal emulator.
5. Implement Desktop Actions for the applications plugin. This might require changing Anyruns source code. My main idea on how to implement this is using the HandleResult::Reset(bool).
6. Firefox bookmarks 
7. Right now entering the dev shell using `nix develop` BUILDS ALL THE PLUGINS! Now this is fine when the person entering the dev shell:
  1. Knows that their code will compile.
  2. Wants to test the plugins.
However, if the person executing `nix develop` knows that their code won't compile, well then the init of the dev shell will fail. This is so incredibly stupid, not only will faulty code cause the initialization of the dev shell to fail, but also a successful initialization will take a long ass amount of time because the code is compiling. After 2+ hours of trying to seperate the development (rust tools and stuff) logic from the testing logic (declaration of config.ron and the test-anyrun script) in `shell.nix` I just wasn't successful...