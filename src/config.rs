use crate::endpoint::Endpoint;
use crate::script::Script;
use anyhow::{Context, Result};
use std::{collections::HashMap, fs, path::Path};
use toml::Table;

pub(crate) struct Config {
    pub(crate) src: Endpoint,
    pub(crate) dests: HashMap<String, Endpoint>,
    pub(crate) script: Script,
}

pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
    let path = path.as_ref();
    let toml =
        fs::read_to_string(path).with_context(|| format!("can't read config from {:?}", path))?;
    let top = toml.parse::<Table>().context("can't parse config")?;

    let cfg_src = top.get("src").context("config lacks src")?;
    let src = Endpoint::from_config("src", cfg_src)?;

    let mut dests = HashMap::<String, Endpoint>::new();
    let cfg_dests = top
        .get("dest")
        .context("config lacks any dests")?
        .as_table()
        .context("dests should be table?")?;

    for (name, table) in cfg_dests {
        dests.insert(name.to_string(), Endpoint::from_config(name, table)?);
    }

    let process = top
        .get("process")
        .context("config missing process section")?
        .as_table()
        .context("process section should be table?")?;

    let script_text = process
        .get("script")
        .context("process section missing script")?
        .as_str()
        .context("process script should be string?")?;

    let script = Script::parse(script_text)?;

    Ok(Config { src, dests, script })
}
