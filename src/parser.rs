use crate::interpreter::expr::Expression;
use crate::interpreter::types::TypeKind;
use crate::utils::errors::{
    Error,
    ErrorCode::{self, *},
};
use crate::{
    ast::{
        CallType, FuncBody, LiteralKind, LiteralType, Statement, Token,
        TokenType::{self, *},
    },
    interpreter::expr::AssignKind,
};
use std::process::exit;

pub struct Parser {
    tokens: Vec<Token>,
    err: Error,
    crnt: usize,
    pub id: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, err: Error) -> Self {
        Self {
            tokens,
            err,
            crnt: 0,
            id: 0,
        }
    }

    pub fn parse(&mut self) -> Vec<Statement> {
        let mut stmts = vec![];
        while !self.check(Eof) {
            let stmt = self.stmt();
            stmts.push(stmt);
        }
        stmts
    }

    fn stmt(&mut self) -> Statement {
        self.advance();
        match self.prev(1).token {
            Let => self.var_stmt(),
            Func => self.func_stmt(),
            If => self.if_stmt(),
            Return => self.return_stmt(),
            While => self.while_stmt(),
            Loop => self.loop_stmt(),
            Break => self.break_stmt(),
            Match => self.match_stmt(),
            Mod => self.mod_stmt(),
            Use => self.use_stmt(),
            Enum => self.enum_stmt(),
            LeftBrace => self.block_stmt(),
            _ => self.expr_stmt(),
        }
    }

    fn var_stmt(&mut self) -> Statement {
        let mut names = vec![];
        let mut pub_names = vec![];
        let mut is_mut = false;
        let mut is_pub = false;
        let mut is_null = false;

        if self.if_token_consume(Mut) {
            is_mut = true;
        } else if self.if_token_consume(Pub) {
            is_pub = true;
            if self.if_token_consume(LeftParen) {
                loop {
                    let name = self.consume(Ident);
                    pub_names.push(name);
                    if !self.if_token_consume(Comma) || self.is_token(RightParen) {
                        break;
                    }
                }
                self.consume(RightParen);
            }
        }

        loop {
            let name = self.consume(Ident);
            names.push(name);

            if self.is_token(Semi) {
                is_null = true;
                break;
            }

            if !self.is_token(Comma) || self.is_token(Colon) {
                break;
            }
            self.advance();
        }

        if pub_names.is_empty() {
            pub_names = names.clone();
        }

        let null_var = Statement::Var {
            names: names.clone(),
            value_type: self.create_null_token(names[0].line),
            value: Some(Expression::Value {
                id: self.id(),
                value: LiteralType::Null,
            }),
            is_mut,
            is_pub,
            pub_names: pub_names.clone(),
            is_func: false,
        };

        if is_null {
            self.advance();
            return null_var;
        }

        self.consume(Colon);
        let value_type = self.consume_type();
        if value_type.token == NullIdent {
            return null_var;
        }
        self.consume(Assign);
        let is_func = self.is_token(Pipe);
        let value = self.expr();
        self.consume(Semi);

        Statement::Var {
            names,
            value_type,
            value: Some(value),
            is_mut,
            is_pub,
            pub_names,
            is_func,
        }
    }

    fn func_stmt(&mut self) -> Statement {
        let mut params = vec![];
        let mut is_async = false;
        let mut is_pub = false;

        if self.if_token_consume(Pub) {
            is_pub = true;
            if self.if_token_consume(Async) {
                is_async = true;
            }
        }

        if self.if_token_consume(Async) {
            is_async = true;
            if self.if_token_consume(Pub) {
                is_pub = true;
            }
        }

        let name = self.consume(Ident);

        self.consume(LeftParen);
        while !self.if_token_consume(RightParen) {
            if self.is_token(Ident) {
                let param_name = self.consume(Ident);
                self.consume(Colon);
                let param_type = self.consume_type();
                params.push((param_name, param_type))
            } else if self.if_token_consume(Comma) {
            } else if !self.is_token(RightParen) {
                self.throw_error(E0x201, vec![self.peek().lexeme]);
            }
        }
        self.consume(Arrow);
        let value_type = self.consume_type();

        if self.if_token_consume(Assign) {
            let body = self.expr();
            self.consume(Semi);
            return Statement::Func {
                name,
                value_type,
                body: FuncBody::Statements(vec![Statement::Return { expr: body }]),
                params,
                is_async,
                is_pub,
            };
        }

        self.consume(LeftBrace);
        let body = self.block_stmts();

        Statement::Func {
            name,
            value_type,
            body: FuncBody::Statements(body),
            params,
            is_async,
            is_pub,
        }
    }

    fn if_stmt(&mut self) -> Statement {
        let cond = self.expr();
        let body = self.block_stmts();
        let mut else_if_branches = vec![];

        while self.if_token_consume(ElseIf) {
            let elif_preds = self.expr();
            let elif_stmt = self.block_stmts();
            else_if_branches.push((elif_preds, elif_stmt))
        }

        let else_branch = if self.if_token_consume(Else) {
            Some(self.block_stmts())
        } else {
            None
        };

        Statement::If {
            cond,
            body,
            else_if_branches,
            else_branch,
        }
    }

    fn return_stmt(&mut self) -> Statement {
        let expr = if self.is_token(Semi) {
            Expression::Value {
                id: self.id(),
                value: LiteralType::Null,
            }
        } else {
            self.expr()
        };
        self.consume(Semi);
        Statement::Return { expr }
    }

    fn while_stmt(&mut self) -> Statement {
        let cond = self.expr();
        let body = self.block_stmts();
        Statement::While { cond, body }
    }

    fn loop_stmt(&mut self) -> Statement {
        let iter = if self.is_token(NumberLit) {
            let num = match self.consume(NumberLit).value {
                Some(LiteralKind::Number { value, .. }) => value,
                _ => self.throw_error(E0x202, vec![self.peek().lexeme]),
            };
            if num < 0.0 {
                Some(1)
            } else {
                Some(num as usize)
            }
        } else {
            None
        };
        let body = self.block_stmts();
        Statement::Loop { iter, body }
    }

    fn break_stmt(&mut self) -> Statement {
        self.consume(Semi);
        Statement::Break {}
    }

    fn match_stmt(&mut self) -> Statement {
        let cond = self.expr();
        self.consume(LeftBrace);
        let mut cases = vec![];

        while self.is_literal() || self.is_uppercase_ident() {
            let expr = self.expr();
            self.consume(ArrowBig);
            if self.if_token_advance(LeftBrace) {
                let body = self.block_stmts();
                self.consume(RightBrace);
                cases.push((expr, FuncBody::Statements(body)))
            } else {
                let body = self.expr();
                self.consume(Comma);
                cases.push((expr, FuncBody::Expression(Box::new(body))))
            }
        }

        let mut def_case = FuncBody::Statements(vec![]);
        if self.if_token_consume(Underscore) {
            self.consume(ArrowBig);
            if self.if_token_consume(LeftBrace) {
                let body = self.block_stmts();
                def_case = FuncBody::Statements(body)
            } else {
                let body = self.expr();
                def_case = FuncBody::Expression(Box::new(body))
            }
        }
        let stmt = Statement::Match {
            cond,
            cases,
            def_case,
        };
        self.consume(RightBrace);
        stmt
    }

    fn mod_stmt(&mut self) -> Statement {
        let src = self.consume(StringLit).lexeme;
        self.consume(Semi);
        Statement::Mod { src }
    }

    fn use_stmt(&mut self) -> Statement {
        let mut names = vec![];
        let mut all = false;
        if self.if_token_advance(Mult) {
            all = true;
            self.consume(From);
        } else {
            while !self.if_token_advance(From) {
                let name = self.consume(Ident);
                if self.if_token_consume(As) {
                    let as_name = self.consume(Ident);
                    names.push((name, Some(as_name)))
                } else {
                    names.push((name, None))
                }
                self.if_token_consume(Comma);
            }
        }
        let src = self.consume(StringLit).lexeme;
        self.consume(Semi);
        Statement::Use { src, names, all }
    }

    fn enum_stmt(&mut self) -> Statement {
        let mut is_pub = false;

        if self.if_token_consume(Pub) {
            is_pub = true;
        }

        let name = self.consume_uppercase_ident();
        self.consume(LeftBrace);

        let mut enums = vec![];
        while !self.if_token_consume(RightBrace) {
            let enm = self.consume(Ident);
            enums.push(enm);
            if !self.if_token_consume(Comma) && !self.is_token(RightBrace) {
                self.throw_error(E0x201, vec![self.peek().lexeme]);
            }
        }
        Statement::Enum {
            name,
            enums,
            is_pub,
        }
    }

    fn block_stmts(&mut self) -> Vec<Statement> {
        match self.block_stmt() {
            Statement::Block { stmts } => {
                self.consume(RightBrace);
                stmts
            }
            _ => self.throw_error(E0x203, vec!["a block statement".to_string()]),
        }
    }

    fn block_stmt(&mut self) -> Statement {
        let mut stmts = vec![];
        while !self.is_token(RightBrace) && !self.is_token(Eof) {
            let stmt = self.stmt();
            stmts.push(stmt);
        }
        Statement::Block { stmts }
    }

    fn expr_stmt(&mut self) -> Statement {
        self.retreat();
        let expr = self.expr();
        self.consume(Semi);
        Statement::Expression { expr }
    }

    fn expr(&mut self) -> Expression {
        let expr = self.binary();
        if self.if_token_consume(Assign) {
            return self.assign(expr, AssignKind::Normal);
        } else if self.if_token_consume(PlusEq) {
            return self.assign(expr, AssignKind::Plus);
        } else if self.if_token_consume(MinEq) {
            return self.assign(expr, AssignKind::Minus);
        } else if self.if_token_consume(MultEq) {
            return self.assign(expr, AssignKind::Mult);
        } else if self.if_token_consume(DivEq) {
            return self.assign(expr, AssignKind::Div);
        }
        expr
    }

    fn assign(&mut self, expr: Expression, kind: AssignKind) -> Expression {
        let value = self.expr();
        if let Expression::Var { id: _, name } = expr {
            Expression::Assign {
                id: self.id(),
                name,
                value: Box::new(value),
                kind,
            }
        } else {
            self.throw_error(E0x201, vec!["Invalid assignment target".to_string()]);
        }
    }

    fn binary(&mut self) -> Expression {
        let mut expr = self.unary();
        while self.are_tokens(vec![
            Plus,
            Minus,
            Mult,
            Divide,
            Percent,
            AndAnd,
            Or,
            Eq,
            NotEq,
            Greater,
            GreaterOrEq,
            Less,
            LessOrEq,
            Square,
            And,
        ]) {
            self.advance();
            let operator = self.prev(1);
            let rhs = self.unary();
            expr = Expression::Binary {
                id: self.id(),
                left: Box::new(expr),
                operator,
                right: Box::new(rhs),
            }
        }
        expr
    }

    fn unary(&mut self) -> Expression {
        if self.are_tokens(vec![Not, NotNot, Queston, Decr, Increment, Minus]) {
            self.advance();
            let operator = self.prev(1);
            let rhs = self.unary();
            Expression::Unary {
                id: self.id(),
                left: Box::new(rhs),
                operator,
            }
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Expression {
        if self.is_token(LeftBracket) && self.prev(1).token == Ident {
            self.advance();
            let arr = self.array_call();
            self.consume(RightBracket);
            return arr;
        }
        let mut expr = self.method();

        loop {
            if self.if_token_consume(Dot) {
                if let Expression::Method { .. } = expr {
                    expr = self.method_body(expr.clone());
                } else {
                    expr = self.struct_call();
                }
            } else if self.if_token_consume(DblColon) {
                expr = self.enum_call();
            } else if self.if_token_consume(LeftParen) {
                expr = self.func_call();
            } else if self.if_token_consume(LeftBracket) {
                expr = self.array_call();
            } else if self.if_token_consume(Ident) {
                expr = self.call();
            } else {
                break;
            }
        }
        expr
    }

    fn array_call(&mut self) -> Expression {
        let name = self.prev(2);
        let mut args = vec![];
        let arg = self.expr();
        args.push(arg);

        Expression::Call {
            id: self.id(),
            name: Box::new(Expression::Var {
                id: self.id(),
                name,
            }),
            args,
            call_type: CallType::Array,
        }
    }

    fn struct_call(&mut self) -> Expression {
        let name = self.prev(2);
        let args = vec![self.expr()];
        Expression::Call {
            id: self.id(),
            name: Box::new(Expression::Var {
                id: self.id(),
                name,
            }),
            args,
            call_type: CallType::Struct,
        }
    }

    fn enum_call(&mut self) -> Expression {
        let name = self.prev(2);
        let mut args = vec![];
        let arg = self.expr();
        args.push(arg);

        Expression::Call {
            id: self.id(),
            name: Box::new(Expression::Var {
                id: self.id(),
                name,
            }),
            args,
            call_type: CallType::Enum,
        }
    }

    fn func_call(&mut self) -> Expression {
        let name = self.prev(2);
        let mut args = vec![];
        while !self.is_token(RightParen) {
            let arg = self.expr();
            args.push(arg);
            if self.is_token(RightParen) {
                break;
            }
            if !self.if_token_consume(Comma) && !self.is_token(RightParen) {
                self.throw_error(E0x201, vec![self.peek().lexeme]);
            }
        }
        self.consume(RightParen);
        Expression::Call {
            id: self.id(),
            name: Box::new(Expression::Var {
                id: self.id(),
                name,
            }),
            args,
            call_type: CallType::Func,
        }
    }

    fn method(&mut self) -> Expression {
        let mut expr = self.primary();
        if self.if_token_consume(Dot) {
            expr = self.method_body(expr.clone())
        }
        expr
    }

    fn method_body(&mut self, expr: Expression) -> Expression {
        let name = self.consume(Ident);
        self.consume(LeftParen);
        let mut args = vec![];
        while !self.if_token_consume(RightParen) {
            args.push(self.expr());
            self.if_token_consume(Comma);
        }
        Expression::Method {
            id: self.id(),
            name,
            left: Box::new(expr),
            args,
        }
    }

    fn primary(&mut self) -> Expression {
        let token = self.peek();
        match token.clone().token {
            Ident => {
                self.advance();
                Expression::Var {
                    id: self.id(),
                    name: token,
                }
            }
            LeftBracket => {
                self.advance();
                self.arr_expr()
            }
            LeftParen => {
                if self.prev(1).token == Ident {
                    self.advance();
                    self.func_call()
                } else {
                    self.group_expr()
                }
            }
            Pipe => self.func_expr(),
            Await => self.await_expr(),
            _ => {
                if self.is_literal() {
                    self.advance();
                    Expression::Value {
                        id: self.id(),
                        value: self.to_value_type(token),
                    }
                } else {
                    self.throw_error(E0x201, vec![self.peek().lexeme]);
                }
            }
        }
    }
    fn to_value_type(&mut self, token: Token) -> LiteralType {
        match token.token {
            NumberLit => {
                let number = match token.value {
                    Some(LiteralKind::Number { value, .. }) => value,
                    _ => self.throw_error(E0x202, vec![self.peek().lexeme]),
                };
                LiteralType::Number(number)
            }
            StringLit => {
                let string = match token.value {
                    Some(LiteralKind::String { value }) => value,
                    _ => self.throw_error(E0x202, vec![self.peek().lexeme]),
                };
                LiteralType::String(string)
            }
            CharLit => {
                let char = match token.value {
                    Some(LiteralKind::Char { value }) => value,
                    _ => self.throw_error(E0x202, vec![self.peek().lexeme]),
                };
                LiteralType::Char(char)
            }
            TrueLit => LiteralType::Boolean(true),
            FalseLit => LiteralType::Boolean(false),
            NullLit => LiteralType::Null,
            _ => LiteralType::Any,
        }
    }

    fn arr_expr(&mut self) -> Expression {
        let mut items = vec![];
        while !self.if_token_consume(RightBracket) {
            let item = self.expr();
            items.push(item);
            if !self.if_token_consume(Comma) && !self.is_token(RightBracket) {
                self.throw_error(E0x201, vec![self.peek().lexeme]);
            }
        }
        Expression::Array {
            id: self.id(),
            items,
        }
    }

    fn group_expr(&mut self) -> Expression {
        self.advance();
        let expr = self.expr();
        self.consume(RightParen);
        Expression::Grouping {
            id: self.id(),
            expression: Box::new(expr),
        }
    }

    fn func_expr(&mut self) -> Expression {
        self.advance();
        let value_type = self.prev(3);
        let mut params = vec![];
        let is_async = false;
        let mut is_pub = false;
        let add = if params.len() > 1 {
            params.len() * 2 - 1
        } else {
            params.len()
        };

        if self.prev(9 + add).token == Pub {
            is_pub = true;
        }
        let name = self.prev(8 + add);
        self.consume(Pipe);
        if self.if_token_consume(Underscore) {
            self.consume(Pipe);
        } else {
            while !self.if_token_consume(Pipe) {
                if self.is_token(Ident) {
                    let param_name = self.consume(Ident);
                    self.consume(Colon);
                    let param_type = self.consume_type();
                    params.push((param_name, param_type))
                } else if self.if_token_consume(Comma) {
                } else if !self.is_token(Pipe) {
                    self.throw_error(E0x201, vec![self.peek().lexeme]);
                }
            }
        }
        if self.if_token_consume(Colon) {
            let body = self.expr();
            self.consume(Semi);
            return Expression::Func {
                id: self.id(),
                name,
                value_type,
                body: FuncBody::Expression(Box::new(body)),
                params,
                is_async,
                is_pub,
            };
        }
        self.consume(LeftBrace);
        let body = self.block_stmts();
        Expression::Func {
            id: self.id(),
            name,
            value_type,
            body: FuncBody::Statements(body),
            params,
            is_async,
            is_pub,
        }
    }

    fn await_expr(&mut self) -> Expression {
        let expr = self.expr();
        Expression::Await {
            id: self.id(),
            expr: Box::new(expr),
        }
    }

    /// checks if current token is literal value
    fn is_literal(&self) -> bool {
        self.are_tokens(vec![
            NumberLit, StringLit, CharLit, TrueLit, FalseLit, NullLit,
        ])
    }

    /// consumes if token matches
    fn if_token_consume(&mut self, token: TokenType) -> bool {
        if self.is_token(token.clone()) {
            self.consume(token);
            return true;
        }
        false
    }

    /// advances if token matches
    fn if_token_advance(&mut self, token: TokenType) -> bool {
        if self.is_token(token) {
            self.advance();
            return true;
        }
        false
    }

    fn is_uppercase_ident(&mut self) -> bool {
        let token = self.peek();
        let first_char = token.lexeme.chars().next().unwrap_or('\0');
        first_char.is_uppercase()
    }

    /// consumes identifiers with Uppercase lexeme
    fn consume_uppercase_ident(&mut self) -> Token {
        let token = self.peek();
        if self.is_uppercase_ident() {
            self.consume(Ident);
            return token;
        }
        self.throw_error(E0x204, vec!["uppercase Ident".to_string()]);
    }

    fn consume_type(&mut self) -> Token {
        match self.peek().token {
            Less => self.parse_array_type(),
            Pipe => self.parse_func_type(),
            StringLit | NumberLit | CharLit | NullLit | TrueLit | ArrayLit | FalseLit => {
                self.parse_literal_type()
            }
            Ident => self.parse_ident_type(),
            AnyIdent | BoolIdent | CharIdent | NullIdent | VoidIdent | ArrayIdent | NumberIdent
            | StringIdent => self.parse_builtin_type(),
            c => Token {
                token: c,
                lexeme: self.peek().lexeme,
                pos: self.peek().pos,
                value: None,
                line: self.peek().line,
            },
        }
    }

    fn parse_builtin_type(&mut self) -> Token {
        let token = self.consume_some(vec![
            AnyIdent,
            BoolIdent,
            CharIdent,
            NullIdent,
            VoidIdent,
            ArrayIdent,
            NumberIdent,
            StringIdent,
        ]);
        let value = Some(LiteralKind::Type(Box::new(TypeKind::Var {
            name: token.clone(),
        })));
        Token {
            token: token.token.clone(),
            lexeme: token.lexeme,
            value,
            line: token.line,
            pos: token.pos,
        }
    }

    fn parse_ident_type(&mut self) -> Token {
        let token = self.consume(Ident);
        let value = Some(LiteralKind::Type(Box::new(TypeKind::Var {
            name: token.clone(),
        })));
        Token {
            token: Ident,
            lexeme: token.lexeme,
            value,
            line: token.line,
            pos: token.pos,
        }
    }

    fn parse_literal_type(&mut self) -> Token {
        let token = self.peek();
        let value = Some(LiteralKind::Type(Box::new(TypeKind::Value {
            kind: self.peek().value.unwrap_or(LiteralKind::Null),
        })));
        match token.token {
            StringLit => {
                self.advance();
                return Token {
                    token: StringLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            NumberLit => {
                self.advance();
                return Token {
                    token: NumberLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            CharLit => {
                self.advance();
                return Token {
                    token: CharLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            TrueLit => {
                self.advance();
                return Token {
                    token: TrueLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            FalseLit => {
                self.advance();
                return Token {
                    token: FalseLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            NullLit => {
                self.advance();
                return Token {
                    token: NullLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            ArrayLit => {
                self.advance();
                return Token {
                    token: ArrayLit,
                    lexeme: self.peek().lexeme,
                    value,
                    line: token.line,
                    pos: token.pos,
                };
            }
            _ => token,
        }
    }

    fn parse_func_type(&mut self) -> Token {
        self.consume(Pipe);
        let mut params: Vec<TypeKind> = vec![];
        while !self.if_token_consume(Pipe) {
            let param = self.consume_type();
            params.push(TypeKind::Var { name: param });
            if !self.if_token_consume(Comma) && !self.is_token(Pipe) {
                self.throw_error(E0x201, vec![self.peek().lexeme]);
            }
        }
        let return_type = self.consume_type();

        Token {
            token: FuncIdent,
            lexeme: "func_type".to_string(),
            value: Some(LiteralKind::Type(Box::new(TypeKind::Func {
                params,
                ret: Box::new(TypeKind::Var { name: return_type }),
            }))),
            line: self.peek().line,
            pos: self.peek().pos,
        }
    }

    fn parse_array_type(&mut self) -> Token {
        self.consume(Less);
        if self.if_token_consume(LeftParen) {
            let mut statics: Vec<TypeKind> = vec![];
            while !self.if_token_consume(RightParen) {
                let static_size = self.consume_type();
                statics.push(TypeKind::Var { name: static_size });
                if !self.if_token_consume(Comma) && !self.is_token(RightParen) {
                    self.throw_error(E0x201, vec![self.peek().lexeme]);
                }
            }
            let typ = self.consume_type();
            self.consume(Greater);
            return Token {
                token: ArrayIdent,
                lexeme: typ.lexeme,
                pos: self.peek().pos,
                value: Some(LiteralKind::Type(Box::new(TypeKind::Array {
                    kind: None,
                    statics: Some(statics),
                }))),
                line: self.peek().line,
            };
        }

        let typ = self.consume_type();
        self.consume(Greater);
        Token {
            token: ArrayIdent,
            lexeme: typ.clone().lexeme,
            pos: self.peek().pos,
            value: Some(LiteralKind::Type(Box::new(TypeKind::Array {
                kind: Some(Box::new(TypeKind::Var { name: typ.clone() })),
                statics: None,
            }))),
            line: self.peek().line,
        }
    }

    /// advances if one of the input tokens matches
    fn consume_some(&mut self, ts: Vec<TokenType>) -> Token {
        for t in ts {
            if self.if_token_advance(t) {
                return self.prev(1);
            }
        }
        self.throw_error(E0x204, vec![self.prev(1).lexeme]);
    }

    /// advances if input token matches
    fn consume(&mut self, t: TokenType) -> Token {
        if self.if_token_advance(t.clone()) {
            return self.prev(1);
        }
        self.throw_error(E0x204, vec![t.to_string()]);
    }

    /// increases current position by 1
    /// and returns advanced token
    fn advance(&mut self) -> Token {
        if !self.is_token(Eof) {
            self.crnt += 1;
        }
        self.prev(1)
    }

    /// decreases current position by 1
    /// and returns advanced token
    fn retreat(&mut self) -> Token {
        if self.crnt > 0 {
            self.crnt -= 1;
        }
        self.prev(1)
    }

    /// returns previous token
    fn prev(&self, back: usize) -> Token {
        if self.crnt < back {
            return Token {
                token: Eof,
                lexeme: "\0".to_string(),
                line: 0,
                pos: (0, 0),
                value: None,
            };
        }
        self.tokens[self.crnt - back].clone()
    }

    /// bulk checks if one of the token matches current token
    fn are_tokens(&self, tokens: Vec<TokenType>) -> bool {
        tokens.iter().any(|token| self.is_token(token.clone()))
    }

    /// checks if token matches current token and
    /// handles EoF
    fn is_token(&self, token: TokenType) -> bool {
        !self.check(Eof) && self.check(token)
    }

    /// checks if token matches current token
    fn check(&self, token: TokenType) -> bool {
        self.peek().token == token
    }

    /// returns current token
    fn peek(&self) -> Token {
        self.tokens[self.crnt].clone()
    }

    /// increases id count, and returns previous id
    fn id(&mut self) -> usize {
        self.id += 1;
        self.id - 1
    }

    /// Helper function to throw errors
    fn throw_error(&mut self, code: ErrorCode, args: Vec<String>) -> ! {
        self.err
            .throw(code, self.peek().line, self.peek().pos, args);
        exit(1);
    }

    /// Helper function to create a null token
    fn create_null_token(&self, line: usize) -> Token {
        Token {
            token: NullIdent,
            pos: self.peek().pos,
            lexeme: "null".to_string(),
            value: None,
            line,
        }
    }
}
