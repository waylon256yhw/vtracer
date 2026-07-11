use serde::{Deserialize, Serialize};

use crate::{StudioConfig, StudioError, StudioResult, FILTER_SPECKLE_MAX};

/// The three experiment axes. Values are validated, sorted and deduplicated
/// (`normalized`) before any total/order/id derivation, so candidate ids are
/// deterministic functions of the parameter tuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixAxes {
    pub gradient_step: Vec<i32>,
    pub filter_speckle: Vec<u32>,
    pub color_precision: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunMode {
    /// Cube corners + center only (≤9 candidates), then finish. The user
    /// prunes axis values before paying for the full product.
    Sparse,
    /// Full Cartesian product, sparse candidates scheduled first (cache hits
    /// from a previous sparse run complete instantly).
    Full,
}

/// One cell of the matrix: the axis values plus its deterministic id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateParams {
    pub id: String,
    pub gradient_step: i32,
    pub filter_speckle: u32,
    pub color_precision: i32,
}

impl CandidateParams {
    fn new(g: i32, f: u32, p: i32) -> Self {
        Self {
            id: format!("g{g}_f{f}_p{p}"),
            gradient_step: g,
            filter_speckle: f,
            color_precision: p,
        }
    }

    /// The full config for this candidate: base config with axis values applied.
    pub fn apply_to(&self, base: &StudioConfig) -> StudioConfig {
        StudioConfig {
            gradient_step: self.gradient_step,
            filter_speckle: self.filter_speckle,
            color_precision: self.color_precision,
            ..base.clone()
        }
    }
}

impl MatrixAxes {
    /// Validate ranges, sort ascending, deduplicate. Errors are zh-CN.
    pub fn normalized(&self) -> StudioResult<MatrixAxes> {
        fn norm<T: Ord + Copy>(mut v: Vec<T>, name: &str) -> StudioResult<Vec<T>> {
            if v.is_empty() {
                return Err(StudioError::InvalidParam(format!("{name} 至少要选一个值")));
            }
            v.sort_unstable();
            v.dedup();
            Ok(v)
        }
        let g = norm(self.gradient_step.clone(), "渐变步长 gradient_step")?;
        let f = norm(self.filter_speckle.clone(), "斑点过滤 filter_speckle")?;
        let p = norm(self.color_precision.clone(), "颜色精度 color_precision")?;
        if let Some(bad) = g.iter().find(|v| !(0..=255).contains(*v)) {
            return Err(StudioError::InvalidParam(format!(
                "渐变步长 gradient_step 必须在 [0,255] 内，当前含 {bad}"
            )));
        }
        if let Some(bad) = f.iter().find(|v| **v > FILTER_SPECKLE_MAX) {
            return Err(StudioError::InvalidParam(format!(
                "斑点过滤 filter_speckle 必须在 [0,{FILTER_SPECKLE_MAX}] 内，当前含 {bad}"
            )));
        }
        if let Some(bad) = p.iter().find(|v| !(1..=8).contains(*v)) {
            return Err(StudioError::InvalidParam(format!(
                "颜色精度 color_precision 必须在 [1,8] 内，当前含 {bad}"
            )));
        }
        Ok(MatrixAxes {
            gradient_step: g,
            filter_speckle: f,
            color_precision: p,
        })
    }

    /// Full Cartesian product in lexicographic axis order. Call on a
    /// `normalized()` result.
    pub fn expand(&self) -> Vec<CandidateParams> {
        let mut out =
            Vec::with_capacity(self.gradient_step.len() * self.filter_speckle.len() * self.color_precision.len());
        for &g in &self.gradient_step {
            for &f in &self.filter_speckle {
                for &p in &self.color_precision {
                    out.push(CandidateParams::new(g, f, p));
                }
            }
        }
        out
    }

    /// The 8 cube corners (min/max per axis) + center, deduplicated while
    /// preserving first-occurrence order. Axes with a single value collapse
    /// corners naturally (all axes singleton → 1 candidate).
    pub fn sparse_set(&self) -> Vec<CandidateParams> {
        fn ends<T: Copy>(v: &[T]) -> Vec<T> {
            if v.len() == 1 {
                vec![v[0]]
            } else {
                vec![v[0], v[v.len() - 1]]
            }
        }
        let mut picked = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut push = |c: CandidateParams, picked: &mut Vec<CandidateParams>| {
            if seen.insert(c.id.clone()) {
                picked.push(c);
            }
        };
        for &g in &ends(&self.gradient_step) {
            for &f in &ends(&self.filter_speckle) {
                for &p in &ends(&self.color_precision) {
                    push(CandidateParams::new(g, f, p), &mut picked);
                }
            }
        }
        let center = CandidateParams::new(
            self.gradient_step[self.gradient_step.len() / 2],
            self.filter_speckle[self.filter_speckle.len() / 2],
            self.color_precision[self.color_precision.len() / 2],
        );
        push(center, &mut picked);
        picked
    }

