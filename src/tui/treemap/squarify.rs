/// Squarified treemap layout.
///
/// Maps weighted items to non-overlapping rectangles within a `Rect`,
/// keeping aspect ratios close to square (squarified-treemap, Bruls et al.).

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    #[cfg(test)]
    fn area(&self) -> f64 {
        (self.w.max(0) * self.h.max(0)) as f64
    }

    pub fn contains(&self, x: i64, y: i64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Placed {
    pub idx: usize,
    pub rect: Rect,
}

struct Item {
    idx: usize,
    val: f64,
}

/// Squarify `items` (idx + weight) into `rect`. Returns placed rectangles.
pub fn squarify(rect: Rect, items: &[(usize, u64)]) -> Vec<Placed> {
    if rect.w <= 0 || rect.h <= 0 {
        return Vec::new();
    }
    let total: f64 = items.iter().map(|(_, v)| *v as f64).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let mut its: Vec<Item> = items
        .iter()
        .map(|(i, v)| Item {
            idx: *i,
            val: *v as f64,
        })
        .collect();
    its.sort_by(|a, b| {
        b.val
            .partial_cmp(&a.val)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = Vec::with_capacity(items.len());
    recurse(rect, &its, 0, total, &mut out);
    out
}

fn recurse(rect: Rect, items: &[Item], start: usize, total: f64, out: &mut Vec<Placed>) {
    if start >= items.len() || rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let short = rect.w.min(rect.h) as f64;

    // grow a row greedily while aspect ratio improves
    let mut row: Vec<&Item> = Vec::new();
    let mut i = start;
    loop {
        if row.is_empty() {
            row.push(&items[i]);
            i += 1;
            if i >= items.len() {
                break;
            }
        } else {
            let mut cand = row.clone();
            cand.push(&items[i]);
            if worst(&cand, short) <= worst(&row, short) {
                row = cand;
                i += 1;
                if i >= items.len() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    let row_sum: f64 = row.iter().map(|it| it.val).sum();
    let thickness = if row_sum > 0.0 {
        row_sum / total * short
    } else {
        0.0
    };

    // lay the row as a bar along the long side, thickness along the short side
    if rect.w <= rect.h {
        // bar on the left, items stacked vertically
        let tw = thickness.max(1.0);
        let bar = Rect {
            x: rect.x,
            y: rect.y,
            w: tw as i64,
            h: rect.h,
        };
        lay_vertical(bar, &row, row_sum, out);
        let remain = Rect {
            x: rect.x + tw as i64,
            y: rect.y,
            w: rect.w - tw as i64,
            h: rect.h,
        };
        recurse(remain, items, i, total, out);
    } else {
        // bar on top, items laid horizontally
        let th = thickness.max(1.0);
        let bar = Rect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: th as i64,
        };
        lay_horizontal(bar, &row, row_sum, out);
        let remain = Rect {
            x: rect.x,
            y: rect.y + th as i64,
            w: rect.w,
            h: rect.h - th as i64,
        };
        recurse(remain, items, i, total, out);
    }
}

fn lay_vertical(bar: Rect, row: &[&Item], row_sum: f64, out: &mut Vec<Placed>) {
    let mut y = bar.y;
    let n = row.len() as i64;
    for (k, it) in row.iter().enumerate() {
        let frac = it.val / row_sum;
        let h = (frac * bar.h as f64).floor() as i64;
        let is_last = k as i64 == n - 1;
        let h = if is_last { bar.y + bar.h - y } else { h };
        if h > 0 {
            out.push(Placed {
                idx: it.idx,
                rect: Rect {
                    x: bar.x,
                    y,
                    w: bar.w,
                    h,
                },
            });
        }
        y += h;
    }
}

fn lay_horizontal(bar: Rect, row: &[&Item], row_sum: f64, out: &mut Vec<Placed>) {
    let mut x = bar.x;
    let n = row.len() as i64;
    for (k, it) in row.iter().enumerate() {
        let frac = it.val / row_sum;
        let w = (frac * bar.w as f64).floor() as i64;
        let is_last = k as i64 == n - 1;
        let w = if is_last { bar.x + bar.w - x } else { w };
        if w > 0 {
            out.push(Placed {
                idx: it.idx,
                rect: Rect {
                    x,
                    y: bar.y,
                    w,
                    h: bar.h,
                },
            });
        }
        x += w;
    }
}

fn worst(row: &[&Item], w: f64) -> f64 {
    let s: f64 = row.iter().map(|it| it.val).sum();
    if s <= 0.0 || w <= 0.0 {
        return f64::INFINITY;
    }
    row.iter()
        .map(|it| {
            let r = it.val;
            let a = (w * w * r) / (s * s);
            let b = (s * s) / (w * w * r);
            a.max(b)
        })
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_area_proportionally() {
        let rect = Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 50,
        };
        let items = vec![(0, 50u64), (1, 30), (2, 20)];
        let placed = squarify(rect, &items);
        let total_area: f64 = placed.iter().map(|p| p.rect.area()).sum();
        // roughly covers the whole rect
        assert!(
            (total_area - rect.area()).abs() < rect.area() * 0.1,
            "covered {total_area}"
        );
        // largest weight -> largest rect
        let areas: Vec<_> = placed.iter().map(|p| p.rect.area()).collect();
        let max_area = areas.iter().cloned().fold(0.0_f64, f64::max);
        assert_eq!(placed[0].rect.area(), max_area);
    }

    #[test]
    fn all_inside_bounds() {
        let rect = Rect {
            x: 5,
            y: 5,
            w: 40,
            h: 30,
        };
        let items = vec![(0, 10u64), (1, 20), (2, 5), (3, 15)];
        for p in squarify(rect, &items) {
            assert!(p.rect.x >= rect.x && p.rect.y >= rect.y);
            assert!(p.rect.x + p.rect.w <= rect.x + rect.w);
            assert!(p.rect.y + p.rect.h <= rect.y + rect.h);
            assert!(p.rect.w > 0 && p.rect.h > 0);
        }
    }
}
