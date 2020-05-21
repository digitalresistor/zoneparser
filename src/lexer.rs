// Zonefile Format from RFC1035:
//
// The format of these files is a sequence of entries.  Entries are
// predominantly line-oriented, though parentheses can be used to continue
// a list of items across a line boundary, and text literals can contain
// CRLF within the text.  Any combination of tabs and spaces act as a
// delimiter between the separate items that make up an entry.  The end of
// any line in the master file can end with a comment.  The comment starts
// with a ";" (semicolon).
//
// The following entries are defined:
//
//     <blank>[<comment>]
//
//     $ORIGIN <domain-name> [<comment>]
//
//     $INCLUDE <file-name> [<domain-name>] [<comment>]
//
//     <domain-name><rr> [<comment>]
//
//     <blank><rr> [<comment>]
//
// The following entry was added by RFC2309 section 4.
//
//     $TTL <TTL> [<comment>]
//
// Blank lines, with or without comments, are allowed anywhere in the file.
//
// Two control entries are defined: $ORIGIN and $INCLUDE.  $ORIGIN is
// followed by a domain name, and resets the current origin for relative
// domain names to the stated name.  $INCLUDE inserts the named file into
// the current file, and may optionally specify a domain name that sets the
// relative domain name origin for the included file.  $INCLUDE may also
// have a comment.  Note that a $INCLUDE entry never changes the relative
// origin of the parent file, regardless of changes to the relative origin
// made within the included file.
//
// The last two forms represent RRs.  If an entry for an RR begins with a
// blank, then the RR is assumed to be owned by the last stated owner.  If
// an RR entry begins with a <domain-name>, then the owner name is reset.
//
// <rr> contents take one of the following forms:
//
//     [<TTL>] [<class>] <type> <RDATA>
//
//     [<class>] [<TTL>] <type> <RDATA>
//
// The RR begins with optional TTL and class fields, followed by a type and
// RDATA field appropriate to the type and class.  Class and type use the
// standard mnemonics, TTL is a decimal integer.  Omitted class and TTL
// values are default to the last explicitly stated values.  Since type and
// class mnemonics are disjoint, the parse is unique.  (Note that this
// order is different from the order used in examples and the order used in
// the actual RRs; the given order allows easier parsing and defaulting.)
//
// <domain-name>s make up a large share of the data in the master file.
// The labels in the domain name are expressed as character strings and
// separated by dots.  Quoting conventions allow arbitrary characters to be
// stored in domain names.  Domain names that end in a dot are called
// absolute, and are taken as complete.  Domain names which do not end in a
// dot are called relative; the actual domain name is the concatenation of
// the relative part with an origin specified in a $ORIGIN, $INCLUDE, or as
// an argument to the master file loading routine.  A relative name is an
// error when no origin is available.
//
// <character-string> is expressed in one or two ways: as a contiguous set
// of characters without interior spaces, or as a string beginning with a "
// and ending with a ".  Inside a " delimited string any character can
// occur, except for a " itself, which must be quoted using \ (back slash).
//
// Because these files are text files several special encodings are
// necessary to allow arbitrary data to be loaded.  In particular:
//
//                 of the root.
//
// @               A free standing @ is used to denote the current origin.
//
// \X              where X is any character other than a digit (0-9), is
//                 used to quote that character so that its special meaning
//                 does not apply.  For example, "\." can be used to place
//                 a dot character in a label.
//
// \DDD            where each D is a digit is the octet corresponding to
//                 the decimal number described by DDD.  The resulting
//                 octet is assumed to be text and is not checked for
//                 special meaning.
//
// ( )             Parentheses are used to group data that crosses a line
//                 boundary.  In effect, line terminations are not
//                 recognized within parentheses.
//
// ;               Semicolon is used to start a comment; the remainder of
//                 the line is ignored.

use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    zf: Peekable<Chars<'a>>,
    lineno: i32,
    charno: i32,
    state: State,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Token {
    Origin {
        domain_name: String,
        lineno: i32,
    },
    Include {
        file_name: String,
        domain_name: Option<String>,
        lineno: i32,
    },
    TTL {
        ttl: i32,
        lineno: i32,
    },
    Text(String),
    DomainName(String),
    Comment,
    OpenParen,
    CloseParen,
    EOF,
}

#[derive(Clone, PartialEq, Debug)]
enum State {
    StartLine,
    Dollar,
    Origin,
    IncludeFileName,
    IncludeDomainName { file_name: String },
    Ttl,
    DomainName,
    Blank,
    Comment,
    RestOfLine,
    Quote,
    EOL,
    EOF,
}

