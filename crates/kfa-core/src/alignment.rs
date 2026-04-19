use ndarray::{Array1, Array2};
use std::f32;

#[derive(Debug, Clone)]
pub struct Point {
    pub token_index: usize,
    pub time_index: usize,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub score: f32,
}

impl Segment {
    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

pub fn log_softmax(emissions: &Array2<f32>) -> Array2<f32> {
    let mut out = emissions.clone();
    for mut row in out.rows_mut() {
        let max_val = row.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let sum_exp = row.iter().fold(0.0, |acc, &x| acc + (x - max_val).exp());
        let log_sum_exp = max_val + sum_exp.ln();
        for val in row.iter_mut() {
            *val -= log_sum_exp;
        }
    }
    out
}

pub fn get_trellis(emission: &Array2<f32>, tokens: &[usize], blank_id: usize) -> Array2<f32> {
    let num_frame = emission.shape()[0];
    let num_tokens = tokens.len();

    let mut trellis = Array2::<f32>::from_elem((num_frame, num_tokens), f32::NEG_INFINITY);

    // Initial boundary
    trellis[[0, 0]] = emission[[0, blank_id]];
    for t in 1..num_frame {
        trellis[[t, 0]] = trellis[[t - 1, 0]] + emission[[t, blank_id]];
    }

    for t in 0..num_frame - 1 {
        for j in 1..num_tokens {
            // Stay at the same token
            let stay = trellis[[t, j]] + emission[[t, blank_id]];
            // Change to the next token
            let change = trellis[[t, j - 1]] + emission[[t, tokens[j]]];

            trellis[[t + 1, j]] = stay.max(change);
        }
    }
    trellis
}

pub fn backtrack(
    trellis: &Array2<f32>,
    emission: &Array2<f32>,
    tokens: &[usize],
    blank_id: usize,
) -> Vec<Point> {
    let mut t = trellis.shape()[0] - 1;
    let mut j = trellis.shape()[1] - 1;
    let mut path = vec![Point {
        token_index: j,
        time_index: t,
        score: emission[[t, blank_id]].exp(),
    }];

    while j > 0 {
        assert!(t > 0);

        let p_stay = emission[[t - 1, blank_id]];
        let p_change = emission[[t - 1, tokens[j]]];

        let stayed = trellis[[t - 1, j]] + p_stay;
        let changed = trellis[[t - 1, j - 1]] + p_change;

        if changed > stayed {
            j -= 1;
        }
        t -= 1;

        let prob = if changed > stayed {
            p_change.exp()
        } else {
            p_stay.exp()
        };

        path.push(Point {
            token_index: j,
            time_index: t,
            score: prob,
        });
    }

    while t > 0 {
        let prob = emission[[t - 1, blank_id]].exp();
        path.push(Point {
            token_index: j,
            time_index: t - 1,
            score: prob,
        });
        t -= 1;
    }

    path.reverse();
    path
}

pub fn merge_repeats(path: &[Point], transcript: &str) -> Vec<Segment> {
    let chars: Vec<char> = transcript.chars().collect();
    let mut segments = Vec::new();

    let mut i1 = 0;
    while i1 < path.len() {
        let mut i2 = i1;
        while i2 < path.len() && path[i1].token_index == path[i2].token_index {
            i2 += 1;
        }
        let score_sum: f32 = path[i1..i2].iter().map(|p| p.score).sum();
        let score = score_sum / ((i2 - i1) as f32);

        let label = if path[i1].token_index < chars.len() {
            chars[path[i1].token_index].to_string()
        } else {
            String::new()
        };

        segments.push(Segment {
            label,
            start: path[i1].time_index,
            end: path[i2 - 1].time_index + 1,
            score,
        });
        i1 = i2;
    }
    segments
}

pub fn merge_words(segments: &[Segment], separator: &str) -> Vec<Segment> {
    let mut words = Vec::new();
    let mut i1 = 0;
    let mut i2 = 0;

    while i1 < segments.len() {
        if i2 >= segments.len() || segments[i2].label == separator {
            if i1 != i2 {
                let segs = &segments[i1..i2];
                let word: String = segs.iter().map(|s| s.label.as_str()).collect();
                
                let mut sum_score_len = 0.0;
                let mut sum_len = 0;
                for seg in segs {
                    let l = seg.length();
                    sum_score_len += seg.score * (l as f32);
                    sum_len += l;
                }
                
                let score = if sum_len > 0 {
                    sum_score_len / (sum_len as f32)
                } else {
                    0.0
                };

                words.push(Segment {
                    label: word,
                    start: segments[i1].start,
                    end: segments[i2 - 1].end,
                    score,
                });
            }
            i2 += 1;
            i1 = i2;
        } else {
            i2 += 1;
        }
    }
    words
}
