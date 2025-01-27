use regex::Regex;

/// Known binary file extensions that should be skipped
#[rustfmt::skip]
pub const BINARY_FILE_EXTENSIONS: &[&str] = &[
    // Executables, Libraries, Core Dumps
    "exe", "dll", "so", "dylib", "ocx", "ax", "drv", "sys", "msi", "app", "ipa", "apk",
    "bin", "out", "a", "lib", "ko", "elf", "o", "nro", "core", "img", "iso",

    // Java / .NET / Archives
    "class", "jar", "war", "ear",
    "nupkg", // NuGet package
    
    // Archives & Compressed
    "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "lz4", "lz", "zst", "lzma",
    "cab", "ar", "cpio", "rpm", "deb", "pkg", "crx", "dmg", "hfs",
    "cso", // Compressed ISO
    "bz", "tbz", "tbz2", "tlz", "txz", "z", "Z", "xapk",

    // Disk & Container Images
    "vhd", "vhdx", "vmdk", "vdi", "qcow", "qcow2",
    "mdf", "mds", "nrg", "uif",

    // Documents & Office
    "pdf",
    "doc", "docx", "dot", "dotx", "docm", "dotm",
    "xls", "xlsx", "xlsm", "xlsb", "xlt", "xltx", "xltm", "xlc", "xlw",
    "ppt", "pptx", "pptm", "pps", "ppsx", "pot", "potx", "potm",
    "pub",  // Microsoft Publisher
    "vsd", "vsdx", // Visio
    "accdb", "accde", "mdb", "mde", // Access
    "odt", "ods", "odp", "odg", "odf", // OpenDocument
    "pages", "numbers", "key", // Apple iWork
    "rtf", // can be binary-like depending on usage

    // Spreadsheets, DB, and Misc Data
    "db", "sqlite", "db3", "s3db", "frm", "myd", "myi", // MySQL
    "bak", "nsf", // Lotus Notes
    "gdb", "fdb", // Firebird
    "wdb", // Works DB

    // Images
    "jpg", "jpeg", "png", "gif", "bmp", "ico", "tiff", "tif", "webp", "jfif", "jp2",
    "psd", "psb", "xcf", "ai", "eps", "raw", "arw", "cr2", "nef", "dng", "raf", "orf",
    "sr2", "heic", "heif", "icns", "bpg",

    // Audio
    "mp3", "mp2", "aac", "ac3", "wav", "ogg", "oga", "flac", "alac", "m4a", "mp4a",
    "wma", "ra", "ram", "ape", "opus", "amr", "awb",

    // Video
    "mp4", "m4v", "mov", "avi", "wmv", "mkv", "flv", "f4v", "f4p", "f4a", "f4b", "3gp",
    "3g2", "mpeg", "mpg", "mpe", "m1v", "m2v", "ts", "mts", "m2ts", "vob", "rm", "rmvb",
    "asf", "ogv", "ogm", "webm", "dv", "divx", "xvid",

    // Font Files
    "ttf", "otf", "woff", "woff2", "eot", "fon", "psf",

    // Firmware / BIOS / ROM / Game Data
    "rom", "gba", "gbc", "nds", "n64", "z64", "v64", "gcm", "ciso", "wbfs",
    "pak", "wad", "dat", "sav", "rpx",

    // Flash / Vector
    "swf", "fla", "svgz", // .svgz is compressed SVG (binary)

    // CAD / 3D
    "dwg", "dxf", "dwf", "skp", "ifc",
    "stl", "obj", "fbx", "dae", "blend", "3ds", "ase", "gltf", "glb",
    
    // E-Books
    "epub", "mobi", "azw", "azw3", "fb2", "lrf", "lit", "pdb",

    // Other
    "swp", "swo", // Vim swap files
    "pch", // Precompiled header
    "xex", // Console executables
    "dmp", "mdmp", // Memory dump
    "bkf", "bkp", // Backup
    "idx", "vcd", // Various binary data
    "hlp", "chm", // Windows help
    "torrent", // BitTorrent
    "mar", // Mozilla archive
    "aab", // Android bundle
    "appx", // Windows app package
];

