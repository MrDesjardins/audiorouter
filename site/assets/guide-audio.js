/* One user-started transport for the samples shown by each tool lesson. */
(() => {
  "use strict";
  function mount(host, guide) {
    if (!host || !guide.audioRelevant) return null;
    let params = { ...guide.parameters },
      context = null,
      voice = null,
      snapshot = null,
      settleTimer = 0;
    let pending = null,
      revision = 0,
      operation = 0,
      dead = false,
      running = false;
    let offset = 0,
      startedAt = 0,
      audition = "output",
      takeDuration = 8,
      recorderMode = "input";
    let recordingState = "ready",
      switchOptions = {},
      initPromise = null;
    const listeners = new Set();
    const duration = () =>
      guide.kind === "recorder" && recorderMode === "file" ? takeDuration : 8;
    // Every lesson except Recorder repeats its example until Stop.
    const looping = () => guide.kind !== "recorder";
    const time = () => {
      if (!running || !context) return Math.min(duration(), offset);
      const elapsed = offset + context.currentTime - startedAt;
      return looping() ? elapsed % duration() : Math.min(duration(), elapsed);
    };
    const FADE = 0.03;
    const notify = () => listeners.forEach((fn) => fn(snapshot));
    const prepare = () => {
      const ticket = ++revision;
      pending = window.AudioRouterGuideSignal.render(guide.kind, params, {
        bands: guide.illustrationBands || [],
        ...switchOptions,
      }).then((next) => {
        if (dead || ticket !== revision) return;
        snapshot = next;
        host.dataset.signalReady = "true";
        // A setting changed while the example plays: hear it now, at the same
        // position, instead of stopping until Play is pressed again.
        if (running && looping()) startVoice(time(), true);
        notify();
      });
      return pending;
    };
    const selectedSignal = () =>
      audition === "output"
        ? snapshot.output
        : snapshot.inputs[Number(audition.slice(5))] || snapshot.output;
    const releaseVoice = (old, fade) => {
      if (!old) return;
      old.source.onended = null;
      const at = context.currentTime;
      try {
        if (fade) {
          old.gain.gain.setValueAtTime(old.gain.gain.value, at);
          old.gain.gain.linearRampToValueAtTime(0, at + FADE);
          old.source.stop(at + FADE + 0.01);
        } else old.source.stop();
      } catch {
        /* already completed */
      }
      setTimeout(
        () => {
          old.source.disconnect();
          old.gain.disconnect();
        },
        fade ? (FADE + 0.05) * 1000 : 0,
      );
    };
    const stopSource = () => {
      releaseVoice(voice, false);
      voice = null;
    };
    /** Start the selected signal at `seconds`; crossfade from any voice already playing. */
    function startVoice(seconds, crossfade) {
      const selected = selectedSignal();
      const buffer = context.createBuffer(
        selected.channels.length,
        selected.channels[0].length,
        snapshot.sampleRate,
      );
      selected.channels.forEach((samples, index) =>
        buffer.copyToChannel(samples, index),
      );
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.loop = looping();
      const gain = context.createGain();
      source.connect(gain);
      gain.connect(context.lessonOutput);
      const previous = voice;
      offset = Math.max(0, Math.min(Number(seconds) || 0, duration() - 0.001));
      startedAt = context.currentTime;
      if (crossfade && previous) {
        gain.gain.setValueAtTime(0, startedAt);
        gain.gain.linearRampToValueAtTime(1, startedAt + FADE);
      }
      releaseVoice(previous, crossfade);
      voice = { source, gain };
      const token = operation;
      source.onended = () => {
        if (dead || token !== operation || voice?.source !== source) return;
        offset = duration();
        running = false;
        host.dataset.audioState = "finished";
      };
      if (source.loop) source.start(startedAt, offset);
      else
        source.start(startedAt, offset, Math.max(0.001, duration() - offset));
    }
    async function init() {
      if (context) return;
      if (initPromise) return initPromise;
      initPromise = (async () => {
        const Constructor = window.AudioContext || window.webkitAudioContext;
        if (!Constructor)
          throw new Error("Web Audio is unavailable in this browser.");
        context = new Constructor();
        const level = context.createGain();
        // Bring synthesized examples to a useful laptop/headphone level while the
        // following compressor catches peaks from boosts and full-scale fixtures.
        level.gain.value = 0.5;
        const safety = context.createDynamicsCompressor();
        safety.threshold.value = -9;
        safety.ratio.value = 12;
        safety.attack.value = 0.003;
        safety.release.value = 0.1;
        level.connect(safety);
        safety.connect(context.destination);
        context.lessonOutput = level;
      })();
      return initPromise;
    }
    async function play(seconds = 0) {
      const token = ++operation;
      await pending;
      await init();
      if (dead || token !== operation || !snapshot) return;
      await context.resume();
      if (dead || token !== operation) return;
      stopSource();
      startVoice(seconds, false);
      running = true;
      host.dataset.audioState = "playing";
      host.dataset.audition = audition;
    }
    async function pause() {
      const token = ++operation;
      offset = time();
      running = false;
      stopSource();
      if (context && context.state === "running") await context.suspend();
      if (!dead && token === operation) host.dataset.audioState = "paused";
    }
    /** Stop playing and return to the start of the example. */
    async function stop() {
      await pause();
      offset = 0;
      if (!dead) host.dataset.audioState = "stopped";
    }
    async function seek(seconds, wasPlaying) {
      offset = Math.max(0, Math.min(Number(seconds) || 0, duration()));
      running = false;
      stopSource();
      if (wasPlaying && offset < duration()) await play(offset);
      else {
        operation++;
        host.dataset.audioState = "paused";
      }
    }
    function setParameters(next) {
      if (
        guide.kind === "inputSwitch" &&
        next.selected !== undefined &&
        next.selected !== params.selected
      ) {
        switchOptions = {
          switchFrom: params.selected || "a",
          switchAt: time(),
        };
        // Once the crossfade has finished, render the selected input for the
        // whole loop, so the next pass does not replay the old input.
        clearTimeout(settleTimer);
        settleTimer = setTimeout(
          () => {
            switchOptions = {};
            prepare();
          },
          ((next.fade ?? params.fade) === "slow" ? 2.2 : 0.7) * 1000,
        );
      }
      params = { ...params, ...next };
      return prepare();
    }
    async function setAudition(next) {
      const at = time(),
        wasRunning = running;
      audition = next;
      host.dataset.audition = next;
      if (wasRunning && snapshot && context) startVoice(at, true);
      else if (wasRunning) await play(at);
    }
    async function record() {
      recorderMode = "input";
      recordingState = "recording";
      takeDuration = 8;
      await play(0);
      host.dataset.recordingState = "recording";
    }
    async function stopRecording() {
      if (recordingState !== "recording") return;
      takeDuration = Math.max(0.25, Math.min(8, time()));
      recordingState = "saved";
      await pause();
      host.dataset.recordingState = "saved";
      notify();
    }
    async function playRecording() {
      if (recordingState !== "saved") return;
      recorderMode = "file";
      audition = "output";
      host.dataset.recordingState = "playing";
      await play(0);
    }
    function subscribe(fn) {
      listeners.add(fn);
      if (snapshot) fn(snapshot);
      return () => listeners.delete(fn);
    }
    async function destroy() {
      if (dead) return;
      dead = true;
      clearTimeout(settleTimer);
      operation++;
      revision++;
      running = false;
      stopSource();
      listeners.clear();
      host.dataset.audioState = "stopped";
      if (context) await context.close().catch(() => {});
    }
    prepare().catch(() => {
      host.dataset.audioState = "error";
    });
    return {
      play,
      pause,
      stop,
      seek,
      record,
      stopRecording,
      playRecording,
      setParameters,
      setAudition,
      subscribe,
      destroy,
      getSnapshot: () => snapshot,
      getTime: time,
      getDuration: duration,
      getRecorderDuration: () => takeDuration,
      isPlaying: () => running,
      getAudition: () => audition,
      getRecorderState: () => recordingState,
    };
  }
  window.AudioRouterGuideAudio = Object.freeze({ mount });
})();
