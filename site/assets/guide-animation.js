/* Local, illustrative audio diagrams. Native WAAPI keeps motion finite and seekable. */
(() => {
  "use strict";
  const duration = 8000;
  const ns = "http://www.w3.org/2000/svg";
  let sequence = 0;
  const clamp = (v, min, max) => Math.min(max, Math.max(min, v));
  const number = (value, fallback, min, max) =>
    Number.isFinite(Number(value)) ? clamp(Number(value), min, max) : fallback;
  function svgElement(tag, attributes = {}, text) {
    const element = document.createElementNS(ns, tag);
    Object.entries(attributes).forEach(([key, value]) =>
      element.setAttribute(key, String(value)),
    );
    if (text !== undefined) element.textContent = text;
    return element;
  }
  function mount(host, options = {}) {
    if (!(host instanceof HTMLElement))
      throw new TypeError("A guide animation needs a host element.");
    host.__guideAnimation?.destroy();
    const id = `guide-visual-${++sequence}`;
    const type = options.type || "waveform";
    const parameters = options.parameters || {};
    const animations = [];
    let timelineDuration = duration;
    let recorderPhase = options.recorderPhase || "ready";
    let recorderFileView = recorderPhase === "filePlaying";
    let recorderBoxes = [];
    let recorderWaves = [];
    let recorderStatusText = null;
    let cursorAnimation = null;
    const root = document.createElement("figure");
    root.className = "guide-animation";
    root.style.cssText =
      "margin:0;max-width:100%;color:var(--text,#eaf0fb);background:var(--panel,#101a2b);border:1px solid var(--border,#34435b);border-radius:18px;overflow:hidden";
    const svg = svgElement("svg", {
      viewBox: "0 0 760 340",
      role: "img",
      "aria-labelledby": `${id}-title ${id}-desc`,
      preserveAspectRatio: "xMidYMid meet",
    });
    svg.style.cssText = "display:block;width:100%;height:auto;max-height:380px";
    svg.append(
      svgElement(
        "title",
        { id: `${id}-title` },
        options.title || "Audio signal example",
      ),
    );
    svg.append(
      svgElement(
        "desc",
        { id: `${id}-desc` },
        options.description ||
          "An illustrative input and output comparison. The diagram is not a measurement or an audio preview.",
      ),
    );
    const add = (tag, attributes, text) => {
      const element = svgElement(tag, attributes, text);
      svg.append(element);
      return element;
    };
    const text = (x, y, value, color = "#eaf0fb", size = 16) =>
      add(
        "text",
        {
          x,
          y,
          fill: color,
          "font-size": size,
          "font-family": "system-ui,sans-serif",
        },
        value,
      );
    const line = (x1, y1, x2, y2, color = "#34435b", dash) =>
      add("line", {
        x1,
        y1,
        x2,
        y2,
        stroke: color,
        "stroke-width": 1,
        ...(dash ? { "stroke-dasharray": dash } : {}),
      });
    const path = (points, color, width = 3) =>
      add("path", {
        d: points
          .map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(2)},${y.toFixed(2)}`)
          .join(" "),
        fill: "none",
        stroke: color,
        "stroke-width": width,
        "stroke-linejoin": "round",
        "stroke-linecap": "round",
      });
    const animate = (element, keyframes, timing = {}) => {
      if (typeof element.animate !== "function") return;
      const animation = element.animate(keyframes, {
        duration,
        // Lessons repeat until Stop; a Recorder take plays once.
        iterations: type === "recording" ? 1 : Infinity,
        fill: "both",
        easing: "linear",
        ...timing,
      });
      animation.pause();
      animation.currentTime = 0;
      animations.push(animation);
      return animation;
    };
    const inputColor = "#6fbdff",
      outputColor = "#7de3c2",
      accent = "#ffc980";
    for (let x = 70; x <= 710; x += 80) line(x, 52, x, 280);
    [88, 140, 192, 244, 280].forEach((y) => line(70, y, 710, y));
    text(28, 32, options.title || "Input → output", "#eaf0fb", 19);
    const wave = (fn, y, color) =>
      path(
        Array.from({ length: 321 }, (_, i) => {
          const t = i / 320;
          return [70 + t * 640, y - fn(t) * 48];
        }),
        color,
      );
    const envelope = (t) =>
      0.23 + 0.67 * Math.pow(Math.sin(t * Math.PI * 3), 2);
    const signal = (t) => Math.sin(t * Math.PI * 32) * envelope(t);
    let detail = "Input and output shown over the same interval.";
    // Time-axis pictures: the playhead crosses the same eight seconds that are
    // heard, so their traces are redrawn from the example audio (see below).
    const timelineTypes = new Set([
      "gain",
      "volume",
      "dynamics",
      "limiter",
      "gate",
      "duck",
      "delay",
      "media",
    ]);
    const traces = [];
    let controlBand = null;
    const pair = (output, input = signal) => {
      text(20, 108, "IN", inputColor, 13);
      text(15, 235, "OUT", outputColor, 13);
      traces.push({
        element: wave(input, 106, inputColor),
        y: 106,
        signal: "input",
      });
      traces.push({
        element: wave(output, 232, outputColor),
        y: 232,
        signal: "output",
      });
    };
    if (type === "meter") {
      text(72, 64, "SIGNAL LEVEL · STEREO", inputColor, 13);
      const top = 82,
        bottom = 246,
        height = bottom - top;
      const yFor = (db) => bottom - ((db + 60) / 66) * height;
      text(108, yFor(6) + 4, "dBFS", "#aebed3", 9);
      [6, 0, -6, -12, -18, -24, -36, -48, -60].forEach((db) => {
        const y = yFor(db);
        line(
          138,
          y,
          628,
          y,
          db === 0 ? "#d38a77" : "#34435b",
          db === 0 ? "4 4" : undefined,
        );
        text(94, y + 4, String(db), "#aebed3", 10);
      });
      text(632, yFor(0) + 14, "CLIP > 0", "#ff766d", 10);
      [244, 438].forEach((x, channel) => {
        add("rect", {
          x,
          y: top,
          width: 66,
          height,
          rx: 8,
          fill: "#101a2b",
          stroke: "#52637b",
        });
        add("rect", {
          x,
          y: top,
          width: 66,
          height: yFor(0) - top,
          rx: 5,
          fill: "#8d3540",
        });
        const rms = add("rect", {
          x: x + 15,
          y: top + 22,
          width: 36,
          height: height - 26,
          rx: 5,
          fill: "#347f73",
          "data-meter-rms": channel ? "right" : "left",
        });
        rms.style.transformBox = "fill-box";
        rms.style.transformOrigin = "bottom";
        const currentY = yFor(-12 - channel * 2),
          holdY = yFor(-6 - channel * 2);
        add("line", {
          x1: x - 4,
          "data-meter-peak": channel,
          y1: currentY,
          x2: x + 70,
          y2: currentY,
          stroke: "#edf2f8",
          "stroke-width": 3,
        });
        add("line", {
          x1: x - 4,
          "data-meter-hold": channel,
          y1: holdY,
          x2: x + 70,
          y2: holdY,
          stroke: accent,
          "stroke-width": 4,
        });
        text(x + 11, 266, channel ? "RIGHT" : "LEFT", "#d9e4f2", 10);
        text(x - 8, 286, "RMS — · PK —", "#b6c6dd", 10).setAttribute(
          "data-meter-values",
          String(channel),
        );
      });
      text(618, 64, "CLIPS 0", "#a4dec9", 10).setAttribute(
        "data-meter-clips",
        "true",
      );
      detail =
        "AudioRouter's Meter shows separate left/right RMS bars, current peak and peak hold against a −60 to +6 dBFS scale. Red marks the over-full-scale region. Readings are computed from the synthetic samples shown below, not from your running session.";
    } else if (type === "fir") {
      const wet = number(parameters.wetPercent, 100, 0, 100) / 100;
      const gain = 10 ** (number(parameters.gainDb, 0, -24, 12) / 20);
      text(
        70,
        66,
        "IMPORTED IMPULSE RESPONSE · SYNTHETIC SHAPE",
        inputColor,
        13,
      );
      line(70, 112, 710, 112, "#45536a");
      const impulse = (t) => {
        const position = t * 10;
        return t < 0.12
          ? Math.exp(-position * 2.8) * Math.sin(position * 29)
          : 0;
      };
      wave(impulse, 112, accent);
      add("rect", {
        x: 70,
        y: 148,
        width: 640,
        height: 104,
        rx: 12,
        fill: "#142338",
        stroke: "#40516a",
      });
      text(86, 174, "DRY · equally delayed", inputColor, 12);
      text(540, 174, `WET · ${Math.round(wet * 100)}%`, outputColor, 12);
      const mixed = (t) => {
        const dry = signal(t);
        const tail =
          t < 0.18 ? 0 : signal(t - 0.18) * Math.exp(-(t - 0.18) * 5);
        return clamp((dry * (1 - wet) + tail * wet) * gain, -1, 1);
      };
      wave((t) => signal(t) * (1 - wet), 207, inputColor);
      wave((t) => mixed(t) - signal(t) * (1 - wet), 207, outputColor);
      text(86, 276, `Wet mix ${Math.round(wet * 100)}%`, accent, 13);
      text(
        410,
        276,
        `Output gain ${number(parameters.gainDb, 0, -24, 12)} dB`,
        outputColor,
        13,
      );
      detail =
        "A synthetic impulse-response shape is convolved with a test tone, then blended with an equally delayed dry path. Real FIR uses the selected impulse-response file; this drawing does not load one.";
    } else if (type === "graphicEq") {
      const centers = options.graphicBands?.length
        ? options.graphicBands
        : [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000].map(
            (frequencyHz, i) => ({
              key: `band${i}Db`,
              label: String(frequencyHz),
              frequencyHz,
            }),
          );
      const values = centers.map((band) =>
        number(parameters[band.key], 0, -18, 18),
      );
      const xFor = (frequency) => 70 + (Math.log10(frequency / 20) / 3) * 640;
      text(70, 65, "TEN FIXED-CENTER BANDS", inputColor, 13);
      text(615, 65, "±18 dB", accent, 12);
      line(70, 158, 710, 158, inputColor, "5 5");
      const response = (t) =>
        centers.reduce((sum, band, i) => {
          const center = Math.log10(band.frequencyHz / 20) / 3;
          return (
            sum +
            (values[i] / 18) * Math.exp(-Math.pow((t - center) / 0.095, 2))
          );
        }, 0);
      path(
        Array.from({ length: 161 }, (_, i) => [
          70 + (i / 160) * 640,
          158 - clamp(response(i / 160), -1, 1) * 74,
        ]),
        outputColor,
        4,
      );
      centers.forEach((band, i) => {
        const x = xFor(band.frequencyHz);
        line(x, 235, x, 259, "#60728c");
        const y = 220 - (values[i] / 18) * 38;
        add("rect", {
          x: x - 12,
          y: Math.min(y, 220),
          width: 24,
          height: Math.max(2, Math.abs(220 - y)),
          rx: 4,
          fill: values[i] >= 0 ? outputColor : accent,
          opacity: 0.8,
          "data-graphic-band": band.key,
        });
        add("circle", { cx: x, cy: y, r: 4, fill: "#f4f7fb" });
        text(x - 19, 282, band.label.replace(" Hz", ""), "#c8d6e9", 9);
      });
      detail =
        "Ten fixed bands match the Graphic EQ faders. Adjust each band below to reshape the combined response; neighboring centers overlap.";
    } else if (type === "parametricEq") {
      const bands = options.illustrationBands || [];
      const selected = number(
        parameters.selectedBand,
        0,
        0,
        Math.max(0, bands.length - 1),
      );
      const xFor = (hz) =>
        70 + (Math.log10(number(hz, 1000, 20, 20000) / 20) / 3) * 640;
      const colors = ["#7de3c2", "#ffc980", "#bb9cff", "#6fbdff"];
      text(70, 65, "COMBINED FILTER RESPONSE", inputColor, 13);
      line(70, 168, 710, 168, inputColor, "5 5");
      const responseFor = (band, i, t) => {
        const center =
          Math.log10(
            number(
              parameters[`band${i}FrequencyHz`],
              band.frequencyHz,
              20,
              20000,
            ) / 20,
          ) / 3;
        const q = number(parameters[`band${i}Q`], band.q, 0.1, 20);
        const gain =
          number(parameters[`band${i}GainDb`], band.gainDb, -24, 24) / 18;
        const typeName = parameters[`band${i}Type`] || band.filterType;
        const spread = 0.15 / Math.sqrt(q);
        const bell = Math.exp(-Math.pow((t - center) / spread, 2));
        const low = 1 / (1 + Math.exp((t - center) * 24));
        if (typeName === "lowShelf") return low * gain;
        if (typeName === "highShelf") return (1 - low) * gain;
        if (typeName === "lowPass") return -(1 - low) * 0.85;
        if (typeName === "highPass") return -low * 0.85;
        if (typeName === "bandPass") return -(1 - bell) * 0.85;
        if (typeName === "notch") return -bell * 0.85;
        return typeName === "allPass" ? 0 : bell * gain;
      };
      bands.forEach((band, i) => {
        const color = colors[i % colors.length];
        const pts = Array.from({ length: 161 }, (_, j) => {
          const t = j / 160;
          return [70 + t * 640, 168 - responseFor(band, i, t) * 70];
        });
        const bandPath = path(pts, color, i === selected ? 2.5 : 1.5);
        bandPath.setAttribute("data-parametric-band", String(i));
        const x = xFor(parameters[`band${i}FrequencyHz`] ?? band.frequencyHz);
        add("circle", {
          cx: x,
          cy:
            168 -
            responseFor(
              band,
              i,
              Math.log10(
                number(
                  parameters[`band${i}FrequencyHz`],
                  band.frequencyHz,
                  20,
                  20000,
                ) / 20,
              ) / 3,
            ) *
              70,
          r: i === selected ? 8 : 5,
          fill: color,
          stroke: "#101a2b",
          "stroke-width": 2,
          opacity: i === selected ? 1 : 0.75,
        });
        text(76 + i * 155, 229, `B${i + 1} · ${band.label}`, color, 11);
        text(
          76 + i * 155,
          249,
          `${parameters[`band${i}Type`] || band.filterType}`,
          "#c8d6e9",
          10,
        );
      });
      const band = bands[selected] || {
        label: "Band",
        frequencyHz: 1000,
        gainDb: 0,
        q: 1,
        filterType: "peaking",
      };
      const selectedType = parameters[`band${selected}Type`] || band.filterType;
      if (selectedType === "allPass") {
        const center =
          Math.log10(
            number(parameters.frequencyHz, band.frequencyHz, 20, 20000) / 20,
          ) / 3;
        path(
          Array.from({ length: 81 }, (_, i) => {
            const t = i / 80;
            return [70 + t * 640, 285 - Math.atan((t - center) * 22) * 8];
          }),
          accent,
          2,
        );
        text(
          76,
          269,
          "ALL PASS · FLAT MAGNITUDE, SCHEMATIC PHASE BELOW",
          accent,
          10,
        );
      } else {
        text(
          76,
          282,
          `Selected: ${band.label} · ${Math.round(number(parameters.frequencyHz, band.frequencyHz, 20, 20000))} Hz · ${number(parameters.gainDb, band.gainDb, -24, 24)} dB · Q ${number(parameters.q, band.q, 0.1, 20)}`,
          accent,
          12,
        );
        text(
          76,
          307,
          "Choose band + shape below; frequency, gain and Q update that band.",
          "#c8d6e9",
          11,
        );
      }
      detail =
        selectedType === "allPass"
          ? "All pass preserves magnitude while changing phase. The lower phase curve is schematic. Four illustrative filters combine into one response; the app supports up to 16 bands."
          : "Four illustrative filters combine into one response. Select a band, choose among the app's eight shapes, then edit its frequency, gain and Q. The app supports up to 16 bands; this guide keeps four visible.";
    } else if (type === "bassTreble") {
      const bass = number(parameters.bassDb, 0, -12, 12) / 12;
      const treble = number(parameters.trebleDb, 0, -12, 12) / 12;
      text(70, 65, "TWO TONE SHELVES", inputColor, 13);
      line(70, 168, 710, 168, inputColor, "5 5");
      const lowShelf = (t) => 1 / (1 + Math.exp((t - 0.18) * 28));
      const highShelf = (t) => 1 - 1 / (1 + Math.exp((t - 0.76) * 28));
      path(
        Array.from({ length: 161 }, (_, i) => {
          const t = i / 160;
          return [
            70 + t * 640,
            168 - (lowShelf(t) * bass + highShelf(t) * treble) * 82,
          ];
        }),
        outputColor,
        4,
      );
      text(
        86,
        268,
        `BASS · ${number(parameters.bassDb, 0, -12, 12)} dB @ ${number(parameters.bassFrequencyHz, 500, 80, 1000)} Hz`,
        inputColor,
        13,
      );
      text(
        388,
        268,
        `TREBLE · ${number(parameters.trebleDb, 0, -12, 12)} dB @ ${number(parameters.trebleFrequencyHz, 1500, 800, 12000)} Hz`,
        accent,
        13,
      );
      text(
        86,
        300,
        "Broad low + high shelves · not a single peaking band",
        "#c8d6e9",
        12,
      );
      detail =
        "Bass & Treble uses two broad shelves at separate frequencies. The Bass and Treble controls below affect their own ends of the spectrum.";
    } else if (type === "eq" || type === "spectrum") {
      text(70, 65, "LOW", inputColor, 12);
      text(350, 65, "MID", inputColor, 12);
      text(666, 65, "HIGH", inputColor, 12);
      const amount = number(parameters.gainDb, 6, -18, 18) / 18;
      const center =
        parameters.frequencyHz === undefined
          ? number(parameters.center, 0.48, 0.05, 0.95)
          : Math.log10(number(parameters.frequencyHz, 1000, 20, 20000) / 20) /
            3;
      const spread = 0.2 / Math.sqrt(number(parameters.q, 1, 0.2, 12));
      line(70, 168, 710, 168, inputColor, "6 5");
      const filter = parameters.filterType || "peaking";
      const response = (t) => {
        const bell = Math.exp(-Math.pow((t - center) / spread, 2));
        const low = 1 / (1 + Math.exp((t - center) * 24));
        if (filter === "lowShelf") return low * amount;
        if (filter === "highShelf") return (1 - low) * amount;
        if (filter === "lowPass") return -(1 - low) * 0.85;
        if (filter === "highPass") return -low * 0.85;
        if (filter === "bandPass") return -(1 - bell) * 0.85;
        if (filter === "notch") return -bell * 0.85;
        if (filter === "allPass") return 0;
        return bell * amount;
      };
      path(
        Array.from({ length: 161 }, (_, i) => [
          70 + (i / 160) * 640,
          168 - response(i / 160) * 95,
        ]),
        outputColor,
        4,
      );
      text(84, 260, "Flat magnitude", inputColor, 14);
      text(
        470,
        260,
        filter === "allPass"
          ? "Phase changes; level stays flat"
          : "Shaped magnitude",
        outputColor,
        14,
      );
      if (filter === "allPass") {
        path(
          Array.from({ length: 161 }, (_, i) => [
            70 + (i / 160) * 640,
            220 - Math.atan((i / 160 - center) / spread) * 18,
          ]),
          accent,
          2,
        );
        text(84, 285, "Schematic phase response", accent, 12);
      }
      detail =
        filter === "allPass"
          ? "All pass preserves magnitude while changing phase. The lower curve is a schematic phase view."
          : "Low frequencies are on the left, high frequencies on the right. The selected filter changes a different part of the spectrum.";
    } else if (type === "dehum") {
      const frequency = number(parameters.frequencyHz, 60, 45, 65);
      const harmonics = number(parameters.harmonics, 4, 1, 8);
      text(70, 68, `MAINS HUM · ${frequency} Hz FUNDAMENTAL`, inputColor, 13);
      for (let i = 1; i <= harmonics; i++) {
        const x = 70 + (Math.log10((frequency * i) / 20) / 3) * 640;
        line(x, 110, x, 232, accent, "4 3");
        text(x - 12, 250, `${i}×`, accent, 10);
      }
      path(
        Array.from({ length: 161 }, (_, i) => {
          const t = i / 160;
          const notch = Array.from({ length: harmonics }, (_, h) =>
            Math.exp(
              -Math.pow(
                (t - Math.log10((frequency * (h + 1)) / 20) / 3) / 0.012,
                2,
              ),
            ),
          ).reduce((a, b) => a + b, 0);
          return [70 + t * 640, 165 + (1 - Math.min(0.82, notch * 0.82)) * 32];
        }),
        outputColor,
        4,
      );
      text(
        70,
        293,
        `${harmonics} narrow notches · ${number(parameters.amountPercent, 50, 0, 100)}% depth`,
        outputColor,
        13,
      );
      detail =
        "Dehum places narrow cuts at the selected mains frequency and its harmonics. It targets hum tones, not broadband hiss.";
    } else if (type === "declick") {
      text(70, 68, "SHORT IMPULSE DETECTION", inputColor, 13);
      const before = [
        [70, 176],
        [230, 176],
        [244, 174],
        [252, 69],
        [260, 279],
        [270, 176],
        [710, 176],
      ];
      const after = [
        [70, 176],
        [230, 176],
        [244, 174],
        [252, 164],
        [260, 179],
        [270, 176],
        [710, 176],
      ];
      path(before, accent, 3);
      path(after, outputColor, 4);
      text(82, 115, "SPIKE", accent, 11);
      line(245, 94, 260, 94, accent, "3 3");
      text(
        70,
        300,
        `Repair very short runs · threshold ${number(parameters.thresholdPercent, 50, 0, 100)}%`,
        outputColor,
        13,
      );
      detail =
        "Declick detects and repairs very short impulsive runs. The sharp spike is a schematic example; long dropouts are outside its repair window.";
    } else if (type === "denoise") {
      text(70, 68, "LEARN THE BACKGROUND · THEN REDUCE IT", inputColor, 13);
      for (let i = 0; i < 32; i++) {
        const x = 78 + i * 19;
        const level = 28 + 52 * Math.abs(Math.sin(i * 2.17));
        add("rect", {
          x,
          y: 165 - level,
          width: 11,
          height: level,
          rx: 3,
          fill: inputColor,
          opacity: 0.62,
        });
        add("rect", {
          x,
          y:
            165 -
            level * (1 - number(parameters.reductionPercent, 70, 0, 100) / 130),
          width: 11,
          height:
            level * (1 - number(parameters.reductionPercent, 70, 0, 100) / 130),
          rx: 3,
          fill: outputColor,
          opacity: 0.9,
        });
      }
      line(70, 165, 710, 165, "#64748b", "4 4");
      text(75, 245, "PROFILE", accent, 11);
      text(75, 273, "noise", inputColor, 12);
      text(
        532,
        245,
        `${number(parameters.reductionPercent, 70, 0, 100)}% reduction`,
        outputColor,
        12,
      );
      text(532, 273, "floor keeps a little background", "#c8d6e9", 11);
      detail =
        "Denoise learns a noise profile while the background alone is present, then reduces matching spectral content. The bars are not measured audio.";
    } else if (type === "speechDenoise") {
      text(
        70,
        68,
        "VOICE + ROOM NOISE → SPEECH-FOCUSED OUTPUT",
        inputColor,
        13,
      );
      const voice = (t) =>
        Math.sin(t * Math.PI * 25) *
        (0.2 + 0.8 * Math.pow(Math.sin(t * Math.PI * 3), 2));
      const noise = (t) =>
        Math.sin(t * Math.PI * 154) * 0.24 + Math.sin(t * Math.PI * 101) * 0.13;
      wave((t) => voice(t) + noise(t), 160, inputColor);
      wave(
        (t) =>
          voice(t) +
          noise(t) * (1 - number(parameters.strengthPercent, 70, 0, 100) / 115),
        235,
        outputColor,
      );
      text(74, 111, "INPUT", inputColor, 11);
      text(
        470,
        318,
        `OUTPUT · strength ${number(parameters.strengthPercent, 70, 0, 100)}%`,
        outputColor,
        11,
      );
      detail =
        "Speech Denoise is a locally adaptive speech-oriented reducer. This simple synthesized example softens a noise-like component; it does not model the production processor.";
    } else if (type === "spectralGate") {
      const reduction = number(parameters.reductionDb, 40, 0, 80);
      text(70, 68, "SPECTRAL GATE · BINS OPEN OR CLOSE", inputColor, 13);
      line(70, 173, 710, 173, accent, "4 4");
      for (let i = 0; i < 24; i++) {
        const x = 82 + i * 26;
        const above = (Math.sin(i * 1.73) + 1) / 2 > 0.42;
        const h = 26 + 75 * Math.abs(Math.sin(i * 2.1));
        add("rect", {
          x,
          y: 173 - h,
          width: 15,
          height: h,
          rx: 3,
          fill: above ? outputColor : inputColor,
          opacity: above ? 0.86 : Math.max(0.14, 0.8 - reduction / 100),
        });
      }
      text(
        80,
        235,
        `THRESHOLD ${number(parameters.thresholdDb, 3, -20, 20)} dB ABOVE LEARNED NOISE`,
        accent,
        11,
      );
      text(
        80,
        270,
        `${reduction} dB reduction on blocked bins`,
        outputColor,
        12,
      );
      text(80, 302, "Frequency moves left → right", "#c8d6e9", 11);
      detail =
        "Each frequency region is compared with the learned noise profile. The threshold is relative to that profile; reduction controls how far closed bins are attenuated.";
    } else if (type === "mixer") {
      const volumes = [100, 62];
      text(70, 67, "TWO EXPLICIT SOURCES · ONE MIXED OUTPUT", inputColor, 13);
      for (let i = 0; i < 2; i++) {
        const y = 115 + i * 84;
        text(72, y + 6, `INPUT ${i ? "B" : "A"}`, inputColor, 12);
        line(156, y, 414, y, i ? accent : inputColor, "4 3");
        add("rect", {
          x: 270,
          y: y - 10,
          width: volumes[i] * 1.05,
          height: 20,
          rx: 8,
          fill: i ? accent : inputColor,
          opacity: 0.75,
        });
        text(286 + volumes[i], y + 6, `${volumes[i]}%`, "#eaf0fb", 11);
      }
      add("rect", {
        x: 467,
        y: 112,
        width: 112,
        height: 112,
        rx: 14,
        fill: "#18273e",
        stroke: outputColor,
        "stroke-width": 2,
      });
      text(485, 172, "MIXER", outputColor, 16);
      line(579, 168, 700, 168, outputColor, "5 3");
      text(607, 196, "SUM", outputColor, 11);
      text(
        70,
        286,
        "Set a separate 0–100% level for each connected source",
        "#c8d6e9",
        12,
      );
      detail =
        "A Mixer sums explicitly connected inputs. The level is stored per source; use a Volume node for a boost above 100%.";
    } else if (type === "mute") {
      const muted = parameters.muted === true;
      text(70, 68, "ONE BRANCH · EXPLICITLY MUTED", inputColor, 13);
      line(
        106,
        171,
        muted ? 326 : 654,
        171,
        muted ? "#52647f" : outputColor,
        muted ? "7 7" : undefined,
      );
      add("rect", {
        x: 325,
        y: 125,
        width: 108,
        height: 92,
        rx: 16,
        fill: muted ? "#392b21" : "#18362e",
        stroke: muted ? accent : outputColor,
        "stroke-width": 2,
      });
      text(
        346,
        177,
        muted ? "MUTED" : "OPEN",
        muted ? accent : outputColor,
        16,
      );
      if (muted) line(464, 145, 494, 197, accent, 5);
      text(110, 250, "other branches keep their own routes", "#c8d6e9", 12);
      detail =
        "Mute silences this node's branch. Independent connected branches keep their own signal.";
    } else if (type === "inputSwitch") {
      const selected = parameters.selected === "b" ? "B" : "A";
      text(70, 68, "TWO WIRED INPUTS · ONE SELECTED", inputColor, 13);
      line(160, 125, 340, 162, selected === "A" ? outputColor : "#52647f", 4);
      line(160, 221, 340, 182, selected === "B" ? outputColor : "#52647f", 4);
      add("rect", {
        x: 340,
        y: 123,
        width: 128,
        height: 96,
        rx: 16,
        fill: "#18273e",
        stroke: outputColor,
        "stroke-width": 2,
      });
      text(373, 164, `INPUT ${selected}`, outputColor, 15);
      text(
        367,
        190,
        parameters.fade === "slow" ? "2 s fade" : "0.5 s fade",
        accent,
        12,
      );
      line(468, 171, 666, 171, outputColor, 4);
      text(72, 289, "Selecting an unconnected side is silent", "#c8d6e9", 12);
      detail =
        "Input Switch selects one of two explicitly connected sources and crossfades between them. Normal is 0.5 seconds; Slow is 2 seconds.";
    } else if (type === "mixerTwoTrack") {
      const levels = [
        number(parameters.track1Percent, 80, 0, 100),
        number(parameters.track2Percent, 55, 0, 100),
      ];
      const tracks = [
        { y: 111, label: "Input A · synthetic rhythm", color: inputColor },
        { y: 235, label: "Input B · synthetic harmony", color: accent },
      ];
      tracks.forEach((track, index) => {
        add("rect", {
          x: 76,
          y: track.y - 35,
          width: 230,
          height: 82,
          rx: 12,
          fill: "#18273e",
          stroke: track.color,
        });
        text(91, track.y - 8, track.label, track.color, 13);
        add("rect", {
          x: 92,
          y: track.y + 7,
          width: 170,
          height: 9,
          rx: 4,
          fill: "#0d1727",
        });
        add("rect", {
          x: 92,
          y: track.y + 7,
          width: (170 * levels[index]) / 100,
          height: 9,
          rx: 4,
          fill: track.color,
          "data-mixer-level": index ? "b" : "a",
        });
        text(264, track.y + 16, `${Math.round(levels[index])}%`, "#dce7f5", 11);
        line(306, track.y + 3, 398, 174, track.color, 3);
        const token = add("circle", {
          cx: 310,
          cy: track.y + 3,
          r: 5,
          fill: track.color,
        });
        animate(token, [
          { transform: "translate(0px,0px)" },
          { transform: `translate(85px,${171 - (track.y + 3)}px)` },
        ]);
      });
      add("rect", {
        x: 398,
        y: 128,
        width: 154,
        height: 92,
        rx: 14,
        fill: "#203b48",
        stroke: outputColor,
        "stroke-width": 2.5,
      });
      text(422, 166, "MIXER", outputColor, 18);
      text(415, 190, "two inputs summed", "#c8d6e9", 12);
      line(552, 174, 696, 174, outputColor, 4);
      text(584, 154, "Combined output", "#dce7f5", 12);
      detail =
        "The Mixer combines two independent connected inputs. Adjust Input A and Input B below; the two synthetic rhythms change separately, then meet at one output. AudioRouter sets a volume for each connected source.";
    } else if (type === "recording") {
      const phase = recorderPhase;
      const recordActive = phase === "recording";
      const fileActive = phase === "filePlaying";
      const box = (x, label, active) => {
        const shape = add("rect", {
          "data-recorder-stage": x === 92 ? "capture" : "file",
          x,
          y: 116,
          width: 188,
          height: 118,
          rx: 16,
          fill: active ? "#203b48" : "#18273e",
          stroke: active ? outputColor : inputColor,
          "stroke-width": active ? 3 : 1.5,
        });
        const [name, role] = label.split(" · ");
        text(x + 14, 142, name, active ? outputColor : inputColor, 15);
        text(x + 14, 160, role, "#b6c6dd", 11);
        line(x + 18, 192, x + 170, 192, "#53657d");
        const waveform = path(
          Array.from({ length: 45 }, (_, i) => [
            x + 20 + i * 3.35,
            192 - Math.sin(i * 1.7) * (i % 4 === 0 ? 28 : 13),
          ]),
          active ? outputColor : "#697990",
          2,
        );
        if (type === "recording") recorderWaves.push(waveform);
        return shape;
      };
      recorderBoxes = [
        box(92, "Recorder · in-memory take", recordActive),
        box(480, "Audio File · completed take", fileActive),
      ];
      line(
        280,
        175,
        480,
        175,
        phase === "filePlaying" ? outputColor : "#53657d",
        4,
      );
      recorderStatusText = text(
        300,
        158,
        phase === "recording"
          ? "CAPTURING SYNTHETIC LOOP"
          : phase === "filePlaying"
            ? "PLAYING COMPLETED TAKE"
            : phase === "saved"
              ? "TAKE READY"
              : "WAITING TO RECORD",
        accent,
        12,
      );
      detail = recordActive
        ? "A generated example is routed through Recorder while the take grows. Stop it, then play the completed in-memory take from the Audio File stage. No microphone or file is used."
        : fileActive
          ? "The completed synthetic take is now being read from the Audio File stage; the playhead stays inside that part of the route."
          : phase === "saved"
            ? "The synthetic take is complete and ready to play from Audio File. It exists only in this page's memory."
            : "Record starts a generated example through Recorder. Stop it to create an in-memory take, then play that take from Audio File.";
    } else if (
      type === "mixer" ||
      type === "routing" ||
      type === "plugin" ||
      type === "switch"
    ) {
      const labels =
        options.routeLabels ||
        (type === "plugin"
          ? ["Audio in", "Plugin worker", "Audio out"]
          : type === "mixer"
            ? ["Input A + B", "Mixer", "Combined output"]
            : type === "switch"
              ? ["Input A / B", "Crossfade", "Selected output"]
              : ["Source", "Audio route", "Destination"]);
      const centers = [150, 380, 610];
      line(150, 169, 610, 169, outputColor);
      centers.forEach((x, i) => {
        add("rect", {
          x: x - 88,
          y: 126,
          width: 176,
          height: 86,
          rx: 14,
          fill: "#18273e",
          stroke: i === 1 ? outputColor : inputColor,
          "stroke-width": 2,
        });
        text(x - 76, 174, labels[i], i === 1 ? outputColor : inputColor, 14);
      });
      for (const [start, phase] of [
        [238, 0],
        [468, 0.125],
      ]) {
        const packet = add("circle", {
          cx: start,
          cy: 169,
          r: 6,
          fill: accent,
        });
        const frames = [];
        for (let k = 0; k < 4; k++) {
          const from = k / 4 + phase,
            to = from + 0.2;
          if (to > 1) continue;
          frames.push(
            { transform: "translateX(0px)", opacity: 0, offset: from },
            { transform: "translateX(0px)", opacity: 1, offset: from + 0.01 },
            { transform: "translateX(54px)", opacity: 1, offset: to - 0.01 },
            { transform: "translateX(54px)", opacity: 0, offset: to },
          );
        }
        if (frames[0].offset > 0)
          frames.unshift({
            transform: "translateX(0px)",
            opacity: 0,
            offset: 0,
          });
        if (frames[frames.length - 1].offset < 1)
          frames.push({ transform: "translateX(54px)", opacity: 0, offset: 1 });
        animate(packet, frames);
      }
      detail =
        type === "plugin"
          ? "Audio crosses an isolated plugin worker. A failed protected path becomes silent."
          : type === "mixer"
            ? "Independent sources combine into one output. Input levels control their balance."
            : type === "switch"
              ? "A crossfade reduces one input as the other rises; both use explicit connections."
              : "Audio follows explicit connections from a selected source to a selected destination.";
    } else if (type === "network") {
      const receiving = options.kind === "networkReceive";
      const panel = (x, label) => {
        add("rect", {
          x,
          y: 61,
          width: 198,
          height: 184,
          rx: 14,
          fill: "#111e31",
          stroke: "#52647f",
          "stroke-width": 1.5,
        });
        text(x + 13, 84, label, inputColor, 12);
        text(x + 13, 105, "AUDIOROUTER SESSION", "#b6c6dd", 10);
      };
      const node = (x, width, label, stroke = inputColor, fontSize = 12) => {
        const group = add("g", { "aria-label": label });
        group.append(
          svgElement("rect", {
            x,
            y: 145,
            width,
            height: 52,
            rx: 9,
            fill: "#18273e",
            stroke,
            "stroke-width": 2,
          }),
          svgElement(
            "text",
            {
              x: x + width / 2,
              y: 176,
              fill: stroke,
              "font-size": fontSize,
              "font-weight": 600,
              "text-anchor": "middle",
              "font-family": "system-ui,sans-serif",
            },
            label,
          ),
        );
        return group;
      };
      panel(28, "PC A · THIS COMPUTER");
      panel(534, "PC B · ANOTHER COMPUTER");
      add("rect", {
        x: 238,
        y: 91,
        width: 284,
        height: 126,
        rx: 15,
        fill: "#142338",
        stroke: "#546783",
        "stroke-width": 1.5,
        "stroke-dasharray": "7 5",
      });
      text(313, 112, "IP NETWORK", accent, 12);

      node(40, 76, "Source");
      node(132, 84, "Network Send", outputColor, 11);
      node(544, 92, "Network Receive", outputColor, 10);
      node(650, 72, "Output");
      line(116, 171, 132, 171, inputColor, "3 3");
      line(226, 171, 544, 171, "#6f87a5", "7 7");
      line(636, 171, 650, 171, outputColor, "3 3");

      const waiting = add("g", { "data-network-stage": "waiting" });
      waiting.append(
        svgElement("rect", {
          x: 274,
          y: 126,
          width: 212,
          height: 26,
          rx: 13,
          fill: "#382b20",
          stroke: accent,
        }),
        svgElement(
          "text",
          {
            x: 380,
            y: 144,
            fill: accent,
            "font-size": 11,
            "font-weight": 700,
            "text-anchor": "middle",
            "font-family": "system-ui,sans-serif",
          },
          receiving
            ? "WAITING FOR NETWORK SEND"
            : "WAITING FOR NETWORK RECEIVE",
        ),
      );
      const paired = add("g", { "data-network-stage": "paired" });
      paired.append(
        svgElement("rect", {
          x: 274,
          y: 126,
          width: 212,
          height: 26,
          rx: 13,
          fill: "#17372f",
          stroke: outputColor,
        }),
        svgElement(
          "text",
          {
            x: 380,
            y: 144,
            fill: outputColor,
            "font-size": 11,
            "font-weight": 700,
            "text-anchor": "middle",
            "font-family": "system-ui,sans-serif",
          },
          receiving
            ? "PAIRED · BUFFERING INCOMING AUDIO"
            : "PAIRED · UDP AUDIO FLOWS",
        ),
      );
      animate(waiting, [
        { opacity: 1, offset: 0 },
        { opacity: 1, offset: 0.26 },
        { opacity: 0, offset: 0.3 },
        { opacity: 0, offset: 1 },
      ]);
      animate(paired, [
        { opacity: 0, offset: 0 },
        { opacity: 0, offset: 0.26 },
        { opacity: 1, offset: 0.3 },
        { opacity: 1, offset: 1 },
      ]);
      const handshake = add("circle", { cx: 226, cy: 171, r: 6, fill: accent });
      animate(handshake, [
        { transform: "translateX(0px)", opacity: 1, offset: 0 },
        { transform: "translateX(0px)", opacity: 1, offset: 0.12 },
        { transform: "translateX(318px)", opacity: 1, offset: 0.27 },
        { transform: "translateX(318px)", opacity: 0, offset: 0.3 },
        { transform: "translateX(318px)", opacity: 0, offset: 1 },
      ]);
      const receiverPulse = add("circle", {
        cx: 588,
        cy: 171,
        r: 35,
        fill: "none",
        stroke: outputColor,
        "stroke-width": 3,
        opacity: 0,
      });
      animate(receiverPulse, [
        { opacity: 0, offset: 0 },
        { opacity: 0, offset: 0.26 },
        { opacity: 0.8, offset: 0.32 },
        { opacity: 0, offset: 0.4 },
        { opacity: 0.75, offset: 0.62 },
        { opacity: 0, offset: 0.71 },
        { opacity: 0.75, offset: 0.83 },
        { opacity: 0, offset: 0.92 },
        { opacity: 0, offset: 1 },
      ]);
      for (const [start, end] of [
        [0.3, 0.52],
        [0.5, 0.72],
        [0.7, 0.92],
      ]) {
        const packet = add("circle", {
          cx: 226,
          cy: 171,
          r: 5,
          fill: outputColor,
          "data-network-packet": "true",
        });
        animate(packet, [
          { transform: "translateX(0px)", opacity: 0, offset: 0 },
          { transform: "translateX(0px)", opacity: 0, offset: start },
          { transform: "translateX(0px)", opacity: 1, offset: start + 0.02 },
          { transform: "translateX(318px)", opacity: 1, offset: end - 0.02 },
          { transform: "translateX(318px)", opacity: 0, offset: end },
          { transform: "translateX(318px)", opacity: 0, offset: 1 },
        ]);
      }
      text(265, 190, "UDP PORT 47800 · MATCHING PAIRING KEY", "#d8e5f5", 10);
      text(42, 225, "Send to PC B's IP", "#b6c6dd", 10);
      text(548, 225, "Receive from PC A", "#b6c6dd", 10);
      text(
        108,
        271,
        receiving
          ? "1 · WAIT FOR NETWORK SEND ON PC A"
          : "1 · WAIT FOR NETWORK RECEIVE ON PC B",
        accent,
        12,
      );
      text(
        108,
        291,
        receiving
          ? "2 · BUFFER PACKETS BEFORE THEY REACH THE ROUTE"
          : "2 · PACKETS CROSS INTO PC B'S ROUTE",
        outputColor,
        12,
      );
      if (receiving) {
        text(542, 263, "RECEIVE BUFFER", accent, 11);
        for (let i = 0; i < 6; i++)
          add("rect", {
            x: 548 + i * 24,
            y: 273,
            width: 17,
            height: 12,
            rx: 3,
            fill: outputColor,
            opacity: 0.28 + (i % 3) * 0.2,
            "data-network-buffer-slot": i,
          });
        text(
          550,
          303,
          `${number(parameters.bufferMs, 40, 10, 500)} ms · added buffer delay`,
          "#b6c6dd",
          9,
        );
      }
      detail = receiving
        ? "PC B's Network Receive waits for Network Send in a second AudioRouter session on PC A, accepts the configured sender and holds a small amount of incoming audio in its receive buffer before playing it through the route. The small delay is illustrative; the drawing is not a live connection."
        : "PC A's Network Send waits for Network Receive in a second AudioRouter session on PC B. Both sides name the other PC and use the same UDP port and pairing key. After pairing, audio packets cross the IP network into PC B's route. The drawing is illustrative; it does not show a cloud relay or encrypted audio.";
    } else if (type === "channels") {
      const mono = parameters.channelMode !== "stereo";
      text(70, 76, "CAPTURE CHANNELS", inputColor, 14);
      text(
        440,
        76,
        mono ? "CENTRED IN BOTH EARS" : "LEFT / RIGHT KEPT SEPARATE",
        outputColor,
        14,
      );
      const left = (t) => Math.sin(t * Math.PI * 16) * 0.6,
        right = (t) => Math.sin(t * Math.PI * 24) * 0.3;
      const selected =
        parameters.channelMode === "left"
          ? left
          : parameters.channelMode === "right"
            ? right
            : (t) => (left(t) + right(t)) * 0.5;
      for (const [y, fn, label] of [
        [135, left, "L"],
        [225, right, "R"],
      ]) {
        text(45, y, label, inputColor, 14);
        path(
          Array.from({ length: 100 }, (_, i) => [
            70 + (i / 99) * 220,
            y - fn(i / 99) * 42,
          ]),
          inputColor,
        );
        path(
          Array.from({ length: 100 }, (_, i) => [
            440 + (i / 99) * 250,
            y - (mono ? selected(i / 99) : fn(i / 99)) * 42,
          ]),
          outputColor,
        );
      }
      line(310, 135, 420, mono ? 180 : 135, accent);
      line(310, 225, 420, mono ? 180 : 225, accent);
      text(324, 260, parameters.channelMode || "mono", accent, 14);
      detail =
        "Stereo keeps channels distinct. Mono sends the selected or averaged input equally to both ears. Choose the channel that actually carries your microphone.";
    } else if (type === "group") {
      add("rect", {
        x: 80,
        y: 84,
        width: 600,
        height: 175,
        rx: 16,
        fill: "#18273e",
        stroke: accent,
        "stroke-dasharray": "8 6",
      });
      text(104, 115, "Voice processing · visual group", accent, 16);
      line(190, 195, 570, 195, outputColor);
      for (const [x, label] of [
        [190, "Gain"],
        [380, "EQ"],
        [570, "Compressor"],
      ]) {
        add("rect", {
          x: x - 68,
          y: 152,
          width: 136,
          height: 80,
          rx: 10,
          fill: "#101a2b",
          stroke: inputColor,
        });
        text(x - 53, 198, label, outputColor, 14);
      }
      detail =
        "The background groups related tools visually. Audio flows through their connections; the group itself never processes it.";
    } else if (type === "media") {
      text(70, 80, "FILE START", inputColor, 14);
      text(565, 80, "FILE END", outputColor, 14);
      traces.push({
        element: wave(signal, 165, inputColor),
        y: 165,
        signal: "input",
      });
      const playhead = add("line", {
        x1: 70,
        x2: 70,
        y1: 105,
        y2: 220,
        stroke: accent,
        "stroke-width": 4,
      });
      animate(playhead, [
        { transform: "translateX(0px)" },
        { transform: "translateX(640px)" },
      ]);
      text(70, 260, "Play · Pause · Stop · Seek", outputColor, 15);
      detail =
        "The playhead reads a chosen local file. Pause keeps its position; Stop returns to the start. Seeking is deliberate.";
    } else if (type === "tone") {
      const cycles = number(parameters.frequencyHz, 440, 20, 20000) / 200;
      const amplitude = 10 ** (number(parameters.levelDb, -12, -60, 0) / 20);
      const tone = (t) => Math.sin(t * Math.PI * 2 * cycles) * amplitude;
      wave(tone, 168, outputColor);
      text(70, 80, "SYNTHETIC TEST SOURCE", inputColor, 14);
      text(
        70,
        260,
        number(parameters.frequencyHz, 440, 20, 20000) +
          " Hz · " +
          number(parameters.levelDb, -12, -60, 0) +
          " dB",
        accent,
        15,
      );
      detail =
        "Higher frequency packs more cycles into the same time. Amplitude controls the signal level; start quietly. The sample trace below follows the generated sine.";
    } else if (type === "buffer") {
      text(70, 80, "PAST", inputColor, 14);
      text(648, 80, "LIVE", outputColor, 14);
      add("rect", {
        x: 70,
        y: 120,
        width: 640,
        height: 80,
        rx: 12,
        fill: "#18273e",
        stroke: inputColor,
      });
      for (let i = 0; i < 24; i++)
        add("rect", {
          x: 80 + i * 26,
          y: 140,
          width: 12,
          height: 40,
          rx: 3,
          fill: inputColor,
          opacity: 0.35 + 0.5 * Math.abs(Math.sin(i)),
        });
      const cursor = add("line", {
        x1: 700,
        x2: 700,
        y1: 104,
        y2: 216,
        stroke: accent,
        "stroke-width": 4,
      });
      animate(cursor, [
        { transform: "translateX(0px)", offset: 0 },
        { transform: "translateX(0px)", offset: 0.25 },
        { transform: "translateX(-380px)", offset: 0.2501 },
        { transform: "translateX(-200px)", offset: 0.75 },
        { transform: "translateX(0px)", offset: 0.7501 },
        { transform: "translateX(0px)", offset: 1 },
      ]);
      text(72, 254, "Read from history", accent, 15);
      text(490, 254, "Return to the live edge", outputColor, 15);
      detail =
        "The pointer stays on the live edge, jumps back into history at 2 s, reads the past until 6 s, then returns to the live edge, as in the example you hear.";
    } else {
      let output = signal;
      const threshold =
        10 **
        (number(
          parameters[type === "limiter" ? "ceilingDb" : "thresholdDb"],
          type === "limiter" ? -1 : -8,
          type === "limiter" ? -12 : -48,
          0,
        ) /
          20);
      if (type === "gain") {
        const gain = 10 ** (number(parameters.gainDb, 0, -18, 12) / 20);
        output = (t) => clamp(signal(t) * gain, -1, 1);
        detail = `Example: ${number(parameters.gainDb, 0, -18, 12)} dB changes amplitude; timing stays the same.`;
      }
      if (type === "volume") {
        const percent = number(parameters.percent, 100, 0, 200) / 100;
        output = (t) => clamp(signal(t) * percent, -1, 1);
        detail = `Volume at ${Math.round(percent * 100)}% changes this source's level; 100% is unity.`;
      }
      if (type === "dynamics") {
        const ratio = number(parameters.ratio, 4, 1, 20);
        output = (t) => {
          const v = signal(t);
          return (
            Math.sign(v) *
            (Math.abs(v) > threshold
              ? threshold * (Math.abs(v) / threshold) ** (1 / ratio)
              : Math.abs(v))
          );
        };
        detail =
          "Loud passages above the threshold are turned down by the ratio; quiet passages keep their level. Compare the IN and OUT heights from 4 s.";
      }
      if (type === "limiter") {
        output = (t) => clamp(signal(t), -threshold, threshold);
        detail =
          "The output never crosses the ceiling, even in the loud passage from 4 s; quieter parts pass unchanged.";
      }
      if (type === "gate") {
        const gateSignal = (t) => signal(t) * (t > 0.3 && t < 0.6 ? 0.014 : 1);
        output = (t) =>
          Math.abs(gateSignal(t)) < threshold
            ? gateSignal(t) * 0.08
            : gateSignal(t);
        pair(output, gateSignal);
        detail =
          "Between phrases the quiet background stays below the threshold and is turned down; the voice passes through.";
      }
      if (type === "duck") {
        const gain = 10 ** (-number(parameters.amountDb, 6, 0, 40) / 20);
        output = (t) => signal(t) * (t > 0.3 && t < 0.72 ? gain : 1);
        detail =
          "The target lowers while the separate control signal is active, then recovers.";
        controlBand = add("path", {
          d: "M262,164 L531,164",
          stroke: accent,
          "stroke-width": 4,
          "stroke-linecap": "round",
          fill: "none",
        });
        text(270, 183, "Ducking active", accent, 13);
      }
      if (type === "delay") {
        const shift =
          parameters.delayMs === undefined
            ? number(parameters.shift, 0.14, 0.01, 0.75)
            : number(parameters.delayMs, 140, 0, 1000) / 1000;
        output = (t) => (t < shift ? 0 : signal(t - shift));
        detail =
          "The same sound arrives later. Across these eight seconds a short delay is a small shift to the right; listen for the later start.";
      }
      if (type === "pitch") {
        const multiplier =
          2 **
          (number(
            parameters.semitones ?? parameters.pitchSemitones,
            7,
            -24,
            24,
          ) /
            12);
        output = (t) => Math.sin(t * Math.PI * 32 * multiplier) * envelope(t);
        detail =
          "More waveform cycles in the same interval means higher pitch. The example is schematic.";
      }
      if (type === "cleanup") {
        pair(signal, (t) =>
          clamp(signal(t) + 0.12 * Math.sin(t * Math.PI * 178), -1, 1),
        );
        detail =
          "Noise or unwanted transients are reduced. Stronger cleanup can also affect wanted detail.";
      } else if (type !== "gate") pair(output);
      // The traces show linear amplitude, so only the limiter ceiling (near
      // full scale) has a readable height; it bounds the output on both sides.
      if (type === "limiter") {
        const reach = Math.min(1.1, threshold) * 48;
        line(70, 232 - reach, 710, 232 - reach, accent, "6 5");
        line(70, 232 + reach, 710, 232 + reach, accent, "6 5");
        text(
          560,
          232 - reach - 6,
          `Ceiling ${number(parameters.ceilingDb, -1, -12, 0)} dB`,
          accent,
          12,
        );
      }
    }
    const cursorStart =
      type === "recording" && recorderPhase === "filePlaying"
        ? 504
        : type === "recording"
          ? 114
          : 70;
    const cursorSpan = type === "recording" ? 140 : 640;
    const hasPlayhead =
      type === "recording" || (timelineTypes.has(type) && type !== "media");
    const cursor = add("line", {
      "data-guide-playhead": "true",
      x1: cursorStart,
      x2: cursorStart,
      y1: 54,
      y2: 280,
      stroke: "#eaf0fb",
      "stroke-width": 1.5,
      opacity: 0.65,
    });
    if (hasPlayhead)
      cursorAnimation = animate(cursor, [
        { transform: "translateX(0px)" },
        { transform: `translateX(${cursorSpan}px)` },
      ]);
    else cursor.remove();
    const footnote = text(
      70,
      333,
      "Illustration • not an audio measurement",
      "#b6c6dd",
      13,
    );
    // Replace drawn traces with the example's own envelope once it is rendered.
    const envelopePath = (samples, y) => {
      const columns = 320,
        size = samples.length / columns,
        top = [],
        bottom = [];
      for (let c = 0; c < columns; c++) {
        let peak = 0;
        for (let i = Math.floor(c * size); i < Math.floor((c + 1) * size); i++)
          peak = Math.max(peak, Math.abs(samples[i]));
        const h = Math.max(0.6, Math.min(1.1, peak) * 48),
          x = 70 + (c / (columns - 1)) * 640;
        top.push(`${x.toFixed(1)},${(y - h).toFixed(1)}`);
        bottom.unshift(`${x.toFixed(1)},${(y + h).toFixed(1)}`);
      }
      return `M${top.join(" L")} L${bottom.join(" L")} Z`;
    };
    const drawFromAudio = (snapshot) => {
      if (!snapshot || destroyed) return;
      for (const trace of traces) {
        const samples =
          trace.signal === "input"
            ? snapshot.inputs[0].channels[0]
            : snapshot.output.channels[0];
        const color = trace.element.getAttribute("stroke");
        trace.element.setAttribute("d", envelopePath(samples, trace.y));
        trace.element.setAttribute("fill", color);
        trace.element.setAttribute("fill-opacity", "0.35");
        trace.element.setAttribute("stroke-width", "1");
      }
      if (controlBand) {
        // Mark where the Duck output is clearly below its music input.
        const input = snapshot.inputs[0].channels[0],
          output = snapshot.output.channels[0],
          columns = 160,
          size = input.length / columns,
          segments = [];
        let start = -1;
        for (let c = 0; c <= columns; c++) {
          let a = 0,
            b = 0;
          for (
            let i = Math.floor(c * size);
            i < Math.floor((c + 1) * size) && i < input.length;
            i++
          ) {
            a += Math.abs(input[i]);
            b += Math.abs(output[i]);
          }
          const ducked = c < columns && a > 0 && b / a < 0.85;
          if (ducked && start < 0) start = c;
          if (!ducked && start >= 0) {
            segments.push([start, c]);
            start = -1;
          }
        }
        controlBand.setAttribute(
          "d",
          segments
            .map(
              ([from, to]) =>
                `M${(70 + (from / columns) * 640).toFixed(1)},164 L${(70 + (to / columns) * 640).toFixed(1)},164`,
            )
            .join(" ") || "M0,0",
        );
      }
      if (traces.length)
        footnote.textContent =
          "Traces drawn from the example audio · the line marks what you hear now";
    };

    root.append(svg);
    const caption = document.createElement("figcaption");
    caption.textContent = options.visualCaption || detail;
    caption.style.cssText =
      "padding:0 22px 16px;font:400 15px/1.6 system-ui,sans-serif;color:inherit";
    root.append(caption);
    const controls = document.createElement("div");
    controls.style.cssText =
      "display:flex;align-items:center;flex-wrap:wrap;gap:10px;padding:0 22px 20px";
    controls.setAttribute("role", "group");
    controls.setAttribute(
      "aria-label",
      `${options.title || "Signal example"} animation controls`,
    );
    const button = (label) => {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = label;
      b.style.cssText =
        "font:600 14px system-ui,sans-serif;padding:9px 14px;color:inherit;background:transparent;border:1px solid currentColor;border-radius:8px;cursor:pointer";
      controls.append(b);
      return b;
    };
    const recorder = options.type === "recording";
    const toggle = recorder
      ? null
      : button(options.audioRelevant ? "Play example" : "Play animation");
    if (toggle) {
      toggle.style.minWidth = options.audioRelevant ? "136px" : "152px";
      toggle.style.textAlign = "center";
    }
    const replayButton = null;
    const recordButton = recorder ? button("Record example") : null;
    const stopRecordButton = recorder ? button("Stop recording") : null;
    const filePlayButton = recorder ? button("Play recording") : null;
    if (filePlayButton) {
      filePlayButton.style.minWidth = "138px";
      filePlayButton.style.textAlign = "center";
    }
    const seekLabel = document.createElement("label");
    seekLabel.style.cssText =
      "display:flex;gap:8px;align-items:center;font:400 13px system-ui,sans-serif";
    seekLabel.append(document.createTextNode("Seek example"));
    const seeker = document.createElement("input");
    seeker.type = "range";
    seeker.min = "0";
    seeker.max = "8";
    seeker.step = "0.1";
    seeker.value = "0";
    seeker.style.cssText = "width:100px;accent-color:#7de3c2";
    seekLabel.append(seeker);
    controls.append(seekLabel);
    const status = document.createElement("span");
    status.style.cssText =
      "font:400 13px system-ui,sans-serif;width:190px;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap";
    status.setAttribute("aria-live", "polite");
    controls.append(status);
    root.append(controls);
    host.append(root);
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    let playing = false,
      destroyed = false,
      transportToken = 0;
    const unsubscribe =
      traces.length && timelineTypes.has(type) && options.transport
        ? options.transport.subscribe(drawFromAudio)
        : null;
    // Position inside the example; looping animations keep counting past 8 s.
    const visualTime = () =>
      animations.length
        ? Number(animations[0].currentTime || 0) % timelineDuration
        : (options.transport?.getTime?.() || 0) * 1000;
    const setRecorderPhase = (phase) => {
      recorderPhase = phase;
      if (!recorderBoxes.length) return;
      const recordingNow = phase === "recording";
      const fileNow = phase === "filePlaying";
      if (recordingNow) recorderFileView = false;
      if (fileNow) recorderFileView = true;
      if (recordingNow || fileNow) {
        timelineDuration = fileNow
          ? (options.transport?.getRecorderDuration?.() || 8) * 1000
          : duration;
        animations.forEach((a) =>
          a.effect?.updateTiming({ duration: timelineDuration }),
        );
        seeker.max = String(timelineDuration / 1000);
      }
      recorderBoxes.forEach((box, index) => {
        const active = index === 0 ? recordingNow : fileNow;
        box.setAttribute("fill", active ? "#203b48" : "#18273e");
        box.setAttribute("stroke", active ? outputColor : inputColor);
        box.setAttribute("stroke-width", active ? "3" : "1.5");
      });
      recorderWaves.forEach((waveform, index) => {
        const active =
          phase === "recording"
            ? index === 0
            : phase === "saved" || phase === "filePlaying"
              ? index === 1
              : false;
        waveform.setAttribute("stroke", active ? outputColor : "#697990");
        waveform.setAttribute(
          "opacity",
          active ? "1" : phase === "ready" ? "0.28" : "0.55",
        );
      });
      if (recorderStatusText)
        recorderStatusText.textContent = recordingNow
          ? "CAPTURING SYNTHETIC LOOP"
          : fileNow
            ? "PLAYING COMPLETED TAKE"
            : phase === "saved"
              ? "TAKE READY"
              : "WAITING TO RECORD";
      const start = recorderFileView ? 504 : 114;
      cursor.setAttribute("x1", String(start));
      cursor.setAttribute("x2", String(start));
      cursorAnimation?.effect?.setKeyframes([
        { transform: "translateX(0px)" },
        { transform: `translateX(${cursorSpan}px)` },
      ]);
      if (phase === "recording" || phase === "filePlaying") {
        if (cursorAnimation) cursorAnimation.currentTime = 0;
        seeker.value = "0";
      }
    };
    const update = (message) => {
      if (toggle) {
        toggle.textContent = playing
          ? options.audioRelevant
            ? "Stop example"
            : "Stop animation"
          : options.audioRelevant
            ? "Play example"
            : "Play animation";
        toggle.setAttribute("aria-pressed", String(playing));
      }
      if (recorder && recordButton && stopRecordButton && filePlayButton) {
        recordButton.disabled = recorderPhase === "recording";
        stopRecordButton.disabled = recorderPhase !== "recording";
        filePlayButton.disabled =
          recorderPhase !== "saved" && recorderPhase !== "filePlaying";
        filePlayButton.textContent =
          recorderPhase === "filePlaying" && playing
            ? "Pause recording"
            : "Play recording";
      }
      status.textContent =
        message ||
        (playing
          ? recorder
            ? "Playing once"
            : "Playing · repeats"
          : "Paused");
      status.title = status.textContent;
    };
    function pause(notifyAudio = true) {
      if (destroyed) return;
      transportToken++;
      const time = Number(animations[0]?.currentTime || 0) / 1000;
      animations.forEach((a) => a.pause());
      playing = false;
      if (notifyAudio) options.transport?.pause?.(time);
      update();
    }
    /** Stop the example and return to its start. */
    function stop() {
      if (destroyed) return;
      transportToken++;
      animations.forEach((a) => {
        a.pause();
        a.currentTime = 0;
      });
      seeker.value = "0";
      playing = false;
      options.transport?.stop?.();
      update("Stopped");
    }
    /** Show a rebuilt picture at the running position without touching audio. */
    function restore(ms, wasPlaying) {
      if (destroyed) return;
      animations.forEach((a) => {
        a.currentTime = ms;
        if (wasPlaying) a.play();
      });
      seeker.value = String(ms / 1000);
      playing = wasPlaying;
      update();
    }
    function seek(seconds, resume = playing) {
      const token = ++transportToken;
      animations.forEach((a) => a.pause());
      playing = false;
      const ms = number(seconds, 0, 0, timelineDuration / 1000) * 1000;
      animations.forEach((a) => (a.currentTime = ms));
      seeker.value = String(ms / 1000);
      const seekTask = options.transport?.seek?.(ms / 1000, resume);
      const resumeVisual = () => {
        if (
          token !== transportToken ||
          destroyed ||
          !resume ||
          ms >= timelineDuration
        )
          return;
        animations.forEach((a) => a.play());
        playing = true;
        update();
      };
      if (resume && seekTask?.then)
        seekTask
          .then(resumeVisual)
          .catch(() => update("Audio could not resume"));
      else resumeVisual();
      update(
        ms === timelineDuration && recorder
          ? "Finished"
          : resume
            ? undefined
            : "Paused",
      );
    }
    function play() {
      if (destroyed || (!animations.length && !options.transport)) return;
      if (
        recorder &&
        animations.length &&
        Number(animations[0].currentTime) >= timelineDuration
      )
        seek(0, false);
      const offset = visualTime() / 1000;
      const token = ++transportToken;
      const beginVisual = () => {
        if (token !== transportToken || destroyed) return;
        animations.forEach((a) => a.play());
        playing = true;
        update();
      };
      const startTask = !recorder ? options.transport?.play?.(offset) : null;
      if (startTask?.then) {
        update("Starting audio example…");
        startTask
          .then(beginVisual)
          .catch(() => update("Audio could not start"));
      } else beginVisual();
    }
    function replay() {
      seek(0, false);
      play();
    }
    toggle?.addEventListener("click", () => (playing ? stop() : play()));
    replayButton?.addEventListener("click", replay);
    seeker.addEventListener("input", () => seek(seeker.value, playing));
    recordButton?.addEventListener("click", () => {
      setRecorderPhase("recording");
      animations.forEach((a) => {
        a.currentTime = 0;
      });
      const token = ++transportToken;
      const start = options.transport?.record?.();
      update("Preparing synthetic recording…");
      Promise.resolve(start)
        .then(() => {
          if (token !== transportToken || destroyed) return;
          animations.forEach((a) => a.play());
          playing = true;
          update("Recording synthetic audio through Recorder");
        })
        .catch(() => update("Recording example could not start"));
    });
    stopRecordButton?.addEventListener("click", () => {
      pause(false);
      setRecorderPhase("saved");
      options.transport?.stopRecording?.();
      update("Take ready · play it from Audio File");
    });
    filePlayButton?.addEventListener("click", () => {
      if (recorderPhase === "filePlaying" && playing) {
        pause();
        setRecorderPhase("saved");
        update("Paused · completed take remains ready");
        return;
      }
      setRecorderPhase("filePlaying");
      animations.forEach((a) => {
        a.currentTime = 0;
      });
      const token = ++transportToken;
      const start = options.transport?.playRecording?.(0);
      update("Opening the completed take…");
      Promise.resolve(start)
        .then(() => {
          if (token !== transportToken || destroyed) return;
          animations.forEach((a) => a.play());
          playing = true;
          update("Playing the completed take from Audio File");
        })
        .catch(() => update("The completed example could not play"));
    });
    // This callback changes controls only. Every visual frame is defined by finite WAAPI keyframes.
    if (animations.length)
      animations[animations.length - 1].onfinish = () => {
        playing = false;
        seeker.value = String(timelineDuration / 1000);
        options.transport?.pause?.(timelineDuration / 1000);
        if (recorder && recorderPhase === "recording") {
          options.transport?.stopRecording?.();
          setRecorderPhase("saved");
          update("Recording complete · play it from Audio File");
          return;
        }
        if (recorder && recorderPhase === "filePlaying")
          setRecorderPhase("saved");
        update("Finished");
      };
    const visibility = () => {
      if (document.hidden) pause();
    };
    document.addEventListener("visibilitychange", visibility);
    const motionChange = () => {
      if (reduced.matches) {
        seek(0);
        update("Reduced motion");
      }
    };
    reduced.addEventListener("change", motionChange);
    const observer =
      typeof IntersectionObserver === "function"
        ? new IntersectionObserver((entries) => {
            if (!entries[0].isIntersecting && playing) pause();
          })
        : null;
    // Controls and sample references are part of the same example. Scrolling
    // to a fader must not pause the transport while that example is in view.
    observer?.observe(host.closest("#see-it") || root);
    const api = {
      play,
      pause,
      replay,
      seek,
      setRecorderPhase,
      stop,
      restore,
      sync(seconds) {
        if (!playing) return;
        const ms = Math.min(timelineDuration, seconds * 1000);
        animations.forEach((a) => {
          a.currentTime = ms;
        });
        seeker.value = String(ms / 1000);
      },
      currentTime: visualTime,
      isPlaying: () => playing,
      destroy() {
        if (destroyed) return;
        destroyed = true;
        unsubscribe?.();
        animations.forEach((a) => a.cancel());
        observer?.disconnect();
        document.removeEventListener("visibilitychange", visibility);
        reduced.removeEventListener("change", motionChange);
        root.remove();
        if (host.__guideAnimation === api) delete host.__guideAnimation;
      },
    };
    host.__guideAnimation = api;
    if (!animations.length && !options.audioRelevant) {
      // A purely visual tool (for example Group) has nothing to play.
      controls.remove();
      update("Static diagram");
    } else
      update(
        reduced.matches
          ? "Reduced motion"
          : recorder
            ? "Ready · 8 seconds"
            : "Ready · 8 second loop",
      );
    return api;
  }
  window.AudioRouterGuideAnimation = Object.freeze({
    mount(host, options = {}) {
      let settings = { ...options, parameters: { ...options.parameters } };
      let current = mount(host, settings),
        disposed = false;
      return {
        play: () => current.play(),
        pause: () => current.pause(),
        stop: () => current.stop(),
        replay: () => current.replay(),
        seek: (seconds) => current.seek(seconds),
        setRecorderPhase(phase) {
          if (!disposed) current.setRecorderPhase(phase);
        },
        setParameters(values) {
          if (disposed) return;
          settings = {
            ...settings,
            parameters: { ...settings.parameters, ...values },
          };
          const time = current.currentTime();
          const wasPlaying = current.isPlaying();
          current = mount(host, settings);
          current.restore(time, wasPlaying);
        },
        destroy() {
          disposed = true;
          current.destroy();
        },
      };
    },
  });
})();
