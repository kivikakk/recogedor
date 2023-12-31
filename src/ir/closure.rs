use anyhow::{bail, Context, Result};

use super::{Insn, IR};
use crate::ast::RecipientPattern;
use crate::endpoint::{DestinationEndpoint, Message, SourceEndpoint};

pub(crate) struct Closure<'i> {
    ir: &'i IR,
    folder: String,
    slots: Vec<Option<Slot>>,
    src_needs_expunge: bool,
}

struct Slot {
    dest: Box<dyn DestinationEndpoint>,
}

impl<'i> Closure<'i> {
    pub(super) fn new(ir: &'i IR, folder: &str) -> Closure<'i> {
        Closure {
            ir,
            folder: folder.to_string(),
            slots: ir.dests.iter().map(|_| None).collect(),
            src_needs_expunge: false,
        }
    }

    async fn slot(&mut self, ix: usize) -> Result<&mut Slot> {
        let slot = self.slots.get_mut(ix).unwrap();
        if let Some(ep) = slot {
            return Ok(ep);
        }

        *slot = Some(Slot {
            dest: self.ir.dests[ix].connect_destination().await?,
        });
        Ok(slot.as_mut().unwrap())
    }

    pub(crate) async fn process(
        &mut self,
        mail: &Message,
        src: &mut Box<dyn SourceEndpoint>,
    ) -> Result<()> {
        let mut stack = Stack::new();
        let mut pc: usize = 0;

        while pc < self.ir.insns.len() {
            let insn = &self.ir.insns[pc];

            match insn {
                Insn::LiteralFlag(fl) => stack.push(Value::Flag(fl.to_string())),
                Insn::LiteralRecipientPattern(mailbox, plus, host) => {
                    stack.push(Value::RecipientPattern(RecipientPattern {
                        mailbox: mailbox.to_owned(),
                        plus: plus.to_owned(),
                        host: host.to_owned(),
                    }))
                }
                &Insn::LiteralDest(dn) => stack.push(Value::Destination(dn)),

                Insn::Flagged => {
                    let fl = stack.pop_flag()?;
                    stack.push(Value::Cond(mail.flagged(&fl)));
                }
                Insn::ReceivedBy => {
                    let p = stack.pop_recipient_pattern()?;
                    stack.push(Value::Cond(mail.received_by(&p)));
                }
                Insn::Or => {
                    let c1 = stack.pop_cond()?;
                    let c2 = stack.pop_cond()?;
                    stack.push(Value::Cond(c1 || c2));
                }

                Insn::Append => {
                    let ix = stack.pop_destination()?;
                    let folder = self.folder.to_string();
                    self.slot(ix).await?.dest.append(&folder, mail).await?;
                }
                Insn::Flag => {
                    let fl = stack.pop_flag()?;
                    src.flag(mail.uid, &fl).await?;
                }
                Insn::Halt => break,
                Insn::Delete => {
                    src.delete(mail.uid).await?;
                    self.src_needs_expunge = true;
                }

                &Insn::Jump(t) => {
                    pc = t;
                    continue;
                }
                &Insn::JumpFalse(t) => {
                    let cond = stack.pop_cond()?;
                    if !cond {
                        pc = t;
                        continue;
                    }
                }
            }

            pc += 1;
        }

        Ok(())
    }

    pub(crate) async fn finish(mut self) -> Result<bool> {
        for slot in self.slots.iter_mut().flatten() {
            slot.dest.disconnect().await.context("disconnecting")?;
        }
        Ok(self.src_needs_expunge)
    }
}

pub(crate) enum Value {
    Flag(String),
    RecipientPattern(RecipientPattern),
    Destination(usize),
    Cond(bool),
}

struct Stack(Vec<Value>);

impl Stack {
    fn new() -> Stack {
        Stack(vec![])
    }

    fn push(&mut self, value: Value) {
        self.0.push(value)
    }

    fn pop(&mut self) -> Result<Value> {
        self.0.pop().context("popped empty stack")
    }

    fn pop_cond(&mut self) -> Result<bool> {
        match self.pop()? {
            Value::Cond(b) => Ok(b),
            _ => bail!("top of stack wasn't cond"),
        }
    }

    fn pop_destination(&mut self) -> Result<usize> {
        match self.pop()? {
            Value::Destination(ix) => Ok(ix),
            _ => bail!("top of stack wasn't dest"),
        }
    }

    fn pop_flag(&mut self) -> Result<String> {
        match self.pop()? {
            Value::Flag(fl) => Ok(fl),
            _ => bail!("top of stack wasn't flag"),
        }
    }

    fn pop_recipient_pattern(&mut self) -> Result<RecipientPattern> {
        match self.pop()? {
            Value::RecipientPattern(rp) => Ok(rp),
            _ => bail!("top of stack wasn't recipient pattern"),
        }
    }
}