impl<'a> Lexer<'a> {
    pub fn new(zonefile: &str) -> Lexer {
        Lexer {
            zf: zonefile.chars().peekable(),
            lineno: 0,
            charno: 0,
            state: State::StartLine,
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, &str> {
        let mut chars: Option<String> = None;

        loop {
            let ch = self.zf.peek();

            //println!(
            //    "ch = {:?}; state = {:?}(chars: {:?})",
            //    ch, self.state, chars
            //);

            match self.state {
                State::StartLine => match ch {
                    Some('\r') | Some('\n') => {
                        self.state = State::EOL;
                    }
                    Some(';') => {
                        self.state = State::Comment;
                        return Ok(Some(Token::Comment));
                    }
                    Some('$') => {
                        self.state = State::Dollar;
                        chars = Some(String::new());
                        self.next();
                    }
                    None => return Ok(Some(Token::EOF)),
                    Some(_) => {
                        unimplemented!();
                    }
                },
                State::Dollar => match ch {
                    Some(ch) if ch.is_control() => {
                        return Err("Unexpected control character found");
                    }
                    Some(ch) if !ch.is_whitespace() => {
                        Self::push_to_str(&mut chars, *ch);
                        self.next();
                    }
                    Some(ch) if ch.is_whitespace() => {
                        let dollar: String = chars.take().unwrap();

                        if "INCLUDE" == dollar {
                            self.state = State::IncludeFileName;
                        } else if "ORIGIN" == dollar {
                            self.state = State::Origin;
                        } else if "TTL" == dollar {
                            self.state = State::Ttl;
                        } else {
                            return Err("Unknown control entry");
                        }
                        
                        chars = Some(String::new());
                        self.next();
                    }
                    None | Some('\r') | Some('\n') | Some(_) => {
                        return Err("Unexpected end of line");
                    }
                },
                State::Origin => match ch {
                    Some(ch) if !ch.is_control() && !ch.is_whitespace() => {
                        Self::push_to_str(&mut chars, *ch);
                        self.next();
                    }
                    None | Some('\r') | Some('\n') | Some(_) => {
                        self.state = State::RestOfLine;
                        let domain_name = chars.take().unwrap_or_else(|| "".into());
                        return Ok(Some(Token::Origin { domain_name: domain_name, lineno: self.lineno }));
                    }
                }
                State::Comment => {
                    self.state = State::RestOfLine;
                    chars = Some(String::new());
                    self.next();
                }
                State::RestOfLine => match ch {
                    None | Some('\r') | Some('\n') => {
                        self.state = State::EOL;
                        return Ok(Some(Token::Text(chars.take().unwrap_or_else(|| "".into()))));
                    }
                    Some(ch) if ch.is_control() => {
                        return Err("Unexpected control character found");
                    }
                    Some(ch) => {
                        Self::push_to_str(&mut chars, *ch);
                        self.next();
                    }
                },
                State::EOL => {
                    match ch {
                        Some('\r') => {
                            self.next();
                        }
                        Some('\n') => {
                            self.lineno += 1;
                            self.charno = 0;
                            self.next();
                            self.state = State::StartLine;
                        }
                        // Shut the compiler up, _ won't ever match
                        Some(_) | None => {
                            return Ok(Some(Token::EOF));
                        }
                    }
                }
                _ => {
                    unimplemented!();
                }
            }
        }
    }

    fn next(&mut self) {
        self.zf.next();
        self.charno += 1;
    }

    fn push_to_str(chars: &mut Option<String>, ch: char) {
        chars.as_mut().unwrap().push(ch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn push_to_str_none() {
        let mut chars: Option<String> = None;

        Lexer::push_to_str(&mut chars, 'a');
    }

    #[test]
    fn push_to_str() {
        let mut chars: Option<String> = Some(String::from("test"));

        Lexer::push_to_str(&mut chars, 'i');
        Lexer::push_to_str(&mut chars, 'n');
        Lexer::push_to_str(&mut chars, 'g');
        assert_eq!(chars.unwrap(), "testing");
    }

    #[test]
    fn comment_only() {
        let zonefile = "; this is a comment\n";
        let mut lexer = Lexer::new(zonefile);
        assert_eq!(lexer.next_token(), Ok(Some(Token::Comment)));
        assert_eq!(
            lexer.next_token(),
            Ok(Some(Token::Text(" this is a comment".into())))
        );
        assert_eq!(lexer.next_token(), Ok(Some(Token::EOF)));
    }

    #[test]
    fn multiple_comment() {
        let zonefile = "; this is a comment\n; this is another comment";
        let mut lexer = Lexer::new(zonefile);
        assert_eq!(lexer.next_token(), Ok(Some(Token::Comment)));
        assert_eq!(
            lexer.next_token(),
            Ok(Some(Token::Text(" this is a comment".into())))
        );
        assert_eq!(lexer.next_token(), Ok(Some(Token::Comment)));
        assert_eq!(
            lexer.next_token(),
            Ok(Some(Token::Text(" this is another comment".into())))
        );
    }

    #[test]
    fn whitespace() {
        let zonefile = "\r\n\r\n";
        let mut lexer = Lexer::new(zonefile);
        assert_eq!(lexer.next_token(), Ok(Some(Token::EOF)));
    }

    #[test]
    fn origin_only() {
        let zonefile = "$ORIGIN cidr.network.";
        let mut lexer = Lexer::new(zonefile);
        assert_eq!(lexer.next_token(), Ok(Some(Token::Origin { domain_name: "cidr.network.".into(), lineno: 0 })));
    }

    #[test]
    fn origin_with_comment() {
        let zonefile = "$ORIGIN cidr.network. ; this is a comment";
        let mut lexer = Lexer::new(zonefile);
        assert_eq!(lexer.next_token(), Ok(Some(Token::Origin { domain_name: "cidr.network.".into(), lineno: 0 })));
        assert_eq!(lexer.next_token(), Ok(Some(Token::Comment)));
    }

}