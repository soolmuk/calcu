//! 공학용 계산기 GUI (Rust + egui/eframe)
//!
//! 실행: cargo run --release

mod eval;

use eframe::egui;
use egui::{
    Align, Button, Color32, FontData, FontDefinitions, FontFamily, Label, Layout, RichText, Sense,
    Vec2,
};
use eval::evaluate;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 720.0])
            .with_min_inner_size([460.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "공학용 계산기",
        options,
        Box::new(|cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::new(CalculatorApp::new()) as Box<dyn eframe::App>)
        }),
    )
}

// ─── 한글 폰트 (플랫폼별 시스템 폰트 폴백) ──────────────────────

fn install_fonts(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "C:\\Windows\\Fonts\\malgun.ttf",
    ];
    let mut fonts = FontDefinitions::default();
    for path in CANDIDATES {
        if let Ok(data) = std::fs::read(path) {
            fonts
                .font_data
                .insert("korean".into(), FontData::from_owned(data));
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push("korean".into());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

// ─── 상태 ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum AngleMode {
    Deg,
    Rad,
}

struct HistoryEntry {
    expr: String,
    result: String,
}

struct CalculatorApp {
    expr: String,
    history: Vec<HistoryEntry>,
    angle_mode: AngleMode,
    error: Option<String>,
}

impl CalculatorApp {
    fn new() -> Self {
        Self {
            expr: String::new(),
            history: Vec::new(),
            angle_mode: AngleMode::Deg,
            error: None,
        }
    }

    fn insert(&mut self, token: &str) {
        self.expr.push_str(token);
        self.error = None;
    }

    fn push_history(&mut self, entry: HistoryEntry) {
        // 기록 상한: 오래된 항목부터 제거해 무한 증가를 막는다.
        const MAX_HISTORY: usize = 100;
        self.history.push(entry);
        if self.history.len() > MAX_HISTORY {
            let overflow = self.history.len() - MAX_HISTORY;
            self.history.drain(0..overflow);
        }
    }

    fn backspace(&mut self) {
        self.expr.pop();
        self.error = None;
    }

    fn clear(&mut self) {
        self.expr.clear();
        self.error = None;
    }

    fn calculate(&mut self) {
        if self.expr.trim().is_empty() {
            return;
        }
        match evaluate(&self.expr, self.angle_mode == AngleMode::Deg) {
            Ok(v) => {
                let result = eval::fmt_value(v);
                let expr = std::mem::take(&mut self.expr);
                self.push_history(HistoryEntry {
                    expr,
                    result: result.clone(),
                });
                self.expr = result;
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }
}

// ─── 키 정의 ────────────────────────────────────────────────────

struct Key {
    label: &'static str,
    action: &'static str,
    kind: KeyKind,
}

#[derive(PartialEq, Clone, Copy)]
enum KeyKind {
    Digit,
    Op,
    Func,
    Paren,
    Clear,
    Equals,
}

const KEYS: &[&[Key]] = &[
    &[k2("sin", "sin(", KeyKind::Func), k2("cos", "cos(", KeyKind::Func), k2("tan", "tan(", KeyKind::Func),
      k2("asin", "asin(", KeyKind::Func), k2("acos", "acos(", KeyKind::Func), k2("atan", "atan(", KeyKind::Func)],
    &[k2("ln", "ln(", KeyKind::Func), k2("log", "log(", KeyKind::Func),
      k2("√", "sqrt(", KeyKind::Func), k2("x^y", "^", KeyKind::Func),
      k2("x²", "^(2)", KeyKind::Func), k2("e^x", "exp(", KeyKind::Func)],
    &[k2("π", "pi", KeyKind::Func), k2("e", "e", KeyKind::Func),
      k2("|x|", "abs(", KeyKind::Func), k2("!", "!", KeyKind::Func),
      k2("%", "%", KeyKind::Func), k2("mod", " mod ", KeyKind::Func)],
    &[k2("(", "(", KeyKind::Paren), k2(")", ")", KeyKind::Paren),
      k2("⌫", "BS", KeyKind::Clear), k2("AC", "AC", KeyKind::Clear)],
    &[d("7"), d("8"), d("9"), k2("÷", "/", KeyKind::Op)],
    &[d("4"), d("5"), d("6"), k2("×", "*", KeyKind::Op)],
    &[d("1"), d("2"), d("3"), k2("−", "-", KeyKind::Op)],
    &[d("0"), k2(".", ".", KeyKind::Digit), k2("=", "=", KeyKind::Equals), k2("+", "+", KeyKind::Op)],
];

const fn k2(label: &'static str, action: &'static str, kind: KeyKind) -> Key {
    Key { label, action, kind }
}

#[allow(non_upper_case_globals)]
const fn d(x: &'static str) -> Key {
    Key { label: x, action: x, kind: KeyKind::Digit }
}

// ─── UI 렌더링 ──────────────────────────────────────────────────

impl eframe::App for CalculatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let deg = self.angle_mode == AngleMode::Deg;

        egui::TopBottomPanel::top("display").show(ctx, |ui| {
            let resp = ui.add_sized(
                [ui.available_width(), 34.0],
                egui::TextEdit::singleline(&mut self.expr)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("수식을 입력하세요 (예: sin(30)+2^10)"),
            );
            // 키보드로 직접 입력했을 때도 오래된 오류 라벨이 남지 않도록 한다.
            if resp.changed() {
                self.error = None;
            }
            ui.add_space(4.0);

            // 오류 또는 실시간 미리보기
            if let Some(err) = &self.error {
                ui.colored_label(Color32::from_rgb(255, 99, 99), format!("⚠ {err}"));
            } else if !self.expr.trim().is_empty() {
                match evaluate(&self.expr, deg) {
                    Ok(v) => {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(eval::fmt_value(v))
                                    .size(26.0)
                                    .strong()
                                    .color(Color32::from_rgb(120, 200, 255)),
                            );
                        });
                    }
                    Err(_) => {} // 입력 중인 불완전 수식은 조용히 무시
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.selectable_label(self.angle_mode == AngleMode::Deg, "DEG").clicked() {
                    self.angle_mode = AngleMode::Deg;
                    self.error = None;
                }
                if ui.selectable_label(self.angle_mode == AngleMode::Rad, "RAD").clicked() {
                    self.angle_mode = AngleMode::Rad;
                    self.error = None;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("기록 지우기").clicked() {
                        self.history.clear();
                    }
                });
            });
            ui.add_space(2.0);
        });

        // Enter 키로 계산 (디스플레이 TextEdit가 포커스를 가져가지 않은 경우 대비해 전역 폴링)
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.calculate();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            for row in KEYS {
                ui.horizontal(|ui| {
                    // 버튼 사이 실제 간격은 egui item_spacing을 따라간다 (기본 8.0px).
                    let gap = ui.style().spacing.item_spacing.x;
                    let bw = (ui.available_width() - gap * (row.len() as f32 - 1.0))
                        / row.len() as f32;
                    for key in *row {
                        let b = Button::new(RichText::new(key.label).size(15.0))
                            .min_size(Vec2::new(bw, 42.0))
                            .fill(key_color(key.kind));
                        if ui.add(b).clicked() {
                            match key.action {
                                "BS" => self.backspace(),
                                "AC" => self.clear(),
                                "=" => self.calculate(),
                                _ => self.insert(key.action),
                            }
                        }
                    }
                });
                ui.add_space(4.0);
            }

            if !self.history.is_empty() {
                ui.add_space(2.0);
                ui.separator();
                ui.label(RichText::new("기록").strong());
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    // 클릭 시 entry.result 삽입이 필요하므로 인덱스로 접근(borrow 충돌 회피)
                    let last = self.history.len();
                    for i in (0..last).rev() {
                        // 문자열을 복제하지 않고 참조로 렌더링한다. 클릭 결과만
                        // 클로저 밖으로 반환해, 클릭 시에만 소유값을 만든다(NLL).
                        let entry = &self.history[i];
                        let clicked = ui
                            .horizontal(|ui| {
                                ui.label(RichText::new(entry.expr.as_str()).size(11.0).weak());
                                ui.add(
                                    Label::new(
                                        RichText::new(entry.result.as_str())
                                            .strong()
                                            .color(Color32::from_rgb(120, 200, 255)),
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            })
                            .inner;
                        if clicked {
                            let result = self.history[i].result.clone();
                            self.insert(&result);
                        }
                    }
                });
            }
        });
    }
}

fn key_color(kind: KeyKind) -> Color32 {
    match kind {
        KeyKind::Digit => Color32::from_rgb(52, 56, 66),
        KeyKind::Op => Color32::from_rgb(62, 66, 78),
        KeyKind::Func => Color32::from_rgb(45, 49, 58),
        KeyKind::Paren => Color32::from_rgb(58, 62, 74),
        KeyKind::Clear => Color32::from_rgb(120, 60, 60),
        KeyKind::Equals => Color32::from_rgb(40, 90, 200),
    }
}
