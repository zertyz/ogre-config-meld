//! Operations for the program's Command Line Interface -- mostly delegated to `clap`

use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::{save_to_file, CmdLineAndConfigIntegration, OgreRootConfig};
use encryptable_tokio_fs::fs;
use clap::Parser;

/// Similarly to [parse_cmdline_args()],
/// parse the CLI options from the program's command line args,
/// but also load the configs and [merge_cmdline_args_with_configs()],
/// then return the effective configuration the application must use
pub async fn parse_cmdline_and_merge_with_loaded_configs<
    CmdLineOptionsType: clap::Parser + CmdLineAndConfigIntegration<RootConfigType>,
    RootConfigType: OgreRootConfig,
>(
    tail_docs: &str,
) -> Result<RootConfigType, crate::Error> {

    let cmdline_options: CmdLineOptionsType = parse_cmdline_args();
    let should_write_effective_config = cmdline_options.should_write_effective_config();
    let should_show_effective_config = cmdline_options.should_show_effective_config();

    let config_file_path = get_config_file_path::<CmdLineOptionsType, RootConfigType>();
    let loaded_config = super::load_or_create_default(&config_file_path, tail_docs).await?;
    let effective_config = merge_cmdline_args_with_configs(cmdline_options, loaded_config);

    if should_show_effective_config {
        eprintln!("EFFECTIVE PROGRAM CONFIGURATION: {effective_config:#?}\n");
        io::stderr()
            .flush()
            .map_err(|err| crate::Error::LoadingConfig {
                message: "Error dumping the Effective Program Configuration to stderr".to_string(),
                cause: err.into(),
            })?;
    }

    if should_write_effective_config {
        let mut backup_config_file_path = config_file_path.clone();
        backup_config_file_path.push("~");

        // generate the docs for the new configs
        let doc_comments = format!(
            r#"
Rewriten from merging the previous configs & the command line options at {date_str}
(previous configuration file backed up to {backup_config_file_path:?})

COMMAND LINE OPTIONS: {cmdline_options:#?}

PREVIOUS CONFIG: {loaded_config:#?}

"#,
            date_str = chrono::Local::now().format("%a %b %e %H:%M:%S %Z %Y"),
            // // recompute previously consumed information -- instead of always cloning unneededly
            cmdline_options = parse_cmdline_args::<CmdLineOptionsType>(),
            loaded_config = super::config_logic::load_or_create_default::<RootConfigType>(
                &config_file_path,
                tail_docs
            )
            .await?,
        );

        fs::rename(&config_file_path, &backup_config_file_path).await
            .map_err(|err| crate::Error::SavingConfig {
                message: format!("Error rewriting the config file {config_file_path:?} with a new effective configuration: the file couldn't be renamed to {backup_config_file_path:?}"),
                cause: err.into(),
            })?;

        save_to_file(&effective_config, &doc_comments, &config_file_path).await?;
    }

    Ok(effective_config)
}

/// Determines the exact path for the configuration file to be used, taking into account:
/// * The program's name & path
/// * Config format CLI options
/// * The existence or not of files at the default locations in any of the supported formats
/// * Overrides on config file location via command line interface
/// * The default config file, if none exists and no command line options are given
///
/// Note that the returned `PathBuf` may either specify an existing file to read
/// or an unexisting file to be created.
pub fn get_config_file_path<
    CmdLineOptionsType: clap::Parser + CmdLineAndConfigIntegration<RootConfigType>,
    RootConfigType: OgreRootConfig,
>() -> PathBuf {

    // Provides a configuration file name if none was specified in CLI.
    // Priority goes for any existing files in the order presented in `CONFIG_SUFFIXES`
    fn default_config_file_path() -> PathBuf {

        const CONFIG_SUFFIXES: &[&str] = &[
            ".config.ron",
            ".config.yaml",
        ];
        let program_name = std::env::args().next()
            .expect("Program name couldn't be retrieve from args. Please specify which configuration file to use via command line.")
            .to_owned();

        // first, try to find any existing file possibilities
        for suffix in CONFIG_SUFFIXES {
            let config_file_candidate = format!("{program_name}{suffix}");
            let config_file_candidate = Path::new(&config_file_candidate);
            // if it exists, return it
            if config_file_candidate.exists() {
                return config_file_candidate.to_path_buf()
            }
        }

        // if no existing file was found, use the first in our priority list
        let uncreated_config_file = format!("{program_name}{}", CONFIG_SUFFIXES[0]);
        Path::new(&uncreated_config_file).to_path_buf()
    }

    let cmdline_options: CmdLineOptionsType = parse_cmdline_args();

    cmdline_options
        .config_file_path()
        .map(Path::new)
        .map(Path::to_path_buf)
        .unwrap_or_else(default_config_file_path)

}

/// Parse the CLI options from the program's command line args.
/// Most likely you'd like to use [parse_cmdline_and_merge_with_configs()]
pub fn parse_cmdline_args<CmdLineOptionsType: Parser>() -> CmdLineOptionsType {
    <CmdLineOptionsType as Parser>::parse()
}

/// Returns the "effective configuration" applications should use:
/// given the specific `root_config` and `cmdline_options`, merge the former
/// into the latter
pub fn merge_cmdline_args_with_configs<
    CmdLineOptionsType: Parser + CmdLineAndConfigIntegration<RootConfigType>,
    RootConfigType: OgreRootConfig,
>(
    cmdline_options: CmdLineOptionsType,
    root_config: RootConfigType,
) -> RootConfigType {
    cmdline_options.merge_with_config(root_config)
}
