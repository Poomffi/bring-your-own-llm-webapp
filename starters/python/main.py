"""Validate a customer-review fixture or a candidate result saved as JSON."""

import argparse
import json
from pathlib import Path

DEFAULT_TEMPLATE = Path("templates/customer-review-triage")


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def has_fields(value, fields):
    return isinstance(value, dict) and value.keys() == set(fields)


def nonblank(value):
    return isinstance(value, str) and bool(value.strip())


def validate(task, batch, result):
    counts = {}
    for category in task["categories"]:
        key = category["id"]
        if not nonblank(key) or key in counts:
            raise ValueError("invalid_categories")
        counts[key] = 0
    if not counts:
        raise ValueError("invalid_categories")

    if not has_fields(batch, ["reviews"]) or not isinstance(batch["reviews"], list) or not batch["reviews"]:
        raise ValueError("invalid_shape")
    reviews = {}
    for review in batch["reviews"]:
        if not has_fields(review, ["id", "text"]) or not all(map(nonblank, review.values())):
            raise ValueError("invalid_shape")
        if review["id"] in reviews:
            raise ValueError("duplicate_input_id")
        reviews[review["id"]] = review["text"]

    if not has_fields(result, ["rows", "unclassified_review_ids"]) or not all(
        isinstance(value, list) for value in result.values()
    ):
        raise ValueError("invalid_shape")

    pairs = set()
    classified = set()
    for row in result["rows"]:
        if not has_fields(row, ["category", "review_id", "excerpt"]) or not all(map(nonblank, row.values())):
            raise ValueError("invalid_shape")
        category, review_id, excerpt = row["category"], row["review_id"], row["excerpt"]
        if category not in counts:
            raise ValueError("unknown_category")
        if review_id not in reviews:
            raise ValueError("unknown_review")
        if excerpt not in reviews[review_id]:
            raise ValueError("missing_excerpt")
        pair = (category, review_id)
        if pair in pairs:
            raise ValueError("duplicate_classification")
        pairs.add(pair)
        classified.add(review_id)
        counts[category] += 1

    unclassified = set()
    for review_id in result["unclassified_review_ids"]:
        if not nonblank(review_id):
            raise ValueError("invalid_shape")
        if review_id not in reviews:
            raise ValueError("unknown_review")
        if review_id in unclassified:
            raise ValueError("invalid_shape")
        if review_id in classified:
            raise ValueError("overlapping_review")
        unclassified.add(review_id)
    if len(classified) + len(unclassified) != len(reviews):
        raise ValueError("missing_review")
    return counts


def check_rejections(template, task, batch):
    for case in read_json(template / "validation-cases.json"):
        try:
            validate(task, case.get("input", batch), case["result"])
        except ValueError as error:
            if str(error) == case["expected_error"]:
                continue
        raise ValueError("Validation case failed: " + case["id"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("template", nargs="?", type=Path, default=DEFAULT_TEMPLATE)
    parser.add_argument("candidate", nargs="?", type=Path)
    args = parser.parse_args()
    try:
        task = read_json(args.template / "task.json")
        batch = read_json(args.template / "sample-input.json")
        if args.candidate:
            result = read_json(args.candidate)
            counts = validate(task, batch, result)
        else:
            expected = read_json(args.template / "expected-output.json")
            result = expected["result"]
            counts = validate(task, batch, result)
            if counts != expected["issue_counts"]:
                raise ValueError("Fixture counts do not match expected-output.json")
            check_rejections(args.template, task, batch)
        print(json.dumps({"result": result, "issue_counts": counts}, indent=2, ensure_ascii=False))
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
