use crate::endpoint::Endpoint;
use anyhow::{Context, Result};
use std::{collections::HashMap, fs, path::Path};
use toml::{Table, Value};

pub(crate) struct Job {
    pub(crate) src: Endpoint,
    pub(crate) dest: Endpoint,
}

impl Job {
    pub(crate) fn from_config_and_endpoints(
        config: &Value,
        endpoints: &HashMap<String, Endpoint>,
    ) -> Result<Job> {
        let eps = config
            .as_table()
            .context("un job de la config no es table")?;
        let name_src = eps
            .get("src")
            .context("un job de la config no hay src")?
            .as_str()
            .context("el src de un job de la config no es una cadena")?;
        let name_dest = eps
            .get("dest")
            .context("un job de la config no hay dest")?
            .as_str()
            .context("el dest de un job de la config no es una cadena")?;
        Ok(Job {
            src: endpoints
                .get(name_src)
                .context("la config no hay un endpoint con este nombre")?
                .clone(),
            dest: endpoints
                .get(name_dest)
                .context("la config no hay un endpoint con este nombre")?
                .clone(),
        })
    }
}

pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Job>> {
    let toml = fs::read_to_string(path).context("no se pudo leer config.toml")?;
    let top = toml
        .parse::<Table>()
        .context("no se pudo analizar la config")?;

    let mut endpoints = HashMap::<String, Endpoint>::new();
    let cfg_endpoints = top
        .get("endpoints")
        .context("la config no tiene algunos endpoints")?
        .as_table()
        .context("los endpoints de la config no es tabla")?;

    for (name, table) in cfg_endpoints {
        endpoints.insert(name.to_string(), Endpoint::from_config(name, table)?);
    }

    let mut jobs = HashMap::<String, Job>::new();
    let cfg_jobs = top
        .get("jobs")
        .context("la config no tiene algunos jobs")?
        .as_table()
        .context("los jobs de la config no es table")?;

    for (name, eps) in cfg_jobs {
        jobs.insert(
            name.to_string(),
            Job::from_config_and_endpoints(eps, &endpoints)?,
        );
    }

    Ok(jobs)
}
