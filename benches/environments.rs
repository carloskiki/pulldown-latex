use criterion::{criterion_group, criterion_main, Criterion};
use pulldown_latex::{push_mathml, Parser, Storage};

fn round_trip(input: &str) {
    let storage = Storage::new();
    let parser = Parser::new(input, &storage);
    let mut str = String::new();
    push_mathml(&mut str, parser, Default::default()).unwrap();
}

fn arrays(c: &mut Criterion) {
    c.bench_function("arrays", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{array}{||c|r|l||}
    a + b \\[2em]
    a + b & c & d \\[2em] \hline
    a + b
\end{array}
\begin{array}{c:c:c}
   a & b & c \\ \hline
   d & e & f \\
   \hdashline
   g & h & i
\end{array}"#,
            );
        })
    });
}

fn matrices(c: &mut Criterion) {
    c.bench_function("matrices", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{matrix}
    a & b & c \\
    d & e & f \\
    g & h & i \\
\end{matrix}
\begin{pmatrix*}
    1 & 2 & 3 & 4 & 5 & 6 & 7 & 8 & 9 & 10 \\
    11 & 12 & 13 & 14 & 15 & 16 & 17 & 18 & 19 & 20 \\
    21 & 22 & 23 & 24 & 25 & 26 & 27 & 28 & 29 & 30 \\
    31 & 32 & 33 & 34 & 35 & 36 & 37 & 38 & 39 & 40 \\
    41 & 42 & 43 & 44 & 45 & 46 & 47 & 48 & 49 & 50 \\
    51 & 52 & 53 & 54 & 55 & 56 & 57 & 58 & 59 & 60 \\
    61 & 62 & 63 & 64 & 65 & 66 & 67 & 68 & 69 & 70 \\
    71 & 72 & 73 & 74 & 75 & 76 & 77 & 78 & 79 & 80 \\
    81 & 82 & 83 & 84 & 85 & 86 & 87 & 88 & 89 & 90 \\
    91 & 92 & 93 & 94 & 95 & 96 & 97 & 98 & 99 & 100 \\
\end{pmatrix*}
"#,
            );
        })
    });
}
// Write benchmarks that cover all of the following types of environments:
// 
//  Cases {
//      /// `left` is true if the environment is `cases` and false if the environment is `rcases`.
//      left: bool,
//  },
//  /// The `equation` environment of `LaTeX`.
//  Equation {
//      /// If `eq_numbers` is true, then equation numbers are displayed.
//      eq_numbers: bool,
//  },
//  /// The `align` environment of `LaTeX`.
//  Align {
//      /// If `eq_numbers` is true, then equation numbers are displayed.
//      eq_numbers: bool,
//  },
//  /// The `aligned` environment of `LaTeX`.
//  Aligned,
//  /// The `subarray` environment of `LaTeX`.
//  SubArray {
//      /// The alignment of the columns in the subarray.
//      alignment: ColumnAlignment,
//  },
//  /// The `alignat` environment of `LaTeX`.
//  Alignat {
//      /// `pairs` specifies the number of left-right column pairs specified in the environment
//      /// declaration.
//      pairs: u16,
//      /// If `eq_numbers` is true, then equation numbers are displayed.
//      eq_numbers: bool,
//  },
//  /// The `alignedat` environment of `LaTeX`.
//  Alignedat {
//      /// `pairs` specifies the number of left-right column pairs specified in the environment
//      pairs: u16,
//  },
//  /// The `gather` environment of `LaTeX`.
//  Gather {
//      /// If `eq_numbers` is true, then equation numbers are displayed.
//      eq_numbers: bool,
//  },
//  /// The `gathered` environment of `LaTeX`.
//  Gathered,
//  /// The `multline` environment of `LaTeX`.
//  Multline,
//  /// The `split` environment of `LaTeX`.
//  Split,

fn cases(c: &mut Criterion) {
    c.bench_function("cases", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{cases}
    1 & \text{if } x \ge 0 \\
    0 & \text{if } x < 0
\end{cases}
\begin{rcases}
    a & \text{if } x \ge 0 \\
    b & \text{if } x < 0
\end{rcases}"#,
            );
        })
    });
}

// fn equation(c: &mut Criterion) {
//     c.bench_function("equation", |b| {
//         b.iter(|| {
//             round_trip(
//                 r#"\begin{equation}
//     a^2 + b^2 = c^2
// \end{equation}
// \begin{equation*}
//     a^2 + b^2 = c^2
// \end{equation*}"#,
//             );
//         })
//     });
// }

fn align(c: &mut Criterion) {
    c.bench_function("align", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{align}
    a &= b + c \\
    d &= e + f
\end{align}
\begin{align*}
    a &= b + c \\
    d &= e + f
\end{align*}"#,
            );
        })
    });
}

fn aligned(c: &mut Criterion) {
    c.bench_function("aligned", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{aligned}
    a &= b + c \\
    d &= e + f
\end{aligned}"#,
            );
        })
    });
}

fn subarray(c: &mut Criterion) {
    c.bench_function("subarray", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{subarray}{c}
    a + b \\
    c + d
\end{subarray}
\begin{subarray}{l}
    a & b \\
    c & d
\end{subarray}"#,
            );
        })
    });
}

fn alignat(c: &mut Criterion) {
    c.bench_function("alignat", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{alignat}{2}
    a &= b + c & d &= e + f \\
    g &= h + i & j &= k + l
\end{alignat}
\begin{alignat*}{2}
    a &= b + c & d &= e + f \\
    g &= h + i & j &= k + l
\end{alignat*}"#,
            );
        })
    });
}

fn alignedat(c: &mut Criterion) {
    c.bench_function("alignedat", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{alignedat}{2}
    a &= b + c & d &= e + f \\
    g &= h + i & j &= k + l
\end{alignedat}"#,
            );
        })
    });
}

fn gather(c: &mut Criterion) {
    c.bench_function("gather", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{gather}
    a = b + c \\
    d = e + f
\end{gather}
\begin{gather*}
    a = b + c \\
    d = e + f
\end{gather*}"#,
            );
        })
    });
}

fn gathered(c: &mut Criterion) {
    c.bench_function("gathered", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{gathered}
    a = b + c \\
    d = e + f
\end{gathered}"#,
            );
        })
    });
}

fn multline(c: &mut Criterion) {
    c.bench_function("multline", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{multline}
    a + b + c \\
    d + e + f
\end{multline}"#,
            );
        })
    });
}

fn split(c: &mut Criterion) {
    c.bench_function("split", |b| {
        b.iter(|| {
            round_trip(
                r#"\begin{split}
    a + b + c \\
    d + e + f
\end{split}"#,
            );
        })
    });
}

criterion_group!(benches, arrays, matrices, cases, align, aligned, subarray, alignat, alignedat, gather, gathered, multline, split);
criterion_main!(benches);
