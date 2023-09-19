use anyhow::{bail, Context, Error, Result};
use lexpr::Value;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use crate::endpoint::{Endpoint, EndpointWriter, Message};

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

enum Stmt {
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
    fn pp(&self, f: &mut Formatter<'_>, indent: usize) -> fmt::Result {
        match self {
            Stmt::If(c, t, e) => {
                write!(f, "{}(if {}", " ".repeat(indent * INDENT), c)?;
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

    fn from_sexp(sexp: &Value) -> Result<Stmt> {
        let vec = sexp.to_vec().context("sexp no es cons")?;
        let head = vec
            .get(0)
            .context("cons no tiene unos elementos")?
            .as_symbol()
            .context("el primer elemento no es un símbolo")?;
        match head {
            "if" => Ok(Stmt::If(
                Cond::from_sexp(
                    vec.get(1)
                        .context("la declaración \"if\" no tiene condición")?,
                )?,
                Box::new(Stmt::from_sexp(vec.get(2).context(
                    "la declaración \"if\" no tiene una declaración \"then\"",
                )?)?),
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
            s => bail!("no sé qué esto es (en Stmt): {:?}", s),
        }
    }
}

enum Cond {
    Or(Vec<Cond>),
    Flagged(Flag),
    ReceivedBy(Pattern),
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
    fn eval(&self, mail: &Message) -> bool {
        match self {
            Cond::Or(cx) => cx.iter().any(|c| c.eval(mail)),
            Cond::Flagged(fl) => mail.flagged(&fl.0),
            Cond::ReceivedBy(p) => mail.received_by(p.into()),
        }
    }

    fn from_sexp(sexp: &Value) -> Result<Cond> {
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
            s => bail!("no sé qué esto es (en Cond): {:?}", s),
        }
    }
}

struct Flag(String);

impl From<&str> for Flag {
    fn from(s: &str) -> Flag {
        Flag(s.into())
    }
}

pub(crate) struct Pattern {
    mailbox: Option<String>,
    plus: Option<String>,
    host: Option<String>,
}

impl Display for Pattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        if let Some(mailbox) = &self.mailbox {
            f.write_str(mailbox)?;
        }
        if let Some(plus) = &self.plus {
            write!(f, "+{}", plus)?;
        }
        f.write_str("@")?;
        if let Some(host) = &self.host {
            f.write_str(host)?;
        }
        f.write_str("\"")?;
        Ok(())
    }
}

impl std::convert::TryFrom<&str> for Pattern {
    type Error = Error;
    fn try_from(s: &str) -> Result<Pattern> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\A(?<mailbox>[^+@]+)?(?:\+(?<plus>[^@]+))?@(?<host>.+)?\z").unwrap()
        });
        let captures = RE.captures(s).context("pattern syntax error")?;

        let mailbox = captures.name("mailbox").map(|m| m.as_str().to_string());
        let plus = captures.name("plus").map(|m| m.as_str().to_string());
        let host = captures.name("host").map(|m| m.as_str().to_string());

        if mailbox.is_none() && plus.is_none() && host.is_none() {
            bail!("pattern needs to match something");
        }

        Ok(Pattern {
            mailbox,
            plus,
            host,
        })
    }
}

struct Destination(String);

impl From<&str> for Destination {
    fn from(s: &str) -> Destination {
        Destination(s.into())
    }
}

pub(crate) struct Closure<'s, 'd> {
    script: &'s Script,
    dests: &'d HashMap<String, Endpoint>,
    connected_dests: HashMap<String, Box<dyn EndpointWriter>>,
}

impl<'s, 'd> Closure<'s, 'd> {
    fn new(script: &'s Script, dests: &'d HashMap<String, Endpoint>) -> Closure<'s, 'd> {
        Closure {
            script,
            dests,
            connected_dests: HashMap::new(),
        }
    }

    pub(crate) fn process(&mut self, mail: Message) -> Result<Vec<Action>> {
        let mut actions = vec![];
        for stmt in &self.script.0 {
            let done = self.process_stmt(&mail, stmt, &mut actions)?;
            if done {
                break;
            }
        }
        Ok(actions)
    }

    fn process_stmt(
        &mut self,
        mail: &Message,
        stmt: &Stmt,
        actions: &mut Vec<Action>,
    ) -> Result<bool> {
        match stmt {
            Stmt::If(c, t, e) => {
                if c.eval(mail) {
                    self.process_stmt(mail, &t, actions)
                } else if let Some(e) = e {
                    self.process_stmt(mail, &e, actions)
                } else {
                    Ok(false)
                }
            }
            Stmt::Append(dn) => unreachable!(),
            Stmt::Flag(fl) => unreachable!(),
            Stmt::Halt => Ok(true),
        }
    }

    pub(crate) async fn finish(mut self) -> Result<()> {
        for (_, dest) in &mut self.connected_dests {
            dest.disconnect().await.context("desconectando")?;
        }
        Ok(())
    }
}

pub(crate) enum Action {}
