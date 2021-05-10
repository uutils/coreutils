// This file is part of the uutils coreutils package.
//
// (c) Daniel Rocco <drocco@gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::ffi::OsString;
use std::iter::Peekable;

/// Represents a parsed token from a test expression
#[derive(Debug, PartialEq)]
pub enum Symbol {
    LParen,
    Bang,
    BoolOp(OsString),
    Literal(OsString),
    StringOp(OsString),
    IntOp(OsString),
    FileOp(OsString),
    StrlenOp(OsString),
    FiletestOp(OsString),
    None,
}

impl Symbol {
    /// Create a new Symbol from an OsString.
    ///
    /// Returns Symbol::None in place of None
    fn new(token: Option<OsString>) -> Symbol {
        match token {
            Some(s) => match s.to_string_lossy().as_ref() {
                "(" => Symbol::LParen,
                "!" => Symbol::Bang,
                "-a" | "-o" => Symbol::BoolOp(s),
                "=" | "!=" => Symbol::StringOp(s),
                "-eq" | "-ge" | "-gt" | "-le" | "-lt" | "-ne" => Symbol::IntOp(s),
                "-ef" | "-nt" | "-ot" => Symbol::FileOp(s),
                "-n" | "-z" => Symbol::StrlenOp(s),
                "-b" | "-c" | "-d" | "-e" | "-f" | "-g" | "-G" | "-h" | "-k" | "-L" | "-O"
                | "-p" | "-r" | "-s" | "-S" | "-t" | "-u" | "-w" | "-x" => Symbol::FiletestOp(s),
                _ => Symbol::Literal(s),
            },
            None => Symbol::None,
        }
    }

    /// Convert this Symbol into a Symbol::Literal, useful for cases where
    /// test treats an operator as a string operand (test has no reserved
    /// words).
    ///
    /// # Panics
    ///
    /// Panics if `self` is Symbol::None
    fn into_literal(self) -> Symbol {
        Symbol::Literal(match self {
            Symbol::LParen => OsString::from("("),
            Symbol::Bang => OsString::from("!"),
            Symbol::BoolOp(s)
            | Symbol::Literal(s)
            | Symbol::StringOp(s)
            | Symbol::IntOp(s)
            | Symbol::FileOp(s)
            | Symbol::StrlenOp(s)
            | Symbol::FiletestOp(s) => s,
            Symbol::None => panic!(),
        })
    }
}

/// Recursive descent parser for test, which converts a list of OsStrings
/// (typically command line arguments) into a stack of Symbols in postfix
/// order.
///
/// Grammar:
///
///   EXPR → TERM | EXPR BOOLOP EXPR
///   TERM → ( EXPR )
///   TERM → ( )
///   TERM → ! EXPR
///   TERM → UOP str
///   UOP → STRLEN | FILETEST
///   TERM → str OP str
///   TERM → str | 𝜖
///   OP → STRINGOP | INTOP | FILEOP
///   STRINGOP → = | !=
///   INTOP → -eq | -ge | -gt | -le | -lt | -ne
///   FILEOP → -ef | -nt | -ot
///   STRLEN → -n | -z
///   FILETEST → -b | -c | -d | -e | -f | -g | -G | -h | -k | -L | -O | -p |
///               -r | -s | -S | -t | -u | -w | -x
///   BOOLOP → -a | -o
///
#[derive(Debug)]
struct Parser {
    tokens: Peekable<std::vec::IntoIter<OsString>>,
    pub stack: Vec<Symbol>,
}

impl Parser {
    /// Construct a new Parser from a `Vec<OsString>` of tokens.
    fn new(tokens: Vec<OsString>) -> Parser {
        Parser {
            tokens: tokens.into_iter().peekable(),
            stack: vec![],
        }
    }

    /// Fetch the next token from the input stream as a Symbol.
    fn next_token(&mut self) -> Symbol {
        Symbol::new(self.tokens.next())
    }

    /// Peek at the next token from the input stream, returning it as a Symbol.
    /// The stream is unchanged and will return the same Symbol on subsequent
    /// calls to `next()` or `peek()`.
    fn peek(&mut self) -> Symbol {
        Symbol::new(self.tokens.peek().map(|s| s.to_os_string()))
    }

    /// Test if the next token in the stream is a BOOLOP (-a or -o), without
    /// removing the token from the stream.
    fn peek_is_boolop(&mut self) -> bool {
        // TODO: change to `matches!(self.peek(), Symbol::BoolOp(_))` once MSRV is 1.42
        // #[allow(clippy::match_like_matches_macro)] // needs MSRV 1.43
        if let Symbol::BoolOp(_) = self.peek() {
            true
        } else {
            false
        }
    }

    /// Parse an expression.
    ///
    ///   EXPR → TERM | EXPR BOOLOP EXPR
    fn expr(&mut self) {
        if !self.peek_is_boolop() {
            self.term();
        }
        self.maybe_boolop();
    }

