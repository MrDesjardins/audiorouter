/**
 * Human labels and one-line explanations for built-in tool settings. The
 * backend names parameters for machines (`wetPercent`); the Properties panel
 * shows these instead. The unit is displayed separately, so labels omit it.
 */
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

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

const TEXT: Record<string, Record<string, { label: MessageKey; help: MessageKey }>> = {
  gain: {
    gainDb: {
      label: "parameterUi.labels.gain",
      help: "parameterUi.gainHelp",
    },
  },
  volume: {
    percent: { label: "parameterUi.labels.volume", help: "parameterUi.volumeHelp" },
  },
  bassTreble: {
    bassDb: { label: "parameterUi.labels.bass", help: "parameterUi.help.bassTreble.bassDb" },
    trebleDb: { label: "parameterUi.labels.treble", help: "parameterUi.help.bassTreble.trebleDb" },
    bassFrequencyHz: {
      label: "parameterUi.labels.bassFrequency",
      help: "parameterUi.help.bassTreble.bassFrequencyHz",
    },
    trebleFrequencyHz: {
      label: "parameterUi.labels.trebleFrequency",
      help: "parameterUi.help.bassTreble.trebleFrequencyHz",
    },
  },
  testSignal: {
    frequencyHz: { label: "parameterUi.labels.toneFrequency", help: "parameterUi.help.testSignal.frequencyHz" },
    levelDb: {
      label: "parameterUi.labels.level",
      help: "parameterUi.help.testSignal.levelDb",
    },
    durationMs: { label: "parameterUi.labels.duration", help: "parameterUi.help.testSignal.durationMs" },
  },
  mute: {
    muted: { label: "parameterUi.labels.muted", help: "parameterUi.help.mute.muted" },
  },
  compressor: {
    thresholdDb: {
      label: "parameterUi.labels.threshold",
      help: "parameterUi.help.compressor.thresholdDb",
    },
    ratio: {
      label: "parameterUi.labels.ratio",
      help: "parameterUi.help.compressor.ratio",
    },
    attackMs: { label: "parameterUi.labels.attack", help: "parameterUi.help.compressor.attackMs" },
    releaseMs: { label: "parameterUi.labels.release", help: "parameterUi.help.compressor.releaseMs" },
    kneeDb: { label: "parameterUi.labels.knee", help: "parameterUi.help.compressor.kneeDb" },
    makeupDb: { label: "parameterUi.labels.makeupGain", help: "parameterUi.help.compressor.makeupDb" },
  },
  gate: {
    thresholdDb: {
      label: "parameterUi.labels.threshold",
      help: "parameterUi.help.gate.thresholdDb",
    },
    rangeDb: {
      label: "parameterUi.labels.range",
      help: "parameterUi.help.gate.rangeDb",
    },
    hysteresisDb: {
      label: "parameterUi.labels.hysteresis",
      help: "parameterUi.help.gate.hysteresisDb",
    },
    ratio: {
      label: "parameterUi.labels.ratio",
      help: "parameterUi.help.gate.ratio",
    },
    attackMs: { label: "parameterUi.labels.attack", help: "parameterUi.help.gate.attackMs" },
    holdMs: {
      label: "parameterUi.labels.hold",
      help: "parameterUi.help.gate.holdMs",
    },
    releaseMs: { label: "parameterUi.labels.release", help: "parameterUi.help.gate.releaseMs" },
  },
  limiter: {
    ceilingDb: {
      label: "parameterUi.labels.ceiling",
      help: "parameterUi.help.limiter.ceilingDb",
    },
    lookaheadMs: {
      label: "parameterUi.labels.lookAhead",
      help: "parameterUi.help.limiter.lookaheadMs",
    },
    releaseMs: { label: "parameterUi.labels.release", help: "parameterUi.help.limiter.releaseMs" },
  },
  delay: {
    delayMs: { label: "parameterUi.labels.delay", help: "parameterUi.help.delay.delayMs" },
  },
  pitch: {
    semitones: { label: "parameterUi.labels.semitones", help: "parameterUi.help.pitch.semitones" },
    cents: { label: "parameterUi.labels.fineTune", help: "parameterUi.help.pitch.cents" },
  },
  dehum: {
    frequencyHz: {
      label: "parameterUi.labels.mainsFrequency",
      help: "parameterUi.help.dehum.frequencyHz",
    },
    amountPercent: { label: "parameterUi.labels.amount", help: "parameterUi.help.dehum.amountPercent" },
    harmonics: {
      label: "parameterUi.labels.harmonics",
      help: "parameterUi.help.dehum.harmonics",
    },
  },
  declick: {
    thresholdPercent: {
      label: "parameterUi.labels.threshold",
      help: "parameterUi.help.declick.thresholdPercent",
    },
  },
  denoise: {
    reductionPercent: { label: "parameterUi.labels.reduction", help: "parameterUi.help.denoise.reductionPercent" },
    floorPercent: {
      label: "parameterUi.labels.noiseLeftIn",
      help: "parameterUi.help.denoise.floorPercent",
    },
    learning: { label: "parameterUi.labels.learning", help: "parameterUi.help.denoise.learning" },
  },
  speechDenoise: {
    strengthPercent: { label: "parameterUi.labels.strength", help: "parameterUi.help.speechDenoise.strengthPercent" },
  },
  firFilter: {
    wetPercent: {
      label: "parameterUi.labels.wetMix",
      help: "parameterUi.help.firFilter.wetPercent",
    },
    gainDb: {
      label: "parameterUi.labels.outputGain",
      help: "parameterUi.help.firFilter.gainDb",
    },
  },
  spectralGate: {
    thresholdDb: {
      label: "parameterUi.labels.thresholdAboveNoise",
      help: "parameterUi.help.spectralGate.thresholdDb",
    },
    reductionDb: { label: "parameterUi.labels.reduction", help: "parameterUi.help.spectralGate.reductionDb" },
    learning: { label: "parameterUi.labels.learning", help: "parameterUi.help.spectralGate.learning" },
  },
  timeShift: {
    bufferSeconds: {
      label: "parameterUi.labels.rewindBuffer",
      help: "parameterUi.help.timeShift.bufferSeconds",
    },
  },
  recorder: {
    format: {
      label: "parameterUi.labels.fileFormat",
      help: "parameterUi.help.recorder.format",
    },
    autoRecord: {
      label: "parameterUi.labels.recordAutomaticallyWhenPlayStarts",
      help: "parameterUi.help.recorder.autoRecord",
    },
    splitMinutes: {
      label: "parameterUi.labels.newFileEvery",
      help: "parameterUi.help.recorder.splitMinutes",
    },
  },
  duck: {
    keyNodeId: {
      label: "parameterUi.labels.triggeredBy",
      help: "parameterUi.help.duck.keyNodeId",
    },
    thresholdDb: {
      label: "parameterUi.labels.triggerLevel",
      help: "parameterUi.help.duck.thresholdDb",
    },
    amountDb: {
      label: "parameterUi.labels.amount",
      help: "parameterUi.help.duck.amountDb",
    },
    attackMs: { label: "parameterUi.labels.attack", help: "parameterUi.help.duck.attackMs" },
    holdMs: {
      label: "parameterUi.labels.hold",
      help: "parameterUi.help.duck.holdMs",
    },
    releaseMs: { label: "parameterUi.labels.release", help: "parameterUi.help.duck.releaseMs" },
  },
  funVoiceChanger: {
    preset: { label: "parameterUi.labels.preset", help: "parameterUi.help.funVoiceChanger.preset" },
    gateDb: {
      label: "parameterUi.labels.noiseGate",
      help: "parameterUi.help.funVoiceChanger.gateDb",
    },
    correctEnabled: {
      label: "parameterUi.labels.pitchCorrection",
      help: "parameterUi.help.funVoiceChanger.correctEnabled",
    },
    key: { label: "parameterUi.labels.key", help: "parameterUi.help.funVoiceChanger.key" },
    scale: {
      label: "parameterUi.labels.scale",
      help: "parameterUi.help.funVoiceChanger.scale",
    },
    retuneMs: {
      label: "parameterUi.labels.retuneSpeed",
      help: "parameterUi.help.funVoiceChanger.retuneMs",
    },
    humanizePercent: {
      label: "parameterUi.labels.humanize",
      help: "parameterUi.help.funVoiceChanger.humanizePercent",
    },
    shiftEnabled: {
      label: "parameterUi.labels.pitchAndFormant",
      help: "parameterUi.help.funVoiceChanger.shiftEnabled",
    },
    pitchSemitones: { label: "parameterUi.labels.pitch", help: "parameterUi.help.funVoiceChanger.pitchSemitones" },
    pitchCents: { label: "parameterUi.labels.finePitch", help: "parameterUi.help.funVoiceChanger.pitchCents" },
    formantSemitones: {
      label: "parameterUi.labels.formant",
      help: "parameterUi.help.funVoiceChanger.formantSemitones",
    },
    ringEnabled: {
      label: "parameterUi.labels.ringModulator",
      help: "parameterUi.help.funVoiceChanger.ringEnabled",
    },
    ringHz: { label: "parameterUi.labels.ringFrequency", help: "parameterUi.help.funVoiceChanger.ringHz" },
    ringDepthPercent: {
      label: "parameterUi.labels.ringDepth",
      help: "parameterUi.help.funVoiceChanger.ringDepthPercent",
    },
    crushEnabled: { label: "parameterUi.labels.bitCrusher", help: "parameterUi.help.funVoiceChanger.crushEnabled" },
    crushBits: { label: "parameterUi.labels.bits", help: "parameterUi.help.funVoiceChanger.crushBits" },
    crushDownsample: {
      label: "parameterUi.labels.downsample",
      help: "parameterUi.help.funVoiceChanger.crushDownsample",
    },
    driveEnabled: { label: "parameterUi.labels.saturation", help: "parameterUi.help.funVoiceChanger.driveEnabled" },
    drivePercent: { label: "parameterUi.labels.drive", help: "parameterUi.help.funVoiceChanger.drivePercent" },
    whisperPercent: {
      label: "parameterUi.labels.whisper",
      help: "parameterUi.help.funVoiceChanger.whisperPercent",
    },
    toneEnabled: { label: "parameterUi.labels.tone", help: "parameterUi.help.funVoiceChanger.toneEnabled" },
    lowCutHz: { label: "parameterUi.labels.lowCut", help: "parameterUi.help.funVoiceChanger.lowCutHz" },
    highCutHz: { label: "parameterUi.labels.highCut", help: "parameterUi.help.funVoiceChanger.highCutHz" },
    peakHz: { label: "parameterUi.labels.peakFrequency", help: "parameterUi.help.funVoiceChanger.peakHz" },
    peakDb: { label: "parameterUi.labels.peak", help: "parameterUi.help.funVoiceChanger.peakDb" },
    modEnabled: { label: "parameterUi.labels.modulation", help: "parameterUi.help.funVoiceChanger.modEnabled" },
    modType: { label: "parameterUi.labels.modulationType", help: "parameterUi.help.funVoiceChanger.modType" },
    modRateHz: { label: "parameterUi.labels.rate", help: "parameterUi.help.funVoiceChanger.modRateHz" },
    modDepthPercent: { label: "parameterUi.labels.depth", help: "parameterUi.help.funVoiceChanger.modDepthPercent" },
    modFeedbackPercent: {
      label: "parameterUi.labels.feedback",
      help: "parameterUi.help.funVoiceChanger.modFeedbackPercent",
    },
    spaceEnabled: { label: "parameterUi.labels.space", help: "parameterUi.help.funVoiceChanger.spaceEnabled" },
    spaceSizePercent: {
      label: "parameterUi.labels.roomSize",
      help: "parameterUi.help.funVoiceChanger.spaceSizePercent",
    },
    spaceDecayMs: { label: "parameterUi.labels.echoLength", help: "parameterUi.help.funVoiceChanger.spaceDecayMs" },
    spaceLevelPercent: {
      label: "parameterUi.labels.echoLevel",
      help: "parameterUi.help.funVoiceChanger.spaceLevelPercent",
    },
    mixPercent: { label: "parameterUi.labels.mix", help: "parameterUi.help.funVoiceChanger.mixPercent" },
    outputDb: { label: "parameterUi.labels.output", help: "parameterUi.help.funVoiceChanger.outputDb" },
  },
  inputSwitch: {
    selected: { label: "parameterUi.labels.activeInput", help: "parameterUi.help.inputSwitch.selected" },
    fade: { label: "parameterUi.labels.switchSpeed", help: "parameterUi.help.inputSwitch.fade" },
  },
  outputSwitch: {
    selected: { label: "parameterUi.labels.activeOutput", help: "parameterUi.help.outputSwitch.selected" },
  },
};

