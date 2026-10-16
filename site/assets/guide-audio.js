/* One user-started transport for the samples shown by each tool lesson. */
(() => {
  "use strict";
  function mount(host, guide) {
    if (!host || !guide.audioRelevant) return null;
    let params = { ...guide.parameters },
      context = null,
      source = null,
      snapshot = null;
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
    const time = () =>
      Math.min(
        duration(),
        running && context ? offset + context.currentTime - startedAt : offset,
      );
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
        notify();
      });
      return pending;
    };
    const stopSource = () => {
      if (source) {
        source.onended = null;
        try {
          source.stop();
        } catch {
          /* already completed */
        }
        source.disconnect();
        source = null;
      }
    };
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
      const selected =
        audition === "output"
          ? snapshot.output
          : snapshot.inputs[Number(audition.slice(5))] || snapshot.output;
      const buffer = context.createBuffer(
        selected.channels.length,
        selected.channels[0].length,
        snapshot.sampleRate,
      );
      selected.channels.forEach((samples, index) =>
        buffer.copyToChannel(samples, index),
      );
      source = context.createBufferSource();
      source.buffer = buffer;
      source.connect(context.lessonOutput);
      offset = Math.max(0, Math.min(Number(seconds) || 0, duration() - 0.001));
      startedAt = context.currentTime;
      running = true;
      source.onended = () => {
        if (dead || token !== operation) return;
        offset = duration();
        running = false;
        host.dataset.audioState = "finished";
      };
      source.start(
        context.currentTime,
        offset,
        Math.max(0.001, duration() - offset),
      );
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
      )
        switchOptions = {
          switchFrom: params.selected || "a",
          switchAt: time(),
        };
      params = { ...params, ...next };
      return prepare();
    }
    async function setAudition(next) {
      const at = time(),
        wasRunning = running;
      audition = next;
      host.dataset.audition = next;
      if (wasRunning) await play(at);
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
