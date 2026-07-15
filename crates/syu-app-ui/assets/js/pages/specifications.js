export function renderSpecifications(specifications, stateOrRoot = document.querySelector('[data-specifications-detail]')) {
  const entries = specifications?.specifications || [];
  const state = stateOrRoot?.api ? stateOrRoot : null;
  const root = state ? document.querySelector('[data-specifications-detail]') : stateOrRoot;
  const rail = document.querySelector('[data-specifications-rail]');
  if (rail) {
    rail.replaceChildren();
    entries.forEach(item => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'rail-item';
      button.textContent = item.id;
      button.addEventListener('click', () => {
        if (state) {
          state.selectedSpecification = item.id;
          state.render();
        }
      });
      rail.append(button);
    });
  }
  if (!root) return entries;
  root.replaceChildren();
  const selected = entries.find(item => item.id === state?.selectedSpecification)
    || entries.find(item => item.status === 'implemented' && item.criteria?.length)
    || entries[0];
  if (!selected) {
    root.textContent = 'No specifications.';
    return entries;
  }
  if (state) state.selectedSpecification = selected.id;
  const heading = document.createElement('h2');
  heading.textContent = `${selected.id}: ${selected.title}`;
  root.append(heading);
  const status = document.createElement('p');
  status.textContent = selected.status || selected.kind;
  root.append(status);
  if (selected.summary || selected.description) {
    const summary = document.createElement('p');
    summary.textContent = selected.summary || selected.description;
    root.append(summary);
  }
  const criteria = selected.criteria || [];
  if (!criteria.length) {
    const empty = document.createElement('p');
    empty.textContent = 'No criteria on this item.';
    root.append(empty);
    return entries;
  }
  const title = document.createElement('h3');
  title.textContent = 'Implemented criteria';
  root.append(title);
  criteria.forEach(criterion => {
    const row = document.createElement('div');
    row.className = 'specification-criterion';
    const statement = document.createElement('p');
    statement.textContent = `${criterion.anchor}: ${criterion.statement}`;
    row.append(statement);
    if (state && selected.status === 'implemented') {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'btn primary compact';
      button.textContent = 'Create Modify Work';
      button.setAttribute('aria-label', `Create Modify Work for ${criterion.anchor}`);
      button.onclick = () => state.runAction(
        () => state.api.createModifyWork(state.projection, criterion.anchor),
        () => {
          state.selectedSlice = null;
          state.go('work');
        },
      );
      row.append(button);
    }
    root.append(row);
  });
  return entries;
}
