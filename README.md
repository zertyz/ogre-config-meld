Melds configs from files, env, and CLI into a clean, validated strong typed "effective configuration".

This crate is still in an experimental stage for providing the following distinctive features:
1) Configs are saved and loaded from files, alongside with their docs.
2) The config file is created if one doesn't exist. Default values are filled in.
3) Different config file formats are supported. Currently, YAML and RON.
4) CLI options are meant to override any configs from files.
5) However, the CLI models are first-class object, as they may contain options not suitable for a configuration file,
   such as specifying "where the config file is located at".

Still missing:
* ENV integration not fully implemented.
* Validation of the given config values. Not only in isolation.
