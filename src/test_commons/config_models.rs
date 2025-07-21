use crate::OgreRootConfig;
use serde::{Deserialize, Serialize};

/// Root configs which may contain other sub-configs
#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppRootConfig {
    pub log_sub_config: LogConfig,
    // ...
}
// we should implement this for the root config
impl OgreRootConfig for AppRootConfig {}

/// Specifies what the application should do with its log messages
#[derive(clap::Args, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogConfig {
    #[clap(long)] // this one may also be used in the CLI
    pub sink: Option<Dummy>,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[clap(rename_all = "lower")]
pub enum Dummy {
    Null,
    StdOut,
    StdError,
}