    /// Parse a term token and possible subsequent symbols: "(", "!", UOP,
    /// literal, or None.
    fn term(&mut self) {
        let symbol = self.next_token();

        match symbol {
            Symbol::LParen => self.lparen(),
            Symbol::Bang => self.bang(),
            Symbol::StrlenOp(_) => self.uop(symbol),
            Symbol::FiletestOp(_) => self.uop(symbol),
            Symbol::None => self.stack.push(symbol),
            literal => self.literal(literal),
        }
    }

    /// Parse a (possibly) parenthesized expression.
    ///
    /// test has no reserved keywords, so "(" will be interpreted as a literal
    /// if it is followed by nothing or a comparison operator OP.
    fn lparen(&mut self) {
        match self.peek() {
            // lparen is a literal when followed by nothing or comparison
            Symbol::None | Symbol::StringOp(_) | Symbol::IntOp(_) | Symbol::FileOp(_) => {
                self.literal(Symbol::Literal(OsString::from("(")));
            }
            // empty parenthetical
            Symbol::Literal(s) if s == ")" => {}
            _ => {
                self.expr();
                match self.next_token() {
                    Symbol::Literal(s) if s == ")" => (),
                    _ => panic!("expected ‘)’"),
                }
            }
        }
    }

    /// Parse a (possibly) negated expression.
    ///
    /// Example cases:
    ///
    /// * `! =`: negate the result of the implicit string length test of `=`
    /// * `! = foo`: compare the literal strings `!` and `foo`
    /// * `! <expr>`: negate the result of the expression
    ///
    fn bang(&mut self) {
        if let Symbol::StringOp(_) | Symbol::IntOp(_) | Symbol::FileOp(_) = self.peek() {
            // we need to peek ahead one more token to disambiguate the first
            // two cases listed above: case 1 — `! <OP as literal>` — and
            // case 2: `<! as literal> OP str`.
            let peek2 = self.tokens.clone().nth(1);

            if peek2.is_none() {
                // op is literal
                let op = self.next_token().into_literal();
                self.stack.push(op);
                self.stack.push(Symbol::Bang);
            } else {
                // bang is literal; parsing continues with op
                self.literal(Symbol::Literal(OsString::from("!")));
            }
        } else {
            self.expr();
            self.stack.push(Symbol::Bang);
        }
    }

    /// Peek at the next token and parse it as a BOOLOP or string literal,
    /// as appropriate.
    fn maybe_boolop(&mut self) {
        if self.peek_is_boolop() {
            let token = self.tokens.next().unwrap(); // safe because we peeked

            // BoolOp by itself interpreted as Literal
            if let Symbol::None = self.peek() {
                self.literal(Symbol::Literal(token))
            } else {
                self.boolop(Symbol::BoolOp(token))
            }
        }
    }

    /// Parse a Boolean expression.
    ///
    /// Logical and (-a) has higher precedence than or (-o), so in an
    /// expression like `foo -o '' -a ''`, the and subexpression is evaluated
    /// first.
    fn boolop(&mut self, op: Symbol) {
        if op == Symbol::BoolOp(OsString::from("-a")) {
            self.term();
            self.stack.push(op);
            self.maybe_boolop();
        } else {
            self.expr();
            self.stack.push(op);
        }
    }

    /// Parse a (possible) unary argument test (string length or file
    /// attribute check).
    ///
    /// If a UOP is followed by nothing it is interpreted as a literal string.
    fn uop(&mut self, op: Symbol) {
        match self.next_token() {
            Symbol::None => self.stack.push(op.into_literal()),
            symbol => {
                self.stack.push(symbol.into_literal());
                self.stack.push(op);
            }
        }
    }

    /// Parse a string literal, optionally followed by a comparison operator
    /// and a second string literal.
    fn literal(&mut self, token: Symbol) {
        self.stack.push(token.into_literal());

        // EXPR → str OP str
        match self.peek() {
            Symbol::StringOp(_) | Symbol::IntOp(_) | Symbol::FileOp(_) => {
                let op = self.next_token();

                match self.next_token() {
                    Symbol::None => panic!("missing argument after {:?}", op),
                    token => self.stack.push(token.into_literal()),
                }

                self.stack.push(op);
            }
            _ => {}
        }
    }

    /// Parser entry point: parse the token stream `self.tokens`, storing the
    /// resulting `Symbol` stack in `self.stack`.
    fn parse(&mut self) -> Result<(), String> {
        self.expr();

        match self.tokens.next() {
            Some(token) => Err(format!("extra argument ‘{}’", token.to_string_lossy())),
            None => Ok(()),
        }
    }
}

/// Parse the token stream `args`, returning a `Symbol` stack representing the
/// operations to perform in postfix order.
pub fn parse(args: Vec<OsString>) -> Result<Vec<Symbol>, String> {
    let mut p = Parser::new(args);
    p.parse()?;
    Ok(p.stack)
}
