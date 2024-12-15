use lazy_static::lazy_static;
use crate::config::Config;

lazy_static! {
    pub static ref CONFIG: Config = Config::load();
}
