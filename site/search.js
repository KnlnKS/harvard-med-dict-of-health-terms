const input = document.querySelector('#search');
const clear = document.querySelector('#clear-search');
const result = document.querySelector('#result-count');
const empty = document.querySelector('#empty-state');
const searchTools = document.querySelector('#search-tools');
const mobile = matchMedia('(max-width: 700px)');
const groups = [...document.querySelectorAll('.letter-section')].map(section => {
  const link = document.querySelector(`.letter-index a[href="#${section.id}"]`);
  const heading = section.querySelector('.section-heading');
  return {
    section,
    link,
    heading,
    title: heading.querySelector('h2'),
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
const highlighted = [];

function clearHighlights() {
  for (const [element, text] of highlighted) element.textContent = text;
  highlighted.length = 0;
}

function highlight(element, expression) {
  const definition = element.querySelector('dd');
  const parts = [element.querySelector('dt a'), ...(definition.children.length ? definition.children : [definition])];
  for (const part of parts) {
    const text = part.textContent;
    expression.lastIndex = 0;
    let match = expression.exec(text);
    if (!match) continue;
    const fragment = document.createDocumentFragment();
    let start = 0;
    do {
      fragment.append(text.slice(start, match.index));
      const mark = document.createElement('mark');
      mark.textContent = match[0];
      fragment.append(mark);
      start = match.index + match[0].length;
      match = expression.exec(text);
    } while (match);
    fragment.append(text.slice(start));
    part.replaceChildren(fragment);
    highlighted.push([part, text]);
  }
}

function filter() {
  const words = [...new Set(input.value.toLowerCase().trim().split(/\s+/).filter(Boolean))];
  const expression = words.length
    ? new RegExp(words.map(word => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'), 'gi')
    : null;
  clearHighlights();
  let matches = 0;
  for (const group of groups) {
    let count = 0;
    for (const entry of group.entries) {
      const hidden = !words.every(word => entry.text.includes(word));
      if (entry.element.hidden !== hidden) entry.element.hidden = hidden;
      if (!hidden) {
        count++;
        if (expression) highlight(entry.element, expression);
      }
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
  const clipTop = mobile.matches ? searchTools.getBoundingClientRect().bottom : 0;
  const atBottom = scrollY > 0 && Math.ceil(scrollY + innerHeight) >= document.documentElement.scrollHeight;
  const updates = [];
  let current;
  for (const { section, link, heading, title } of groups) {
    if (section.hidden) continue;
    const visibility = clipTop && title.getBoundingClientRect().top < clipTop ? 'hidden' : '';
    if (heading.style.visibility !== visibility) updates.push([heading, visibility]);
    if (current && !atBottom && section.getBoundingClientRect().top > top) break;
    current = link;
  }
  for (const [heading, visibility] of updates) heading.style.visibility = visibility;
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
searchTools.hidden = false;
