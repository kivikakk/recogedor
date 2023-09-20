use anyhow::Result;
use std::{
    collections::hash_map::HashMap,
    fmt::{self, Display, Formatter},
    str,
};

use crate::endpoint::Endpoint;

mod closure;
mod cond;
mod stmt;
mod value;

use closure::Closure;
use stmt::Stmt;
pub(crate) use value::RecipientPattern;

pub(crate) struct Script(Vec<Stmt>);

impl Display for Script {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n")?;
        for s in &self.0 {
            s.pp(f, 0)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl Script {
    pub(crate) fn parse(text: &str) -> Result<Script> {
        let parser = lexpr::Parser::from_reader(text.as_bytes());
        let mut stmts = vec![];
        for sexp in parser {
            stmts.push(Stmt::from_sexp(&sexp?)?);
        }
        Ok(Script(stmts))
    }

    pub(crate) fn closure<'s, 'd>(
        &'s self,
        dests: &'d HashMap<String, Endpoint>,
    ) -> Closure<'s, 'd> {
        Closure::new(self, dests)
    }
}
