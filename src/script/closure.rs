use crate::endpoint::{Endpoint, EndpointReader, EndpointWriter, Message};
use anyhow::{bail, Context, Result};
use std::{
    collections::hash_map::{Entry, HashMap},
    str,
};

use super::stmt::Stmt;
use super::value::{Destination, Flag};
use super::Script;

pub(crate) struct Closure<'s, 'd> {
    script: &'s Script,
    dests: &'d HashMap<String, Endpoint>,
    connected_dests: HashMap<String, Box<dyn EndpointWriter>>,
}

impl<'s, 'd> Closure<'s, 'd> {
    pub(super) fn new(script: &'s Script, dests: &'d HashMap<String, Endpoint>) -> Closure<'s, 'd> {
        Closure {
            script,
            dests,
            connected_dests: HashMap::new(),
        }
    }

    async fn dest(&mut self, key: &str) -> Result<&mut Box<dyn EndpointWriter>> {
        match self.connected_dests.entry(key.to_string()) {
            Entry::Occupied(oe) => Ok(oe.into_mut()),
            Entry::Vacant(ve) => {
                let ep = self
                    .dests
                    .get(key)
                    .context("internal error: unknown dest from closure")?;
                let wr = ep.connect_writer().await?;
                Ok(ve.insert(wr))
            }
        }
    }

    pub(crate) fn process(&self, mail: &Message) -> Result<Vec<Action<'s>>> {
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
        &self,
        mail: &Message,
        stmt: &'s Stmt,
        actions: &mut Vec<Action<'s>>,
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
            Stmt::Append(dn) => {
                if !self.dests.contains_key(&dn.0) {
                    bail!("unknown destination {:?}", dn.0);
                }
                actions.push(Action::Append(dn));
                Ok(false)
            }
            Stmt::Flag(fl) => {
                actions.push(Action::Flag(fl));
                Ok(false)
            }
            Stmt::Halt => Ok(true),
        }
    }

    pub(crate) async fn action(
        &mut self,
        mail: &Message,
        action: Action<'_>,
        src: &mut Box<dyn EndpointReader>,
    ) -> Result<()> {
        match action {
            Action::Append(dn) => self.dest(&dn.0).await?.append(mail).await,
            Action::Flag(fl) => src.flag(mail.uid, &fl.0).await,
        }
    }

    pub(crate) async fn finish(mut self) -> Result<()> {
        for (_, dest) in &mut self.connected_dests {
            dest.disconnect().await.context("disconnecting")?;
        }
        Ok(())
    }
}

pub(crate) enum Action<'s> {
    Append(&'s Destination),
    Flag(&'s Flag),
}
