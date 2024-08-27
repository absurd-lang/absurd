use super::TokenType;

impl TokenType {
    pub fn to_string(&self) -> String {
        let s = match self {
            Self::Not => "!",
            Self::NotNot => "!!",
            Self::Tilde => "~",
            Self::Percent => "%",
            Self::And => "&",
            Self::AndAnd => "&&",
            Self::Mult => "*",
            Self::Square => "**",
            Self::LeftParen => "(",
            Self::RightParen => ")",
            Self::Minus => "-",
            Self::Decr => "--",
            Self::Arrow => "->",
            Self::ArrowBig => "=>",
            Self::Underscore => "_",
            Self::Plus => "+",
            Self::Increment => "++",
            Self::Assign => "=",
            Self::Eq => "==",
            Self::NotEq => "!=",
            Self::PlusEq => "+=",
            Self::MinEq => "-=",
            Self::MultEq => "*=",
            Self::DivEq => "/=",
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::LeftBracket => "[",
            Self::RightBracket => "]",
            Self::Semi => ";",
            Self::Colon => ":",
            Self::DblColon => "::",
            Self::CharLit => "char literal",
            Self::StringLit => "string literal",
            Self::NumberLit => "number literal",
            Self::TrueLit => "true literal",
            Self::FalseLit => "false literal",
            Self::NullLit => "null literal",
            Self::ArrayLit => "array literal",
            Self::FuncIdent => "function type",
            Self::Less => "<",
            Self::LessOrEq => "<=",
            Self::Greater => ">",
            Self::GreaterOrEq => ">=",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::Divide => "/",
            Self::Escape => "\\",
            Self::StartParse => "\\{",
            Self::EndParse => "\\}",
            Self::Queston => "?",
            Self::Pipe => "|",
            Self::Or => "||",
            Self::Ident => "identifier",
            Self::Eof => "end of file",
            Self::Let => "let keyword",
            Self::If => "if keyword",
            Self::Else => "else keyword",
            Self::ElseIf => "elif keyword",
            Self::Return => "return keyword",
            Self::While => "while keyword",
            Self::Loop => "loop keyword",
            Self::Break => "break keyword",
            Self::Match => "match keyword",
            Self::Mod => "mod keyword",
            Self::Use => "use keyword",
            Self::As => "as keyword",
            Self::From => "from keyword",
            Self::Enum => "enum keyword",
            Self::Async => "async keyword",
            Self::Await => "await keyword",
            Self::Pub => "pub keyword",
            Self::Mut => "mut keyword",
            Self::Func => "function keyword",
            Self::NumberIdent => "number",
            Self::StringIdent => "string",
            Self::CharIdent => "char",
            Self::BoolIdent => "bool",
            Self::NullIdent => "null",
            Self::VoidIdent => "void",
            Self::ArrayIdent => "array",
            Self::AnyIdent => "any",
        };

        s.to_string()
    }
}
