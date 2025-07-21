//! SERializer & DEserializer operations for the configs,
//! able to load & write RON and YAML files

use crate::{Error, OgreRootConfig};
use once_cell::sync::Lazy;
use regex::Regex;
use ron::ser::{to_string_pretty, PrettyConfig};

pub trait ConfigSerde {
    fn serialize_config(
        &self,
        config: &impl OgreRootConfig,
        tail_comment: &str,
    ) -> Result<String, crate::Error>;

    fn deserialize_config<RootConfigType: OgreRootConfig>(
        &self,
        txt_config: &str,
    ) -> Result<RootConfigType, crate::Error>;
}

/// Supported config file formats
pub enum SerdeFormat {
    Ron,
    Yaml,
}

/// Automatically selects between [RonSerde] and [YamlSerde]
pub struct AutomaticSerde {
    format: SerdeFormat,
    ron_serde: RonSerde,
    yaml_serde: YamlSerde,
}

impl AutomaticSerde {
    pub fn new(format: SerdeFormat) -> Self {
        Self {
            format,
            ron_serde: RonSerde {},
            yaml_serde: YamlSerde {},
        }
    }

    pub fn for_file_extension(file_extension: &str) -> Result<Self, crate::Error> {
        let format = match file_extension {
            ".ron" => Ok(SerdeFormat::Ron),
            ".yaml" => Ok(SerdeFormat::Yaml),
            ".yml" => Ok(SerdeFormat::Yaml),
            _ => Err(crate::Error::UnsupportedConfigFileFormat { message: format!("`cli-config`: Unsupported config file extension: '{file_extension}'. Supported extensions are '.ron', '.yaml' and '.yml'") })
        }?;
        Ok(Self::new(format))
    }
}

impl ConfigSerde for AutomaticSerde {
    fn serialize_config(
        &self,
        config: &impl OgreRootConfig,
        tail_comment: &str,
    ) -> Result<String, Error> {
        match self.format {
            SerdeFormat::Ron => self.ron_serde.serialize_config(config, tail_comment),
            SerdeFormat::Yaml => self.yaml_serde.serialize_config(config, tail_comment),
        }
    }

    fn deserialize_config<RootConfigType: OgreRootConfig>(
        &self,
        txt_config: &str,
    ) -> Result<RootConfigType, Error> {
        match self.format {
            SerdeFormat::Ron => self.ron_serde.deserialize_config(txt_config),
            SerdeFormat::Yaml => self.yaml_serde.deserialize_config(txt_config),
        }
    }
}

struct RonSerde {}
impl ConfigSerde for RonSerde {
    fn serialize_config(
        &self,
        config: &impl OgreRootConfig,
        tail_comment: &str,
    ) -> Result<String, crate::Error> {
        to_string_pretty(&config, PrettyConfig::default())
            .map_err(|err| crate::Error::Ron {
                message: format!("RON serialization Error for config '{config:?}'"),
                cause: err,
            })
            .map(|mut txt_config| {
                if !tail_comment.is_empty() {
                    txt_config.push_str("\n\n/*\n");
                    txt_config.push_str(
                        "///////////////////////////// DOCS //////////////////////////////\n",
                    );
                    txt_config.push_str(tail_comment);
                    txt_config.push_str("\n*/\n");
                }
                txt_config
            })
    }

    fn deserialize_config<RootConfigType: OgreRootConfig>(
        &self,
        txt_config: &str,
    ) -> Result<RootConfigType, crate::Error> {
        ron::Options::default()
            .from_str(txt_config)
            .map_err(|err| crate::Error::Ron {
                message: format!("RON deserialization error for config text '{txt_config}'"),
                cause: err.into(),
            })
    }
}

struct YamlSerde {}
impl ConfigSerde for YamlSerde {
    fn serialize_config(
        &self,
        config: &impl OgreRootConfig,
        tail_comment: &str,
    ) -> Result<String, crate::Error> {
        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(?m)^").expect("Bad Regex"));

        serde_yaml::to_string(config)
            .map_err(|err| crate::Error::Yaml {
                message: format!("YAML serialization error for config '{config:?}'"),
                cause: err,
            })
            .map(|mut txt_config| {
                if !tail_comment.is_empty() {
                    let tail_comment = REGEX.replace_all(tail_comment, "# ");
                    txt_config.push('\n');
                    txt_config.push_str(
                        "############################# DOCS ##############################\n",
                    );
                    txt_config.push_str(&tail_comment);
                }
                txt_config
            })
    }

    fn deserialize_config<RootConfigType: OgreRootConfig>(
        &self,
        txt_config: &str,
    ) -> Result<RootConfigType, crate::Error> {
        serde_yaml::from_str(txt_config).map_err(|err| crate::Error::Yaml {
            message: format!("YAML deserialization error for config text '{txt_config}'"),
            cause: err,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_commons::config_models::*;

    #[test]
    fn ron_serde() {
        let test = |tail_docs| {
            let expected_config = AppRootConfig::default();
            let ron_serde = RonSerde {};
            let config_txt = ron_serde
                .serialize_config(&expected_config, tail_docs)
                .unwrap();
            let deserialized_config: AppRootConfig =
                ron_serde.deserialize_config(&config_txt).unwrap();
            println!("RON:\n{config_txt}");
            assert_eq!(
                deserialized_config, expected_config,
                "RON serde didn't work"
            );
        };

        test("");
        test("I\nhave\nmultiline\ntail docs");
    }

    #[test]
    fn yaml_serde() {
        let test = |tail_docs| {
            let expected_config = AppRootConfig::default();
            let yaml_serde = YamlSerde {};
            let config_txt = yaml_serde
                .serialize_config(&expected_config, tail_docs)
                .unwrap();
            let deserialized_config: AppRootConfig =
                yaml_serde.deserialize_config(&config_txt).unwrap();
            println!("YAML:\n{config_txt}");
            assert_eq!(
                deserialized_config, expected_config,
                "YAML serde didn't work"
            );
        };

        test("");
        test("I\nhave\nmultiline\ntail docs");
    }

    #[test]
    fn automatic_serde() {
        // unsupported extension
        let expected_error_message = "`cli-config`: Unsupported config file extension: '.unsupported.file.extension'. Supported extensions are '.ron', '.yaml' and '.yml'";
        let result = AutomaticSerde::for_file_extension(".unsupported.file.extension");
        assert!(
            result.is_err(),
            "Passing an unsupported config file extension should result in an error"
        );
        match result {
            Err(crate::Error::UnsupportedConfigFileFormat {
                message: observed_error_message,
            }) => assert_eq!(
                observed_error_message, expected_error_message,
                "Unexpected error message"
            ),
            _ => panic!("Unexpected result"),
        }

        // supported extensions
        let test = |file_extension| {
            let expected_config = AppRootConfig::default();
            let serde = AutomaticSerde::for_file_extension(file_extension).unwrap();
            let config_txt = serde.serialize_config(&expected_config, "").unwrap();
            let deserialized_config: AppRootConfig = serde.deserialize_config(&config_txt).unwrap();
            println!("{file_extension}:\n{config_txt}");
            assert_eq!(
                deserialized_config, expected_config,
                "Automatic serde for '{file_extension}' didn't work"
            );
        };

        test(".ron");
        test(".yaml");
        test(".yml");
    }
}
