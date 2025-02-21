// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (grammar) BOOLOP STRLEN FILETEST FILEOP INTOP STRINGOP ; (vars) LParen StrlenOp

use std::ffi::{OsStr, OsString};
use std::iter::Peekable;

use super::error::{ParseError, ParseResult};

use uucore::display::Quotable;

/// Represents one of the binary comparison operators for strings, integers, or files
#[derive(Debug, PartialEq, Eq)]
pub enum Operator {
    String(OsString),
    Int(OsString),
    File(OsString),
}

/// Represents one of the unary test operators for strings or files
#[derive(Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    StrlenOp(OsString),
    FiletestOp(OsString),
}

/// Represents a parsed token from a test expression
#[derive(Debug, PartialEq, Eq)]
pub enum Symbol {
    LParen,
    Bang,
    BoolOp(OsString),
    Literal(OsString),
    Op(Operator),
    UnaryOp(UnaryOperator),
    None,
}

impl Symbol {
    /// Create a new Symbol from an OsString.
    ///
    /// Returns Symbol::None in place of None
    fn new(token: Option<OsString>) -> Self {
        match token {
            Some(s) => match s.to_str() {
                Some(t) => match t {
                    "(" => Self::LParen,
                    "!" => Self::Bang,
                    "-a" | "-o" => Self::BoolOp(s),
                    "=" | "==" | "!=" | "<" | ">" => Self::Op(Operator::String(s)),
                    "-eq" | "-ge" | "-gt" | "-le" | "-lt" | "-ne" => Self::Op(Operator::Int(s)),
                    "-ef" | "-nt" | "-ot" => Self::Op(Operator::File(s)),
                    "-n" | "-z" => Self::UnaryOp(UnaryOperator::StrlenOp(s)),
                    "-b" | "-c" | "-d" | "-e" | "-f" | "-g" | "-G" | "-h" | "-k" | "-L" | "-N"
                    | "-O" | "-p" | "-r" | "-s" | "-S" | "-t" | "-u" | "-w" | "-x" => {
                        Self::UnaryOp(UnaryOperator::FiletestOp(s))
                    }
                    _ => Self::Literal(s),
                },
                None => Self::Literal(s),
            },
            None => Self::None,
        }
    }

    /// Convert this Symbol into a Symbol::Literal, useful for cases where
    /// test treats an operator as a string operand (test has no reserved
    /// words).
    ///
    /// # Panics
    ///
    /// Panics if `self` is Symbol::None
    fn into_literal(self) -> Self {
        Self::Literal(match self {
            Self::LParen => OsString::from("("),
            Self::Bang => OsString::from("!"),
            Self::BoolOp(s)
            | Self::Literal(s)
            | Self::Op(Operator::String(s))
            | Self::Op(Operator::Int(s))
            | Self::Op(Operator::File(s))
            | Self::UnaryOp(UnaryOperator::StrlenOp(s))
            | Self::UnaryOp(UnaryOperator::FiletestOp(s)) => s,
            Self::None => panic!(),
        })
    }
}

