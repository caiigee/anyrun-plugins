# Important! This is a work in progress...
This repository contains some Anyrun plugins I have modified or outright created myself.

As this is my first Rust project, while coding I was also reading the Rust book and prompting Claude in order to learn more about Rust. This means that sometimes my "style" and solutions are inconsistent. I will try to fix any inconsistencies I find but this is definitely not a priority.

When it comes to modified plugin, I think it goes without saying, but I didn't only modify the functionality of the plugins but also the logic, style and solutions. One big change was the config implementation. Every time Anyrun runs it will check for user defined config files in `~/.config/anyrun` and these config files override the default config as expected. However, if you only decide to change some of the config's fields and not all, the parser would fail while parsing that custom config file and the config would fall back to default. This is a problem because it is obvious that sometimes you might change some and not all fields in the config. For that reason I coded every `Config` struct to accept only the `Option` enum. The parser will resolve the `Option` to `None` if it is missing in the plugin config file and the code will make `None` fields fall back to default.

Also, for all plugins I have added a `Bib` (blank input behaviour) field in the config which allows you to choose which matches should be displayed on an empty input. Please make sure to specify `show_results_immediately` as `true` in `~/.config/anyrun/config.ron` so the `Bib` option can work correctly for plugins which have an empty string ("") as their prefix. 

# Shell

I have modified the shell plugin so it executes commands with the `interactive` option enabled. This option loads the `~/.bashrc` file before executing the command, so any aliases that you have defined can be used with Anyrun. I do not know if there are any negative side effects with enabling the `interactive` option.

I have also added an option for a custom icon to be displayed in the match.

Example config with default values:

```
Config (
    prefix: Some("$ "),
    shell: Some($SHELL),
    icon: Some("utilities-terminal"),
)
```

Note that the default value for the shell field is retrieved from the `$SHELL` env variable.

# Applications

Currently I have only implemented executing Desktop Entries and not Desktop Actions. I have to seriously think about how to implement actions in the best way possible and what I have planned right now might require changing some of the source code of Anyrun.

# Browser

Browser isn't actually a plugin, but a group of plugins. All of them require or interact with the browser so I grouped them together into a seperate folder. For browser plugins to work `xdg-utils` needs to be installed (it is required for finding the default browser via `xdg-settings get default-web-browser`)

## Websearch

This plugin will open up the browser and query a search engine. You can define as many engines as you would like and both the plugin itself and the engines have a prefix.

Example config with default values and example engines:
```
Config(
    prefix: Some(""),
    engines: Some([
        Engine(
            name: "Searxng",
            url: "search.hbubli.cc/search?q={}",
            prefix: ""
        ),
        Engine(
            name: "Github",
            url: "github.com/search?q={}",
            prefix: "gh: ",
            icon: Some("github")
        ),
        Engine(
            name: "Arch Linux Packages",
            url: "archlinux.org/packages/?q={}",
            prefix: "alp: ",
            icon: Some("arch")
        ),
        Engine(
            name: "Arch User Repository",
            url: "aur.archlinux.org/packages?O=0&K={}",
            prefix: "aur: ",
            icon: Some("arch")
        ),
        Engine(
            name: "Arch Wiki",
            url: "wiki.archlinux.org/index.php?search={}",
            prefix: "aw: ",
            icon: Some("arch")
        ),
        Engine(
            name: "Youtube",
            url: "www.youtube.com/results?search_query={}",
            prefix: "y: ",
            icon: Some("youtube")
        ),
        Engine(
            name: "Wikipedia",
            url: "en.wikipedia.org/wiki/Special:Search?search={}",
            prefix: "w: ",
            icon: Some("wikipedia")
        ),
        Engine(
            name: "Cargo",
            url: "crates.io/search?q={}",
            prefix: "cg: ",
            icon: Some("cargo")
        ),
    ])
)
```

Note that I have downloaded some icons (pngs) and put them in `~/.local/share/icons` so I can them as custom icons for the search engines.

## Webpages
This plugin will open up a webpage using the browser. Currently, it recognizes domains, localhost with port number, IPv4 addresses and "about:" pages.

The config only includes a prefix field, so that is the only thing you can change.

## Bookmarks
This plugin will open up a webpage that you have saved in your browser profile's bookmarks. The plugin implies that the profile name from which bookmarks are retrieved is called "default", but this can be changed in the config.

Currently only Firefox is supported but this has a big problem. The problem is that the bookmarks load very very slowly when target Firefox profile is already open somewhere. Also, the bookmarks that you create will only show up in Anyrun's matches when you close all instances of the Firefox profile running. For more info about this take a look at TODO 6. in `DEVNOTES.md`.

If you want the plugin to include other browsers and not only Firefox, please code it yourself and create a pull request.

Example config with default values:
```
Config {
    prefix: Some("*"),
    max_entries: Some(7),
    profile_name: Some("default"),
    bib: Some(All),
}
```

## Webapps
