import { readFileSync } from "node:fs";
import { join } from "node:path";

const defaultTemplate = "templates/customer-review-triage";

type Task = { categories: { id: string }[] };
type Expected = { result: unknown; issue_counts: Record<string, number> };
type ValidationCase = { id: string; expected_error: string; input?: unknown; result: unknown };

function readJson<T = unknown>(path: string): T {
  return JSON.parse(readFileSync(path, "utf8"));
}

function hasFields(value: unknown, fields: string[]): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && Object.keys(value).length === fields.length
    && fields.every(field => Object.hasOwn(value, field));
}

function nonblank(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function validate(task: Task, batch: unknown, result: unknown): Record<string, number> {
  const counts = new Map<string, number>();
  for (const { id } of task.categories) {
    if (!nonblank(id) || counts.has(id)) throw new Error("invalid_categories");
    counts.set(id, 0);
  }
  if (!counts.size) throw new Error("invalid_categories");

  if (!hasFields(batch, ["reviews"]) || !Array.isArray(batch.reviews) || !batch.reviews.length) {
    throw new Error("invalid_shape");
  }
  const reviews = new Map<string, string>();
  for (const review of batch.reviews) {
    if (!hasFields(review, ["id", "text"]) || !nonblank(review.id) || !nonblank(review.text)) {
      throw new Error("invalid_shape");
    }
    if (reviews.has(review.id)) throw new Error("duplicate_input_id");
    reviews.set(review.id, review.text);
  }

  if (!hasFields(result, ["rows", "unclassified_review_ids"])
      || !Array.isArray(result.rows) || !Array.isArray(result.unclassified_review_ids)) {
    throw new Error("invalid_shape");
  }
  const pairs = new Set<string>();
  const classified = new Set<string>();
  for (const row of result.rows) {
    if (!hasFields(row, ["category", "review_id", "excerpt"])
        || !nonblank(row.category) || !nonblank(row.review_id) || !nonblank(row.excerpt)) {
      throw new Error("invalid_shape");
    }
    const { category, review_id: reviewId, excerpt } = row;
    const count = counts.get(category);
    if (count === undefined) throw new Error("unknown_category");
    const source = reviews.get(reviewId);
    if (source === undefined) throw new Error("unknown_review");
    if (!source.includes(excerpt)) throw new Error("missing_excerpt");
    const pair = JSON.stringify([category, reviewId]);
    if (pairs.has(pair)) throw new Error("duplicate_classification");
    pairs.add(pair);
    classified.add(reviewId);
    counts.set(category, count + 1);
  }

  const unclassified = new Set<string>();
  for (const reviewId of result.unclassified_review_ids) {
    if (!nonblank(reviewId)) throw new Error("invalid_shape");
    if (!reviews.has(reviewId)) throw new Error("unknown_review");
    if (unclassified.has(reviewId)) throw new Error("invalid_shape");
    if (classified.has(reviewId)) throw new Error("overlapping_review");
    unclassified.add(reviewId);
  }
  if (classified.size + unclassified.size !== reviews.size) throw new Error("missing_review");
  return Object.fromEntries(counts);
}

function checkRejections(template: string, task: Task, batch: unknown) {
  const cases = readJson<ValidationCase[]>(join(template, "validation-cases.json"));
  for (const item of cases) {
    try {
      validate(task, item.input ?? batch, item.result);
    } catch (error) {
      if (error instanceof Error && error.message === item.expected_error) continue;
    }
    throw new Error("Validation case failed: " + item.id);
  }
}

function main() {
  const [template = defaultTemplate, candidate, ...extra] = process.argv.slice(2);
  if (extra.length) throw new Error("Usage: node starters/typescript/main.ts [template-directory] [candidate-result.json]");
  const task = readJson<Task>(join(template, "task.json"));
  const batch = readJson(join(template, "sample-input.json"));
  let result: unknown;
  let counts: Record<string, number>;
  if (candidate) {
    result = readJson(candidate);
    counts = validate(task, batch, result);
  } else {
    const expected = readJson<Expected>(join(template, "expected-output.json"));
    result = expected.result;
    counts = validate(task, batch, result);
    const reference = expected.issue_counts;
    if (Object.keys(counts).length !== Object.keys(reference).length
        || Object.keys(counts).some(key => !Object.hasOwn(reference, key) || counts[key] !== reference[key])) {
      throw new Error("Fixture counts do not match expected-output.json");
    }
    checkRejections(template, task, batch);
  }
  console.log(JSON.stringify({ result, issue_counts: counts }, null, 2));
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
