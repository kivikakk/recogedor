use anyhow::{bail, Context, Result};
use lexpr::Value;
use std::fmt::{self, Display, Formatter};

use super::value::{Flag, RecipientPattern};

pub(crate) enum Cond {
    Or(Vec<Cond>),
    Flagged(Flag),
    ReceivedBy(RecipientPattern),
}

impl Display for Cond {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Cond::Or(cx) => {
                f.write_str("(or")?;
                for c in cx {
                    f.write_str(" ")?;
                    c.fmt(f)?;
                }
                f.write_str(")")?;
                Ok(())
            }
            Cond::Flagged(fl) => write!(f, "(flagged {:?})", fl.0),
            Cond::ReceivedBy(p) => write!(f, "(received-by {})", p),
        }
    }
}

impl Cond {
    pub(super) fn from_sexp(sexp: &Value) -> Result<Cond> {
        let vec = sexp.to_vec().context("?")?;
        match vec.get(0).context("?")?.as_symbol().context("?")? {
            "or" => Ok(Cond::Or(
                vec.get(1..)
                    .context("?")?
                    .iter()
                    .map(Cond::from_sexp)
                    .collect::<Result<_>>()?,
            )),
            "flagged" => Ok(Cond::Flagged(
                vec.get(1).context("?")?.as_str().context("?")?.into(),
            )),
            "received-by" => Ok(Cond::ReceivedBy(
                vec.get(1).context("?")?.as_str().context("?")?.try_into()?,
            )),
            s => bail!("unknown (in Cond): {:?}", s),
        }
    }
}
