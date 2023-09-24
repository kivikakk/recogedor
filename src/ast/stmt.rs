use anyhow::{bail, Context, Result};
use lexpr::Value;
use std::fmt::{self, Display, Formatter};

use super::cond::Cond;
use super::value::{Destination, Flag};

pub(crate) enum Stmt {
    If(Cond, Box<Stmt>, Option<Box<Stmt>>),
    Append(Destination),
    Flag(Flag),
    Halt,
}

impl Display for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.pp(f, 0)
    }
}

const INDENT: usize = 4;

impl Stmt {
    pub(super) fn pp(&self, f: &mut Formatter<'_>, indent: usize) -> fmt::Result {
        match self {
            Stmt::If(c, t, e) => {
                write!(f, "\n{}(if {}", " ".repeat(indent * INDENT), c)?;
                t.pp(f, indent + 1)?;
                if let Some(e) = e {
                    e.pp(f, indent + 1)?;
                }
                f.write_str(")")?;
                Ok(())
            }
            Stmt::Append(d) => write!(f, "\n{}(append! {:?})", " ".repeat(indent * INDENT), d.0),
            Stmt::Flag(fl) => write!(f, "\n{}(flag! {:?})", " ".repeat(indent * INDENT), fl.0),
            Stmt::Halt => write!(f, "\n{}(halt!)", " ".repeat(indent * INDENT)),
        }
    }

    pub(crate) fn from_sexp(sexp: &Value) -> Result<Stmt> {
        let vec = sexp.to_vec().context("stmt isn't cons")?;
        let head = vec
            .get(0)
            .context("stmt cons empty")?
            .as_symbol()
            .context("stmt car isn't sym")?;
        match head {
            "if" => Ok(Stmt::If(
                Cond::from_sexp(vec.get(1).context("'if' statement missing condition")?)?,
                Box::new(Stmt::from_sexp(
                    vec.get(2).context("'if' statement missing 'then'")?,
                )?),
                // Someone help me budget my family is dying
                // Please help me vec.get(3).map my way through this.
                match vec.get(3) {
                    Some(s) => Some(Box::new(Stmt::from_sexp(s)?)),
                    None => None,
                },
            )),
            "append!" => Ok(Stmt::Append(
                vec.get(1).context("?")?.as_str().context("?")?.into(),
            )),
            "flag!" => Ok(Stmt::Flag(
                vec.get(1).context("?")?.as_str().context("?")?.into(),
            )),
            "halt!" => Ok(Stmt::Halt),
            s => bail!("unknown (in Stmt): {:?}", s),
        }
    }
}
