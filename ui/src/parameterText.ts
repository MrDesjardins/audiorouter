/**
 * Human labels and one-line explanations for built-in tool settings. The
 * backend names parameters for machines (`wetPercent`); the Properties panel
 * shows these instead. The unit is displayed separately, so labels omit it.
 */
export type ParameterText = { label: string; help: string };

const GRAPHIC_EQ_BANDS = [
  "31.5 Hz",
  "63 Hz",
  "125 Hz",
  "250 Hz",
  "500 Hz",
  "1 kHz",
  "2 kHz",
  "4 kHz",
  "8 kHz",
  "16 kHz",
];

const TEXT: Record<string, Record<string, ParameterText>> = {
  gain: {
    gainDb: {
      label: "Gain",
      help: "Makes the sound louder (positive) or quieter (negative). 0 dB leaves it unchanged.",
    },
  },
  volume: {
    percent: { label: "Volume", help: "100 % leaves the sound unchanged, 0 % silences it, 200 % doubles it." },
  },
  bassTreble: {
    bassDb: { label: "Bass", help: "Boosts or cuts voice warmth and the low end. 0 dB leaves it unchanged." },
    trebleDb: { label: "Treble", help: "Boosts or cuts voice brightness and articulation. 0 dB leaves it unchanged." },
    bassFrequencyHz: {
      label: "Bass frequency",
      help: "Raise this frequency to affect more of your voice. Default: 500 Hz. Original tuning: 120 Hz.",
    },
    trebleFrequencyHz: {
      label: "Treble frequency",
      help: "Lower this frequency to affect more of your voice. Default: 1500 Hz. Original tuning: 6000 Hz.",
    },
  },
  testSignal: {
    frequencyHz: { label: "Tone frequency", help: "Pitch of the test tone. 440 Hz is the A above middle C." },
    levelDb: {
      label: "Level",
      help: "Loudness of the tone; 0 dBFS is the loudest possible. Start low, for example −18 dBFS.",
    },
    durationMs: { label: "Duration", help: "How long the tone plays each time you press its Play button." },
  },
  mute: {
    muted: { label: "Muted", help: "Silences everything passing through this node." },
  },
  compressor: {
    thresholdDb: {
      label: "Threshold",
      help: "Sound louder than this is turned down, evening out loud and quiet moments.",
    },
    ratio: {
      label: "Ratio",
      help: "How strongly sound above the threshold is reduced: at 4, 4 dB over the threshold comes out 1 dB over.",
    },
    attackMs: { label: "Attack", help: "How quickly compression starts when the sound gets loud." },
    releaseMs: { label: "Release", help: "How quickly compression lets go when the sound gets quieter." },
    kneeDb: { label: "Knee", help: "Softens the change around the threshold. 0 dB is an abrupt change." },
    makeupDb: { label: "Makeup gain", help: "Adds level back after compression so the result is not quieter overall." },
  },
  gate: {
    thresholdDb: {
      label: "Threshold",
      help: "Sound quieter than this is turned down, for example background noise between words.",
    },
    rangeDb: {
      label: "Range",
      help: "How much quiet sound is turned down. 80 dB is nearly silent; lower values sound more natural.",
    },
    hysteresisDb: {
      label: "Hysteresis",
      help: "The gate closes only once the sound falls this far below the threshold, which prevents rapid on/off chatter.",
    },
    ratio: {
      label: "Ratio",
      help: "How steeply sound below the threshold is reduced. Higher values act more like an on/off switch.",
    },
    attackMs: { label: "Attack", help: "How quickly the gate opens when you start speaking." },
    holdMs: {
      label: "Hold",
      help: "How long the gate stays open after the sound drops, so word endings are not cut off.",
    },
    releaseMs: { label: "Release", help: "How quickly the gate closes after the hold time." },
  },
  limiter: {
    ceilingDb: {
      label: "Ceiling",
      help: "The loudest level allowed out. Peaks above it are held down so they never clip.",
    },
    lookaheadMs: {
      label: "Look-ahead",
      help: "Delays the sound slightly so sudden peaks are caught before they happen.",
    },
    releaseMs: { label: "Release", help: "How quickly the limiter stops reducing after a peak." },
  },
  delay: {
    delayMs: { label: "Delay", help: "Holds the sound back by this time, for example to line audio up with video." },
  },
  pitch: {
    semitones: { label: "Semitones", help: "Shifts the pitch in musical steps; 12 semitones is one octave." },
    cents: { label: "Fine tune", help: "Small pitch adjustment; 100 cents make one semitone." },
  },
  dehum: {
    frequencyHz: {
      label: "Mains frequency",
      help: "Base frequency of the hum: 50 Hz in most of Europe, Asia, Africa and Australia, 60 Hz in North America.",
    },
    amountPercent: { label: "Amount", help: "How strongly the hum is removed." },
    harmonics: {
      label: "Harmonics",
      help: "How many multiples of the hum frequency (for example 100 Hz, 150 Hz) are also removed.",
    },
  },
  declick: {
    thresholdPercent: {
      label: "Threshold",
      help: "How much a spike must stand out to count as a click. Lower values remove more clicks but can soften consonants.",
    },
  },
  denoise: {
    reductionPercent: { label: "Reduction", help: "How strongly the learned noise is removed." },
    floorPercent: {
      label: "Noise left in",
      help: "How much of the noise stays. 0 % removes all of it, which can sound unnatural.",
    },
    learning: { label: "Learning", help: "On while the tool measures the noise to remove. Use Learn noise above." },
  },
  speechDenoise: {
    strengthPercent: { label: "Strength", help: "How strongly background noise around speech is reduced." },
  },
  firFilter: {
    wetPercent: {
      label: "Wet mix",
      help: "How much filtered (“wet”) sound you hear. 100 % is only the filtered sound, 0 % only the original (“dry”) sound; values in between blend them.",
    },
    gainDb: {
      label: "Output gain",
      help: "Adjusts the level after filtering, for example to match the original loudness.",
    },
  },
  spectralGate: {
    thresholdDb: {
      label: "Threshold above noise",
      help: "How much louder than the learned noise a frequency must be to pass. Raise it if noise still comes through; lower it if your voice sounds thin.",
    },
    reductionDb: { label: "Reduction", help: "How much blocked frequencies are turned down. 80 dB is nearly silent." },
    learning: { label: "Learning", help: "On while the tool measures the noise. Use Learn noise above." },
  },
  timeShift: {
    bufferSeconds: {
      label: "Rewind buffer",
      help: "How many seconds of recent audio are kept so you can jump back and replay them.",
    },
  },
  recorder: {
    format: {
      label: "File format",
      help: "WAV 24-bit keeps full quality for editing; MP3 makes small files to share.",
    },
    autoRecord: {
      label: "Record automatically when Play starts",
      help: "Every Play starts a new recording; Stop saves it.",
    },
    splitMinutes: {
      label: "New file every",
      help: "Start a new file after this many minutes so long sessions stay manageable. 0 keeps one file per take.",
    },
  },
  duck: {
    keyNodeId: {
      label: "Triggered by",
      help: "The source or tool whose level turns this audio down, usually your microphone or voice chain.",
    },
    thresholdDb: {
      label: "Trigger level",
      help: "Ducking starts when the trigger is louder than this. Set it between your room noise and your voice.",
    },
    amountDb: {
      label: "Amount",
      help: "How far this audio is turned down while the trigger is active. 6 dB is about half as loud.",
    },
    attackMs: { label: "Attack", help: "How quickly the audio goes down when you start talking." },
    holdMs: {
      label: "Hold",
      help: "How long the audio stays down after the trigger falls, so short pauses between words do not pump.",
    },
    releaseMs: { label: "Release", help: "How quickly the audio comes back after the hold time." },
  },
  inputSwitch: {
    selected: { label: "Active input", help: "Which input you hear: A or B." },
    fade: { label: "Switch speed", help: "Normal changes over half a second; Slow fades over two seconds." },
  },
};

