//! 수식 해석기: 토크나이저 + 재귀 하강 파서 + 평가기.
//!
//! 지원: 사칙연산, `^`(지수), `mod`, `%`(백분율), `!`(팩토리얼),
//! 삼각/역삼각/쌍곡 함수(DEG·RAD 모드), 로그, nCr·nPr, gcd·lcm,
//! 상수 `pi`/`π`/`e`, 진법 리터럴(`0xFF`, `0b1011`, `0o17`),
//! 과학적 표기(`2e5`), 암시적 곱셈(`2pi`, `3(4+5)`).

use std::f64::consts::{E, PI};

// ─── 토큰 ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Ln,
    Log,
    Log2,
    Sqrt,
    Cbrt,
    Exp,
    Abs,
    Floor,
    Ceil,
    Round,
    Sign,
    Fact,
    NCr,
    NPr,
    Gcd,
    Lcm,
    Min,
    Max,
    Pow,
    #[allow(dead_code)]
    Mod,
    Atan2,
    Hypot,
}

impl Func {
    fn name(self) -> &'static str {
        match self {
            Func::Sin => "sin",
            Func::Cos => "cos",
            Func::Tan => "tan",
            Func::Asin => "asin",
            Func::Acos => "acos",
            Func::Atan => "atan",
            Func::Sinh => "sinh",
            Func::Cosh => "cosh",
            Func::Tanh => "tanh",
            Func::Ln => "ln",
            Func::Log => "log",
            Func::Log2 => "log2",
            Func::Sqrt => "sqrt",
            Func::Cbrt => "cbrt",
            Func::Exp => "exp",
            Func::Abs => "abs",
            Func::Floor => "floor",
            Func::Ceil => "ceil",
            Func::Round => "round",
            Func::Sign => "sign",
            Func::Fact => "fact",
            Func::NCr => "nCr",
            Func::NPr => "nPr",
            Func::Gcd => "gcd",
            Func::Lcm => "lcm",
            Func::Min => "min",
            Func::Max => "max",
            Func::Pow => "pow",
            Func::Mod => "mod",
            Func::Atan2 => "atan2",
            Func::Hypot => "hypot",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    Num(f64),
    Pi,
    Euler,
    Func(Func),
    Plus,
    Minus,
    Mul,
    Div,
    Pow,
    #[allow(dead_code)]
    Mod,
    LPar,
    RPar,
    Comma,
    Fact,    // 후위: 팩토리얼
    Percent, // 후위: 백분율
}

const NAMES: &[(&str, Tok)] = &[
    ("asin", Tok::Func(Func::Asin)),
    ("acos", Tok::Func(Func::Acos)),
    ("atan2", Tok::Func(Func::Atan2)),
    ("atan", Tok::Func(Func::Atan)),
    ("sinh", Tok::Func(Func::Sinh)),
    ("sin", Tok::Func(Func::Sin)),
    ("cosh", Tok::Func(Func::Cosh)),
    ("cos", Tok::Func(Func::Cos)),
    ("tanh", Tok::Func(Func::Tanh)),
    ("tan", Tok::Func(Func::Tan)),
    ("sqrt", Tok::Func(Func::Sqrt)),
    ("cbrt", Tok::Func(Func::Cbrt)),
    ("log2", Tok::Func(Func::Log2)),
    ("log", Tok::Func(Func::Log)),
    ("ln", Tok::Func(Func::Ln)),
    ("exp", Tok::Func(Func::Exp)),
    ("abs", Tok::Func(Func::Abs)),
    ("floor", Tok::Func(Func::Floor)),
    ("ceil", Tok::Func(Func::Ceil)),
    ("round", Tok::Func(Func::Round)),
    ("sign", Tok::Func(Func::Sign)),
    ("fact", Tok::Func(Func::Fact)),
    ("nCr", Tok::Func(Func::NCr)),
    ("nPr", Tok::Func(Func::NPr)),
    ("gcd", Tok::Func(Func::Gcd)),
    ("lcm", Tok::Func(Func::Lcm)),
    ("min", Tok::Func(Func::Min)),
    ("max", Tok::Func(Func::Max)),
    ("pow", Tok::Func(Func::Pow)),
    ("mod", Tok::Mod),
    ("hypot", Tok::Func(Func::Hypot)),
    ("pi", Tok::Pi),
    ("e", Tok::Euler),
];

// ─── 토크나이저 ─────────────────────────────────────────────────

fn match_name(rest: &str) -> Option<(&'static str, Tok)> {
    NAMES
        .iter()
        .filter(|(name, _)| rest.starts_with(name))
        .max_by_key(|(name, _)| name.len())
        .map(|(name, tok)| (*name, *tok))
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out: Vec<Tok> = Vec::new();
    let mut i = 0;

    while i < n {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // 진법 리터럴: 0x1F, 0b1011, 0o17 (자릿수 구분 `_` 허용)
        if c == '0' && i + 1 < n {
            let radix = match chars[i + 1].to_ascii_lowercase() {
                'x' => Some(16u32),
                'b' => Some(2),
                'o' => Some(8),
                _ => None,
            };
            if let Some(radix) = radix {
                let mut j = i + 2;
                let mut digits = String::new();
                while j < n {
                    let d = chars[j];
                    if d == '_' {
                        j += 1;
                        continue;
                    }
                    let valid = match radix {
                        16 => d.is_ascii_hexdigit(),
                        8 => ('0'..='7').contains(&d),
                        _ => d == '0' || d == '1',
                    };
                    if valid {
                        digits.push(d);
                        j += 1;
                    } else {
                        break;
                    }
                }
                if digits.is_empty() {
                    return Err(format!("'0{}' 뒤에 유효한 숫자가 필요합니다", chars[i + 1]));
                }
                let v = u64::from_str_radix(&digits, radix)
                    .map_err(|_| "진법 리터럴을 해석할 수 없습니다")? as f64;
                out.push(Tok::Num(v));
                i = j;
                continue;
            }
        }

        // 숫자 (소수점 1개 + 선택적 지수 표기 e/E)
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            let mut seen_dot = false;
            while i < n {
                let d = chars[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' {
                    if seen_dot {
                        return Err("숫자에 소수점(.)이 여러 개 있습니다".into());
                    }
                    seen_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            if text == "." {
                return Err("완전하지 않은 숫자입니다".into());
            }
            let mut value: f64 = text
                .parse()
                .map_err(|_| format!("잘못된 숫자: {text}"))?;

            // 지수부: 5e3, 2.5e-4, 1E+6
            if i < n && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                let mut neg = false;
                if j < n && (chars[j] == '+' || chars[j] == '-') {
                    neg = chars[j] == '-';
                    j += 1;
                }
                if j < n && chars[j].is_ascii_digit() {
                    let ds = j;
                    while j < n && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    let exp: i32 = chars[ds..j]
                        .iter()
                        .collect::<String>()
                        .parse()
                        .unwrap_or(0);
                    value *= 10f64.powi(if neg { -exp } else { exp });
                    i = j;
                }
                // 지수부가 없으면 'e'는 상수 e → 다음 토큰 처리에서 암시적 곱셈
            }
            out.push(Tok::Num(value));
            continue;
        }

        // 함수·상수 이름
        let rest: String = chars[i..].iter().collect();
        if let Some((name, tok)) = match_name(&rest) {
            out.push(tok);
            i += name.len();
            continue;
        }

        // 기호
        let tok = match c {
            '+' => Tok::Plus,
            '-' | '−' => Tok::Minus,
            '*' | '×' => Tok::Mul,
            '/' | '÷' => Tok::Div,
            '^' => Tok::Pow,
            '!' => Tok::Fact,
            '(' | '[' => Tok::LPar,
            ')' | ']' => Tok::RPar,
            ',' => Tok::Comma,
            '%' => Tok::Percent,
            'π' => Tok::Pi,
            '√' => Tok::Func(Func::Sqrt),
            other => return Err(format!("알 수 없는 문자: '{other}'")),
        };
        out.push(tok);
        i += 1;
    }

    if out.is_empty() {
        return Err("빈 수식입니다".into());
    }

    // 암시적 곱셈: 값 뒤에 값이 오면 `*` 삽입 (2pi, 3(4+5), (1+2)(3+4), 2e3 아님 등)
    let mut toks: Vec<Tok> = Vec::with_capacity(out.len());
    for t in out {
        if let Some(last) = toks.last() {
            let ends_value =
                matches!(last, Tok::Num(_) | Tok::Pi | Tok::Euler | Tok::RPar | Tok::Fact | Tok::Percent);
            let starts_value =
                matches!(t, Tok::Num(_) | Tok::Pi | Tok::Euler | Tok::Func(_) | Tok::LPar);
            if ends_value && starts_value {
                toks.push(Tok::Mul);
            }
        }
        toks.push(t);
    }
    Ok(toks)
}

// ─── 파서 ───────────────────────────────────────────────────────

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn parse(&mut self, deg: bool) -> Result<f64, String> {
        let v = self.expr(deg)?;
        if let Some(t) = self.peek() {
            return Err(format!("해석할 수 없는 토큰: {t:?}"));
        }
        Ok(v)
    }

    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.pos).copied()
    }

    /// 덧셈/뺄셈 (가장 낮은 우선순위)
    fn expr(&mut self, deg: bool) -> Result<f64, String> {
        let mut lhs = self.term(deg)?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.pos += 1;
                    lhs += self.term(deg)?;
                }
                Some(Tok::Minus) => {
                    self.pos += 1;
                    lhs -= self.term(deg)?;
                }
                _ => return Ok(lhs),
            }
        }
    }

    /// 곱셈/나눗셈/모듈로
    fn term(&mut self, deg: bool) -> Result<f64, String> {
        let mut lhs = self.unary(deg)?;
        loop {
            match self.peek() {
                Some(Tok::Mul) => {
                    self.pos += 1;
                    lhs *= self.unary(deg)?;
                }
                Some(Tok::Div) => {
                    self.pos += 1;
                    lhs /= self.unary(deg)?;
                }
                Some(Tok::Mod) => {
                    self.pos += 1;
                    let rhs = self.unary(deg)?;
                    if rhs == 0.0 {
                        return Err("0으로 나눌 수 없습니다 (mod)".into());
                    }
                    lhs %= rhs;
                }
                _ => return Ok(lhs),
            }
        }
    }

    /// 단항 +/-
    fn unary(&mut self, deg: bool) -> Result<f64, String> {
        match self.peek() {
            Some(Tok::Plus) => {
                self.pos += 1;
                self.unary(deg)
            }
            Some(Tok::Minus) => {
                self.pos += 1;
                Ok(-self.unary(deg)?)
            }
            _ => self.power(deg),
        }
    }

    /// 거듭제곱 `^` — 오른쪽 결합 (2^3^2 = 2^(3^2) = 512)
    fn power(&mut self, deg: bool) -> Result<f64, String> {
        let base = self.postfix(deg)?;
        if self.peek() == Some(Tok::Pow) {
            self.pos += 1;
            let exp = self.unary(deg)?; // 오른쪽 결합 + 단항 부호 허용: 2^-3
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    /// 후위 연산자: ! (팩토리얼), % (백분율)
    fn postfix(&mut self, deg: bool) -> Result<f64, String> {
        let mut v = self.primary(deg)?;
        loop {
            match self.peek() {
                Some(Tok::Fact) => {
                    self.pos += 1;
                    v = factorial_checked(v)?;
                }
                Some(Tok::Percent) => {
                    self.pos += 1;
                    v /= 100.0;
                }
                _ => return Ok(v),
            }
        }
    }

    fn primary(&mut self, deg: bool) -> Result<f64, String> {
        let tok = self.peek();
        match tok {
            Some(Tok::Num(v)) => {
                self.pos += 1;
                Ok(v)
            }
            Some(Tok::Pi) => {
                self.pos += 1;
                Ok(PI)
            }
            Some(Tok::Euler) => {
                self.pos += 1;
                Ok(E)
            }
            Some(Tok::Func(f)) => {
                self.pos += 1;
                if self.peek() != Some(Tok::LPar) {
                    return Err(format!("함수 {} 뒤에는 '('가 필요합니다", f.name()));
                }
                self.pos += 1;

                let mut args = Vec::new();
                if self.peek() == Some(Tok::RPar) {
                    self.pos += 1; // 빈 인수
                } else {
                    loop {
                        args.push(self.expr(deg)?);
                        match self.peek() {
                            Some(Tok::Comma) => self.pos += 1,
                            Some(Tok::RPar) => {
                                self.pos += 1;
                                break;
                            }
                            other => {
                                return Err(format!(
                                    "함수 {}: 인수 뒤에는 ',' 또는 ')'가 필요합니다 (발견: {other:?})",
                                    f.name()
                                ));
                            }
                        }
                    }
                }
                eval_func(f, &args, deg)
            }
            Some(Tok::LPar) => {
                self.pos += 1;
                let v = self.expr(deg)?;
                if self.peek() != Some(Tok::RPar) {
                    return Err("닫는 괄호 ')'가 필요합니다".into());
                }
                self.pos += 1;
                Ok(v)
            }
            other => Err(format!("수식을 해석할 수 없습니다: {other:?}")),
        }
    }
}

fn factorial_checked(v: f64) -> Result<f64, String> {
    if v < 0.0 || v.fract() != 0.0 || v > 170.0 {
        return Err("팩토리얼(!)은 0~170 범위의 정수에만 사용할 수 있습니다".into());
    }
    let mut acc = 1.0f64;
    let mut k = 2.0;
    while k <= v {
        acc *= k;
        k += 1.0;
    }
    Ok(acc)
}

fn gcd_checked(a: f64, b: f64) -> Result<u64, String> {
    if a.fract() != 0.0 || b.fract() != 0.0 || a < 0.0 || b < 0.0 {
        return Err("gcd/lcm에는 0 이상의 정수를 입력하세요".into());
    }
    let (mut a, mut b) = (a as u64, b as u64);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    Ok(a)
}

// ─── 함수 평가 ──────────────────────────────────────────────────

fn eval_func(f: Func, args: &[f64], deg: bool) -> Result<f64, String> {
    let one = |g: &dyn Fn(f64) -> f64| -> Result<f64, String> {
        if args.len() != 1 {
            Err(format!("{} 함수는 인수 1개가 필요합니다", f.name()))
        } else {
            Ok(g(args[0]))
        }
    };
    let two = |g: &dyn Fn(f64, f64) -> Result<f64, String>| -> Result<f64, String> {
        if args.len() != 2 {
            Err(format!("{} 함수는 인수 2개가 필요합니다", f.name()))
        } else {
            g(args[0], args[1])
        }
    };

    let conv_in = |x: f64| if deg { x.to_radians() } else { x };
    let conv_out = |x: f64| if deg { x.to_degrees() } else { x };

    match f {
        Func::Sin => one(&|x| conv_in(x).sin()),
        Func::Cos => one(&|x| conv_in(x).cos()),
        Func::Tan => one(&|x| conv_in(x).tan()),
        Func::Asin => one(&|x| conv_out(x.asin())),
        Func::Acos => one(&|x| conv_out(x.acos())),
        Func::Atan => one(&|x| conv_out(x.atan())),
        // 쌍곡형 함수의 입력은 각도가 아니므로 변환하지 않는다
        Func::Sinh => one(&|x| x.sinh()),
        Func::Cosh => one(&|x| x.cosh()),
        Func::Tanh => one(&|x| x.tanh()),
        Func::Ln => one(&|x| x.ln()),
        Func::Log => one(&|x| x.log10()),
        Func::Log2 => one(&|x| x.log2()),
        Func::Sqrt => one(&|x| x.sqrt()),
        Func::Cbrt => one(&|x| x.cbrt()),
        Func::Exp => one(&|x| x.exp()),
        Func::Abs => one(&|x| x.abs()),
        Func::Floor => one(&|x| x.floor()),
        Func::Ceil => one(&|x| x.ceil()),
        Func::Round => one(&|x| x.round()),
        Func::Sign => one(&|x| x.signum()),
        Func::Fact => one(&|x| factorial_checked(x).unwrap_or(f64::NAN)),
        Func::Min => two(&|a, b| Ok(a.min(b))),
        Func::Max => two(&|a, b| Ok(a.max(b))),
        Func::Pow => two(&|a, b| Ok(a.powf(b))),
        Func::Atan2 => two(&|a, b| Ok(conv_out(a.atan2(b)))),
        Func::Hypot => two(&|a, b| Ok(a.hypot(b))),
        Func::Mod => two(&|a, b| {
            if b == 0.0 {
                Err("0으로 나눌 수 없습니다 (mod)".into())
            } else {
                Ok(a % b)
            }
        }),
        Func::Gcd => two(&|a, b| Ok(gcd_checked(a, b)? as f64)),
        Func::Lcm => two(&|a, b| {
            let g = gcd_checked(a, b)?;
            if g == 0 {
                return Ok(0.0);
            }
            Ok((a as u128 * b as u128 / g as u128) as f64)
        }),
        Func::NCr => two(&|n, r| {
            if n < 0.0 || r < 0.0 || r > n || n.fract() != 0.0 || r.fract() != 0.0 || n > 170.0 {
                return Err("nCr(n, r): 0 ≤ r ≤ n 인 정수를 입력하세요".into());
            }
            let r = r as u64;
            let mut result = 1.0f64;
            for k in 1..=r {
                result = result * (n - (r - k) as f64) / k as f64;
            }
            Ok(result)
        }),
        Func::NPr => two(&|n, r| {
            if n < 0.0 || r < 0.0 || r > n || n.fract() != 0.0 || r.fract() != 0.0 || n > 170.0 {
                return Err("nPr(n, r): 0 ≤ r ≤ n 인 정수를 입력하세요".into());
            }
            let mut result = 1.0f64;
            for k in 0..r as u64 {
                result *= n - k as f64;
            }
            Ok(result)
        }),
    }
}

// ─── 공개 API ───────────────────────────────────────────────────

/// 수식을 계산한다. `deg = true`이면 각도 함수는 도 단위로 동작한다.
pub fn evaluate(src: &str, deg: bool) -> Result<f64, String> {
    let toks = tokenize(src)?;
    Parser { toks, pos: 0 }.parse(deg)
}

/// 결과를 사람이 읽기 좋게 포맷한다.
pub fn fmt_value(v: f64) -> String {
    if v.is_nan() {
        return "정의되지 않음".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "∞" } else { "-∞" }.into();
    }
    if v == 0.0 {
        return "0".into();
    }

    let abs = v.abs();
    if abs >= 1e15 || abs < 1e-9 {
        return format!("{v:.6e}");
    }

    // 소수 12자리로 반올림해 부동소수점 오차를 정리한 뒤 불필요한 0 제거
    let s = format!("{v:.12}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    if s == "-0" {
        return "0".into();
    }
    // 표시가 너무 길면 유효숫자 12자리로 재포맷
    if s.len() > 17 {
        return format!("{v:.6e}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_arithmetic() {
        assert_eq!(evaluate("1+2*3", true), Ok(7.0));
        assert_eq!(evaluate("(1+2)*3", true), Ok(9.0));
        assert_eq!(evaluate("2^3^2", true), Ok(512.0));
        assert_eq!(evaluate("2^-3", true), Ok(0.125));
        assert_eq!(evaluate("7 mod 3", true), Ok(1.0));
        assert_eq!(evaluate("50%", true), Ok(0.5));
    }

    #[test]
    fn implicit_multiplication() {
        assert_eq!(evaluate("2pi", true), Ok(2.0 * PI));
        assert_eq!(evaluate("3(4+5)", true), Ok(27.0));
        assert_eq!(evaluate("(1+2)(3+4)", true), Ok(21.0));
    }

    #[test]
    fn functions() {
        assert!((evaluate("sin(30)", true).unwrap() - 0.5).abs() < 1e-12);
        assert!((evaluate("sin(pi/2)", false).unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(evaluate("sqrt(16)", true), Ok(4.0));
        assert_eq!(evaluate("log(100)", true), Ok(2.0));
        assert_eq!(evaluate("5!", true), Ok(120.0));
        assert_eq!(evaluate("nCr(5,2)", true), Ok(10.0));
        assert_eq!(evaluate("gcd(12,18)", true), Ok(6.0));
    }

    #[test]
    fn bases_and_scientific() {
        assert_eq!(evaluate("0xFF", true), Ok(255.0));
        assert_eq!(evaluate("0b1011", true), Ok(11.0));
        assert_eq!(evaluate("0o17", true), Ok(15.0));
        assert_eq!(evaluate("2e5", true), Ok(200000.0));
    }

    #[test]
    fn formatting() {
        assert_eq!(fmt_value(0.1 + 0.2), "0.3");
        assert_eq!(fmt_value(3.0), "3");
        assert_eq!(fmt_value(2.5), "2.5");
    }
}
