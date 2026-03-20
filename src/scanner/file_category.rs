#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Video,
    Music,
    Images,
    Documents,
    Executables,
    Archives,
    Code,
    Other,
}

impl FileCategory {
    pub const COUNT: usize = 8;

    pub const ALL: [FileCategory; Self::COUNT] = [
        FileCategory::Video,
        FileCategory::Music,
        FileCategory::Images,
        FileCategory::Documents,
        FileCategory::Executables,
        FileCategory::Archives,
        FileCategory::Code,
        FileCategory::Other,
    ];

    pub fn index(self) -> usize {
        match self {
            FileCategory::Video => 0,
            FileCategory::Music => 1,
            FileCategory::Images => 2,
            FileCategory::Documents => 3,
            FileCategory::Executables => 4,
            FileCategory::Archives => 5,
            FileCategory::Code => 6,
            FileCategory::Other => 7,
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            FileCategory::Video => "\u{1F3AC}",
            FileCategory::Music => "\u{1F3B5}",
            FileCategory::Images => "\u{1F5BC}",
            FileCategory::Documents => "\u{1F4C4}",
            FileCategory::Executables => "\u{2699}\u{FE0F}",
            FileCategory::Archives => "\u{1F4E6}",
            FileCategory::Code => "\u{1F4BB}",
            FileCategory::Other => "\u{1F4CE}",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FileCategory::Video => "Video",
            FileCategory::Music => "Music",
            FileCategory::Images => "Images",
            FileCategory::Documents => "Documents",
            FileCategory::Executables => "Executables",
            FileCategory::Archives => "Archives",
            FileCategory::Code => "Code",
            FileCategory::Other => "Other",
        }
    }

    pub fn from_extension(ext: &str) -> FileCategory {
        let mut buf = [0u8; 16];
        let len = ext.len().min(buf.len());
        buf[..len].copy_from_slice(&ext.as_bytes()[..len]);
        buf[..len].make_ascii_lowercase();
        let lower = match std::str::from_utf8(&buf[..len]) {
            Ok(s) => s,
            Err(_) => return FileCategory::Other,
        };
        match lower {
            // Video
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
            | "vob" | "3gp" => FileCategory::Video,

            // Music
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "aiff" | "alac" => {
                FileCategory::Music
            }

            // Images
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "tiff" | "tif" | "ico"
            | "psd" | "raw" | "cr2" | "nef" | "heic" | "heif" | "avif" => FileCategory::Images,

            // Documents
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt"
            | "ods" | "odp" | "csv" | "epub" | "md" | "pages" | "numbers" | "key" => {
                FileCategory::Documents
            }

            // Executables
            "exe" | "msi" | "dll" | "so" | "dylib" | "app" | "bat" | "cmd" | "sh" | "com"
            | "sys" | "drv" => FileCategory::Executables,

            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "zst" | "iso" | "cab" | "dmg"
            | "lz4" | "lzma" => FileCategory::Archives,

            // Code
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "java" | "c" | "cpp" | "h" | "hpp"
            | "cs" | "go" | "rb" | "php" | "html" | "css" | "scss" | "less" | "json" | "xml"
            | "yaml" | "yml" | "toml" | "sql" | "swift" | "kt" | "lua" | "r" | "pl" | "ex"
            | "exs" | "vue" | "svelte" => FileCategory::Code,

            _ => FileCategory::Other,
        }
    }
}
