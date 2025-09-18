//! Operations for the program's config file

use std::fmt::Debug;
use crate::logic::serde::{AutomaticSerde, ConfigSerde};
use crate::OgreRootConfig;
use once_cell::sync::Lazy;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

/// Loads the configuration from the given `config_file_path`
/// or creates it (with default values & comments) if it doesn't exist
pub async fn load_or_create_default<RootConfigType: OgreRootConfig>(
    config_file_path: impl AsRef<Path> + Debug,
    tail_comments: &str,
) -> Result<RootConfigType, crate::Error> {
    let config = load_from_file(&config_file_path).await?;
    match config {
        Some(config) => Ok(config),
        None => {
            let default_config = RootConfigType::default();
            save_to_file(&default_config, tail_comments, config_file_path).await?;
            Ok(default_config)
        }
    }
}

/// Saves the `config` to `config_file_path`,
/// including documentation from the original [config_model] sources
pub async fn save_to_file(
    config: &impl OgreRootConfig,
    tail_comment: &str,
    config_file_path: impl AsRef<Path> + Debug,
) -> Result<(), crate::Error> {
    let Some(file_extension) = ext_with_dot(&config_file_path) else {
        let cause = crate::Error::UnsupportedConfigFileFormat {
            message: "Config file without an extension is not supported".to_string(),
        };
        return Err(crate::Error::SavingConfig {
            message: format!(
                "Error instantiating the automatic serde for file {config_file_path:?}"
            ),
            cause: Box::new(cause),
        });
    };
    let txt_config = AutomaticSerde::for_file_extension(&file_extension)
        .map_err(|err| crate::Error::SavingConfig {
            message: format!(
                "Error instantiating the automatic serde for file {config_file_path:?}"
            ),
            cause: Box::new(err),
        })?
        .serialize_config(config, tail_comment)
        .map_err(|err| crate::Error::SavingConfig {
            message: format!("Error serializing config for saving into {config_file_path:?}"),
            cause: Box::new(err),
        })?;
    fs::write(&config_file_path, &txt_config).map_err(|err| crate::Error::SavingConfig {
        message: format!("Error saving config into {config_file_path:?}"),
        cause: Box::new(err),
    })?;
    Ok(())
}

/// Returns `Ok(None)` if the file doesn't exist.
async fn load_from_file<RootConfigType: OgreRootConfig>(
    config_file_path: impl AsRef<Path> + Debug,
) -> Result<Option<RootConfigType>, crate::Error> {
    let Some(file_extension) = ext_with_dot(&config_file_path) else {
        let cause = crate::Error::UnsupportedConfigFileFormat {
            message: "Config file without an extension is not supported".to_string(),
        };
        return Err(crate::Error::LoadingConfig {
            message: format!(
                "Error instantiating the automatic serde for file {config_file_path:?}"
            ),
            cause: Box::new(cause),
        });
    };
    let txt_config_result = fs::read_to_string(&config_file_path);
    let txt_config = match txt_config_result {
        Ok(txt_config) => Ok(txt_config),
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return Ok(None);
            }
            Err(crate::Error::LoadingConfig {
                message: format!("Error loading config from {config_file_path:?}"),
                cause: Box::new(err),
            })
        }
    }?;
    let config = AutomaticSerde::for_file_extension(&file_extension)
        .map_err(|err| crate::Error::LoadingConfig {
            message: format!(
                "Error instantiating the automatic serde for file {config_file_path:?}"
            ),
            cause: Box::new(err),
        })?
        .deserialize_config(&txt_config)
        .map_err(|err| crate::Error::LoadingConfig {
            message: format!("Error deserializing config after loading from {config_file_path:?}"),
            cause: Box::new(err),
        })?;
    Ok(Some(config))
}

fn ext_with_dot(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|os| os.to_str())
        .and_then(|name| name.rfind('.').map(|idx| &name[idx..]))
        .map(ToString::to_string)
}

//////////////
// Config Docs
//////////////

use regex::{Regex, RegexBuilder};

/// Gives access to the configuration documentation, so they may be
/// included when saving config files, for a better user experience
pub fn documented_config_models(configs_root_dir: &include_dir::Dir<'_>) -> String {
    // Regexes and their replacements to apply to model source files when writing the docs
    static REPLACEMENTS: Lazy<[(Regex, &str); 6]> = Lazy::new(|| {
        [
            ("\n//![^\n]*", ""),                   // remove file doc comments
            ("\nmod [^\n]*|\npub use [^\n]*", ""), // remove 'mod' & 'pub use' clauses
            ("\nuse [^\n]*", ""),                  // remove 'use' clauses
            ("\n#[^\n]*", ""),                     // remove macros & #[derive(...)] clauses
            ("\nimpl .*?\n}.*?\n?", "\n"),         // remove any impls
            ("\n\n+", "\n\n"), // standardize the number of consecutive empty lines
        ]
        .map(|(regex, replacement)| {
            let regex = RegexBuilder::new(regex)
                .dot_matches_new_line(true)
                .build()
                .expect("Error parsing Regex");
            (regex, replacement)
        })
    });

    let mut merged_docs = String::new();
    merged_docs.push('\n');

    for file in configs_root_dir.files() {
        let src = String::from_utf8_lossy(file.contents());
        merged_docs.push('\n');
        merged_docs.push_str(&src);
    }

    // replace
    let docs_section =
        REPLACEMENTS
            .iter()
            .fold(merged_docs, |docs_section, (regex, replacement)| {
                regex.replace_all(&docs_section, *replacement).to_string()
            });

    docs_section
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_commons::config_models::*;
    use include_dir::{include_dir, Dir};
    use ron::ser::{to_string_pretty, PrettyConfig};

    static DOCS: Lazy<String> = Lazy::new(|| {
        // For docs extraction that will be placed alongside the config file
        static CONFIGS_DIR_SRC: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/test_commons/");
        documented_config_models(&CONFIGS_DIR_SRC)
    });

    #[tokio::test]
    async fn load_or_create_default_test() {
        let _config_path = std::env::temp_dir().join("cli-config-load_and_save.ron");
        let config_path = _config_path.to_string_lossy();
        let _expected_config = AppRootConfig::default();
        let observed_config_new_file: AppRootConfig =
            load_or_create_default(config_path.as_ref(), &DOCS)
                .await
                .unwrap();
        let observed_config_existing_file: AppRootConfig =
            load_or_create_default(config_path.as_ref(), &DOCS)
                .await
                .unwrap();
        assert_eq!(
            observed_config_new_file, observed_config_existing_file,
            "Creating a new config file failed"
        );
        assert_eq!(
            observed_config_existing_file, observed_config_existing_file,
            "Loading config from existing file failed"
        );
    }

    #[test]
    fn ron_with_docs() {
        let default_config = AppRootConfig::default();
        let raw_ron = to_string_pretty(&default_config, PrettyConfig::default()).unwrap();
        println!("===> PLEASE, VALIDATE THIS TEST MANUALLY: are the config models & docs at the end of this print?");
        println!("{raw_ron}");
        println!("\n");
        println!("///////////////////////////// DOCS //////////////////////////////");
        println!("\n/*");
        println!("{}", DOCS.as_str());
        println!("*/\n");
    }
}
