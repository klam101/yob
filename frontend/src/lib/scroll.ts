/** Scrolls the page to the top. The body is the page's scroller (see globals.css), not the window. */
export function scrollToTop() {
  document.body.scrollTo({ top: 0 });
}
