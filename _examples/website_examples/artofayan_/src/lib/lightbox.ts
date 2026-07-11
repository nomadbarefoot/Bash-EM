/**
 * Shared lightbox module — single source of truth for PhotoSwipe.
 * Fetches /data/gallery-manifest.json on first call (cached for the session).
 * Provides continuous project-to-project navigation via keyboard and touch.
 */
import PhotoSwipe from "photoswipe";

export interface Slide {
  src: string;
  width: number;
  height: number;
}

export interface ManifestEntry {
  slug: string;
  title: string;
  category: string;
  categoryLabel: string;
  year: number;
  description: string;
  slides: Slide[];
}

let manifestCache: ManifestEntry[] | null = null;

// Single glass backdrop element shared across project-to-project navigation.
// Lives on body (outside .pswp stacking context) so backdrop-filter actually works.
let activeBackdrop: HTMLElement | null = null;

function showBackdrop(): void {
  if (activeBackdrop) return;
  const el = document.createElement("div");
  el.className = "pswp-glass-backdrop";
  document.body.appendChild(el);
  activeBackdrop = el;
  // Trigger transition on next frame
  requestAnimationFrame(() => el.classList.add("is-visible"));
}

function hideBackdrop(): void {
  const el = activeBackdrop;
  if (!el) return;
  activeBackdrop = null;
  el.classList.remove("is-visible");
  el.addEventListener("transitionend", () => el.remove(), { once: true });
}

async function getManifest(): Promise<ManifestEntry[]> {
  if (manifestCache) return manifestCache;
  const res = await fetch("/data/gallery-manifest.json");
  if (!res.ok) throw new Error(`gallery-manifest: ${res.status}`);
  manifestCache = (await res.json()) as ManifestEntry[];
  return manifestCache;
}

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

const NAV_LINK_STYLE = [
  "color:rgba(255,255,255,0.5)",
  "font-size:0.72rem",
  "text-decoration:none",
  "font-family:var(--font-ui,system-ui,sans-serif)",
  "border-bottom:1px solid rgba(255,255,255,0.18)",
  "padding-bottom:1px",
].join(";");

function buildNavHtml(
  slug: string,
  currIndex: number,
  totalSlides: number,
  entries: ManifestEntry[],
  entryIdx: number
): string {
  const isFirst = currIndex === 0;
  const isLast = currIndex === totalSlides - 1;
  const prev = entryIdx > 0 ? entries[entryIdx - 1] : null;
  const next = entryIdx < entries.length - 1 ? entries[entryIdx + 1] : null;

  if (isFirst && prev) {
    return `<a data-pswp-nav="prev" href="/series/${prev.slug}" style="${NAV_LINK_STYLE}">← ${esc(prev.title)}</a>`;
  }
  if (isLast && next) {
    return `<a data-pswp-nav="next" href="/series/${next.slug}" style="${NAV_LINK_STYLE}">${esc(next.title)} →</a>`;
  }
  return `<a href="/series/${slug}" style="${NAV_LINK_STYLE}">View series page →</a>`;
}

function buildCaption(entry: ManifestEntry, navHtml: string): string {
  return `
    <div style="position:absolute;bottom:0;left:0;right:0;pointer-events:none;padding:0 22px 22px;">
      <div style="
        width:min(420px,calc(100vw - 44px));
        padding:18px 20px 19px 0;
        pointer-events:auto;
      ">
        <div style="font-size:0.64rem;text-transform:uppercase;letter-spacing:0.13em;color:var(--accent,#ea6735);margin-bottom:5px;font-family:var(--font-ui,system-ui,sans-serif);font-weight:500;text-shadow:0 2px 8px rgba(18,22,22,0.9);">
          ${esc(entry.categoryLabel)}
        </div>
        <div style="font-size:clamp(0.92rem,2.4vw,1.05rem);font-weight:400;letter-spacing:0;color:#f4f1ec;line-height:1.22;margin-bottom:4px;font-family:var(--font-ui,system-ui,sans-serif);text-shadow:0 2px 14px rgba(18,22,22,0.9);">
          ${esc(entry.title)}
        </div>
        <div style="font-size:0.68rem;color:rgba(244,241,236,0.52);margin-bottom:11px;font-family:var(--font-ui,system-ui,sans-serif);text-shadow:0 1px 8px rgba(18,22,22,0.82);">
          ${entry.year}
        </div>
        ${navHtml}
      </div>
    </div>`;
}

