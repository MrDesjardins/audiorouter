/* Deterministic lesson fixtures. Browser examples are independent of the AudioRouter engine. */
(() => {
  "use strict";
  const rate = 48000,
    seconds = 8,
    frames = rate * seconds;
  const gain = (db) => 10 ** (db / 20);
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  const value = (p, k, fallback) => Number(p[k] ?? fallback);
  const channel = () => new Float32Array(frames);
  const copy = (samples) => Float32Array.from(samples);
  const envelope = (t, start, end, attack = 0.04, release = 0.12) =>
    t < start || t >= end
      ? 0
      : Math.min(1, (t - start) / attack, (end - t) / release);
  const notes = [523.25, 659.25, 783.99, 987.77, 880, 659.25, 587.33, 783.99];
  function fixture(kind, p) {
    const a = channel(),
      b = channel(),
      voice = channel(),
      noise = channel(),
      clicks = channel();
    let seed = 0x1459a31,
      previousNoise = 0;
    for (let i = 0; i < frames; i++) {
      const t = i / rate,
        beat = t % 0.5,
        noteTime = (t + 0.125) % 0.5;
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      const random = seed / 2147483648 - 1;
      const bassHz = [82.41, 110, 123.47, 98][Math.floor(t / 2) % 4];
      const kick =
        Math.sin(2 * Math.PI * (44 * beat + 2.5 * (1 - Math.exp(-beat * 28)))) *
        Math.exp(-beat * 28);
      const bass =
        Math.sin(2 * Math.PI * bassHz * t) *
        envelope(beat, 0, 0.43, 0.015, 0.08);
      const hat = (random - previousNoise) * Math.exp(-((t + 0.25) % 0.5) * 65);
      a[i] = 0.33 * kick + 0.2 * bass + 0.045 * hat;
      const hz =
        notes[Math.floor((t + 0.125) / 0.5) % notes.length] *
        (kind === "pitch"
          ? 2 ** ((value(p, "semitones", 0) + value(p, "cents", 0) / 100) / 12)
          : 1);
      const bell =
        Math.sin(2 * Math.PI * hz * noteTime) +
        0.34 * Math.sin(2 * Math.PI * hz * 2.76 * noteTime);
      b[i] =
        0.29 *
        bell *
        envelope(noteTime, 0, 0.34, 0.008, 0.1) *
        Math.exp(-noteTime * 5);
      const syllable = t % 0.38;
      const phrase = envelope(t, 1.1, 3.3) + envelope(t, 4.6, 7.3);
      voice[i] =
        phrase *
        envelope(syllable, 0, 0.29, 0.025, 0.07) *
        (0.23 * Math.sin(2 * Math.PI * 145 * t) +
          0.11 * Math.sin(2 * Math.PI * 725 * t) +
          0.07 * Math.sin(2 * Math.PI * 2030 * t));
      noise[i] = random * 0.065;
      previousNoise = random;
    }
    const clean = channel(),
      input = channel();
    for (let i = 0; i < frames; i++) {
      const t = i / rate;
      clean[i] = a[i] * 0.75 + b[i] * 0.8;
      if (["bassTreble", "graphicEq", "parametricEq"].includes(kind))
        clean[i] +=
          0.08 * Math.sin(2 * Math.PI * 31.5 * t) +
          0.06 * Math.sin(2 * Math.PI * 250 * t) +
          noise[i] * 0.65;
      if (["compressor", "limiter", "meter"].includes(kind)) {
        const level = [0.16, 0.44, 1.38, 0.28][Math.floor(t / 2)];
        clean[i] =
          level *
          Math.sin(2 * Math.PI * 220 * t) *
          (0.65 + 0.35 * Math.sin(2 * Math.PI * 3 * t) ** 2);
      }
      if (
        [
          "dehum",
          "declick",
          "denoise",
          "speechDenoise",
          "spectralGate",
          "gate",
        ].includes(kind)
      )
        clean[i] = voice[i];
      if (kind === "pitch") clean[i] = b[i];
      if (kind === "testSignal")
        clean[i] =
          t < value(p, "durationMs", 8000) / 1000
            ? gain(value(p, "levelDb", -18)) *
              Math.sin(2 * Math.PI * value(p, "frequencyHz", 440) * t)
            : 0;
      input[i] = clean[i];
      if (kind === "gate") input[i] += noise[i] * 0.04;
      if (["denoise", "speechDenoise", "spectralGate"].includes(kind))
        input[i] += noise[i];
      if (kind === "dehum")
        for (let harmonic = 1; harmonic <= 8; harmonic++)
          input[i] +=
            (0.09 / harmonic) * Math.sin(2 * Math.PI * 60 * harmonic * t);
    }
    if (kind === "declick") {
      for (const [time, amplitude] of [
        [0.7, 0.9],
        [2.15, 0.35],
        [3.85, 1.2],
        [5.2, 0.65],
        [6.8, 0.4],
      ]) {
        const at = Math.floor(time * rate);
        clicks[at] = amplitude;
        clicks[at + 1] = -amplitude * 0.75;
        clicks[at + 2] = amplitude * 0.35;
        for (let j = 0; j < 3; j++) input[at + j] += clicks[at + j];
      }
    }
    if (kind === "mixer" || kind === "inputSwitch")
      return {
        inputs: [
          { label: "Input A · drums & bass", channels: [a] },
          { label: "Input B · bell melody", channels: [b] },
        ],
        clean,
        noise,
        clicks,
      };
    if (kind === "duck")
      return {
        inputs: [
          { label: "Music input · drums & bass", channels: [a] },
          {
            label: "Trigger reference · voice-like phrases",
            channels: [voice],
            controlOnly: true,
          },
        ],
        clean,
        noise,
        clicks,
      };
    return {
      inputs: [
        {
          label:
            kind === "testSignal"
              ? "Generated sine source"
              : kind === "audioFile"
                ? "Example file · beat & melody"
                : kind === "networkReceive" || kind === "networkSend"
                  ? "PC A · source signal"
                  : "Input · " +
                    ([
                      "dehum",
                      "declick",
                      "denoise",
                      "speechDenoise",
                      "spectralGate",
                      "gate",
                    ].includes(kind)
                      ? "voice-like phrase with background"
                      : "lesson source"),
          channels:
            kind === "meter"
              ? [input, Float32Array.from(input, (v) => v * 0.76)]
              : [input],
        },
      ],
      clean,
      noise,
      clicks,
    };
  }
  function dynamics(input, kind, p) {
    const result = channel(),
      threshold = value(p, "thresholdDb", kind === "gate" ? -45 : -18);
    const ratio = value(p, "ratio", kind === "gate" ? 4 : 3),
      range = value(p, "rangeDb", 60);
    const attack = Math.exp(
      -1 / ((rate * value(p, "attackMs", kind === "gate" ? 5 : 10)) / 1000),
    );
    const release = Math.exp(-1 / ((rate * value(p, "releaseMs", 150)) / 1000));
    const detector = Math.exp(-1 / (rate * 0.008));
    let level = 0,
      currentGain = 1,
      hold = 0,
      opened = false;
    for (let i = 0; i < frames; i++) {
      level = Math.max(Math.abs(input[i]), level * detector);
      const db = 20 * Math.log10(Math.max(1e-9, level));
      let targetDb = 0;
      if (kind === "compressor") {
        const knee = value(p, "kneeDb", 6),
          over = db - threshold;
        if (over > knee / 2) targetDb = -(1 - 1 / ratio) * over;
        else if (knee > 0 && over > -knee / 2)
          targetDb = (-(1 - 1 / ratio) * (over + knee / 2) ** 2) / (2 * knee);
      } else {
        if (db >= threshold) {
          opened = true;
          hold = (value(p, "holdMs", 50) * rate) / 1000;
        } else if (db < threshold - value(p, "hysteresisDb", 3)) {
          if (hold-- <= 0) opened = false;
        }
        if (!opened)
          targetDb = -Math.min(
            range,
            Math.max(0, (threshold - db) * (ratio - 1)),
          );
      }
      const target = gain(targetDb),
        coefficient =
          kind === "gate"
            ? target > currentGain
              ? attack
              : release
            : target < currentGain
              ? attack
              : release;
      currentGain = target + coefficient * (currentGain - target);
      result[i] =
        input[i] *
        currentGain *
        (kind === "compressor" ? gain(value(p, "makeupDb", 0)) : 1);
    }
    return result;
  }
  async function filtered(inputChannels, kind, p, bands) {
    const context = new OfflineAudioContext(inputChannels.length, frames, rate);
    // Fill the buffer before giving it to the source: Firefox takes the
    // buffer's contents when it is assigned, so writing afterwards rendered
    // silence there (Chrome and Edge tolerated it).
    const input = context.createBuffer(inputChannels.length, frames, rate);
    inputChannels.forEach((samples, i) => input.copyToChannel(samples, i));
    const source = context.createBufferSource();
    source.buffer = input;
    let stage = source;
    const add = (type, hz, db, q = 1) => {
      const f = context.createBiquadFilter();
      f.type = type;
      f.frequency.value = clamp(hz, 20, 20000);
      f.gain.value = db;
      f.Q.value = q;
      stage.connect(f);
      stage = f;
    };
    if (kind === "bassTreble") {
      add(
        "lowshelf",
        value(p, "bassFrequencyHz", 500),
        value(p, "bassDb", 0),
        0.707,
      );
      add(
        "highshelf",
        value(p, "trebleFrequencyHz", 1500),
        value(p, "trebleDb", 0),
        0.707,
      );
    } else if (kind === "graphicEq") {
      [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000].forEach(
        (hz, i) => add("peaking", hz, value(p, "band" + i + "Db", 0)),
      );
    } else if (kind === "parametricEq") {
      const types = {
        lowShelf: "lowshelf",
        highShelf: "highshelf",
        lowPass: "lowpass",
        highPass: "highpass",
        bandPass: "bandpass",
        allPass: "allpass",
      };
      bands.forEach((band, i) => {
        if (p["band" + i + "Enabled"] === false) return;
        const type = p["band" + i + "Type"] || band.filterType;
        add(
          types[type] || type,
          value(p, "band" + i + "FrequencyHz", band.frequencyHz),
          value(p, "band" + i + "GainDb", band.gainDb),
          value(p, "band" + i + "Q", band.q),
        );
      });
    } else if (kind === "dehum") {
      for (let h = 1; h <= value(p, "harmonics", 4); h++)
        add(
          "peaking",
          value(p, "frequencyHz", 60) * h,
          (-36 * value(p, "amountPercent", 50)) / 100,
          18,
        );
    } else if (kind === "firFilter") {
      const ir = context.createBuffer(1, Math.floor(rate * 0.3), rate),
        impulse = ir.getChannelData(0);
      let energy = 0;
      for (const [time, amplitude] of [
        [0, 1],
        [0.025, 0.58],
        [0.061, -0.36],
        [0.115, 0.24],
        [0.19, 0.16],
        [0.26, 0.08],
      ]) {
        impulse[Math.floor(time * rate)] = amplitude;
        energy += amplitude ** 2;
      }
      for (let i = 0; i < impulse.length; i++) impulse[i] /= Math.sqrt(energy);
      const convolver = context.createConvolver();
      convolver.buffer = ir;
      convolver.normalize = false;
      const delay = context.createDelay();
      delay.delayTime.value = 512 / rate;
      source.connect(delay);
      delay.connect(convolver);
      const wet = context.createGain(),
        dry = context.createGain(),
        sum = context.createGain();
      wet.gain.value = value(p, "wetPercent", 100) / 100;
      dry.gain.value = 1 - wet.gain.value;
      sum.gain.value = gain(value(p, "gainDb", 0));
      delay.connect(dry);
      convolver.connect(wet);
      wet.connect(sum);
      dry.connect(sum);
      stage = sum;
    }
    stage.connect(context.destination);
    source.start();
    const result = await context.startRendering();
    return Array.from({ length: result.numberOfChannels }, (_, i) =>
      copy(result.getChannelData(i)),
    );
  }
  async function render(kind, p = {}, options = {}) {
    const unshifted = fixture(
      kind,
      kind === "pitch" ? { ...p, semitones: 0, cents: 0 } : p,
    );
    const data = kind === "pitch" ? fixture(kind, p) : unshifted;
    const inputs = unshifted.inputs,
      input = inputs[0].channels[0];
    let output = copy(input),
      channels;
    if (kind === "mixer")
      for (let i = 0; i < frames; i++)
        output[i] =
          (input[i] * value(p, "track1Percent", 80)) / 100 +
          (inputs[1].channels[0][i] * value(p, "track2Percent", 55)) / 100;
    else if (kind === "inputSwitch") {
      const selected = p.selected === "b" ? 1 : 0,
        from = options.switchFrom === "b" ? 1 : 0;
      for (let i = 0; i < frames; i++) {
        const progress =
          options.switchAt === undefined
            ? 1
            : clamp(
                (i / rate - options.switchAt) / (p.fade === "slow" ? 2 : 0.5),
                0,
                1,
              );
        const angle = (progress * Math.PI) / 2;
        output[i] =
          inputs[from].channels[0][i] * Math.cos(angle) +
          inputs[selected].channels[0][i] * Math.sin(angle);
        if (from === selected) output[i] = inputs[selected].channels[0][i];
      }
    } else if (kind === "duck") {
      let attenuation = 1,
        hold = 0;
      const key = inputs[1].channels[0],
        attack = Math.exp(-1 / ((rate * value(p, "attackMs", 20)) / 1000)),
        release = Math.exp(-1 / ((rate * value(p, "releaseMs", 500)) / 1000));
      for (let i = 0; i < frames; i++) {
        if (Math.abs(key[i]) >= gain(value(p, "thresholdDb", -35)))
          hold = (value(p, "holdMs", 300) * rate) / 1000;
        else hold = Math.max(0, hold - 1);
        const target = hold > 0 ? gain(-value(p, "amountDb", 6)) : 1;
        attenuation =
          target +
          (target < attenuation ? attack : release) * (attenuation - target);
        output[i] = input[i] * attenuation;
      }
    } else if (kind === "gain" || kind === "volume" || kind === "mute") {
      const amount =
        kind === "gain"
          ? gain(value(p, "gainDb", 0))
          : kind === "volume"
            ? value(p, "percent", 100) / 100
            : p.muted
              ? 0
              : 1;
      output = Float32Array.from(input, (v) => v * amount);
    } else if (kind === "compressor" || kind === "gate")
      output = dynamics(input, kind, p);
    else if (kind === "limiter") {
      const ceiling = gain(value(p, "ceilingDb", -1)),
        ahead = Math.round((rate * value(p, "lookaheadMs", 5)) / 1000),
        release = Math.exp(-1 / ((rate * value(p, "releaseMs", 100)) / 1000));
      let attenuation = 1;
      for (let i = 0; i < frames; i++) {
        const desired = Math.min(
          1,
          ceiling / Math.max(1e-9, Math.abs(input[i])),
        );
        attenuation =
          desired < attenuation
            ? desired
            : desired + release * (attenuation - desired);
        if (i + ahead < frames)
          output[i + ahead] = clamp(input[i] * attenuation, -ceiling, ceiling);
      }
      output.fill(0, 0, ahead);
    } else if (
      kind === "delay" ||
      kind === "networkSend" ||
      kind === "networkReceive"
    ) {
      const delay = Math.round(
        rate *
          (kind === "delay"
            ? value(p, "delayMs", 0) / 1000
            : 0.06 +
              (kind === "networkReceive"
                ? value(p, "bufferMs", 40) / 1000
                : 0)),
      );
      output.fill(0);
      for (let i = delay; i < frames; i++)
        if (kind === "delay" || i / rate >= 2.4) output[i] = input[i - delay];
    } else if (kind === "timeShift") {
      output.fill(0);
      for (let i = 0; i < frames; i++) {
        const t = i / rate;
        if (t < 2 || t >= 6) output[i] = input[i];
        else if (t >= 3)
          output[i] = input[Math.max(0, i - Math.floor(rate * 1.5))];
      }
    } else if (kind === "pitch") output = data.inputs[0].channels[0];
    else if (kind === "declick") {
      const cutoff = 0.2 + value(p, "thresholdPercent", 50) / 100;
      for (let i = 0; i < frames; i++)
        if (
          Math.abs(data.clicks[i]) > cutoff ||
          Math.abs(data.clicks[i - 1] || 0) > cutoff ||
          Math.abs(data.clicks[i - 2] || 0) > cutoff
        )
          output[i] = data.clean[i];
    } else if (["denoise", "speechDenoise", "spectralGate"].includes(kind)) {
      const learned = p.profileReady !== false;
      for (let i = 0; i < frames; i++) {
        let residual = 1,
          wanted = data.clean[i];
        if (kind === "speechDenoise")
          residual =
            1 -
            (value(p, "strengthPercent", 70) / 100) *
              (Math.abs(data.clean[i]) > 0.015 ? 0.72 : 0.95);
        else if (learned && kind === "denoise")
          residual = Math.max(
            value(p, "floorPercent", 10) / 100,
            1 - value(p, "reductionPercent", 70) / 100,
          );
        else if (learned) {
          residual = gain(-value(p, "reductionDb", 40));
          if (Math.abs(wanted) < 0.02 * gain(value(p, "thresholdDb", 3)))
            wanted *= residual;
        }
        output[i] = wanted + data.noise[i] * residual;
      }
    } else if (
      [
        "bassTreble",
        "graphicEq",
        "parametricEq",
        "dehum",
        "firFilter",
      ].includes(kind)
    )
      channels = await filtered(
        inputs[0].channels,
        kind,
        p,
        options.bands || [],
      );
    if (!channels)
      channels = kind === "meter" ? inputs[0].channels.map(copy) : [output];
    return {
      sampleRate: rate,
      duration: seconds,
      inputs,
      output: {
        label:
          kind === "duck"
            ? "Duck output · music only"
            : kind === "networkSend" || kind === "networkReceive"
              ? "PC B · received output"
              : "Tool output",
        channels,
      },
    };
  }
  function reading(channels, time, windowSeconds = 0.045) {
    return channels.map((samples) => {
      const start = Math.max(0, Math.floor(time * rate)),
        end = Math.min(
          samples.length,
          start + Math.round(windowSeconds * rate),
        );
      let peak = 0,
        sum = 0;
      for (let i = start; i < end; i++) {
        peak = Math.max(peak, Math.abs(samples[i]));
        sum += samples[i] ** 2;
      }
      return {
        peakDb: 20 * Math.log10(Math.max(1e-6, peak)),
        rmsDb:
          20 *
          Math.log10(Math.max(1e-6, Math.sqrt(sum / Math.max(1, end - start)))),
      };
    });
  }
  window.AudioRouterGuideSignal = Object.freeze({
    render,
    reading,
    sampleRate: rate,
    duration: seconds,
  });
})();
