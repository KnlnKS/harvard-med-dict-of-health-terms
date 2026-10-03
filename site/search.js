const input = document.querySelector('#search');
const clear = document.querySelector('#clear-search');
const result = document.querySelector('#result-count');
const empty = document.querySelector('#empty-state');
const groups = [...document.querySelectorAll('.letter-section')].map(section => {
  const link = document.querySelector(`.letter-index a[href="#${section.id}"]`);
  return {
    section,
    link,
    count: section.querySelector('.section-count'),
    indexCount: link.querySelector('.letter-count'),
    entries: [...section.querySelectorAll('.entry')].map(element => ({
      element,
      text: element.textContent.toLowerCase()
    }))
  };
});
const total = groups.reduce((sum, group) => sum + group.entries.length, 0);
const format = new Intl.NumberFormat('en-US');
let frame;
let letterFrame;
let activeLink;

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
    group.indexCount.textContent = count;
    if (count) {
      group.link.href = `#${group.section.id}`;
      group.link.removeAttribute('aria-disabled');
      group.link.setAttribute('aria-label', `${group.section.id}, ${count} terms`);
    } else {
      group.link.removeAttribute('href');
      group.link.setAttribute('aria-disabled', 'true');
      group.link.setAttribute('aria-label', `${group.section.id}, no matching terms`);
    }
  }
  clear.hidden = !input.value;
  empty.hidden = !!matches;
  result.textContent = words.length
    ? `${format.format(matches)} of ${format.format(total)} terms`
    : `All ${format.format(total)} terms`;
  currentLetter();
}

function reset() {
  cancelAnimationFrame(frame);
  input.value = '';
  filter();
  input.focus({ preventScroll: true });
}

function currentLetter() {
  const top = parseFloat(getComputedStyle(groups[0].section).scrollMarginTop) + 1;
  const atBottom = scrollY > 0 && Math.ceil(scrollY + innerHeight) >= document.documentElement.scrollHeight;
  let current;
  for (const { section, link } of groups) {
    if (section.hidden) continue;
    if (current && !atBottom && section.getBoundingClientRect().top > top) break;
    current = link;
  }
  if (current === activeLink) return;
  activeLink?.removeAttribute('aria-current');
  current?.setAttribute('aria-current', 'location');
  activeLink = current;
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
window.addEventListener('resize', currentLetter);
window.addEventListener('scroll', () => {
  cancelAnimationFrame(letterFrame);
  letterFrame = requestAnimationFrame(currentLetter);
}, { passive: true });
currentLetter();
document.querySelector('#search-tools').hidden = false;