    /// Scheduling order for a run: Sparse → the sparse set only; Full → sparse
    /// set first, then every remaining combination in lexicographic order.
    pub fn ordered(&self, mode: RunMode) -> Vec<CandidateParams> {
        let sparse = self.sparse_set();
        match mode {
            RunMode::Sparse => sparse,
            RunMode::Full => {
                let mut seen: std::collections::HashSet<String> =
                    sparse.iter().map(|c| c.id.clone()).collect();
                let mut out = sparse;
                for c in self.expand() {
                    if seen.insert(c.id.clone()) {
                        out.push(c);
                    }
                }
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axes() -> MatrixAxes {
        MatrixAxes {
            gradient_step: vec![32, 8, 16],
            filter_speckle: vec![8, 2, 4],
            color_precision: vec![8, 4, 6],
        }
    }

    #[test]
    fn normalized_sorts_and_dedups() {
        let a = MatrixAxes {
            gradient_step: vec![16, 8, 16],
            filter_speckle: vec![4],
            color_precision: vec![6, 6],
        }
        .normalized()
        .unwrap();
        assert_eq!(a.gradient_step, vec![8, 16]);
        assert_eq!(a.color_precision, vec![6]);
    }

    #[test]
    fn normalized_rejects_bad_values() {
        assert!(MatrixAxes { gradient_step: vec![], ..axes() }.normalized().is_err());
        assert!(MatrixAxes { filter_speckle: vec![129], ..axes() }.normalized().is_err());
        assert!(MatrixAxes { color_precision: vec![0], ..axes() }.normalized().is_err());
        assert!(MatrixAxes { filter_speckle: vec![128], ..axes() }.normalized().is_ok());
    }

    #[test]
    fn expand_full_product_deterministic_ids() {
        let a = axes().normalized().unwrap();
        let all = a.expand();
        assert_eq!(all.len(), 27);
        assert_eq!(all[0].id, "g8_f2_p4");
        assert_eq!(all[26].id, "g32_f8_p8");
        let ids: std::collections::HashSet<_> = all.iter().map(|c| &c.id).collect();
        assert_eq!(ids.len(), 27);
    }

    #[test]
    fn sparse_is_corners_plus_center() {
        let a = axes().normalized().unwrap();
        let s = a.sparse_set();
        assert_eq!(s.len(), 9);
        assert!(s.iter().any(|c| c.id == "g8_f2_p4"));
        assert!(s.iter().any(|c| c.id == "g32_f8_p8"));
        assert_eq!(s[8].id, "g16_f4_p6", "中心点最后入列");
    }

    #[test]
    fn sparse_dedups_single_value_axes() {
        let a = MatrixAxes {
            gradient_step: vec![16],
            filter_speckle: vec![2, 8],
            color_precision: vec![4, 8],
        }
        .normalized()
        .unwrap();
        let s = a.sparse_set();
        // 4 corners + center(g16_f8_p8 == an existing corner? center idx = len/2 = 1 → f8,p8)
        let ids: Vec<_> = s.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids.len(), 4, "{ids:?}");

        let single = MatrixAxes {
            gradient_step: vec![16],
            filter_speckle: vec![4],
            color_precision: vec![6],
        }
        .normalized()
        .unwrap();
        assert_eq!(single.sparse_set().len(), 1);
    }

    #[test]
    fn ordered_full_puts_sparse_first_and_covers_all() {
        let a = axes().normalized().unwrap();
        let ordered = a.ordered(RunMode::Full);
        assert_eq!(ordered.len(), 27);
        let sparse_ids: Vec<_> = a.sparse_set().iter().map(|c| c.id.clone()).collect();
        for (i, id) in sparse_ids.iter().enumerate() {
            assert_eq!(&ordered[i].id, id, "稀疏集必须排在最前");
        }
        assert_eq!(a.ordered(RunMode::Sparse).len(), 9);
    }

    #[test]
    fn apply_to_overrides_only_axis_fields() {
        let base = StudioConfig { corner_threshold: 90, ..StudioConfig::default() };
        let c = CandidateParams::new(32, 8, 4).apply_to(&base);
        assert_eq!(c.gradient_step, 32);
        assert_eq!(c.filter_speckle, 8);
        assert_eq!(c.color_precision, 4);
        assert_eq!(c.corner_threshold, 90);
    }
}
