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
    let toml = fs::read_to_string(path).context("no se pudo leer config.toml")?;
    let top = toml
        .parse::<Table>()
        .context("no se pudo analizar la config")?;

    let cfg_src = top
        .get("src")
        .context("la config no tiene endpoint de origen")?;
    let src = Endpoint::from_config("el endpoint de origen", cfg_src)?;

    let mut dests = HashMap::<String, Endpoint>::new();
    let cfg_dests = top
        .get("dest")
        .context("la config no tiene algunos endpoints de destino")?
        .as_table()
        .context("los endpoints de destino de la config no es tabla")?;

    for (name, table) in cfg_dests {
        dests.insert(name.to_string(), Endpoint::from_config(name, table)?);
    }

    let process = top
        .get("process")
        .context("la config no tiene la sección de proceso")?
        .as_table()
        .context("la sección de proceso no es tabla")?;

    let script_text = process
        .get("script")
        .context("la sección de proceso no tiene script")?
        .as_str()
        .context("el script de la sección de proceso no es una cadena")?;

    let script = Script::parse(script_text)?;

    Ok(Config { src, dests, script })
}