/// Default sets of ignore patterns (separate from .gitignore)
#[allow(dead_code)]
pub fn default_ignore_patterns() -> Vec<Regex> {
    let raw = vec![
        r"^LICENSE$",
        r"^\.git/",
        r"^\.next/",
        r"^node_modules/",
        r"^vendor/",
        r"^dist/",
        r"^build/",
        r"^out/",
        r"^target/",
        r"^bin/",
        r"^obj/",
        r"^\.idea/",
        r"^\.vscode/",
        r"^\.vs/",
        r"^\.settings/",
        r"^\.gradle/",
        r"^\.mvn/",
        r"^\.pytest_cache/",
        r"^__pycache__/",
        r"^\.sass-cache/",
        r"^\.vercel/",
        r"^\.turbo/",
        r"^coverage/",
        r"^test-results/",
        r"pnpm-lock\.yaml",
        r"tyk\.toml",
        r"package-lock\.json",
        r"yarn\.lock",
        r"Cargo\.lock",
        r"Gemfile\.lock",
        r"composer\.lock",
        r"mix\.lock",
        r"poetry\.lock",
        r"Pipfile\.lock",
        r"packages\.lock\.json",
        r"paket\.lock",
        r"\.pyc$",
        r"\.pyo$",
        r"\.pyd$",
        r"\.class$",
        r"\.o$",
        r"\.obj$",
        r"\.dll$",
        r"\.exe$",
        r"\.so$",
        r"\.dylib$",
        r"\.log$",
        r"\.tmp$",
        r"\.temp$",
        r"\.swp$",
        r"\.swo$",
        r"\.DS_Store$",
        r"Thumbs\.db$",
        r"\.env(\..+)?$",
        r"\.bak$",
        r"~$",
    ];
    raw.into_iter()
        .map(|pat| Regex::new(pat).unwrap())
        .collect()
}

