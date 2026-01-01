/// Known binary file extensions that should be skipped
#[rustfmt::skip]
pub const BINARY_FILE_EXTENSIONS: &[&str] = &[
    // Executables, Libraries, Core Dumps
    "exe", "dll", "so", "dylib", "ocx", "ax", "drv", "sys", "msi", "app", "ipa", "apk",
    "bin", "out", "a", "lib", "ko", "elf", "o", "nro", "core", "img", "iso",

    // Java / .NET / Archives
    "class", "jar", "war", "ear",
    "resources", // sometimes included in Java archives
    "nupkg", // NuGet package
    "exe.config", // sometimes for .NET
    "dll.config",
    
    // Archives & Compressed
    "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "lz4", "lz", "zst", "lzma",
    "cab", "ar", "cpio", "rpm", "deb", "pkg", "crx", "bin", "dmg", "hfs", "img",
    "cso", // Compressed ISO
    "bz", "tbz", "tbz2", "tlz", "txz", "z", "Z", "apk", "xapk",

    // Disk & Container Images
    "iso", "img", "dmg", "vhd", "vhdx", "vmdk", "vdi", "qcow", "qcow2",
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
    "mdb", "bak", "nsf", // Lotus Notes
    "gdb", "fdb", // Firebird
    "mdb", // Access DB
    "wdb", // Works DB

    // Images
    "jpg", "jpeg", "png", "gif", "bmp", "ico", "tiff", "tif", "webp", "jfif", "jp2",
    "psd", "psb", "xcf", "ai", "eps", "raw", "arw", "cr2", "nef", "dng", "raf", "orf",
    "sr2", "heic", "heif", "icns", "img", "bpg",

    // Audio
    "mp3", "mp2", "aac", "ac3", "wav", "ogg", "oga", "flac", "alac", "m4a", "mp4a",
    "wma", "ra", "ram", "ape", "opus", "amr", "awb",

    // Video
    "mp4", "m4v", "mov", "avi", "wmv", "mkv", "flv", "f4v", "f4p", "f4a", "f4b", "3gp",
    "3g2", "mpeg", "mpg", "mpe", "m1v", "m2v", "mts", "m2ts", "vob", "rm", "rmvb",
    "asf", "ogv", "ogm", "webm", "dv", "divx", "xvid",

    // Font Files
    "ttf", "otf", "woff", "woff2", "eot", "fon", "psf",

    // Firmware / BIOS / ROM / Game Data
    "rom", "iso", "bin", "gba", "gbc", "nds", "n64", "z64", "v64", "gcm", "ciso", "wbfs",
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
    "xex", "elf", // Console executables
    "dmp", "mdmp", // Memory dump
    "bkf", "bkp", // Backup
    "pak", // Common game data archives
    "idx", "dat", "vcd", // Various binary data
    "icns", // macOS icon
    "hlp", "chm", // Windows help
    "torrent", // BitTorrent
    "mar", // Mozilla archive
    "qcow", "qcow2", // QEMU disk
    "apk", "aab", // Android package/bundle
    "crx", // Chrome extension
    "appx", // Windows app package
    "xap", // Windows Phone app
];

/// Default sets of ignore patterns (separate from .gitignore)
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "LICENSE",
    ".git/**",
    ".next/**",
    "node_modules/**",
    "vendor/**",
    "dist/**",
    "build/**",
    "out/**",
    "target/**",
    "bin/**",
    "obj/**",
    ".idea/**",
    ".vscode/**",
    ".vs/**",
    ".settings/**",
    ".gradle/**",
    ".mvn/**",
    ".pytest_cache/**",
    "__pycache__/**",
    ".sass-cache/**",
    ".vercel/**",
    ".turbo/**",
    "coverage/**",
    "test-results/**",
    ".gitignore",
    "pnpm-lock.yaml",
    "yek.toml",
    "yek.yaml",
    "yek.json",
    "package-lock.json",
    "yarn.lock",
    "Cargo.lock",
    "Gemfile.lock",
    "composer.lock",
    "mix.lock",
    "poetry.lock",
    "Pipfile.lock",
    "packages.lock.json",
    "paket.lock",
    "*.pyc",
    "*.pyo",
    "*.pyd",
    "*.class",
    "*.o",
    "*.obj",
    "*.dll",
    "*.exe",
    "*.so",
    "*.dylib",
    "*.log",
    "*.tmp",
    "*.temp",
    "*.swp",
    "*.swo",
    ".DS_Store",
    "Thumbs.db",
    ".env*",
    "*.bak",
    "*~",
];

pub const DEFAULT_OUTPUT_TEMPLATE: &str = ">>>> FILE_PATH\nFILE_CONTENT";
