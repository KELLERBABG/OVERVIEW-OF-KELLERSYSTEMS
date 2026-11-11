/* ==========================================================================
   KELLER SYSTEMS — SCROLL STORY
   The scroll effects are driven by a computed scroll progress, published as
   CSS custom properties (--p, --herop, --scrollp). Why not CSS
   animation-timeline: it attaches correctly but resolves no progress value in
   the environment this was built and verified in, and shipping an experience
   that cannot be checked is guesswork. Position drives value, so the motion is
   continuous and reversible either way.

   Per frame this does ~12 custom-property writes, zero layout reads and no
   per-frame layout thrash. Native scrolling is never intercepted.
   ========================================================================== */
(function () {
  'use strict';

  var root = document.documentElement;
  var reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  var DATA = (typeof REPOSITORIES !== 'undefined' && Array.isArray(REPOSITORIES))
    ? REPOSITORIES.slice() : [];
  var FEATURED = ['Vantablack', 'BauLens', 'ClauseLens', 'Ambient Thermal Arbitrage', 'HoloForge'];
  var DOMAIN_LABEL = { Crypto: 'Cryptography & ZK', OS: 'Real-time / OS', Rust: 'Systems & Rust', General: 'Applied systems' };

  function esc(v) {
    return String(v == null ? '' : v)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }
  function initial(name) {
    var m = String(name || '').match(/[A-Za-z0-9]/);
    return m ? m[0].toUpperCase() : '?';
  }
  function clamp(v) { return v < 0 ? 0 : v > 1 ? 1 : v; }

  /* --- 1. Shelf + the single total, from the registry ------------------- */
  function renderShelf() {
    var host = document.getElementById('moreGrid');
    if (!host || !DATA.length) return;
    var rest = DATA
      .filter(function (r) {
        return FEATURED.indexOf(r.name) === -1 &&
               FEATURED.indexOf(r.raw_name) === -1 &&
               r.name !== 'LexiClear';
      })
      .sort(function (a, b) { return a.name.localeCompare(b.name, 'en'); });
    var tilesHtml = rest.map(function (r) {
      var isFocus = (r.name === 'AeroResonance');
      var badge = isFocus
        ? '<span class="tile-focus-badge">' +
            '<svg class="pulse-dot-green" width="10" height="10" viewBox="0 0 10 10" style="display:inline-block;vertical-align:middle;margin-right:5px;flex-shrink:0;" aria-hidden="true">' +
              '<circle cx="5" cy="5" r="3.5" fill="#2ea44f">' +
                '<animate attributeName="r" values="3.2;4.2;3.2" dur="1.8s" repeatCount="indefinite"/>' +
                '<animate attributeName="opacity" values="1;0.7;1" dur="1.8s" repeatCount="indefinite"/>' +
              '</circle>' +
              '<circle cx="5" cy="5" r="4.5" fill="none" stroke="#2ea44f" stroke-width="1.2" opacity="0.8">' +
                '<animate attributeName="r" values="4;8.5;4" dur="1.8s" repeatCount="indefinite"/>' +
                '<animate attributeName="opacity" values="0.8;0;0.8" dur="1.8s" repeatCount="indefinite"/>' +
              '</circle>' +
            '</svg>' +
            'In Progress' +
          '</span>'
        : '';
      var extraClass = isFocus ? ' tile-focus' : '';
      return '<a class="tile' + extraClass + '" href="' + esc(r.url) + '" target="_blank" rel="noopener noreferrer">' +
        badge +
        '<span class="tile-mark" aria-hidden="true">' + esc(initial(r.name)) + '</span>' +
        '<h3>' + esc(r.name) + '</h3>' +
        (r.description ? '<p class="tile-desc">' + esc(r.description) + '</p>' : '') +
        '<span class="tilemeta">' + esc(DOMAIN_LABEL[r.category] || r.category || '') +
          ' &middot; ' + esc(r.language || '') + '</span>' +
        '</a>';
    }).join('');

    host.innerHTML = tilesHtml;
  }
  function renderTotal() {
    var el = document.getElementById('totalLine');
    if (el && DATA.length) el.textContent = DATA.length + ' systems published to date';
  }

  /* --- 2. Scrub layer --------------------------------------------------- */
  var heroInner = document.querySelector('.hero-inner');
  var heroDraws = Array.prototype.slice.call(document.querySelectorAll('.sheet .draw'));
  var railFill = document.querySelector('.progress-fill');
  var scrubbed = Array.prototype.slice.call(document.querySelectorAll('.draw, .drift'))
    .filter(function (el) { return !el.closest('.sheet'); });
  var cache = [];

  /* Layout position, deliberately not getBoundingClientRect: a transform must
     never feed back into the progress value that produced it. */
  function docTop(el) {
    var t = 0;
    var node = el;
    while (node) { t += node.offsetTop || 0; node = node.offsetParent; }
    return t;
  }

  /* SVG elements have no offsetTop/offsetHeight, so a scrubbed <path> is
     measured through its nearest HTML ancestor (the plate it is drawn in). */
  function hostOf(el) {
    var node = el;
    while (node && typeof node.offsetTop !== 'number') { node = node.parentNode; }
    return node || el;
  }

  function measure() {
    cache = scrubbed.map(function (el) {
      var host = hostOf(el);
      return {
        el: el,
        kind: el.classList.contains('draw') ? 'draw' : 'drift',
        top: docTop(host),
        h: host.offsetHeight || 1
      };
    });
    frame();
  }

  function frame() {
    if (!root.classList.contains('motion-on')) return;
    var vh = window.innerHeight;
    var y = window.scrollY;

    /* The pinned sheet eases over one full viewport of scroll, and only to
       half opacity: dimming harder or faster turns the hero copy grey while
       it is still the thing being read. The values published are finished
       ones — no calc() in the stylesheet — because stroke-dashoffset would
       not resolve a calc() here. */
    var hp = clamp(y / (vh || 1));
    if (heroInner) {
      heroInner.style.setProperty('--hero-op', (1 - hp * 0.5).toFixed(3));
      heroInner.style.setProperty('--hero-scale', (1 - hp * 0.03).toFixed(4));
    }
    for (var h = 0; h < heroDraws.length; h++) {
      heroDraws[h].style.setProperty('--dash', (1 - clamp((hp - 0.06) / 0.7)).toFixed(4));
    }

    /* everything below: 0 when the element's top reaches the viewport bottom,
       1 when its bottom reaches the viewport top */
    for (var i = 0; i < cache.length; i++) {
      var t = cache[i];
      var p = clamp((y + vh - t.top) / (vh + t.h));
      if (t.kind === 'draw') {
        t.el.style.setProperty('--dash', (1 - clamp((p - 0.12) / 0.6)).toFixed(4));
      } else {
        t.el.style.setProperty('--drift-y', ((p - 0.5) * 4.4).toFixed(3) + '%');
      }
    }

    if (railFill) {
      var max = root.scrollHeight - vh;
      railFill.style.setProperty('--rail', (max > 0 ? clamp(y / max) : 0).toFixed(4));
    }

    updateTicks();
  }

  function resetScrub() {
    /* `scrubbed` holds elements; `cache` holds the measurement records. */
    scrubbed.forEach(function (el) {
      el.style.removeProperty('--dash');
      el.style.removeProperty('--drift-y');
    });
    heroDraws.forEach(function (el) { el.style.removeProperty('--dash'); });
    if (heroInner) {
      heroInner.style.removeProperty('--hero-op');
      heroInner.style.removeProperty('--hero-scale');
    }
    if (railFill) railFill.style.removeProperty('--rail');
  }

  /* --- 3. Motion gate --------------------------------------------------- */
  var toggle = document.getElementById('motionToggle');
  var note = document.getElementById('motionNote');

  function setMotion(on) {
    root.classList.toggle('motion-on', on);
    root.setAttribute('data-motion', on ? 'on' : 'off');
    if (toggle) {
      toggle.setAttribute('aria-pressed', String(on));
      toggle.textContent = on ? 'Motion: on' : 'Motion: off';
    }
    if (note) {
      note.textContent = on
        ? (reduce
            ? 'scroll story on, overriding your system motion setting'
            : 'scroll-driven, following your system motion setting')
        : (reduce
            ? 'your system asks for reduced motion \u2014 switch Motion on for the full story'
            : 'motion is off');
    }
    if (on) { measure(); } else { resetScrub(); }
  }

  /* --- 4. Progress-rail ticks ------------------------------------------- */
  var ticksEl = document.getElementById('progressTicks');
  var markers = ['ch1', 'ch2', 'ch3', 'ch4', 'ch5', 'shelf'].map(function (id) {
    return { id: id, el: document.getElementById(id) };
  }).filter(function (m) { return m.el; });
  var tickList = [];

  function layoutTicks() {
    if (!ticksEl) return;
    var top = document.querySelector('.top');
    var headerH = top ? top.offsetHeight : 0;
    var max = root.scrollHeight - window.innerHeight;
    ticksEl.innerHTML = '';
    tickList = markers.map(function (m) {
      var y = docTop(m.el) - headerH;
      var tick = document.createElement('i');
      tick.style.top = (max > 0 ? Math.min(Math.max((y / max) * 100, 0), 100) : 0) + '%';
      ticksEl.appendChild(tick);
      return { y: y, tick: tick };
    });
    updateTicks();
  }

  function updateTicks() {
    for (var i = 0; i < tickList.length; i++) {
      tickList[i].tick.classList.toggle('is-on', window.scrollY >= tickList[i].y - 4);
    }
  }

  /* --- 5. Scrollspy: contents list + sticky chapter label --------------- */
  function initSpy() {
    var chapters = Array.prototype.slice.call(document.querySelectorAll('.chapter'));
    var items = Array.prototype.slice.call(document.querySelectorAll('#contentsList li'));
    var label = document.getElementById('chapterLabel');
    var indexEl = label ? label.querySelector('b') : null;
    var nameEl = label ? label.querySelector('span') : null;
    if (!chapters.length || !('IntersectionObserver' in window)) return;

    var active = null;
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          active = entry.target;
          var id = entry.target.id;
          items.forEach(function (li) { li.classList.toggle('is-on', li.dataset.for === id); });
          if (label && indexEl && nameEl) {
            indexEl.textContent = entry.target.dataset.index;
            nameEl.textContent = entry.target.dataset.name;
            label.classList.add('is-on');
          }
        } else if (entry.target === active) {
          active = null;
          if (label) label.classList.remove('is-on');
        }
      });
    }, { rootMargin: '-45% 0px -50% 0px' });

    chapters.forEach(function (c) { io.observe(c); });
  }

  /* --- 6. One passive listener, called directly --------------------------
     Not rAF-throttled on purpose: if animation frames are starved (a
     backgrounded or occluded frame), an rAF gate can latch shut and stop
     responding for good. The work here is a dozen custom-property writes and
     zero layout reads, and browsers already coalesce scroll events to about
     one per frame, so calling straight through is both cheaper to reason
     about and safe. Native scrolling is never intercepted. */
  window.addEventListener('scroll', frame, { passive: true });

  function remeasure() { layoutTicks(); measure(); }
  window.addEventListener('resize', remeasure);
  window.addEventListener('load', remeasure);
  if (document.fonts && document.fonts.ready) { document.fonts.ready.then(remeasure); }

  /* --- boot ------------------------------------------------------------- */
  renderShelf();
  renderTotal();
  layoutTicks();
  initSpy();
  setMotion(!reduce);
  if (toggle) {
    toggle.addEventListener('click', function () {
      setMotion(!root.classList.contains('motion-on'));
    });
  }
})();
