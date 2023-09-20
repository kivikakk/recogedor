use anyhow::{bail, Context, Error, Result};
use once_cell::sync::Lazy;
use regex::bytes::Regex;
use std::{
    fmt::{self, Display, Formatter},
    str,
};

use crate::endpoint::Recipient;

pub(crate) struct Flag(pub(super) String);

impl From<&str> for Flag {
    fn from(s: &str) -> Flag {
        Flag(s.into())
    }
}

pub(crate) struct RecipientPattern {
    mailbox: Option<Vec<u8>>,
    plus: Option<Vec<u8>>,
    host: Option<Vec<u8>>,
}

impl RecipientPattern {
    pub(crate) fn matches(&self, recipient: &Recipient) -> bool {
        let r_pluspos = recipient.mailbox.iter().position(|&c| c == b'+');
        if let Some(p_mailbox) = &self.mailbox {
            if let Some(r_pluspos) = r_pluspos {
                if !Self::parts_equal(p_mailbox, &recipient.mailbox[..r_pluspos]) {
                    return false;
                }
            } else if !Self::parts_equal(p_mailbox, &recipient.mailbox) {
                return false;
            }
        }

        if let Some(p_plus) = &self.plus {
            if let Some(r_pluspos) = r_pluspos {
                if !Self::parts_equal(p_plus, &recipient.mailbox[r_pluspos + 1..]) {
                    return false;
                }
            } else if p_plus.len() != 0 {
                return false;
            }
        }

        if let Some(p_host) = &self.host {
            if !Self::parts_equal(p_host, &recipient.host) {
                return false;
            }
        }

        true
    }

    fn parts_equal(a: &[u8], b: &[u8]) -> bool {
        a.iter()
            .map(Self::part_lower)
            .zip(b.iter().map(Self::part_lower))
            .all(|(x, y)| x == y)
    }

    fn part_lower(c: &u8) -> u8 {
        match *c {
            b'A'..=b'Z' => c + 32,
            c => c,
        }
    }
}

impl Display for RecipientPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        if let Some(mailbox) = &self.mailbox {
            f.write_str(str::from_utf8(mailbox).unwrap())?;
        }
        if let Some(plus) = &self.plus {
            write!(f, "+{}", str::from_utf8(plus).unwrap())?;
        }
        f.write_str("@")?;
        if let Some(host) = &self.host {
            f.write_str(str::from_utf8(host).unwrap())?;
        }
        f.write_str("\"")?;
        Ok(())
    }
}

impl std::convert::TryFrom<&str> for RecipientPattern {
    type Error = Error;
    fn try_from(s: &str) -> Result<RecipientPattern> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\A(?<mailbox>[^+@]+)?(?:\+(?<plus>[^@]+))?@(?<host>.+)?\z").unwrap()
        });
        let captures = RE.captures(s.as_bytes()).context("pattern syntax error")?;

        let mailbox = captures.name("mailbox").map(|m| m.as_bytes().to_vec());
        let plus = captures.name("plus").map(|m| m.as_bytes().to_vec());
        let host = captures.name("host").map(|m| m.as_bytes().to_vec());

        if mailbox.is_none() && plus.is_none() && host.is_none() {
            bail!("pattern needs to match something");
        }

        Ok(RecipientPattern {
            mailbox,
            plus,
            host,
        })
    }
}

pub(crate) struct Destination(pub(super) String);

impl From<&str> for Destination {
    fn from(s: &str) -> Destination {
        Destination(s.into())
    }
}