const OPTION_LABELS: Record<string, MessageKey> = {
  wavPcm24: "parameterUi.wavPcm24",
  wavPcm16: "parameterUi.wavPcm16",
  wavFloat32: "parameterUi.wavFloat32",
  flac24: "parameterUi.flac24",
  flac16: "parameterUi.flac16",
  mp3: "parameterUi.mp3",
  a: "parameterUi.inputA",
  b: "parameterUi.inputB",
  normal: "parameterUi.normal",
  slow: "parameterUi.slow",
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
  if (known) return { label: tx(known.label), help: tx(known.help) };
  const band = /^band(\d+)Db$/.exec(name);
  if (kind === "graphicEq" && band && GRAPHIC_EQ_BANDS[Number(band[1])]) {
    return {
      label: GRAPHIC_EQ_BANDS[Number(band[1])],
      help: tx("parameterUi.bandHelp", { frequency: GRAPHIC_EQ_BANDS[Number(band[1])] }),
    };
  }
  return { label: humanizeParameterName(name), help: "" };
}

/** Display text for an option of a choice setting. */
export function optionLabel(option: string): string {
  const key = Object.hasOwn(OPTION_LABELS, option) ? OPTION_LABELS[option] : undefined;
  return key ? tx(key) : humanizeParameterName(option);
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