export async function openProjectLightbox(
  slug: string,
  startIndex = 0
): Promise<void> {
  let entries: ManifestEntry[];
  try {
    entries = await getManifest();
  } catch (err) {
    console.error("[lightbox] manifest unavailable — cannot open lightbox", err);
    return;
  }

  const entryIdx = entries.findIndex((e) => e.slug === slug);
  if (entryIdx === -1) {
    console.warn("[lightbox] slug not found in manifest:", slug);
    return;
  }
  const entry = entries[entryIdx]!;

  // Show glass backdrop before opening (persists across project navigation)
  showBackdrop();

  const pswp = new PhotoSwipe({
    dataSource: entry.slides,
    index: Math.min(startIndex, Math.max(0, entry.slides.length - 1)),
    showHideAnimationType: "zoom",
    wheelToZoom: true,
    paddingFn: () => ({ top: 20, bottom: 20, left: 20, right: 20 }),
  });

  // Set when navigating to adjacent project — prevents treating close as "user dismissed"
  let pendingNav: { slug: string; index: number } | null = null;

  function navigateTo(toSlug: string, toIndex: number): void {
    pendingNav = { slug: toSlug, index: toIndex };
    pswp.close();
  }

  // Rich caption with static metadata + slide-aware nav hint
  pswp.on("uiRegister", () => {
    pswp.ui.registerElement({
      name: "custom-caption",
      order: 9,
      isButton: false,
      appendTo: "root",
      html: "",
      onInit: (el: HTMLElement) => {
        el.style.cssText = "position:absolute;inset:0;pointer-events:none;";

        const render = () => {
          const navHtml = buildNavHtml(
            slug,
            pswp.currIndex,
            entry.slides.length,
            entries,
            entryIdx
          );
          el.innerHTML = buildCaption(entry, navHtml);

          const navLink = el.querySelector<HTMLAnchorElement>("[data-pswp-nav]");
          if (!navLink) return;
          navLink.addEventListener("click", (e) => {
            e.preventDefault();
            const dir = navLink.getAttribute("data-pswp-nav");
            if (dir === "next") {
              const next = entries[entryIdx + 1];
              if (next) navigateTo(next.slug, 0);
            } else if (dir === "prev") {
              const prev = entries[entryIdx - 1];
              if (prev) navigateTo(prev.slug, prev.slides.length - 1);
            }
          });
        };

        pswp.on("change", render);
        render();
      },
    });
  });

  // Register keyboard + touch edge navigation after PhotoSwipe initialises its own handlers.
  // NOTE: pswp.on("open") does NOT exist in PhotoSwipe 5 — use "afterInit".
  pswp.on("afterInit", () => {
    // ── Keyboard: ArrowLeft/Right at edge → navigate to adjacent project ──
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight" && pswp.currIndex === entry.slides.length - 1) {
        const next = entries[entryIdx + 1];
        if (next) {
          e.stopImmediatePropagation();
          navigateTo(next.slug, 0);
        }
      } else if (e.key === "ArrowLeft" && pswp.currIndex === 0) {
        const prev = entries[entryIdx - 1];
        if (prev) {
          e.stopImmediatePropagation();
          navigateTo(prev.slug, prev.slides.length - 1);
        }
      }
    };
    // Capture phase fires before PhotoSwipe's document-bubble handler
    window.addEventListener("keydown", onKey, { capture: true });
    pswp.on("destroy", () =>
      window.removeEventListener("keydown", onKey, { capture: true })
    );

    // ── Touch: swipe-past-edge → navigate to adjacent project ──
    const pswpEl = pswp.element;
    if (pswpEl) {
      let touchStartX = 0;
      let touchStartIdx = 0;

      // Record position AND slide index at gesture start
      pswpEl.addEventListener(
        "touchstart",
        (e) => {
          touchStartX = (e as TouchEvent).touches[0]?.clientX ?? 0;
          touchStartIdx = pswp.currIndex;
        },
        { passive: true }
      );

      // After gesture: only trigger cross-project nav if swipe started at an edge slide
      pswpEl.addEventListener(
        "touchend",
        (e) => {
          const endX = (e as TouchEvent).changedTouches[0]?.clientX ?? 0;
          const deltaX = endX - touchStartX;
          if (Math.abs(deltaX) < 50) return;

          if (deltaX < 0 && touchStartIdx === entry.slides.length - 1) {
            const next = entries[entryIdx + 1];
            if (next) navigateTo(next.slug, 0);
          } else if (deltaX > 0 && touchStartIdx === 0) {
            const prev = entries[entryIdx - 1];
            if (prev) navigateTo(prev.slug, prev.slides.length - 1);
          }
        },
        { passive: true }
      );
    }
  });

  // After close: hide backdrop (unless navigating to adjacent project — keep it visible)
  pswp.on("destroy", () => {
    if (pendingNav) {
      // Navigating project-to-project: backdrop stays, new lightbox opens immediately
      const { slug: toSlug, index: toIndex } = pendingNav;
      pendingNav = null;
      requestAnimationFrame(() => openProjectLightbox(toSlug, toIndex));
    } else {
      // User closed the lightbox: fade backdrop out
      hideBackdrop();
    }
  });

  pswp.on("contentLoadError", ({ content }) => {
    const el = content.element;
    if (!el) return;
    el.style.cssText = "background:#0a0a0a;position:relative;";
    el.innerHTML = `<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;font-size:0.7rem;color:rgba(255,255,255,0.2);text-transform:uppercase;letter-spacing:0.08em;font-family:var(--font-ui,system-ui,sans-serif);">Image unavailable</div>`;
  });

  pswp.init();
}
