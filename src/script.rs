use anyhow::Result;
use std::{collections::hash_map::HashMap, str};

use crate::ast::Stmt;
use crate::endpoint::Endpoint;
use crate::ir::IR;

pub(crate) fn compile(text: &str, dests: HashMap<String, Endpoint>) -> Result<IR> {
    let parser = lexpr::Parser::from_reader(text.as_bytes());
    let mut stmts = vec![];
    for sexp in parser {
        stmts.push(Stmt::from_sexp(&sexp?)?);
    }
    IR::compile(&stmts, dests)
}
