use anyhow::{bail, Result};
use std::{collections::HashMap, fmt, str};

use crate::ast::{Cond, Destination, Flag, RecipientPattern, Stmt};
use crate::endpoint::Endpoint;

mod closure;
use closure::Closure;

pub(crate) struct IR {
    insns: Vec<Insn>,
    dests: Vec<Endpoint>,
}

impl IR {
    pub(super) fn compile(stmts: &[Stmt], dests: HashMap<String, Endpoint>) -> Result<IR> {
        IRCompiler::compile(stmts, dests)
    }

    pub(crate) fn closure(&self) -> Closure {
        Closure::new(&self)
    }
}

impl fmt::Display for IR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\n")?;

        let mut ix = 0;
        for insn in &self.insns {
            write!(f, "{:02x} {}\n", ix, insn)?;
            ix += 1;
        }
        Ok(())
    }
}

struct IRCompiler {
    i_dests: HashMap<String, Endpoint>,

    insns: Vec<Insn>,
    dests: Vec<Endpoint>,

    dest_mappings: HashMap<String, usize>,
}

impl IRCompiler {
    fn compile(stmts: &[Stmt], dests: HashMap<String, Endpoint>) -> Result<IR> {
        let mut irc = IRCompiler {
            i_dests: dests,
            insns: vec![],
            dests: vec![],
            dest_mappings: HashMap::new(),
        };

        for stmt in stmts {
            irc.compile_stmt(stmt)?;
        }

        Ok(IR {
            insns: irc.insns,
            dests: irc.dests,
        })
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::If(c, t, e) => {
                self.compile_cond(c)?;

                let else_target = self.insns.len();
                self.insns.push(Insn::JumpFalse(0));

                self.compile_stmt(t)?;
                if let Some(e) = e {
                    let done_target = self.insns.len();
                    self.insns.push(Insn::Jump(0));

                    self.insns[else_target] = Insn::JumpFalse(self.insns.len());
                    self.compile_stmt(e)?;

                    self.insns[done_target] = Insn::Jump(self.insns.len());
                } else {
                    self.insns[else_target] = Insn::JumpFalse(self.insns.len());
                }
            }
            Stmt::Append(dn) => {
                self.compile_dest(dn)?;
                self.insns.push(Insn::Append);
            }
            Stmt::Flag(fl) => {
                self.compile_flag(fl)?;
                self.insns.push(Insn::Flag);
            }
            Stmt::Halt => self.insns.push(Insn::Halt),
        }
        Ok(())
    }

    fn compile_dest(&mut self, dn: &Destination) -> Result<()> {
        let ix = if let Some(ix) = self.dest_mappings.get(&dn.0) {
            *ix
        } else if let Some(dest) = self.i_dests.remove(&dn.0) {
            let ix = self.dests.len();
            self.dests.push(dest);
            self.dest_mappings.insert(dn.0.to_owned(), ix);
            ix
        } else {
            bail!("unknown destination {:?}", dn.0);
        };
        self.insns.push(Insn::LiteralDest(ix));
        Ok(())
    }

    fn compile_cond(&mut self, cond: &Cond) -> Result<()> {
        match cond {
            Cond::Or(cx) => {
                if cx.len() == 0 {
                    bail!("or needs at least one argument");
                }

                let mut ix = cx.len() - 1;
                self.compile_cond(&cx[ix])?;
                loop {
                    if ix == 0 {
                        break;
                    }
                    ix -= 1;
                    self.compile_cond(&cx[ix])?;
                    self.insns.push(Insn::Or);
                }
            }
            Cond::Flagged(fl) => {
                self.compile_flag(fl)?;
                self.insns.push(Insn::Flagged);
            }
            Cond::ReceivedBy(p) => {
                self.compile_recipient_pattern(p)?;
                self.insns.push(Insn::ReceivedBy);
            }
        };
        Ok(())
    }

    fn compile_flag(&mut self, fl: &Flag) -> Result<()> {
        self.insns.push(Insn::LiteralFlag(fl.0.to_owned()));
        Ok(())
    }

    fn compile_recipient_pattern(&mut self, p: &RecipientPattern) -> Result<()> {
        self.insns.push(Insn::LiteralRecipientPattern(
            p.mailbox.to_owned(),
            p.plus.to_owned(),
            p.host.to_owned(),
        ));
        Ok(())
    }
}

enum Insn {
    LiteralFlag(String),
    LiteralRecipientPattern(Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>),
    LiteralDest(usize),

    Flagged,
    ReceivedBy,
    Or,

    Append,
    Flag,
    Halt,

    Jump(usize),
    JumpFalse(usize),
}

impl fmt::Display for Insn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Insn::LiteralFlag(fl) => write!(f, "f{:?}", fl),
            Insn::LiteralRecipientPattern(mailbox, plus, host) => {
                f.write_str("rp\"")?;
                if let Some(mailbox) = mailbox {
                    f.write_str(str::from_utf8(&mailbox).unwrap())?;
                }
                if let Some(plus) = plus {
                    write!(f, "+{}", str::from_utf8(&plus).unwrap())?;
                }
                f.write_str("@")?;
                if let Some(host) = host {
                    f.write_str(str::from_utf8(&host).unwrap())?;
                }
                f.write_str("\"")?;
                Ok(())
            }
            Insn::LiteralDest(dn) => write!(f, "d{}", dn),

            Insn::Flagged => f.write_str("flagged?"),
            Insn::ReceivedBy => f.write_str("received-by?"),
            Insn::Or => f.write_str("or"),

            Insn::Append => f.write_str("append!"),
            Insn::Flag => f.write_str("flag!"),
            Insn::Halt => f.write_str("halt!"),

            Insn::Jump(d) => write!(f, "j {:02x}", d),
            Insn::JumpFalse(d) => write!(f, "jfalse {:02x}", d),
        }
    }
}

// 00 f"Recogido"
// 01 flagged?
// 02 jfalse 04
// 03 halt!
// 04 rp"fox@den.com"
// 05 received-by?
// 06 rp"fox@foxden.net"
// 07 received-by?
// 08 or
// 09 rp"fx@"
// 0a received-by?
// 0b or
// 0c jfalse 10
// 0d d0
// 0e append!
// 0f j 12
// 10 d1
// 11 append!
// 12 f"Recogido"
// 13 flag!
