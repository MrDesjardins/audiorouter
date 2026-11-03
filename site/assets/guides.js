(() => {
  "use strict";
  // Tool pages group their sections into tabs. Every in-page link (table of
  // contents, hero button, "Watch in the app", a shared #anchor) first opens
  // the tab that holds its target, so no anchor lands on a hidden section.
  const tabs = [...document.querySelectorAll('.guide-tabs [role="tab"]')];
  const panelOf = (tab) =>
    document.getElementById(tab.getAttribute("aria-controls"));
  const layout = document.querySelector(".guide-layout");
  const player = document.querySelector("[data-guide-player]");
  const playerVideo = player?.querySelector("video");
  const clips = player ? [...player.querySelectorAll(".guide-clip")] : [];
  function selectTab(tab, { focus = false } = {}) {
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      const panel = panelOf(other);
      if (selected) panel.hidden = false;
      else if (!panel.hidden) {
        panel.hidden = true;
        // Nothing keeps playing in a panel the reader cannot see.
        for (const video of panel.querySelectorAll("video")) video.pause();
      }
    }
    if (layout) layout.dataset.activeTab = tab.dataset.tab;
    if (focus) tab.focus();
  }
  function openPanelOf(element) {
    const panel = element.closest('[role="tabpanel"]');
    if (!panel || !panel.hidden) return;
    const tab = tabs.find((t) => panelOf(t) === panel);
    if (tab) selectTab(tab);
  }
  // Keep the top of a newly shown tab in view when the reader had scrolled
  // past it; the sticky tab bar stays where it is.
  function scrollToTabs() {
    const bar = document.querySelector(".guide-tabs");
    if (!bar) return;
    const top =
      bar.parentElement.getBoundingClientRect().top +
      scrollY -
      parseFloat(getComputedStyle(document.documentElement).scrollPaddingTop || "0");
    if (scrollY > top) scrollTo({ top, behavior: "instant" });
  }
  function selectClip(link, { play = false } = {}) {
    if (!playerVideo) return;
    const source = playerVideo.querySelector("source");
    for (const clip of clips)
      clip.toggleAttribute("aria-current", clip === link);
    if (source.getAttribute("src") !== link.getAttribute("href")) {
      playerVideo.pause();
      // Only the chosen clip's poster and video are ever requested.
      playerVideo.poster = link.dataset.poster;
      source.src = link.getAttribute("href");
      playerVideo.setAttribute("aria-label", link.dataset.title);
      playerVideo.load();
      const caption = player.querySelector(".guide-player-caption");
      caption.querySelector("h3").textContent = link.dataset.title;
      caption.querySelector("p").textContent = link.dataset.description;
    }
    if (play) playerVideo.play().catch(() => {});
  }
  function showPlayer() {
    const screen = player.querySelector(".guide-player-screen");
    const box = screen.getBoundingClientRect();
    if (box.top < 90 || box.bottom > innerHeight)
      screen.scrollIntoView({ block: box.height > innerHeight - 90 ? "start" : "nearest" });
  }
  /** Open the tab holding `#id`, then bring it into view. */
  function reveal(id, { play = false } = {}) {
    const target = id && document.getElementById(id);
    if (!target) return false;
    openPanelOf(target);
    if (target.classList.contains("guide-clip")) {
      selectClip(target, { play });
      showPlayer();
    } else target.scrollIntoView({ block: "start" });
    return true;
  }
  if (tabs.length) {
    if (layout) layout.dataset.activeTab = "overview";
    tabs.forEach((tab, index) => {
      tab.addEventListener("click", () => {
        if (tab.getAttribute("aria-selected") === "true") return;
        selectTab(tab);
        const first = panelOf(tab).querySelector(".guide-section[id]");
        if (first) history.replaceState(null, "", "#" + first.id);
        scrollToTabs();
      });
      tab.addEventListener("keydown", (event) => {
        const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
        let next = null;
        if (step) next = tabs[(index + step + tabs.length) % tabs.length];
        else if (event.key === "Home") next = tabs[0];
        else if (event.key === "End") next = tabs[tabs.length - 1];
        if (!next) return;
        event.preventDefault();
        next.click();
        next.focus();
      });
    });
    document.addEventListener("click", (event) => {
      if (event.defaultPrevented || event.button !== 0) return;
      if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      const link = event.target.closest("a");
      if (!link) return;
      if (link.classList.contains("guide-clip")) {
        event.preventDefault();
        history.replaceState(null, "", "#" + link.id);
        selectClip(link, { play: true });
        showPlayer();
        return;
      }
      const href = link.getAttribute("href");
      if (!href?.startsWith("#") || href.length < 2) return;
      const target = document.getElementById(decodeURIComponent(href.slice(1)));
      if (!target) return;
      if (target.classList.contains("guide-clip")) {
        event.preventDefault();
        history.pushState(null, "", href);
        reveal(target.id, { play: true });
        return;
      }
      // Unhide first; the browser's own anchor scroll then finds the target.
      openPanelOf(target);
    });
    // Back/forward to an anchor in another tab. A visible target is already
    // scrolled to by the browser.
    addEventListener("hashchange", () => {
      const id = decodeURIComponent(location.hash.slice(1));
      const target = id && document.getElementById(id);
      if (target?.classList.contains("guide-clip")) reveal(id);
      else if (target?.closest('[role="tabpanel"]')?.hidden) reveal(id);
    });
    if (location.hash.length > 1) {
      const id = decodeURIComponent(location.hash.slice(1));
      const target = document.getElementById(id);
      if (target?.closest('[role="tabpanel"]')?.hidden)
        requestAnimationFrame(() => reveal(id));
      else if (target?.classList.contains("guide-clip")) selectClip(target);
    }
  }
  const linkStatus = document.createElement("span");
  linkStatus.className = "guide-link-status";
  linkStatus.setAttribute("role", "status");
  document.body.append(linkStatus);
  for (const heading of document.querySelectorAll(".guide-section[id] > h2")) {
    const section = heading.parentElement;
    const link = document.createElement("a");
    link.className = "guide-heading-link";
    link.href = `#${section.id}`;
    link.textContent = "#";
    const label = `Copy link to ${heading.textContent.trim()}`;
    link.setAttribute("aria-label", label);
    link.title = label;
    link.addEventListener("click", async (event) => {
      event.preventDefault();
      const url = new URL(location.href);
      url.search = "";
      url.hash = section.id;
      history.replaceState(null, "", url.href);
      section.scrollIntoView({ block: "start" });
      try {
        await navigator.clipboard.writeText(url.href);
        linkStatus.textContent = "Section link copied.";
        link.title = "Link copied";
      } catch {
        location.hash = section.id;
        linkStatus.textContent = "Copy the section link from the address bar.";
      }
    });
    heading.append(" ", link);
  }
  const data = document.getElementById("guide-data");
  if (data) {
    const guide = JSON.parse(data.textContent);
    const parameters = Object.fromEntries(
      guide.controls.map((control) => [control[0], control[4]]),
    );
    const bands = guide.illustrationBands || [];
    const parameterSuffix = {
      frequencyHz: "FrequencyHz",
      gainDb: "GainDb",
      q: "Q",
    };
    const perBandKey = (index, key) => `band${index}${parameterSuffix[key]}`;

    if (bands.length) {
      bands.forEach((band, index) => {
        parameters[perBandKey(index, "frequencyHz")] ??= band.frequencyHz;
        parameters[perBandKey(index, "gainDb")] ??= band.gainDb;
        parameters[perBandKey(index, "q")] ??= band.q;
        parameters[`band${index}Type`] ??= band.filterType;
      });
      parameters.selectedBand = Number(parameters.selectedBand || 0);
      const active = bands[parameters.selectedBand];
      parameters.frequencyHz =
        parameters[perBandKey(parameters.selectedBand, "frequencyHz")];
      parameters.gainDb =
        parameters[perBandKey(parameters.selectedBand, "gainDb")];
      parameters.q = parameters[perBandKey(parameters.selectedBand, "q")];
      parameters.filterType =
        parameters[`band${parameters.selectedBand}Type`] || active.filterType;
    }

    const audio = window.AudioRouterGuideAudio?.mount(
      document.querySelector("[data-guide-demo]"),
      {
        ...guide,
        parameters,
      },
    );
    const demo = window.AudioRouterGuideAnimation.mount(
      document.querySelector("[data-guide-demo]"),
      { ...guide, parameters, transport: audio },
    );
    const playback =
      audio &&
      window.AudioRouterGuidePlayback.mount(
        document.querySelector("[data-guide-demo]"),
        audio,
      );
    const rangeInputs = [...document.querySelectorAll("[data-demo-key]")];
    const recordings = playerVideo ? [playerVideo] : [];
    // Leaving the Overview tab needs no handling here: the animation's own
    // IntersectionObserver pauses the example once its panel is hidden.
    for (const video of recordings) video.addEventListener("play", () => demo.stop());
    const recordingTransportObserver = new MutationObserver(() => {
      if (
        document.querySelector("[data-guide-demo]").dataset.audioState ===
        "playing"
      )
        for (const video of recordings) video.pause();
    });
    recordingTransportObserver.observe(
      document.querySelector("[data-guide-demo]"),
      {
        attributes: true,
        attributeFilter: ["data-audio-state"],
      },
    );
    const updateRangeInputs = () => {
      rangeInputs.forEach((input) => {
        const key = input.dataset.demoKey;
        input.value = String(parameters[key]);
        document.getElementById(`demo-value-${key}`).textContent =
          `${input.value} ${input.dataset.unit}`.trim();
      });
    };
    rangeInputs.forEach((input) =>
      input.addEventListener("input", () => {
        const key = input.dataset.demoKey;
        parameters[key] = Number(input.value);
        if (bands.length && Object.hasOwn(parameterSuffix, key)) {
          const index = Number(parameters.selectedBand);
          parameters[perBandKey(index, key)] = parameters[key];
        }
        document.getElementById(`demo-value-${key}`).textContent =
          `${input.value} ${input.dataset.unit}`.trim();
        audio?.setParameters(parameters);
        demo.setParameters(parameters);
      }),
    );
    document.querySelectorAll("[data-demo-option]").forEach((select) =>
      select.addEventListener("change", () => {
        const key = select.dataset.demoOption;
        if (key === "selectedBand" && bands.length) {
          const index = Number(select.value);
          const band = bands[index];
          parameters.selectedBand = index;
          parameters.frequencyHz = parameters[perBandKey(index, "frequencyHz")];
          parameters.gainDb = parameters[perBandKey(index, "gainDb")];
          parameters.q = parameters[perBandKey(index, "q")];
          parameters.filterType =
            parameters[`band${index}Type`] || band.filterType;
          document.querySelector('[data-demo-option="filterType"]').value =
            parameters.filterType;
          updateRangeInputs();
        } else if (key === "filterType" && bands.length) {
          const index = Number(parameters.selectedBand);
          parameters.filterType = select.value;
          parameters[`band${index}Type`] = select.value;
        } else {
          parameters[key] =
            key === "muted" ? select.value === "true" : select.value;
        }
        audio?.setParameters(parameters);
        demo.setParameters(parameters);
      }),
    );
    window.addEventListener(
      "pagehide",
      () => {
        recordingTransportObserver.disconnect();
        playback?.destroy();
        demo.destroy();
        audio?.destroy();
      },
      { once: true },
    );
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries)
          if (entry.isIntersecting) {
            document
              .querySelectorAll(".guide-toc a")
              .forEach((link) =>
                link.toggleAttribute(
                  "aria-current",
                  link.hash === `#${entry.target.id}`,
                ),
              );
          }
      },
      { rootMargin: "-15% 0px -65% 0px" },
    );
    document
      .querySelectorAll(".guide-section")
      .forEach((section) => observer.observe(section));
  }
  const search = document.getElementById("guide-search");
  if (search)
    search.addEventListener("input", () => {
      const words = search.value.toLowerCase().trim().split(/\s+/);
      let count = 0;
      document.querySelectorAll("[data-guide-search]").forEach((card) => {
        card.hidden = !words.every((word) =>
          card.dataset.guideSearch.includes(word),
        );
        if (!card.hidden) count++;
      });
      document.getElementById("guide-count").textContent =
        count + " visual guide" + (count === 1 ? "" : "s");
      document.querySelector(".guide-no-results").hidden = count !== 0;
    });
})();
