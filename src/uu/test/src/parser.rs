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
                "=" | "==" | "!=" => Symbol::StringOp(s),
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
///   STRINGOP → = | == | !=
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
        matches!(self.peek(), Symbol::BoolOp(_))
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
                self.literal(Symbol::LParen.into_literal());
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
    /// * `! = = str`: negate comparison of literal `=` and `str`
    /// * `!`: bang followed by nothing is literal
    /// * `! EXPR`: negate the result of the expression
    ///
    /// Combined Boolean & negation:
    ///
    /// * `! ( EXPR ) [BOOLOP EXPR]`: negate the parenthesized expression only
    /// * `! UOP str BOOLOP EXPR`: negate the unary subexpression
    /// * `! str BOOLOP str`: negate the entire Boolean expression
    /// * `! str BOOLOP EXPR BOOLOP EXPR`: negate the value of the first `str` term
    ///
    fn bang(&mut self) {
        match self.peek() {
            Symbol::StringOp(_) | Symbol::IntOp(_) | Symbol::FileOp(_) | Symbol::BoolOp(_) => {
                // we need to peek ahead one more token to disambiguate the first
                // three cases listed above
                let peek2 = Symbol::new(self.tokens.clone().nth(1));

                match peek2 {
                    // case 1: `! <OP as literal>`
                    // case 3: `! = OP str`
                    Symbol::StringOp(_) | Symbol::None => {
                        // op is literal
                        let op = self.next_token().into_literal();
                        self.literal(op);
                        self.stack.push(Symbol::Bang);
                    }
                    // case 2: `<! as literal> OP str [BOOLOP EXPR]`.
                    _ => {
                        // bang is literal; parsing continues with op
                        self.literal(Symbol::Bang.into_literal());
                        self.maybe_boolop();
                    }
                }
            }

            // bang followed by nothing is literal
            Symbol::None => self.stack.push(Symbol::Bang.into_literal()),

            _ => {
                // peek ahead up to 4 tokens to determine if we need to negate
                // the entire expression or just the first term
                let peek4: Vec<Symbol> = self
                    .tokens
                    .clone()
                    .take(4)
                    .map(|token| Symbol::new(Some(token)))
                    .collect();

                match peek4.as_slice() {
                    // we peeked ahead 4 but there were only 3 tokens left
                    [Symbol::Literal(_), Symbol::BoolOp(_), Symbol::Literal(_)] => {
                        self.expr();
                        self.stack.push(Symbol::Bang);
                    }
                    _ => {
                        self.term();
                        self.stack.push(Symbol::Bang);
                    }
                }
            }
        }
    }

    /// Peek at the next token and parse it as a BOOLOP or string literal,
    /// as appropriate.
    fn maybe_boolop(&mut self) {
        if self.peek_is_boolop() {
            let symbol = self.next_token();

            // BoolOp by itself interpreted as Literal
            if let Symbol::None = self.peek() {
                self.literal(symbol.into_literal());
            } else {
                self.boolop(symbol);
                self.maybe_boolop();
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
        } else {
            self.expr();
        }
        self.stack.push(op);
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