/// Known text file extensions that can skip binary checks
#[allow(dead_code)]
#[rustfmt::skip]
pub const TEXT_FILE_EXTENSIONS: &[&str] = &[
    // Programming Languages (common & less common)
    "c", "cpp", "cc", "cxx", "c++",
    "h", "hpp", "hh", "hxx", "h++",
    "java",
    "cs", "csx", "csproj",
    "py", "pyw", "pyi", "pyx", "pxd", "pxi",
    "rb", "rbw", "rbt", "gemspec",
    "js", "mjs", "cjs",
    "ts", "mts", "cts",
    "go",
    "rs", "ron",
    "swift",
    "kt", "kts", "ktm",
    "scala", "sc", "sbt",
    "php", "php3", "php4", "php5", "phtml", "ctp", // various PHP
    "pl", "pm", "perl", "cgi", "fcgi",
    "lua",
    "r", "rprofile", "renviron", "rproj",
    "m", // Objective-C or MATLAB/Octave
    "mm", // Objective-C++
    "fs", "fsi", "fsproj", "fsx",
    "hs", "lhs", "cabal", "hsc", "hs-boot", "hsig",
    "elm",
    "erl", "hrl", "rebar.config", "app.src",
    "ex", "exs", "eex", "leex",
    "clj", "cljs", "cljc", "edn",
    "lisp", "lsp", "cl", "el", // Emacs Lisp
    "dart",
    "groovy", "gvy",
    "julia",
    "nim", "nimble",
    "tcl",
    "vb", "vbs", "vbproj", "bas", "frm", "cls",
    "ada", "adb", "ads",
    "d", "di",
    "f", "for", "f77", "f90", "f95", "f03", "f08",
    "cobol", "cbl", "cob",
    "pas", "pp", // Pascal
    "ahk", // AutoHotkey
    "au3", // AutoIt
    "cr", // Crystal
    "crk", // Additional for Crystal? Rare.
    "bf", // Brainf**k
    "hx", "hxml", "hxproj", // Haxe
    "gd", "godot", // GDScript, Godot
    "pwn", "inc", // Pawn
    "sma", "sp", // SourceMod/AMX Mod X
    "nut", // Squirrel
    "moon", "moonc", // MoonScript

    // Web Development & Templates
    "html", "htm", "xhtml",
    "css", "scss", "sass", "less",
    "jsx", "tsx",
    "vue",
    "svelte",
    "jsp",
    "asp", // Classic ASP
    "liquid",
    "pug", "jade",
    "hbs", "handlebars",
    "ejs",
    "twig",
    "erb", "rhtml",
    "slim",
    "haml",
    "mjml",
    "njk", "nunjucks",
    "soy", // Google Closure templates

    // Data & Config
    "json", "jsonl", "json5", "cson",
    "yaml", "yml",
    "toml",
    "xml", "xsd", "xsl", "xslt", "plist",
    "ini", "conf", "cfg",
    "properties", "prop",
    "env",
    "csv", "tsv",
    "sql", "cql", "hql", "msql", "mysql", // various SQL dialects
    "graphql", "gql",
    "prisma",
    "dhall",
    "ron",
    "hcl", "tf", "tfvars", "tfstate", "nomad",
    "cue",
    "lock", // e.g. package-lock, Yarn lock
    "resx", "resw", "resjson", // Windows resource files
    "pbxproj", "xcconfig", "xcscheme", "xcworkspacedata", "xccheckout",
    "xcsettings",

    // Shell & Scripts
    "sh", "bash", "zsh", "fish",
    "ps1", "psm1", "psd1", "ps1xml",
    "bat", "cmd",
    "awk", "sed",
    "ksh",
    "tmux.conf", "screenrc",

    // Documentation & Markup
    "md", "markdown", "mdx", "mdwn", "mkd", "mkdn", "markdn",
    "txt",
    "rst",
    "adoc", "asciidoc",
    "asc",
    "org",
    "nfo",
    "tex", "sty", "cls", "dtx", "ltx",
    "rtf",
    "log", // Often text logs
    "opml",
    "sgml",
    "xmi",
    "docbase", // Some doc generators
    "feature", // Gherkin/Cucumber
    "story", // For various BDD frameworks
    "1", "2", "3", // man pages in *nix (usually text)
    "pod", // Plain Old Documentation for Perl
    "wiki",
    "ms", "me", "man", // troff/groff macros

    // Build & System Config
    "make", "mak", "mk", "makefile",
    "cmake", "cmakelists.txt",
    "gradle", "gradlew",
    "pom", "sbt",
    "rake", "rakefile",
    "dockerfile",
    "vagrantfile",
    "jenkinsfile",
    "gemfile",
    "podfile",
    "brewfile",
    "build", // generic build scripts
    "buck", "bazel", "bzl", "workspace", // Bazel/Google
    "module", "mod",
    "cabal", // Haskell
    "stack", // Haskell stack
    "gyp", "gypi", // Chromium build config

    // Version Control
    "gitignore",
    "gitattributes",
    "gitmodules",
    "gitconfig",
    "gitkeep",
    "gitmessage",
    "mailmap",

    // Project & Editor Config
    "editorconfig",
    "eslintrc", "eslintignore",
    "prettierrc",
    "babelrc",
    "npmrc", "yarnrc", "pnpmfile", "pnpm-lock",
    "dockerignore",
    "flowconfig",
    "npmignore",
    "tern-project",
    "stylelintrc", "stylelintignore",
    "releaserc", // semantic-release
    "commitlintrc", // commitlint

    // IDE & Editor
    "project", "workspace",
    "sublime-project", "sublime-workspace",
    "iml",
    "sln",
    "vbproj",
    "fsproj",
    "xcodeproj", // Xcode project directory, but can appear as file
    "xcworkspace",
    "ipynb", // Jupyter notebooks (JSON)
    "pyproj",
    "Rproj",
    "tsconfig", "jsconfig",

    // License, Legal, & Misc
    "license",
    "license.md",
    "copying",
    "readme", "readme.md",
    "changelog", "changelog.md",
    "authors", "contributors",
    "todo", // Some projects keep TODO in a text file

    // Markup-like or Misc
    "ics", "ical",
    "xaml",
    "purs", // PureScript
    "sgf", // Smart Game Format (Go/Baduk, text-based)
    "pgn", // Chess game notation
    "ttl", "n3", "rdf", "owl", "nt", // RDF notations
    "resjson", // JSON-based resource
    "strings", // iOS/macOS localizations
    "stringsdict",
    "storyboard", "xib",
    "svg", // XML-based vector
    "mcfunction", "mcmeta", // Minecraft data
    "mc", // Another Minecraft script extension
    "cwl", // Common Workflow Language
    "wdl", // Workflow Definition Language
    "jsonnet", "libsonnet",
    "ink", // Inklewriter / Ink scripts
    "fen", // Chess FEN notation
    "q", // Kdb/Q scripts
    "vash",
    "latte",
    "volt",

    // Various shader or GPU language
    "glsl", "vert", "frag", "tesc", "tese", "geom", "comp",
    "wgsl",

    // Additional catch-all / overshadowing known text expansions
    "example", // Often text placeholders
    "template", // Often text
    "sample",
];