const OPTION_LABELS: Record<string, string> = {
  wavPcm24: "WAV 24-bit (best for editing)",
  wavPcm16: "WAV 16-bit",
  wavFloat32: "WAV 32-bit float",
  flac24: "FLAC 24-bit (smaller, lossless)",
  flac16: "FLAC 16-bit",
  mp3: "MP3 (small, to share)",
  a: "Input A",
  b: "Input B",
  normal: "Normal",
  slow: "Slow",
};

/** Turn a machine name such as `wetPercent` into "Wet percent". */
export function humanizeParameterName(name: string): string {
  const words = name
    .replace(/(Percent|Db|Hz|Ms|Seconds)$/, "")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([a-zA-Z])(\d)/g, "$1 $2")
    .toLowerCase()
    .trim();
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : name;
}

/** Label and explanation for a tool setting, with a readable fallback. */
export function parameterText(kind: string, name: string): ParameterText {
  const known = TEXT[kind]?.[name];
  if (known) return known;
  const band = /^band(\d+)Db$/.exec(name);
  if (kind === "graphicEq" && band && GRAPHIC_EQ_BANDS[Number(band[1])]) {
    return {
      label: GRAPHIC_EQ_BANDS[Number(band[1])],
      help: `Boosts or cuts the sound around ${GRAPHIC_EQ_BANDS[Number(band[1])]}.`,
    };
  }
  return { label: humanizeParameterName(name), help: "" };
}

/** Display text for an option of a choice setting. */
export function optionLabel(option: string): string {
  return OPTION_LABELS[option] ?? humanizeParameterName(option);
}

/** Readable value with its unit, for the caption readout: "+3.0 dB", "1.5 kHz", "1.2 s". */
export function formatParameterValue(value: number, unit: string | undefined, step = 0.1): string {
  if (!Number.isFinite(value)) return "—";
  const digits = step >= 1 ? 0 : step >= 0.1 ? 1 : 2;
  const signed = (text: string) => (value > 0 ? `+${text}` : value < 0 ? `−${text.replace("-", "")}` : text);
  switch (unit) {
    case "dB":
    case "dBFS":
      return `${signed(value.toFixed(1))} ${unit}`;
    case "semitones":
      return `${signed(value.toFixed(digits))} st`;
    case "cents":
      return `${signed(value.toFixed(0))} ct`;
    case "Hz":
      return value >= 1000
        ? `${(value / 1000).toFixed(value >= 10000 ? 1 : 2).replace(/\.?0+$/, "")} kHz`
        : `${value.toFixed(digits)} Hz`;
    case "ms":
      return value >= 1000
        ? `${(value / 1000).toFixed(2).replace(/\.?0+$/, "")} s`
        : `${value.toFixed(value < 10 && step < 1 ? 1 : 0)} ms`;
    case "%":
      return `${value.toFixed(digits)} %`;
    case "s":
      return `${value.toFixed(digits)} s`;
    case undefined:
    case "":
      return value.toFixed(digits);
    default:
      return `${value.toFixed(digits)} ${unit}`;
  }
}
