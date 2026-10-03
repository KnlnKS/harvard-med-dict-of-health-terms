const input = document.querySelector('#search');
const clear = document.querySelector('#clear-search');
const result = document.querySelector('#result-count');
const empty = document.querySelector('#empty-state');
const groups = [...document.querySelectorAll('.letter-section')].map(section => ({
  section,
  link: document.querySelector(`.letter-index a[href="#${section.id}"]`),
  count: section.querySelector('.section-count'),
  entries: [...section.querySelectorAll('.entry')].map(element => ({
    element,
    text: element.textContent.toLowerCase()
  }))
}));
const total = groups.reduce((sum, group) => sum + group.entries.length, 0);
const format = new Intl.NumberFormat('en-US');
let frame;

function filter() {
  const words = input.value.toLowerCase().trim().split(/\s+/).filter(Boolean);
  let matches = 0;
  for (const group of groups) {
    let count = 0;
    for (const entry of group.entries) {
      const hidden = !words.every(word => entry.text.includes(word));
      if (entry.element.hidden !== hidden) entry.element.hidden = hidden;
      if (!hidden) count++;
    }
    matches += count;
    group.section.hidden = !count;
    group.count.textContent = `${count} ${count === 1 ? 'term' : 'terms'}`;
    if (count) {
      group.link.href = `#${group.section.id}`;
      group.link.removeAttribute('aria-disabled');
      group.link.setAttribute('aria-label', `${group.section.id}, ${count} terms`);
    } else {
      group.link.removeAttribute('href');
      group.link.setAttribute('aria-disabled', 'true');
      group.link.setAttribute('aria-label', `${group.section.id}, no matching terms`);
      group.link.removeAttribute('aria-current');
    }
  }
  clear.hidden = !input.value;
  empty.hidden = !!matches;
  result.textContent = words.length
    ? `${format.format(matches)} of ${format.format(total)} terms`
    : `All ${format.format(total)} terms`;
}

function reset() {
  cancelAnimationFrame(frame);
  input.value = '';
  filter();
  input.focus({ preventScroll: true });
}

function currentLetter() {
  for (const { section, link } of groups) {
    if (location.hash === `#${section.id}` && !section.hidden) link.setAttribute('aria-current', 'location');
    else link.removeAttribute('aria-current');
  }
}

input.addEventListener('input', () => {
  cancelAnimationFrame(frame);
  frame = requestAnimationFrame(filter);
});
input.addEventListener('keydown', event => {
  if (event.key === 'Escape') reset();
});
clear.addEventListener('click', reset);
document.querySelector('#reset-search').addEventListener('click', reset);
window.addEventListener('hashchange', currentLetter);
currentLetter();
document.querySelector('#search-tools').hidden = false;
