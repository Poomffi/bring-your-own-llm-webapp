use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Deserialize)]
pub struct Task {
    categories: Vec<Category>,
}

#[derive(Deserialize)]
struct Category {
    id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub reviews: Vec<Review>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub id: String,
    pub text: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewResult {
    pub rows: Vec<Row>,
    pub unclassified_review_ids: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub category: String,
    pub review_id: String,
    pub excerpt: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub code: &'static str,
    pub row_index: Option<usize>,
}

impl From<&'static str> for ValidationIssue {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            row_index: None,
        }
    }
}

pub fn validate(
    task: &Task,
    input: &Input,
    result: &ReviewResult,
) -> Result<BTreeMap<String, usize>, &'static str> {
    validate_detailed(task, input, result).map_err(|issue| issue.code)
}

pub fn validate_detailed(
    task: &Task,
    input: &Input,
    result: &ReviewResult,
) -> Result<BTreeMap<String, usize>, ValidationIssue> {
    let mut counts = BTreeMap::new();
    for category in &task.categories {
        if category.id.trim().is_empty() || counts.insert(category.id.clone(), 0).is_some() {
            return Err("invalid_categories".into());
        }
    }
    if counts.is_empty() {
        return Err("invalid_categories".into());
    }
    if input.reviews.is_empty() {
        return Err("invalid_shape".into());
    }

    let mut reviews = HashMap::new();
    for review in &input.reviews {
        if review.id.trim().is_empty() || review.text.trim().is_empty() {
            return Err("invalid_shape".into());
        }
        if reviews
            .insert(review.id.as_str(), review.text.as_str())
            .is_some()
        {
            return Err("duplicate_input_id".into());
        }
    }

    let mut classified = HashSet::new();
    let mut pairs = HashSet::new();
    for (index, row) in result.rows.iter().enumerate() {
        let issue = |code| ValidationIssue {
            code,
            row_index: Some(index),
        };
        if row.category.trim().is_empty()
            || row.review_id.trim().is_empty()
            || row.excerpt.trim().is_empty()
        {
            return Err(issue("invalid_shape"));
        }
        let count = counts
            .get_mut(&row.category)
            .ok_or_else(|| issue("unknown_category"))?;
        let source = reviews
            .get(row.review_id.as_str())
            .ok_or_else(|| issue("unknown_review"))?;
        if !source.contains(row.excerpt.as_str()) {
            return Err(issue("missing_excerpt"));
        }
        if !pairs.insert((row.category.as_str(), row.review_id.as_str())) {
            return Err(issue("duplicate_classification"));
        }
        classified.insert(row.review_id.as_str());
        *count += 1;
    }

    let mut unclassified = HashSet::new();
    for id in &result.unclassified_review_ids {
        if id.trim().is_empty() {
            return Err("invalid_shape".into());
        }
        if !reviews.contains_key(id.as_str()) {
            return Err("unknown_review".into());
        }
        if !unclassified.insert(id.as_str()) {
            return Err("invalid_shape".into());
        }
        if classified.contains(id.as_str()) {
            return Err("overlapping_review".into());
        }
    }
    if classified.len() + unclassified.len() != reviews.len() {
        return Err("missing_review".into());
    }
    Ok(counts)
}
