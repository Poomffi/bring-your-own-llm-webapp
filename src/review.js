const reviewEditor = (() => {
  const panel = document.getElementById('review-panel');
  const rows = document.getElementById('review-rows');
  const sources = document.getElementById('review-sources');
  const check = document.getElementById('review-check');
  const download = document.getElementById('review-download');
  const feedback = document.getElementById('review-feedback');
  const counts = document.getElementById('review-counts');
  const add = document.getElementById('review-add');
  let draft = null;
  let revision = 0;
  let busy = false;

  const messages = {
    unknown_category: 'Choose a category from the template.',
    unknown_review: 'Choose a review ID from the source list.',
    missing_excerpt: 'Use an exact excerpt from that review.',
    duplicate_classification: 'Keep one row per category and review ID.',
    overlapping_review: 'A classified review cannot also be marked unclassified.',
    missing_review: 'Mark every review as classified or unclassified.',
    invalid_shape: 'Complete the required fields and use a nonempty excerpt.',
  };

  function writeAssessment(assessment) {
    const error = assessment?.error;
    for (const [index, row] of [...rows.children].entries()) {
      row.dataset.invalid = String(error?.row_index === index);
    }
    if (error) {
      const where = error.row_index == null ? '' : 'Row ' + (error.row_index + 1) + '. ';
      feedback.textContent = where + (messages[error.code] ?? 'Review validation failed.');
    } else {
      feedback.textContent = assessment ? 'Review rows checked. CSV is ready.' : 'Check your edits before exporting.';
    }
    const outcomes = assessment?.issue_counts;
    counts.textContent = outcomes ? draft.categories.map(({ id, label }) => label + ' · ' + outcomes[id]).join('  |  ') : '';
    download.disabled = !outcomes || busy;
  }

  function changed() {
    revision++;
    draft.assessment = null;
    writeAssessment(null);
  }

  function addChoice(selected, choices, label, update) {
    const select = document.createElement('select');
    select.setAttribute('aria-label', label);
    for (const [value, name] of choices) select.add(new Option(name, value));
    if (!choices.some(([value]) => value === selected)) {
      select.add(new Option(selected ? 'Unknown: ' + selected : 'Choose…', selected));
    }
    select.value = selected;
    select.addEventListener('change', () => { update(select.value); changed(); });
    return select;
  }

  function renderRows() {
    rows.replaceChildren();
    for (const [index, row] of draft.result.rows.entries()) {
      const group = document.createElement('div');
      group.className = 'review-row';
      const category = addChoice(row.category, draft.categories.map(item => [item.id, item.label]), 'Category for row ' + (index + 1), value => { row.category = value; });
      const review = addChoice(row.review_id, draft.input.reviews.map(item => [item.id, item.id]), 'Source for row ' + (index + 1), value => { row.review_id = value; });
      const excerpt = document.createElement('input');
      excerpt.setAttribute('aria-label', 'Exact excerpt for row ' + (index + 1));
      excerpt.placeholder = 'Exact excerpt from the review';
      excerpt.value = row.excerpt;
      excerpt.addEventListener('input', () => { row.excerpt = excerpt.value; changed(); });
      const remove = document.createElement('button');
      remove.type = 'button';
      remove.className = 'secondary';
      remove.textContent = 'Remove';
      remove.setAttribute('aria-label', 'Remove review row ' + (index + 1));
      remove.addEventListener('click', () => {
        draft.result.rows.splice(index, 1);
        renderRows();
        changed();
      });
      group.append(category, review, excerpt, remove);
      rows.append(group);
    }
  }

  function renderSources() {
    sources.replaceChildren();
    const records = new Map(draft.input.reviews.map(item => [item.id, item.text]));
    for (const id of draft.result.unclassified_review_ids) {
      if (!records.has(id)) records.set(id, 'Unknown review ID');
    }
    for (const [id, text] of records) {
      const row = document.createElement('div');
      row.className = 'review-source';
      const quote = document.createElement('span');
      quote.textContent = id + ' · ' + text;
      const label = document.createElement('label');
      const box = document.createElement('input');
      box.type = 'checkbox';
      box.checked = draft.result.unclassified_review_ids.includes(id);
      box.addEventListener('change', () => {
        draft.result.unclassified_review_ids = draft.result.unclassified_review_ids.filter(value => value !== id);
        if (box.checked) draft.result.unclassified_review_ids.push(id);
        changed();
      });
      label.append(box, ' Unclassified');
      row.append(quote, label);
      sources.append(row);
    }
  }

  async function run(action) {
    if (!draft || busy) return;
    const current = revision;
    busy = true;
    check.disabled = download.disabled = true;
    try {
      const response = await fetch('/review/' + action, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ input: draft.input, result: draft.result }),
        signal: AbortSignal.timeout(10000),
      });
      if (current !== revision) return;
      if (action === 'export' && response.ok) {
        const file = await response.blob();
        if (current !== revision) return;
        const url = URL.createObjectURL(file);
        const link = document.createElement('a');
        link.href = url;
        link.download = 'customer-review-triage.csv';
        link.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      } else if (response.ok || response.status === 422 && response.headers.get('Content-Type')?.includes('application/json')) {
        const assessment = await response.json();
        if (current === revision) draft.assessment = assessment;
      } else {
        throw new Error('Could not validate the review rows.');
      }
    } catch {
      if (current === revision) {
        draft.assessment = null;
        feedback.textContent = 'Could not check these rows. Try again.';
        counts.textContent = '';
      }
      return;
    } finally {
      busy = false;
      check.disabled = false;
      download.disabled = !draft?.assessment?.issue_counts;
    }
    if (current === revision) writeAssessment(draft.assessment);
  }

  add.addEventListener('click', () => {
    draft.result.rows.push({ category: '', review_id: '', excerpt: '' });
    renderRows();
    changed();
    rows.lastElementChild.firstElementChild.focus();
  });
  check.addEventListener('click', () => run('check'));
  download.addEventListener('click', () => run('export'));

  return {
    show(value, origin) {
      revision++;
      draft = value;
      document.getElementById('review-origin').textContent = origin;
      renderSources();
      renderRows();
      writeAssessment(draft.assessment);
      panel.hidden = false;
    },
    hide() {
      revision++;
      draft = null;
      panel.hidden = true;
    },
  };
})();