/// Implement Display trait for Symbol to make it easier to print useful errors.
/// We will try to match the format in which the symbol appears in the input.
impl std::fmt::Display for Symbol {
    /// Format a Symbol for printing
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match &self {
            Self::LParen => OsStr::new("("),
            Self::Bang => OsStr::new("!"),
            Self::BoolOp(s)
            | Self::Literal(s)
            | Self::Op(Operator::String(s))
            | Self::Op(Operator::Int(s))
            | Self::Op(Operator::File(s))
            | Self::UnaryOp(UnaryOperator::StrlenOp(s))
            | Self::UnaryOp(UnaryOperator::FiletestOp(s)) => OsStr::new(s),
            Self::None => OsStr::new("None"),
        };
        write!(f, "{}", s.quote())
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
///   FILETEST → -b | -c | -d | -e | -f | -g | -G | -h | -k | -L | -N | -O | -p |
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
    fn new(tokens: Vec<OsString>) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
            stack: vec![],
        }
    }

    /// Fetch the next token from the input stream as a Symbol.
    fn next_token(&mut self) -> Symbol {
        Symbol::new(self.tokens.next())
    }

    /// Consume the next token & verify that it matches the provided value.
    fn expect(&mut self, value: &str) -> ParseResult<()> {
        match self.next_token() {
            Symbol::Literal(s) if s == value => Ok(()),
            _ => Err(ParseError::Expected(value.quote().to_string())),
        }
    }

    /// Peek at the next token from the input stream, returning it as a Symbol.
    /// The stream is unchanged and will return the same Symbol on subsequent
    /// calls to `next()` or `peek()`.
    fn peek(&mut self) -> Symbol {
        Symbol::new(self.tokens.peek().cloned())
    }

    /// Test if the next token in the stream is a BOOLOP (-a or -o), without
    /// removing the token from the stream.
    fn peek_is_boolop(&mut self) -> bool {
        matches!(self.peek(), Symbol::BoolOp(_))
    }

    /// Parse an expression.
    ///
    ///   EXPR → TERM | EXPR BOOLOP EXPR
    fn expr(&mut self) -> ParseResult<()> {
        if !self.peek_is_boolop() {
            self.term()?;
        }
        self.maybe_boolop()?;
        Ok(())
    }

    /// Parse a term token and possible subsequent symbols: "(", "!", UOP,
    /// literal, or None.
    fn term(&mut self) -> ParseResult<()> {
        let symbol = self.next_token();

        match symbol {
            Symbol::LParen => self.lparen()?,
            Symbol::Bang => self.bang()?,
            Symbol::UnaryOp(_) => self.uop(symbol),
            Symbol::None => self.stack.push(symbol),
            literal => self.literal(literal)?,
        }
        Ok(())
    }

    /// Parse a (possibly) parenthesized expression.
    ///
    /// test has no reserved keywords, so "(" will be interpreted as a literal
    /// in certain cases:
    ///
    /// * when found at the end of the token stream
    /// * when followed by a binary operator that is not _itself_ interpreted
    ///   as a literal
    ///
    fn lparen(&mut self) -> ParseResult<()> {
        // Look ahead up to 3 tokens to determine if the lparen is being used
        // as a grouping operator or should be treated as a literal string
        let peek3: Vec<Symbol> = self
            .tokens
            .clone()
            .take(3)
            .map(|token| Symbol::new(Some(token)))
            .collect();

        match peek3.as_slice() {
            // case 1: lparen is a literal when followed by nothing
            [] => {
                self.literal(Symbol::LParen.into_literal())?;
                Ok(())
            }

            // case 2: error if end of stream is `( <any_token>`
            [symbol] => Err(ParseError::MissingArgument(format!("{symbol}"))),

            // case 3: `( uop <any_token> )` → parenthesized unary operation;
            //         this case ensures we don’t get confused by `( -f ) )`
            //         or `( -f ( )`, for example
            [Symbol::UnaryOp(_), _, Symbol::Literal(s)] if s == ")" => {
                let symbol = self.next_token();
                self.uop(symbol);
                self.expect(")")?;
                Ok(())
            }

            // case 4: binary comparison of literal lparen, e.g. `( != )`
            [Symbol::Op(_), Symbol::Literal(s)] | [Symbol::Op(_), Symbol::Literal(s), _]
                if s == ")" =>
            {
                self.literal(Symbol::LParen.into_literal())?;
                Ok(())
            }

            // case 5: after handling the prior cases, any single token inside
            //         parentheses is a literal, e.g. `( -f )`
            [_, Symbol::Literal(s)] | [_, Symbol::Literal(s), _] if s == ")" => {
                let symbol = self.next_token();
                self.literal(symbol)?;
                self.expect(")")?;
                Ok(())
            }

            // case 6: two binary ops in a row, treat the first op as a literal
            [Symbol::Op(_), Symbol::Op(_), _] => {
                let symbol = self.next_token();
                self.literal(symbol)?;
                self.expect(")")?;
                Ok(())
            }

            // case 7: if earlier cases didn’t match, `( op <any_token>…`
            //         indicates binary comparison of literal lparen with
            //         anything _except_ ")" (case 4)
            [Symbol::Op(_), _] | [Symbol::Op(_), _, _] => {
                self.literal(Symbol::LParen.into_literal())?;
                Ok(())
            }

            // Otherwise, lparen indicates the start of a parenthesized
            // expression
            _ => {
                self.expr()?;
                self.expect(")")?;
                Ok(())
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
    fn bang(&mut self) -> ParseResult<()> {
        match self.peek() {
            Symbol::Op(_) | Symbol::BoolOp(_) => {
                // we need to peek ahead one more token to disambiguate the first
                // three cases listed above
                let peek2 = Symbol::new(self.tokens.clone().nth(1));

                match peek2 {
                    // case 1: `! <OP as literal>`
                    // case 3: `! = OP str`
                    Symbol::Op(_) | Symbol::None => {
                        // op is literal
                        let op = self.next_token().into_literal();
                        self.literal(op)?;
                        self.stack.push(Symbol::Bang);
                    }
                    // case 2: `<! as literal> OP str [BOOLOP EXPR]`.
                    _ => {
                        // bang is literal; parsing continues with op
                        self.literal(Symbol::Bang.into_literal())?;
                        self.maybe_boolop()?;
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
                        self.expr()?;
                        self.stack.push(Symbol::Bang);
                    }
                    _ => {
                        self.term()?;
                        self.stack.push(Symbol::Bang);
                    }
                }
            }
        }
        Ok(())
    }

    /// Peek at the next token and parse it as a BOOLOP or string literal,
    /// as appropriate.
    fn maybe_boolop(&mut self) -> ParseResult<()> {
        if self.peek_is_boolop() {
            let symbol = self.next_token();

            // BoolOp by itself interpreted as Literal
            if let Symbol::None = self.peek() {
                self.literal(symbol.into_literal())?;
            } else {
                self.boolop(symbol)?;
                self.maybe_boolop()?;
            }
        }
        Ok(())
    }

    /// Parse a Boolean expression.
    ///
    /// Logical and (-a) has higher precedence than or (-o), so in an
    /// expression like `foo -o '' -a ''`, the and subexpression is evaluated
    /// first.
    fn boolop(&mut self, op: Symbol) -> ParseResult<()> {
        if op == Symbol::BoolOp(OsString::from("-a")) {
            self.term()?;
        } else {
            self.expr()?;
        }
        self.stack.push(op);
        Ok(())
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
    fn literal(&mut self, token: Symbol) -> ParseResult<()> {
        self.stack.push(token.into_literal());

        // EXPR → str OP str
        if let Symbol::Op(_) = self.peek() {
            let op = self.next_token();

            match self.next_token() {
                Symbol::None => {
                    return Err(ParseError::MissingArgument(format!("{op}")));
                }
                token => self.stack.push(token.into_literal()),
            }

            self.stack.push(op);
        }
        Ok(())
    }

    /// Parser entry point: parse the token stream `self.tokens`, storing the
    /// resulting `Symbol` stack in `self.stack`.
    fn parse(&mut self) -> ParseResult<()> {
        self.expr()?;

        match self.tokens.next() {
            Some(token) => Err(ParseError::ExtraArgument(token.quote().to_string())),
            None => Ok(()),
        }
    }
}

/// Parse the token stream `args`, returning a `Symbol` stack representing the
/// operations to perform in postfix order.
pub fn parse(args: Vec<OsString>) -> ParseResult<Vec<Symbol>> {
    let mut p = Parser::new(args);
    p.parse()?;
    Ok(p.stack)
}
