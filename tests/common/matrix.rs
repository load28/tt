use std::collections::BTreeSet;
use std::path::PathBuf;

pub type Named = (String, PathBuf);

pub struct Sample {
    pub sampled: Vec<Named>,
    pub unsampled: Vec<Named>,
    pub summary: Option<String>,
}

pub fn sample(matrix: Vec<Named>, count: usize, seed: u64) -> Sample {
    let total = matrix.len();
    if total == 0 {
        return Sample {
            sampled: matrix,
            unsampled: Vec::new(),
            summary: None,
        };
    }
    let requested = std::env::var("TT_MATRIX_CASES").unwrap_or_default();
    if requested == "all" {
        return Sample {
            sampled: matrix,
            unsampled: Vec::new(),
            summary: Some(format!("all {total} matrix cases")),
        };
    }
    let count = if requested.is_empty() {
        count
    } else {
        requested
            .parse()
            .unwrap_or_else(|_| panic!("TT_MATRIX_CASES takes a count or `all`, not `{requested}`"))
    }
    .min(total);
    let seed = std::env::var("TT_MATRIX_SEED")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("TT_MATRIX_SEED takes a number, not `{value}`"))
        })
        .unwrap_or(seed);
    let mut state = seed;
    let mut indices: Vec<usize> = (0..total).collect();
    for i in 0..count {
        let j = i + (splitmix(&mut state) % (total - i) as u64) as usize;
        indices.swap(i, j);
    }
    let picked: BTreeSet<usize> = indices[..count].iter().copied().collect();
    let mut sampled = Vec::new();
    let mut unsampled = Vec::new();
    for (index, case) in matrix.into_iter().enumerate() {
        if picked.contains(&index) {
            sampled.push(case);
        } else {
            unsampled.push(case);
        }
    }
    Sample {
        sampled,
        unsampled,
        summary: Some(format!(
            "{count} of {total} matrix cases, seed {seed} (TT_MATRIX_CASES=all for every one)"
        )),
    }
}

pub fn stratified(groups: Vec<Vec<Named>>, seed: u64) -> Sample {
    let total: usize = groups.iter().map(Vec::len).sum();
    if total == 0 {
        return Sample {
            sampled: Vec::new(),
            unsampled: Vec::new(),
            summary: None,
        };
    }
    if std::env::var("TT_MATRIX_CASES").is_ok_and(|requested| requested == "all") {
        return Sample {
            sampled: groups.into_iter().flatten().collect(),
            unsampled: Vec::new(),
            summary: Some(format!("all {total} cases")),
        };
    }
    let seed = std::env::var("TT_MATRIX_SEED")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("TT_MATRIX_SEED takes a number, not `{value}`"))
        })
        .unwrap_or(seed);
    let mut state = seed;
    let mut sampled = Vec::new();
    let mut unsampled = Vec::new();
    let count = groups.iter().filter(|group| !group.is_empty()).count();
    for group in groups {
        if group.is_empty() {
            continue;
        }
        let pick = (splitmix(&mut state) % group.len() as u64) as usize;
        for (index, case) in group.into_iter().enumerate() {
            if index == pick {
                sampled.push(case);
            } else {
                unsampled.push(case);
            }
        }
    }
    Sample {
        sampled,
        unsampled,
        summary: Some(format!(
            "one case of each of {count} groups, {total} cases in all, seed {seed} (TT_MATRIX_CASES=all for every one)"
        )),
    }
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
