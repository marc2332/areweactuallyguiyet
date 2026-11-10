export function validDate(value) {
  return typeof value === 'string'
    && /^\d{4}-\d{2}-\d{2}$/.test(value)
    && Number.isFinite(Date.parse(value))
    && new Date(value).toISOString().slice(0, 10) === value;
}

function https(value) {
  try { return new URL(value).protocol === 'https:'; }
  catch { return false; }
}

export function validateLibraries(data) {
  if (!Array.isArray(data?.libraries) || !data.libraries.length) throw new Error('Missing libraries');
  const identifiers = new Set();
  const provenanceStatuses = ['unknown', 'no-ai', 'ai-assisted', 'ai-co-developed', 'ai-led'];
  for (const library of data.libraries) {
    if (!/^[a-z0-9-]+$/.test(library.id) || identifiers.has(library.id) || !library.name) throw new Error('Invalid or duplicate library');
    identifiers.add(library.id);
    for (const key of ['repository', 'homepage', 'metadataEvidence']) {
      if (!https(library[key])) throw new Error(`Invalid ${key} for ${library.id}`);
    }
    if (!(library.creationYear === null || Number.isInteger(library.creationYear) && library.creationYear >= 1900 && library.creationYear <= new Date().getUTCFullYear())) throw new Error(`Invalid creation year for ${library.id}`);
    if (library.firstCommit && (!/^[a-f0-9]{40}$/.test(library.firstCommit.sha) || !validDate(library.firstCommit.date) || !validDate(library.firstCommit.reviewedDate) || Number(library.firstCommit.date.slice(0, 4)) !== library.creationYear || library.metadataFieldEvidence?.['Creation year'] !== `${library.repository}/commit/${library.firstCommit.sha}`)) throw new Error(`Invalid first commit for ${library.id}`);
    for (const key of ['latestRelease', 'sourceUpdated', 'reviewedDate']) {
      if (library[key] !== null && !validDate(library[key])) throw new Error(`Invalid ${key} for ${library.id}`);
    }
    if (!library.reviewedVersion || !library.status || !library.classification || !library.backend) throw new Error(`Missing metadata for ${library.id}`);
    if (library.reviewedCommit !== undefined && (!/^[a-f0-9]{40}$/.test(library.reviewedCommit) || !validDate(library.reviewedDate))) throw new Error(`Invalid reviewed commit for ${library.id}`);
    if (library.metadataFieldEvidence !== undefined && (!library.metadataFieldEvidence || typeof library.metadataFieldEvidence !== 'object' || Object.values(library.metadataFieldEvidence).some(value => !https(value)))) throw new Error(`Invalid metadata field evidence for ${library.id}`);
    const provenance = library.provenance;
    if (!provenance || !provenanceStatuses.includes(provenance.status)) throw new Error(`Invalid AI involvement for ${library.id}`);
    if (!validDate(provenance.reviewedDate) || !/^[a-f0-9]{40}$/.test(provenance.reviewedCommit)) throw new Error(`Invalid AI involvement review for ${library.id}`);
    const maintainerStatement = provenance.basis === 'site-maintainer' && typeof provenance.quote === 'string' && provenance.quote.trim();
    if (provenance.status !== 'unknown' && (!provenance.scope || !https(provenance.evidence) && !maintainerStatement)) throw new Error(`AI involvement needs scope and evidence for ${library.id}`);
    if ([library.creationYear, library.latestRelease, library.sourceUpdated].some(value => value !== null) && !validDate(library.reviewedDate)) throw new Error(`Dated metadata needs a review date for ${library.id}`);
  }
  return data;
}

export function validateFeatures(features, libraries) {
  if (!Array.isArray(features?.claims) || !Array.isArray(features.categories) || !Array.isArray(features.statuses)) throw new Error('Missing feature definitions');
  const identifiers = new Set(libraries.map(library => library.id));
  const keys = new Set();
  const rows = new Set();
  for (const claim of features.claims) {
    const row = `${claim.category}/${claim.name}`;
    const key = `${claim.library}/${row}`;
    if (!identifiers.has(claim.library) || !features.categories.includes(claim.category) || !features.statuses.includes(claim.status) || !claim.name || keys.has(key)) throw new Error(`Invalid or duplicate claim ${key}`);
    keys.add(key);
    rows.add(row);
    if (!claim.notes || !claim.reviewedVersion || !https(claim.evidence)) throw new Error(`Incomplete claim ${key}`);
    if (claim.reviewedDate !== null && !validDate(claim.reviewedDate)) throw new Error(`Invalid review date for ${key}`);
    if (claim.status !== 'unknown' && !validDate(claim.reviewedDate)) throw new Error(`Reviewed claim requires a review date for ${key}`);
  }
  for (const row of rows) {
    for (const library of libraries) {
      if (!keys.has(`${library.id}/${row}`)) throw new Error(`Missing claim ${library.id}/${row}`);
    }
  }
  if (features.review !== undefined) validateSourceReview(features, libraries);
  return features;
}

function validateSourceReview(features, libraries) {
  const review = features.review;
  if (!review || !validDate(review.date) || !review.scope || !review.method || review.claimCount !== features.claims.length || !Array.isArray(review.repositories) || !review.repositories.length) throw new Error('Invalid source review');
  const snapshots = new Map();
  const sourceIds = new Set();
  for (const source of review.repositories) {
    if (!/^[a-z0-9-]+$/.test(source.id) || sourceIds.has(source.id) || !/^[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+$/.test(source.repository) || !/^[a-f0-9]{40}$/.test(source.sha) || !source.branch || !Number.isFinite(Date.parse(source.commitDate))) throw new Error('Invalid repository snapshot');
    sourceIds.add(source.id);
    snapshots.set(`https://github.com/${source.repository}`, source);
  }
  for (const library of libraries) {
    const snapshot = snapshots.get(library.repository);
    if (!snapshot || library.reviewedCommit !== snapshot.sha || library.reviewedDate !== (snapshot.reviewedDate ?? review.date)) throw new Error(`Missing or mismatched source snapshot for ${library.id}`);
  }
  const prefixes = [...snapshots].map(([repository, source]) => ({ prefix: `${repository}/blob/${source.sha}/`, date: source.reviewedDate ?? review.date }));
  for (const claim of features.claims) {
    if (claim.reviewedVersion === 'unreviewed' || !prefixes.some(({ prefix, date }) => claim.reviewedDate === date && claim.evidence.startsWith(prefix) && claim.evidence.length > prefix.length)) throw new Error(`Claim needs a dated immutable source citation: ${claim.library}/${claim.name}`);
  }
}
