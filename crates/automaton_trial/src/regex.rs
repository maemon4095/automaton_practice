#[derive(Debug)]
pub struct Regex {
    pub root: RegexNode,
}

#[derive(Debug)]
pub enum RegexNode {
    Atom(RegexAtom),
    Repeat(Box<RegexRepeat>),
    Or(Box<RegexOr>),
    Join(Box<RegexJoin>),
}

#[derive(Debug)]
pub struct RegexAtom {
    pub literal: String,
}

// p *
#[derive(Debug)]
pub struct RegexRepeat {
    pub pattern: RegexNode,
}

// p0 | p1
#[derive(Debug)]
pub struct RegexOr {
    pub left: RegexNode,
    pub right: RegexNode,
}

// p0 p1
#[derive(Debug)]
pub struct RegexJoin {
    pub left: RegexNode,
    pub right: RegexNode,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("input is empty.")]
    Empty,
    #[error("unexpected end of input.")]
    UnexpectedEnd,
    #[error("unexpected token.")]
    UnexpectedToken,
}

impl std::str::FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (root, _) = parse(s, None)?;
        let regex = Self { root };
        Ok(regex)
    }
}

fn parse<'a>(s: &'a str, terminal: Option<&Token>) -> Result<(RegexNode, &'a str), Error> {
    let (token, rest) = match take_token(s) {
        Some(e) => e,
        None => return Err(Error::Empty),
    };

    let (mut node, mut rest) = match token {
        Token::Literal(literal) => {
            let p = RegexNode::Atom(RegexAtom { literal });
            (p, rest)
        }
        Token::LParen => {
            let (p, rest) = parse(rest, terminal)?;
            match take_token(rest) {
                Some((Token::RParen, rest)) => (p, rest),
                Some(_) => return Err(Error::UnexpectedToken),
                None => return Err(Error::UnexpectedEnd),
            }
        }
        _ => return Err(Error::UnexpectedToken),
    };

    match take_token(rest) {
        Some((Token::Asterisk, r)) => {
            node = RegexNode::Repeat(Box::new(RegexRepeat { pattern: node }));
            rest = r;
        }
        _ => (),
    };

    loop {
        let (token, r) = match take_token(rest) {
            Some(v) => v,
            None => break,
        };

        match token {
            t if Some(&t) == terminal => break,
            Token::VerticalBar => match parse(r, None) {
                Ok((right, r)) => {
                    let p = RegexOr { left: node, right };
                    node = RegexNode::Or(Box::new(p));
                    rest = r;
                }
                Err(Error::Empty) => return Err(Error::UnexpectedEnd),
                Err(e) => return Err(e),
            },
            Token::RParen => break,

            _ => {
                match parse(rest, Some(&Token::VerticalBar)) {
                    Ok((right, r)) => {
                        let p = RegexJoin { left: node, right };
                        node = RegexNode::Join(Box::new(p));
                        rest = r;
                    }
                    Err(e) => return Err(e),
                };
            }
        };
    }

    Ok((node, rest))
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Literal(String),
    LParen,
    RParen,
    Asterisk,
    VerticalBar,
}

fn take_token(s: &str) -> Option<(Token, &str)> {
    let mut chars = s.chars();
    let Some(c) = chars.next() else {
        return None;
    };

    let t = match c {
        '(' => (Token::LParen, chars.as_str()),
        ')' => (Token::RParen, chars.as_str()),
        '*' => (Token::Asterisk, chars.as_str()),
        '|' => (Token::VerticalBar, chars.as_str()),
        _ => {
            let len = s
                .find(|c| match c {
                    '(' | ')' | '*' | '|' => true,
                    _ => false,
                })
                .unwrap_or(s.len());

            let (lit, rest) = s.split_at(len);
            (Token::Literal(lit.to_string()), rest)
        }
    };

    Some(t)
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_take_token_must_return_none_on_input_is_empty() {
        let r = take_token("");
        assert!(r.is_none())
    }

    #[test]
    fn test_take_token_must_return_lit_on_input_starts_with_ordinal_characters() {
        const PREFIX: &str = "abcde";
        const SUFFIX: &str = "fgh";
        const SPECIAL_CHARS: &[char] = &['(', ')', '*', '|'];

        for c in SPECIAL_CHARS {
            let input = format!("{PREFIX}{}{SUFFIX}", c);
            let (token, rest) = take_token(&input).unwrap();
            assert_eq!(token, Token::Literal(PREFIX.into()));
            assert_eq!(rest, &format!("{}{SUFFIX}", c));
        }
    }

    #[test]
    fn test_take_token_must_return_corresponding_token_on_input_starts_with_special_char() {
        const SUFFIX: &str = "fgh";
        const SPECIALS: &[(char, Token)] = &[
            ('(', Token::LParen),
            (')', Token::RParen),
            ('*', Token::Asterisk),
            ('|', Token::VerticalBar),
        ];

        for (c, expected) in SPECIALS {
            let input = format!("{}{SUFFIX}", c);
            let (token, rest) = take_token(&input).unwrap();
            assert_eq!(&token, expected);
            assert_eq!(rest, &format!("{SUFFIX}"));
        }
    }

    #[test]
    fn test_parse_fails_if_input_contains_repeated_asterisk() {
        let r = Regex::from_str("a**");

        assert_eq!(r.unwrap_err(), Error::UnexpectedToken);
    }
}
