(() => {
  "use strict";
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
    const recordings = [...document.querySelectorAll(".guide-recording video")];
    for (const video of recordings) {
      video.addEventListener("play", () => {
        demo.stop();
        for (const other of recordings) if (other !== video) other.pause();
      });
    }
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
