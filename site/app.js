import { renderBenchmarks } from './benchmarks.js';

const $ = selector => document.querySelector(selector);
const labels = {
  'built-in': '● Built-in', ecosystem: '◇ Ecosystem', partial: '◐ Partial',
  'different model': '≈ Different model', unsupported: '○ Unsupported', unknown: '? Unknown', 'not applicable': 'N/A',
};

function element(tag, text, className) {
  const item = document.createElement(tag);
  if (text !== undefined && text !== null) item.textContent = text;
  if (className) item.className = className;
  return item;
}

const aiLabels = {
  'no-ai': 'No AI', 'ai-assisted': 'AI-assisted', 'ai-generated': 'AI-generated', unknown: 'Unknown',
};

async function loadJson(path) {
  const response = await fetch(path, { cache: 'no-cache' });
  if (!response.ok) throw new Error(`Loading ${path}: ${response.status}`);
  return response.json();
}

function showClaimDetails(title, paragraphs, evidenceUrl) {
  let dialog = $('#claim-dialog');
  if (!dialog) {
    dialog = element('dialog', undefined, 'claim-dialog');
    dialog.id = 'claim-dialog';
    dialog.setAttribute('aria-labelledby', 'claim-dialog-title');
    document.body.append(dialog);
  }
  const heading = element('h3', title);
  heading.id = 'claim-dialog-title';
  const close = element('button', 'Close', 'claim-dialog-close');
  close.type = 'button';
  close.autofocus = true;
  close.addEventListener('click', () => dialog.close());
  dialog.replaceChildren(heading, close, ...paragraphs.map(text => element('p', text)));
  if (evidenceUrl) {
    const evidence = element('a', 'Source');
    evidence.href = evidenceUrl;
    evidence.rel = 'noreferrer';
    dialog.append(evidence);
  } else dialog.append(element('p', 'No public source linked.'));
  dialog.showModal();
}

function claimButton(text, title, paragraphs, evidenceUrl) {
  const button = element('button', text, 'claim-button');
  button.type = 'button';
  button.title = `${title}. Open details`;
  button.setAttribute('aria-label', title);
  button.setAttribute('aria-haspopup', 'dialog');
  button.addEventListener('click', () => showClaimDetails(title, paragraphs, evidenceUrl));
  return button;
}

function renderMatrix(libraries, data) {
  $('#legend').replaceChildren(...Object.values(labels).map(label => element('span', label)));
  const table = $('#features');
  const head = element('thead');
  const headings = element('tr');
  headings.append(element('th', 'Feature'));
  for (const library of libraries) {
    const heading = element('th');
    const link = element('a', library.name);
    link.href = library.repository;
    heading.append(link);
    headings.append(heading);
  }
  head.append(headings);
  const body = element('tbody');
  body.append(metadataRows(libraries));
  for (const category of data.categories) {
    const categoryRow = element('tr', undefined, 'release-date feature-category');
    const categoryHeading = element('th', category);
    categoryHeading.scope = 'rowgroup';
    categoryHeading.colSpan = libraries.length + 1;
    categoryRow.append(categoryHeading);
    body.append(categoryRow);
    const claims = data.claims.filter(claim => claim.category === category);
    for (const claimName of new Set(claims.map(claim => claim.name))) {
      const row = element('tr');
      const heading = element('th', claimName);
      heading.scope = 'row';
      row.append(heading);
      for (const library of libraries) {
        const claim = claims.find(item => item.name === claimName && item.library === library.id);
        const cell = element('td', undefined, claim.status);
        const symbol = labels[claim.status].split(' ')[0];
        cell.append(claimButton(symbol, `${library.name}, ${claimName}: ${labels[claim.status]}`, [
          claim.notes,
          `${claim.reviewedVersion} · reviewed ${claim.reviewedDate ?? 'unknown'}`,
        ], claim.evidence));
        row.append(cell);
      }
      body.append(row);
    }
  }
  table.replaceChildren(element('caption', 'Select a cell for details.'), head, body);
}

function metadataRows(libraries) {
  const fields = [
    ['License', library => library.license ?? 'Unknown'],
    ['Creation year', library => library.creationYear ?? 'Unknown'],
    ['Latest release', library => library.latestRelease ?? 'Unknown'],
    ['AI involvement', library => aiLabels[library.provenance.status]],
  ];
  const body = document.createDocumentFragment();
  for (const [name, valueFor] of fields) {
    const row = element('tr');
    const heading = element('th', name);
    heading.scope = 'row';
    row.append(heading);
    for (const library of libraries) {
      const cell = element('td');
      const value = valueFor(library);
      const involvement = name === 'AI involvement' ? library.provenance : null;
      const firstCommit = name === 'Creation year' ? library.firstCommit : null;
      const evidenceUrl = involvement
        ? involvement.evidence
        : library.metadataFieldEvidence ? library.metadataFieldEvidence[name] : library.metadataEvidence;
      const notes = firstCommit
        ? `First repository commit: ${firstCommit.date}. This dates the available Git history, not necessarily the project's original start.`
        : involvement
          ? involvement.scope ?? 'No explicit AI authorship disclosure found. This does not mean no AI was used.'
          : library.metadataNotes ?? 'Scope or supporting declaration is unknown.';
      const paragraphs = [value, notes];
      if (involvement?.quote) paragraphs.push(`${involvement.basis === 'site-maintainer' ? 'Site maintainer statement' : 'Disclosure'}: ${involvement.quote}`);
      if (involvement?.status === 'ai-assisted') paragraphs.push('This establishes AI help on some changes, not the share of the whole project.');
      const version = firstCommit ? firstCommit.sha.slice(0, 12) : involvement ? involvement.reviewedCommit.slice(0, 12) : library.reviewedVersion;
      const date = firstCommit ? firstCommit.reviewedDate : involvement ? involvement.reviewedDate : library.reviewedDate;
      paragraphs.push(`${version} · reviewed ${date ?? 'unknown'}`);
      const button = claimButton(value, `${library.name}, ${name}: ${value}`, paragraphs, evidenceUrl);
      button.classList.add('metadata-value');
      cell.append(button);
      row.append(cell);
    }
    body.append(row);
  }
  return body;
}

try {
  const [libraryData, featureData, history] = await Promise.all([
    loadJson('data/libraries.json'), loadJson('data/features.json'), loadJson('data/benchmark-runs.json'),
  ]);
  renderBenchmarks(history, libraryData.libraries);
  renderMatrix(libraryData.libraries, featureData);
} catch (error) {
  $('#empty').hidden = false;
  $('#empty').textContent = `Could not render site data: ${error.message}`;
}
