use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

/// Trait to be implemented by root config types, enabling them to be written / loaded from disk
pub trait OgreRootConfig: Debug + Serialize + for<'r> Deserialize<'r> + Sized + Default {}

/// Trait to allow merging command line options into the application's configs
pub trait CmdLineAndConfigIntegration<RootConfigType: OgreRootConfig>: clap::Parser + Debug {
    /// Specifies the configuration file to be used by the application.
    /// If none is specified, use the default file, located at the same path as the executable,
    /// having the same name + the '.config.ron' extension.
    ///
    /// Supported formats & extensions are:
    ///   - '.ron': use the RON file format
    ///   - '.yaml' & '.yml': use the YML file format.
    ///
    /// If the specified file doesn't exist, one will be created with the default values.
    ///
    /// Note to implementers: use a field like this:
    /// ```nocompile
    ///   #[clap(long, short = 'c')]
    ///   pub config_file: Option<String>,
    fn config_file_path(&self) -> Option<&str>;

    /// USE WITH CAUTION: If specified, cause the configuration file to be re-written with the effective
    /// configuration after merging the existing config file and the given command line options.
    ///
    /// --> Any comments or data overridden by the command line arguments will be lost.
    ///
    /// As a backup, the old config file will be renamed by adding a '~' (tilde) at the end of its name.
    ///
    /// Note to implementers: use a field like this:
    /// ```nocompile
    ///   #[clap(long)]
    ///   pub write_effective_config: bool,
    fn should_write_effective_config(&self) -> bool;

    /// Makes the program dump (to stderr) the "effective configuration" being used
    /// -- the result from loading the configuration file, then applying the command line options.
    ///
    /// Note to implementers: use a field like this:
    /// ```nocompile
    ///   #[clap(long)]
    ///   pub show_effective_config: bool,
    fn should_show_effective_config(&self) -> bool;

    /// Given the specific `RootConfig` and `CmdLineOptionsType` types,
    /// allow the given `RootConfig` to be updated with the given command line options (from `self`)
    fn merge_with_config(self, config: RootConfigType) -> RootConfigType;
}

/// Error variants for the `cli-configs` trait
#[derive(Debug)]
pub enum Error {
    LoadingConfig {
        message: String,
        cause: Box<dyn std::error::Error + Send + Sync>,
    },
    SavingConfig {
        message: String,
        cause: Box<dyn std::error::Error + Send + Sync>,
    },
    UnsupportedConfigFileFormat {
        message: String,
    },
    Ron {
        message: String,
        cause: ron::Error,
    },
    Yaml {
        message: String,
        cause: serde_yaml::Error,
    },
    Io {
        message: String,
        cause: std::io::Error,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}
impl std::error::Error for Error {}
