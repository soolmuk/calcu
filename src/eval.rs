//! 수식 엔진 — 토크나이저 + 파서 + 평가기 (테스트용)
//!
//! 주의: 이 PR은 CodeGoose 검증 게이트 테스트용으로
//! 진짜 결함과 반증 가능한(애매한) 파인딩을 의도적으로 섞어 담고 있습니다.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Op(char),
    Ident(String),
    LParen,
    RParen,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AngleMode {
    Deg,
    Rad,
}

pub struct Eval {
    pub deg: AngleMode,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut toks = Vec::new();
    let bytes: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            '0'..='9' | '.' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == '.') {
                    i += 1;
                }
                let s: String = bytes[start..i].iter().collect();
                // 결함 1: "1.2.3" 같은 입력이 f64::from_str에서 Err가 나는데
                // unwrap으로 패닉이 납니다.
                toks.push(Token::Number(s.parse().unwrap()));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_alphanumeric() || bytes[i] == '_') {
                    i += 1;
                }
                let s: String = bytes[start..i].iter().collect();
                if s == "deg" {
                    toks.push(Token::Ident("deg".into()));
                } else {
                    toks.push(Token::Ident(s));
                }
            }
            '+' | '-' | '*' | '/' | '^' | '%' | '!' => {
                toks.push(Token::Op(c));
                i += 1;
            }
            '(' => { toks.push(Token::LParen); i += 1; }
            ')' => { toks.push(Token::RParen); i += 1; }
            ' ' | '\t' => { i += 1; }
            _ => return Err(format!("알 수 없는 문자: '{}'", c)),
        }
    }
    Ok(toks)
}

impl Eval {
    fn to_rad(&self, x: f64) -> f64 {
        match self.deg {
            AngleMode::Deg => x * std::f64::consts::PI / 180.0,
            AngleMode::Rad => x,
        }
    }

    pub fn call_fn(&self, name: &str, arg: f64) -> Option<f64> {
        Some(match name {
            "sin" => self.to_rad(arg).sin(),
            "cos" => self.to_rad(arg).cos(),
            "tan" => self.to_rad(arg).tan(),
            "ln" => arg.ln(),
            "log" => arg.log10(),
            "log2" => arg.log2(),
            "sqrt" => arg.sqrt(),
            "abs" => arg.abs(),
            _ => return None,
        })
    }

    /// 사칙연산 우선순위 평가기 (파서 없이 토큰 순회).
    /// 결함 2: unary minus를 처리하지 않아 "-1+2"가 파싱 에러로 갑니다.
    /// 결함 3(애매): f64 부동소수 오차가 있는데 is_ok로 판정하는 테스트 하나 —
    ///   리플렉션이 반증하거나 확인해야 할 종류의 파인딩입니다.
    pub fn eval_expr(&self, input: &str) -> Result<f64, String> {
        let toks = tokenize(input)?;
        let mut pos = 0usize;
        let val = self.parse_add(&toks, &mut pos)?;
        if pos != toks.len() {
            return Err(format!("남은 토큰: {:?}", &toks[pos..]));
        }
        Ok(val)
    }

    fn parse_add(&self, toks: &[Token], pos: &mut usize) -> Result<f64, String> {
        let mut left = self.parse_mul(toks, pos)?;
        while *pos < toks.len() {
            if let Token::Op(op @ ('+' | '-')) = toks[*pos] {
                *pos += 1;
                let right = self.parse_mul(toks, pos)?;
                left = match op {
                    '+' => left + right,
                    '-' => left - right,
                    _ => unreachable!(),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_mul(&self, toks: &[Token], pos: &mut usize) -> Result<f64, String> {
        let mut left = self.parse_pow(toks, pos)?;
        while *pos < toks.len() {
            if let Token::Op(op @ ('*' | '/' | '%')) = toks[*pos] {
                *pos += 1;
                let right = self.parse_pow(toks, pos)?;
                left = match op {
                    '*' => left * right,
                    '/' => left / right, // 결함 4: 0으로 나누면 IEEE inf (명시적 처리 없음)
                    '%' => left % right,
                    _ => unreachable!(),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_pow(&self, toks: &[Token], pos: &mut usize) -> Result<f64, String> {
        let base = self.parse_atom(toks, pos)?;
        if *pos < toks.len() {
            if let Token::Op('^') = toks[*pos] {
                *pos += 1;
                // 결함 5: 오른쪽 결합이 아니라 왼쪽 결합 — "2^3^2"가 64가 아니라 64...?
                // 실제로는 (2^3)^2 = 64가 되어야 하는데 512가 나와야 합니다.
                let exp = self.parse_pow(toks, pos)?;
                return Ok(base.powf(exp));
            }
        }
        Ok(base)
    }

    fn parse_atom(&self, toks: &[Token], pos: &mut usize) -> Result<f64, String> {
        match toks.get(*pos) {
            Some(Token::Number(n)) => { *pos += 1; Ok(*n) }
            Some(Token::Ident(name)) => {
                *pos += 1;
                match name.as_str() {
                    "pi" => Some(std::f64::consts::PI),
                    "e" => Some(std::f64::consts::E),
                    _ => None,
                }.ok_or_else(|| format!("알 수 없는 식별자: {}", name))
            }
            Some(Token::LParen) => {
                *pos += 1;
                let v = self.parse_add(toks, pos)?;
                match toks.get(*pos) {
                    Some(Token::RParen) => { *pos += 1; Ok(v) }
                    _ => Err("닫는 괄호 없음".to_string()),
                }
            }
            _ => Err(format!("예상치 못한 토큰: {:?}", toks.get(*pos))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ops() {
        let ev = Eval { deg: AngleMode::Rad };
        assert!((ev.eval_expr("1+2*3").unwrap() - 7.0).abs() < 1e-9);
        assert!((ev.eval_expr("(1+2)*3").unwrap() - 9.0).abs() < 1e-9);
    }

    #[test]
    fn floats() {
        let ev = Eval { deg: AngleMode::Rad };
        // 애매한 결함: 부동소수 비교
        assert_eq!(ev.eval_expr("0.1+0.2").unwrap(), 0.3);
    }

    #[test]
    fn constants() {
        let ev = Eval { deg: AngleMode::Deg };
        assert!((ev.eval_expr("sin(30)").unwrap() - 0.5).abs() < 1e-9);
    }
}
