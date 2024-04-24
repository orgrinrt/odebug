use std::path::{Path, PathBuf};

pub fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

pub fn workspace_dir_str() -> String {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_str().unwrap().to_string()
}

pub mod __debug_file {

    pub static mut IS_DEBUG_FILE_REFRESHED: bool = false;
    pub const __NO_HEADER: &str = "___NOHEAD___";
    pub const __SEPARATOR_ABOVE: &str = "___SEPART_AB___";
    pub const __SEPARATOR_BELOW: &str = "___SEPART_BL___";
    pub const __SEPARATOR: &str = __SEPARATOR_ABOVE;
    pub const __DEBUG_FILE_NAME: &str = "debug.txt";
    pub const __DEBUG_OUTPUT_DIR: &str = ".debug";
    #[cfg(feature = "use_workspace")]
    pub const __USE_WORKSPACE_DIR: bool = true;
    #[cfg(not(feature = "use_workspace"))]
    pub const __USE_WORKSPACE_DIR: bool = false;

    #[macro_export]
    macro_rules! use_debug_file_deps {
        () => {
            use std::env as DBG_OUTPUT_ENV;
            use std::fs::{remove_file as DBG_OUTPUT_FILE_REMOVER, OpenOptions as DBG_OUTPUT_FILE};
            use std::io::Write as DBG_OUTPUT_WRITE;
            use std::path::Path as DBG_OUTPUT_PATH;

            use paste::paste as DBG_PASTE;
            use $crate::__debug_file::{
                IS_DEBUG_FILE_REFRESHED as DBG_IS_DEBUG_FILE_REFRESHED,
                __DEBUG_FILE_NAME as DBG_FILE_NAME,
                __DEBUG_OUTPUT_DIR as DBG_OUTPUT_DIR,
                __NO_HEADER as DBG_NOHEAD,
                __SEPARATOR as DBG_SEP,
                __SEPARATOR_ABOVE as DBG_SEP_ABOVE,
                __SEPARATOR_BELOW as DBG_SEP_BELOW,
                __USE_WORKSPACE_DIR as DBG_USE_WORKSPACE_DIR,
            };
        };
    }

    #[macro_export]
    macro_rules! debug_file {
        (!$fmt_str:expr, $($fmt_args:expr),*) => {
            let content = format!($fmt_str, $($fmt_args),*);
                $crate::debug_file!(content, DBG_NOHEAD, __);
        };
        ($content:expr, Separator) => {
                $crate::debug_file!($content, DBG_SEP, __);
        };
        ($content:expr, SeparatorBelow) => {
                $crate::debug_file!($content, DBG_SEP_BELOW, __);
        };
        ($content:expr, SeparatorAbove) => {
                $crate::debug_file!($content, DBG_SEP_ABOVE, __);
        };
        ($content:expr) => {
                $crate::debug_file!($content, DBG_NOHEAD, __);
        };
        ($content:expr, $header:expr) => {

                $crate::debug_file!($content, $header, __);
        };
        ($content:expr, $header:expr, __) => {
            {
                $crate::use_debug_file_deps!();
                DBG_PASTE! {{
                    let mut expanded_str = "".to_string();
                    if $header == DBG_SEP_ABOVE {
                        expanded_str = format!(
                            "\n--------------------------------------------------------------------------------------\n \
                            {}",
                        $content
                        .to_string());
                    }
                    else if $header == DBG_SEP_BELOW {
                        expanded_str = format!(
                            "\n {} \
                            \n--------------------------------------------------------------------------------------",
                        $content.to_string());
                    }
                    else if $header != DBG_NOHEAD {
                        expanded_str = format!(
                            "\n--------------------------------------------------------------------------------------\n  \
                            \t> {} \
                            \n--------------------------------------------------------------------------------------\n \
                            {}",
                            $header.to_string(),
                            $content.to_string());
                    }
                    else {
                        expanded_str = format!("\n {}",
                        $content.to_string());
                    }

                    let workspace_dir = $crate::workspace_dir_str();
                    let manifest_dir = DBG_OUTPUT_ENV::var("CARGO_MANIFEST_DIR").unwrap();
                    let crate_name = DBG_OUTPUT_ENV::var("CARGO_CRATE_NAME").unwrap();
                    let base_dir = if DBG_USE_WORKSPACE_DIR { workspace_dir } else {
                        manifest_dir };
                    let debug_path = DBG_OUTPUT_PATH::new(&base_dir)
                        .join(DBG_OUTPUT_DIR)
                        .join(format!("{}_{}", crate_name, DBG_FILE_NAME));

                    unsafe {
                        if !DBG_IS_DEBUG_FILE_REFRESHED {
                            let mut file = match DBG_OUTPUT_FILE_REMOVER(&debug_path) {
                                Ok(f) => f,
                                Err(error) => panic!("Problem removing the file: {:?}", error),
                            };

                            DBG_IS_DEBUG_FILE_REFRESHED = true;
                        }
                    }

                    let mut file = match DBG_OUTPUT_FILE::new().append(true).create(true).open
                    (&debug_path) {
                        Ok(f) => f,
                        Err(error) => panic!("Problem opening the file: {:?}", error),
                    };
                    if let Err(error) = file.write_all(expanded_str.as_bytes()) {
                        panic!("Problem writing to the file(start): {:?}", error);
                    }
                    // if let Err(error) = file.write_all(end_str.as_bytes()) {
                    //     panic!("Problem writing to the file (end): {:?}", error);
                    // }
                    }
                };
            }
        };
    }
}
