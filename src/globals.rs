use std::sync::LazyLock;
use crate::config::Config;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);
