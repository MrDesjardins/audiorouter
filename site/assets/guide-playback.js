/* Sample views and reference listening share the lesson's only transport. */
(() => {
  "use strict";
  function mount(host, audio) {
    const panel = document.createElement("section");
    panel.className = "guide-signal-panel";
    panel.setAttribute("aria-label", "Example inputs and output");
    const heading = document.createElement("h3");
    heading.textContent = "Follow the sound";
    const label = document.createElement("label");
    label.textContent = "Listen to";
    const select = document.createElement("select");
    select.setAttribute("aria-label", "Listen to example signal");
    label.append(select);
    const rows = document.createElement("div");
    rows.className = "guide-signal-rows";
    const note = document.createElement("p");
    note.className = "guide-note";
    note.textContent =
      "Waveforms and levels come from the example samples, before the browser monitor gain is set to 50%. Use the Play/Record controls above for every listening choice.";
    panel.append(heading, label, rows, note);
    (host.closest("#see-it") || host).append(panel);
    let snapshot = null,
      views = [],
      prefix = [],
      destroyed = false;
    select.addEventListener("change", () => audio.setAudition(select.value));
    const unsubscribe = audio.subscribe((next) => {
      if (!next || destroyed) return;
      snapshot = next;
      const signals = [...next.inputs, next.output];
      select.replaceChildren();
      [
        { label: "Processed output", value: "output" },
        ...next.inputs.map((input, i) => ({
          label: input.label,
          value: `input${i}`,
        })),
      ].forEach((choice) => {
        const option = document.createElement("option");
        option.value = choice.value;
        option.textContent = choice.label;
        select.append(option);
      });
      select.value = audio.getAudition();
      // Recorder always monitors its capture/playback source; no independent audition can interrupt a take.
      select.disabled = host.querySelector("[data-recorder-stage]") !== null;
      rows.replaceChildren();
      views = signals.map((signal, index) => {
        const row = document.createElement("div");
        row.className = "guide-signal-row";
        row.dataset.signal =
          index === signals.length - 1 ? "output" : `input${index}`;
        const name = document.createElement("strong");
        name.textContent =
          signal.label + (signal.controlOnly ? " · control only" : "");
        const plot = document.createElement("div");
        plot.className = "guide-signal-plot";
        const canvas = document.createElement("canvas");
        canvas.width = 960;
        canvas.height = 70;
        canvas.setAttribute("role", "img");
        canvas.setAttribute(
          "aria-label",
          `${signal.label}, eight-second sample waveform`,
        );
        const color =
          index === signals.length - 1
            ? "#7de3c2"
            : index
              ? "#ffc980"
              : "#6fbdff";
        const ctx = canvas.getContext("2d");
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        const samples = signal.channels[0],
          size = samples.length / canvas.width;
        ctx.beginPath();
        for (let x = 0; x < canvas.width; x++) {
          let low = 0,
            high = 0;
          for (
            let i = Math.floor(x * size);
            i < Math.floor((x + 1) * size);
            i++
          ) {
            low = Math.min(low, samples[i]);
            high = Math.max(high, samples[i]);
          }
          ctx.moveTo(x + 0.5, 35 - Math.min(1.4, high) * 24);
          ctx.lineTo(x + 0.5, 35 - Math.max(-1.4, low) * 24);
        }
        ctx.stroke();
        const cursor = document.createElement("span");
        cursor.className = "guide-signal-cursor";
        cursor.setAttribute("aria-hidden", "true");
        plot.append(canvas, cursor);
        const reading = document.createElement("output");
        reading.textContent = "RMS — · Peak —";
        row.append(name, plot, reading);
        rows.append(row);
        return { cursor, reading, signal };
      });
      prefix = next.output.channels.map((samples) => {
        const peaks = new Float32Array(801),
          clips = new Uint32Array(801);
        let peak = 0,
          count = 0;
        for (let bucket = 0; bucket <= 800; bucket++) {
          for (
            let i = Math.floor(
              (Math.max(0, bucket - 1) * next.sampleRate) / 100,
            );
            i <
            Math.min(
              samples.length,
              Math.floor((bucket * next.sampleRate) / 100),
            );
            i++
          ) {
            peak = Math.max(peak, Math.abs(samples[i]));
            if (Math.abs(samples[i]) > 1) count++;
          }
          peaks[bucket] = peak;
          clips[bucket] = count;
        }
        return { peaks, clips };
      });
      update();
    });
    const db = (v) => (v <= -60 ? "−∞" : v.toFixed(1));
    function update() {
      if (!snapshot || destroyed) return;
      const animation = host.__guideAnimation;
      const seconds = audio.isPlaying()
        ? audio.getTime()
        : (animation?.currentTime() || 0) / 1000;
      if (audio.isPlaying()) animation?.sync?.(seconds);
      views.forEach(({ cursor, reading, signal }) => {
        cursor.style.left = `${Math.min(100, (seconds / 8) * 100)}%`;
        const values = window.AudioRouterGuideSignal.reading(
          signal.channels,
          seconds,
        )[0];
        reading.textContent = `RMS ${db(values.rmsDb)} · Peak ${db(values.peakDb)} dBFS`;
      });
      const values = window.AudioRouterGuideSignal.reading(
        snapshot.output.channels,
        seconds,
      );
      const bucket = Math.min(800, Math.floor(seconds * 100));
      const y = (level) =>
        246 - ((Math.max(-60, Math.min(6, level)) + 60) / 66) * 164;
      host.querySelectorAll("[data-meter-rms]").forEach((bar, channel) => {
        const value = values[channel] || values[0],
          top = y(value.rmsDb);
        bar.setAttribute("y", String(top));
        bar.setAttribute("height", String(246 - top));
        const peak = host.querySelector(`[data-meter-peak='${channel}']`),
          hold = host.querySelector(`[data-meter-hold='${channel}']`);
        const currentY = y(value.peakDb),
          held = prefix[channel] || prefix[0],
          heldY = y(20 * Math.log10(Math.max(1e-9, held.peaks[bucket])));
        for (const [marker, position] of [
          [peak, currentY],
          [hold, heldY],
        ]) {
          marker?.setAttribute("y1", String(position));
          marker?.setAttribute("y2", String(position));
        }
        const text = host.querySelector(`[data-meter-values='${channel}']`);
        if (text)
          text.textContent = `RMS ${db(value.rmsDb)} · PK ${db(value.peakDb)}`;
      });
      const clipLabel = host.querySelector("[data-meter-clips]");
      if (clipLabel)
        clipLabel.textContent = `CLIPS ${prefix.reduce((sum, channel) => sum + channel.clips[bucket], 0)}`;
    }
    const timer = setInterval(update, 50);
    return {
      destroy() {
        destroyed = true;
        clearInterval(timer);
        unsubscribe();
        panel.remove();
      },
    };
  }
  window.AudioRouterGuidePlayback = Object.freeze({ mount });
})();
