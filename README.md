# calcu — 공학용 계산기 (Rust)

Rust + [egui/eframe](https://github.com/emilk/egui) 기반 데스크톱 공학용 계산기.

> 🚧 프로젝트 진행 중 — 기능은 PR로 단계적으로 추가됩니다.

## 실행 (기능 병합 후)

```bash
cargo run --release
```

## 계획된 기능

- 수식 파서 기반 계산 (사칙연산, `^`, `mod`, `%`, `!`)
- 삼각/역삼각/쌍곡 함수, 로그, 지수 — DEG/RAD 모드
- 조합론 (nCr, nPr), 정수론 (gcd, lcm)
- 상수(pi, e), 진법 리터럴(0x/0b/0o), 과학적 표기
- 실시간 미리보기, 계산 기록, 한글 UI

## 구조

```
src/
├── main.rs   # GUI (egui)
└── eval.rs   # 수식 엔진 (토크나이저 + 파서 + 평가기)
```
## CI

AI review: CodeGoose (release-only distribution, tracks releases/latest).
